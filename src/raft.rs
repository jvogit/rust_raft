use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};

use crate::rpc::{AppendEntriesRes, RPCConfig, RequestVoteRes, RPC};

#[derive(PartialEq, Eq)]
enum ServerState {
    Follower,
    Candidate,
    Leader,
}

pub struct Server<T> {
    // Server metadata
    id: usize,
    config: RPCConfig<T>,
    state: ServerState,
    election_votes: HashSet<usize>,
    append_entries_count: HashMap<usize, usize>,
    // RAFT variables
    // Stable
    current_term: usize,
    voted_for: Option<usize>,
    log: Vec<(usize, T)>,
    // Volatile
    commit_index: usize,
    last_applied: usize,
    // Volatile for leaders
    next_index: Vec<usize>,
    match_index: Vec<usize>,
}

impl<T> Server<T>
where
    T: Send + 'static,
{
    pub fn new<F>(id: usize, config: RPCConfig<T>, dummy: F) -> Server<T>
    where
        F: FnOnce() -> T,
    {
        Server {
            id,
            state: ServerState::Follower,
            election_votes: HashSet::new(),
            append_entries_count: HashMap::new(),
            current_term: 0,
            voted_for: Option::None,
            // dummy vaue at index 0. Real entries begin at index 1
            log: vec![(0, dummy())],
            commit_index: 0,
            last_applied: 0,
            next_index: vec![1; config.connections_len()],
            match_index: vec![0; config.connections_len()],
            config,
        }
    }

    pub fn start_thread(mut self, recv: Receiver<RPC<T>>) -> JoinHandle<()> {
        thread::spawn(move || loop {
            match self.state {
                ServerState::Follower => self.handle_follower(&recv),
                ServerState::Candidate => self.handle_candidate(&recv),
                ServerState::Leader => self.handle_leader(&recv),
            }
        })
    }

    fn handle_follower(&mut self, recv: &Receiver<RPC<T>>) {
        match recv.recv_timeout(self.config.election_timeout) {
            Ok(rpc) => self.handle_rpc(rpc),
            Err(mpsc::RecvTimeoutError::Timeout) => self.handle_timeout(),
            Err(_) => return,
        }
    }

    fn handle_candidate(&mut self, recv: &Receiver<RPC<T>>) {
        match recv.recv_timeout(self.config.election_timeout) {
            Ok(rpc) => self.handle_rpc(rpc),
            Err(mpsc::RecvTimeoutError::Timeout) => self.handle_timeout(),
            Err(_) => return,
        }
    }

    fn handle_leader(&mut self, recv: &Receiver<RPC<T>>) {}

    fn handle_rpc(&mut self, rpc: RPC<T>) {
        match rpc {
            RPC::AppendEntries(args) => {
                let sender = self.config.get_connection(args.leader_id).unwrap().clone();
                self.handle_term_conversion_to_follower(args.term);

                if args.term < self.current_term {
                    sender
                        .send(RPC::AppendEntriesRes(AppendEntriesRes {
                            id: self.id,
                            term: self.current_term,
                            success: false,
                            replicated_index: self.log.len() - 1,
                        }))
                        .expect("Sender to not fail!");
                    return;
                }

                if let None = self.log.get(args.prev_log_index) {
                    sender
                        .send(RPC::AppendEntriesRes(AppendEntriesRes {
                            id: self.id,
                            term: self.current_term,
                            success: false,
                            replicated_index: self.log.len() - 1,
                        }))
                        .expect("Sender to not fail!");
                    return;
                }

                let (prev_log_term, _) = self.log.get(args.prev_log_index).unwrap();
                if *prev_log_term != args.prev_log_term {
                    sender
                        .send(RPC::AppendEntriesRes(AppendEntriesRes {
                            id: self.id,
                            term: self.current_term,
                            success: false,
                            replicated_index: self.log.len() - 1,
                        }))
                        .expect("Sender to not fail!");
                    return;
                }

                // Replacing existing entries or append
                for (i, entry) in args.entries.into_iter().enumerate() {
                    let index = i + args.prev_log_index + 1;
                    if index < self.log.len() {
                        self.log[index] = (args.term, entry);
                    } else {
                        self.log.push((args.term, entry));
                    }
                }

                if args.leader_commit > self.commit_index {
                    self.commit_index = min(args.leader_commit, self.log.len() - 1);
                    self.apply_log_entries();
                }

                sender
                    .send(RPC::AppendEntriesRes(AppendEntriesRes {
                        id: self.id,
                        term: self.current_term,
                        success: true,
                        replicated_index: self.log.len() - 1,
                    }))
                    .expect("Sender to not fail!");
            }
            RPC::RequestVote(args) => {
                let sender = self
                    .config
                    .get_connection(args.candidate_id)
                    .unwrap()
                    .clone();
                self.handle_term_conversion_to_follower(args.term);

                if args.term < self.current_term {
                    sender
                        .send(RPC::RequestVoteRes(RequestVoteRes {
                            id: self.id,
                            term: self.current_term,
                            vote_granted: false,
                        }))
                        .expect("Sender to not fail!");
                    return;
                }

                let vote_granted = self.voted_for.is_none()
                    || (args.last_log_index >= self.log.len()
                        || (args.last_log_index == self.log.len() - 1
                            && args.last_log_term >= self.log.last().unwrap().0));
                if vote_granted {
                    self.voted_for = Some(args.candidate_id);
                }

                sender
                    .send(RPC::RequestVoteRes(RequestVoteRes {
                        id: self.id,
                        term: self.current_term,
                        vote_granted,
                    }))
                    .expect("Sender to not fail!");
            }
            RPC::AppendEntriesRes(res) => {
                if self.state != ServerState::Leader
                    || self.handle_term_conversion_to_follower(res.term)
                {
                    return;
                }

                if !res.success {
                    // TODO: decrement next index and try again
                    self.next_index[res.id] -= 1;
                    todo!();
                }

                self.next_index[res.id] = max(self.next_index[res.id], res.replicated_index + 1);
                self.match_index[res.id] = max(self.match_index[res.id], res.replicated_index);

                // Check if exists N > commitIndex where majority of matchIndex[i] >= N for all servers i and log[N].term == currentTerm -> set commitIndex to N
                let num_of_replicated = self
                    .match_index
                    .iter()
                    .filter(|&&i| i >= res.replicated_index)
                    .count();
                let num_needed = self.config.connections_len() / 2 + 1;

                if num_of_replicated >= num_needed {
                    self.commit_index = max(self.commit_index, res.replicated_index);
                    self.apply_log_entries();
                }
            }
            RPC::RequestVoteRes(res) => {
                if self.state != ServerState::Candidate
                    || self.handle_term_conversion_to_follower(res.term)
                    || !res.vote_granted
                {
                    return;
                }

                self.election_votes.insert(res.id);

                // TODO: Check if elected as leader then promote
                todo!()
            }
        }
    }

    fn handle_timeout(&self) {
        todo!()
    }

    fn handle_term_conversion_to_follower(&mut self, term: usize) -> bool {
        if term > self.current_term {
            self.current_term = term;
            self.state = ServerState::Follower;

            return true;
        }

        false
    }

    fn apply_log_entries(&mut self) {
        while self.commit_index > self.last_applied {
            self.last_applied += 1;
            // TODO: apply log[self.last_applied] to state machine
            todo!()
        }
    }
}
