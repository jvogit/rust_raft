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
    currentTerm: usize,
    votedFor: usize,
    log: Vec<(String, String)>,
    // Volatile
    commitIndex: usize,
    lastApplied: usize,
    // Volatile for leaders
    nextIndex: Vec<usize>,
    matchIndex: Vec<usize>,
}

impl Server {
    pub fn new(id: usize, config: RPCConfig) -> Server {
        Server {
            id,
            config,
            state: ServerState::Follower,
            currentTerm: 0,
            votedFor: 0,
            log: vec![],
            commitIndex: 0,
            lastApplied: 0,
            nextIndex: vec![],
            matchIndex: vec![],
        }
    }

    pub fn start_thread(self, recv: Receiver<RPC>) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                match recv.recv_timeout(self.config.election_timeout) {
                    Ok(rpc) => self.handle(rpc),
                    Err(mpsc::RecvTimeoutError::Timeout) => self.handle_timeout(),
                    Err(_) => break,
                }
            }
        })
    }

    fn handle(&self, rpc: RPC) {

    }

    fn handle_timeout(&self) {

    }
}
