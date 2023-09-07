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
    RequestVoteRes(usize, bool)
}
