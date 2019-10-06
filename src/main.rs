mod errors;
mod message;
mod p2p;

use async_std::task;
use futures_channel::mpsc;

fn main() {
    let (out_sender, out_receiver) = mpsc::unbounded();
    let (in_sender, in_receiver) = mpsc::unbounded();
    task::spawn(async {
        if let Err(err) = p2p::connect_peer("137.74.30.99:8333", out_sender, in_receiver).await {
            eprintln!("{}", err);
        }
    });
    task::block_on(p2p::handle_peer(in_sender, out_receiver)).unwrap();
}
