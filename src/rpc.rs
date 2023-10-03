use std::{collections::HashMap, sync::mpsc::{self, Sender}, time::Duration};

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
    ClientRequest(Sender<ClientResponse>, ClientRequest<T>),
}

#[derive(Debug, Clone)]
pub struct ClientRequest<T> {
    pub value: T,
    pub xid: usize,
}

#[derive(Debug, Clone)]
pub enum ClientResponse {
    Success,
    LeaderRedirect(usize),
    Fail,
}

#[derive(Debug, Clone)]
pub struct RPCConfig<T>
where
    T: Clone,
{
    pub connections: HashMap<usize, mpsc::Sender<RPC<T>>>,
    pub election_timeout: Duration,
}
