use std::{collections::HashMap, sync::mpsc, time::Duration};

#[derive(Debug)]
pub struct AppendEntries {
    pub term: usize,
    pub leader_id: usize,
    pub prev_log_index: usize,
    pub entries: Vec<usize>,
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
pub enum RPC {
    AppendEntries(AppendEntries),
    AppendEntriesRes(usize, bool),
    RequestVote(RequestVote),
    RequestVoteRes(usize, bool),
}

pub struct RPCConfig {
    connections: HashMap<usize, mpsc::Sender<RPC>>,
    pub election_timeout: Duration,
}

impl RPCConfig {
    pub fn get_connection(&self, id: usize) -> Option<&mpsc::Sender<RPC>> {
        self.connections.get(&id)
    }
}
