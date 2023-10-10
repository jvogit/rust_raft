use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
    thread::JoinHandle,
    time::Duration,
};

use rust_raft::{
    raft::Server,
    rpc::{self, ClientRequest, ClientResponse, RPCConfig},
};

type RPC = rpc::RPC<(usize, usize)>;

fn send_client_request(
    senders: &HashMap<usize, Sender<RPC>>,
    client_sender: &Sender<ClientResponse>,
    client_receiver: &Receiver<ClientResponse>,
    value: (usize, usize),
) -> ClientResponse {
    let req = rpc::RPC::ClientRequest(client_sender.clone(), ClientRequest { value, xid: 1 });
    let mut to_send = 0;
    senders[&to_send].send(req.clone()).unwrap();
    while let Ok(res) = client_receiver.recv() {
        match res {
            ClientResponse::Success => return res,
            ClientResponse::LeaderRedirect(leader_id) => {
                to_send = leader_id;
                senders[&to_send].send(req.clone()).unwrap();
            }
            ClientResponse::Fail => {
                senders[&to_send].send(req.clone()).unwrap();
            }
        };
    }

    return ClientResponse::Fail;
}

fn main() {
    let num_servers: usize = 3;
    let connections: HashMap<usize, (Sender<RPC>, Receiver<RPC>)> = (0..num_servers)
        .into_iter()
        .map(|id| (id, mpsc::channel::<RPC>()))
        .collect();
    let senders = connections
        .iter()
        .map(|(&id, (sender, _))| (id, sender.clone()))
        .collect::<HashMap<usize, Sender<RPC>>>();
    let mut config = RPCConfig {
        connections: senders.clone(),
        election_timeout: Duration::from_secs(5),
    };
    let dummy = || (0, 0);
    let handles: Vec<JoinHandle<()>> = connections
        .into_iter()
        .map(|(id, (_, recv))| Server::new(id, config.clone(), dummy).start_thread(recv))
        .collect();
    let (client_sender, client_receiver) = mpsc::channel::<ClientResponse>();

    send_client_request(&senders, &client_sender, &client_receiver, (1, 1));
    
    handles.into_iter().for_each(|handle| {
        handle.join();
    });
}
