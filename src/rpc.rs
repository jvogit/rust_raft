use std::{collections::HashMap, sync::mpsc, time::Duration};

pub struct AppendEntries {
    term: usize,
    leaderId: usize,
    prevLogIndex: usize,
    entries: Vec<usize>,
    leaderCommit: usize,
}

pub struct RequestVote {
    term: usize,
    candidateId: usize,
    lastLogIndex: usize,
    lastLogTerm: usize,
}

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
