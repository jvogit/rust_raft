use std::{collections::HashMap, sync::mpsc, time::Duration};

#[derive(Debug)]
pub struct AppendEntries<T> {
    pub term: usize,
    pub leader_id: usize,
    pub prev_log_index: usize,
    pub prev_log_term: usize,
    pub entries: Vec<T>,
    pub leader_commit: usize,
}

#[derive(Debug)]
pub struct RequestVote {
    pub term: usize,
    pub candidate_id: usize,
    pub last_log_index: usize,
    pub last_log_term: usize,
}

#[derive(Debug)]
pub enum RPC<T> {
    AppendEntries(AppendEntries<T>),
    AppendEntriesRes(usize, bool),
    RequestVote(RequestVote),
    RequestVoteRes(usize, bool),
}

pub struct RPCConfig<T> {
    connections: HashMap<usize, mpsc::Sender<RPC<T>>>,
    pub election_timeout: Duration,
}

impl<T> RPCConfig<T> {
    pub fn new(
        connections: HashMap<usize, mpsc::Sender<RPC<T>>>,
        election_timeout: Duration,
    ) -> RPCConfig<T> {
        Self {
            connections,
            election_timeout,
        }
    }

    pub fn get_connection(&self, id: usize) -> Option<&mpsc::Sender<RPC<T>>> {
        self.connections.get(&id)
    }
}
