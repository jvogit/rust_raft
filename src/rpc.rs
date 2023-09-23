use std::{collections::HashMap, sync::mpsc, time::Duration};

#[derive(Debug, Clone)]
pub struct AppendEntries<T> {
    pub term: usize,
    pub leader_id: usize,
    pub prev_log_index: usize,
    pub prev_log_term: usize,
    pub entries: Vec<(usize, T)>,
    pub leader_commit: usize,
}

#[derive(Debug, Clone)]
pub struct RequestVote {
    pub term: usize,
    pub candidate_id: usize,
    pub last_log_index: usize,
    pub last_log_term: usize,
}

#[derive(Debug, Clone)]
pub struct AppendEntriesRes {
    pub id: usize,
    pub term: usize,
    pub success: bool,
    pub replicated_index: usize,
}

#[derive(Debug, Clone)]
pub struct RequestVoteRes {
    pub id: usize,
    pub term: usize,
    pub vote_granted: bool,
}

#[derive(Debug, Clone)]
pub enum RPC<T> {
    AppendEntries(AppendEntries<T>),
    AppendEntriesRes(AppendEntriesRes),
    RequestVote(RequestVote),
    RequestVoteRes(RequestVoteRes),
}

pub struct RPCConfig<T>
where
    T: Clone,
{
    connections: HashMap<usize, mpsc::Sender<RPC<T>>>,
    pub election_timeout: Duration,
}

impl<T> RPCConfig<T>
where
    T: Clone,
{
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

    pub fn connections_len(&self) -> usize {
        self.connections.len()
    }

    pub fn broadcast_rpc(&self, rpc: RPC<T>) {
        self.connections.values().for_each(|c| {
            c.send(rpc.clone()).unwrap();
        });
    }
}
