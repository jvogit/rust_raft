use std::{
    cmp::{max, min},
    collections::HashSet,
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::rpc::{
    AppendEntries, AppendEntriesRes, ClientResponse, RPCConfig, RequestVote, RequestVoteRes, RPC,
};

#[derive(PartialEq, Eq)]
enum ServerState {
    Follower,
    Candidate,
    Leader,
}

pub struct Server<T>
where
    T: Clone,
{
    // Server metadata
    id: usize,
    config: RPCConfig<T>,
    state: ServerState,
    election_votes: HashSet<usize>,
    leader_id: Option<usize>,
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
    T: Clone + Send + 'static,
{
    pub fn new<F>(id: usize, config: RPCConfig<T>, dummy: F) -> Server<T>
    where
        F: FnOnce() -> T,
    {
        Server {
            id,
            state: ServerState::Follower,
            election_votes: HashSet::new(),
            leader_id: None,
            current_term: 0,
            voted_for: Option::None,
            // dummy vaue at index 0. Real entries begin at index 1
            log: vec![(0, dummy())],
            commit_index: 0,
            last_applied: 0,
            next_index: vec![1; config.connections.len()],
            match_index: vec![0; config.connections.len()],
            config,
        }
    }

    pub fn start_thread(mut self, recv: Receiver<RPC<T>>) -> JoinHandle<()> {
        // If id is 0 handle timeout immediately to elect itself leader first!
        if self.id == 0 {
            self.handle_timeout();
        }
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

    fn handle_leader(&mut self, recv: &Receiver<RPC<T>>) {
        match recv.recv_timeout(self.config.election_timeout - Duration::from_millis(500)) {
            Ok(rpc) => self.handle_rpc(rpc),
            Err(mpsc::RecvTimeoutError::Timeout) => self.handle_timeout(),
            Err(_) => return,
        }
    }

    fn handle_rpc(&mut self, rpc: RPC<T>) {
        match rpc {
            RPC::AppendEntries(args) => {
                let sender = &self.config.connections[&args.leader_id].clone();
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
                        self.log[index] = entry;
                    } else {
                        self.log.push(entry);
                    }
                }

                if args.leader_commit > self.commit_index {
                    self.commit_index = min(args.leader_commit, self.log.len() - 1);
                    self.apply_log_entries();
                }

                self.leader_id = Some(args.leader_id);

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
                let sender = &self.config.connections[&args.candidate_id].clone();
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

                let vote_granted = (self.voted_for.is_none()
                    || self.voted_for.unwrap() == args.candidate_id)
                    && (args.last_log_index >= self.log.len()
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
                    self.next_index[res.id] -= 1;

                    let at = self.next_index[res.id];
                    let conn = &self.config.connections[&res.id];
                    let rpc = RPC::AppendEntries(AppendEntries {
                        term: self.current_term,
                        leader_id: self.id,
                        prev_log_index: at - 1,
                        prev_log_term: self.log[at - 1].0,
                        entries: Vec::from(&self.log[at..]),
                        leader_commit: self.commit_index,
                    });

                    conn.send(rpc).unwrap();

                    return;
                }

                self.next_index[res.id] = max(self.next_index[res.id], res.replicated_index + 1);
                self.match_index[res.id] = max(self.match_index[res.id], res.replicated_index);

                // Check if exists N > commitIndex where majority of matchIndex[i] >= N for all servers i and log[N].term == currentTerm -> set commitIndex to N
                let num_of_replicated = self
                    .match_index
                    .iter()
                    .filter(|&&i| i >= res.replicated_index)
                    .count();
                let num_needed = self.config.connections.len() / 2 + 1;

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

                let num_of_votes = self.election_votes.len();
                let num_needed = self.config.connections.len() / 2 + 1;

                if num_of_votes >= num_needed {
                    self.state = ServerState::Leader;
                    println!("[{}] Elected leader", self.id);
                    self.append_entries();
                }
            }
            RPC::ClientRequest(client_sender, client_req) => match self.state {
                ServerState::Follower => {
                    match self.leader_id {
                        Some(leader_id) => {
                            client_sender
                                .send(ClientResponse::LeaderRedirect(leader_id))
                                .expect("Client Sender to not fail!");
                        }
                        None => {
                            client_sender
                                .send(ClientResponse::Fail)
                                .expect("Client Sender to not fail!");
                        }
                    }
                }
                ServerState::Candidate => {
                    client_sender
                        .send(ClientResponse::Fail)
                        .expect("Client Sender to not fail!");
                }
                ServerState::Leader => {
                    self.log.push((self.current_term, client_req.value));
                    self.append_entries();
                }
            },
        }
    }

    fn handle_timeout(&mut self) {
        println!("[{}] Timed out!", self.id);
        match self.state {
            ServerState::Follower => {
                self.state = ServerState::Candidate;
                self.handle_timeout();
            }
            ServerState::Candidate => {
                // Start new election
                self.current_term += 1;
                self.election_votes.clear();
                self.election_votes.insert(self.id);
                self.voted_for = Some(self.id);
                self.broadcast_rpc(RPC::RequestVote(RequestVote {
                    term: self.current_term,
                    candidate_id: self.id,
                    last_log_index: self.log.len() - 1,
                    last_log_term: self.log.last().unwrap().0,
                }));
            }
            ServerState::Leader => {
                // heartbeat or idempotent retry
                self.append_entries();
            }
        }
    }

    fn handle_term_conversion_to_follower(&mut self, term: usize) -> bool {
        if term > self.current_term {
            self.current_term = term;
            self.voted_for = None;
            self.leader_id = None;
            self.state = ServerState::Follower;

            return true;
        }

        false
    }

    fn apply_log_entries(&mut self) {
        while self.commit_index > self.last_applied {
            self.last_applied += 1;
            // TODO: apply log[self.last_applied] to state machine
            // TODO: if leader then respond back to client
            if self.state == ServerState::Leader {
                println!(
                    "[{}] Applying and responding to client {}",
                    self.id, self.last_applied
                )
            } else {
                println!("[{}] Applying {}", self.id, self.last_applied)
            }
        }
    }

    fn append_entries(&mut self) {
        self.next_index.iter().enumerate().for_each(|(id, at)| {
            if id == self.id {
                return;
            }

            let conn = &self.config.connections[&id];
            let rpc = RPC::AppendEntries(AppendEntries {
                term: self.current_term,
                leader_id: self.id,
                prev_log_index: at - 1,
                prev_log_term: self.log[at - 1].0,
                entries: Vec::from(&self.log[*at..]),
                leader_commit: self.commit_index,
            });

            conn.send(rpc).unwrap();
        });
    }

    fn broadcast_rpc(&self, rpc: RPC<T>) {
        self.config
            .connections
            .iter()
            .filter(|(&id, _)| id != self.id)
            .for_each(|(id, conn)| {
                conn.send(rpc.clone())
                    .expect(&format!("{} sender to not fail!", id))
            })
    }
}
