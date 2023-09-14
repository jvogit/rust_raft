use std::{sync::mpsc::{Receiver, self}, thread::{JoinHandle, self}};

use crate::rpc::{RPCConfig, RPC};

enum ServerState {
    Follower,
    Candidate,
    Leader,
}

pub struct Server {
    // Server metadata
    id: usize,
    config: RPCConfig,
    state: ServerState,
    // RAFT variables
    // Stable
    current_term: usize,
    voted_for: Option<usize>,
    log: Vec<(String, String)>,
    // Volatile
    commit_index: usize,
    last_applied: usize,
    // Volatile for leaders
    next_index: Vec<usize>,
    match_index: Vec<usize>,
}

impl Server {
    pub fn new(id: usize, config: RPCConfig) -> Server {
        Server {
            id,
            config,
            state: ServerState::Follower,
            current_term: 0,
            voted_for: Option::None,
            log: vec![],
            commit_index: 0,
            last_applied: 0,
            next_index: vec![],
            match_index: vec![],
        }
    }

    pub fn start_thread(mut self, recv: Receiver<RPC>) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                match self.state {
                    ServerState::Follower => self.handle_follower(&recv),
                    ServerState::Candidate => self.handle_candidate(&recv),
                    ServerState::Leader => self.handle_leader(&recv),
                }
            }
        })
    }

    fn handle_follower(&mut self, recv: &Receiver<RPC>) {
        match recv.recv_timeout(self.config.election_timeout) {
            Ok(rpc) => self.handle_rpc(rpc),
            Err(mpsc::RecvTimeoutError::Timeout) => self.handle_timeout(),
            Err(_) => return,
        }
    }

    fn handle_candidate(&mut self, recv: &Receiver<RPC>) {
        match recv.recv_timeout(self.config.election_timeout) {
            Ok(rpc) => self.handle_rpc(rpc),
            Err(mpsc::RecvTimeoutError::Timeout) => self.handle_timeout(),
            Err(_) => return,
        }
    }

    fn handle_leader(&mut self, recv: &Receiver<RPC>) {
        
    }

    fn handle_rpc(&mut self, rpc: RPC) {
        match rpc {
            RPC::AppendEntries(args) => {
                let sender = self.config.get_connection(args.leader_id).unwrap().clone();
                self.handle_term_conversion_to_follower(args.term);

                if args.term < self.current_term {
                    sender.send(RPC::AppendEntriesRes(self.current_term, false));
                    return;
                }
            },
            RPC::RequestVote(args) => todo!(),
            RPC::AppendEntriesRes(term, success) => todo!(),
            RPC::RequestVoteRes(term, vote_granted) => todo!(),
        }
    }

    fn handle_timeout(&self) {

    }

    fn handle_term_conversion_to_follower(&mut self, term: usize) -> bool {
        if term > self.current_term {
            self.current_term = term;
            self.state = ServerState::Follower;

            return true
        }

        false
    }
}
