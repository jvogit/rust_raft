enum ServerState {
    Follower,
    Candidate,
    Leader,
}

pub struct Server {
    state: ServerState,
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
    
}
