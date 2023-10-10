# raft
implement [https://raft.github.io/raft.pdf](https://raft.github.io/raft.pdf)
## summary
raft is a consensus algorithm for a distributed cluster of systems to have a total order of operations that is consistent and fault tolerant. Essentially,
a consensus algorithm provides linearizability meaning operations applied to the cluster of nodes acts the same as if those same operations
were applied to just one node. This exercise implements a basic distributed key value database using the raft consesus algorithm to achieve fault tolerant consistency.
## implementation
implementation is in rust. each node is represented as a thread. each node communicates via async channels. implementation of the algorithm is in the
`raft.rs` and `rpc.rs` and follows the procedure and RPC structure detailed in the raft paper. 