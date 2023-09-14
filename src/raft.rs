use std::{
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle}, cmp::max,
};

use crate::rpc::{RPCConfig, RPC};

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
            config,
            state: ServerState::Follower,
            current_term: 0,
            voted_for: Option::None,
            // dummy vaue at index 0. Real entries begin at index 1
            log: vec![(0, dummy())],
            commit_index: 0,
            last_applied: 0,
            next_index: vec![],
            match_index: vec![],
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
                        .send(RPC::AppendEntriesRes(self.current_term, false))
                        .expect("Sender to not fail!");
                    return;
                }

                if let None = self.log.get(args.prev_log_index) {
                    sender
                        .send(RPC::AppendEntriesRes(self.current_term, false))
                        .expect("Sender to not fail!");
                    return;
                }

                let (term, _) = self.log.get(args.prev_log_index).unwrap();
                if *term != args.term {
                    sender
                        .send(RPC::AppendEntriesRes(self.current_term, false))
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
                    self.commit_index = max(args.leader_commit, self.log.len() - 1);

                    while self.commit_index > self.last_applied {
                        self.last_applied += 1;
                        // apply log[self.last_applied] to state machine
                    }
                }
            }
            RPC::RequestVote(args) => todo!(),
            RPC::AppendEntriesRes(term, success) => todo!(),
            RPC::RequestVoteRes(term, vote_granted) => todo!(),
        }
    }

    fn handle_timeout(&self) {}

    fn handle_term_conversion_to_follower(&mut self, term: usize) -> bool {
        if term > self.current_term {
            self.current_term = term;
            self.state = ServerState::Follower;

            return true;
        }

        false
    }
}
