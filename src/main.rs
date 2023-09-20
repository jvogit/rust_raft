use std::{collections::HashMap, time::Duration};

use rust_raft::{raft::Server, rpc::RPCConfig};

fn main() {
    let config = RPCConfig::new(HashMap::new(), Duration::from_secs(5));
    let server: Server<(usize, usize)> = Server::new(0, config, || { (0, 0) });
}
