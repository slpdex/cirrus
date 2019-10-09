mod errors;
mod message;
mod p2p;

use async_std::prelude::*;
use async_std::task;

async fn run() -> errors::Result<()> {
    use crate::message::{Message, VerackMessage, PingMessage, PongMessage};
    let mut peer = p2p::Peer::start("137.74.30.99:8333".parse().unwrap()).await?;
    while let Some(packet) = peer.message_stream().next().await {
        println!("msg: {}", packet);
        match packet.header().command_name() {
            b"verack" => {
                peer.send_message(VerackMessage.packet())?;
            }
            b"ping" => {
                let ping = PingMessage::from_payload(packet.payload())?;
                peer.send_message(PongMessage { nonce: ping.nonce }.packet())?;
            }
            _ => {
            }
        }
    }
    Ok(())
}

fn main() {
    task::block_on(async {
        if let Err(err) = run().await {
            eprintln!("{}", err);
        }
    });
    println!("shutdown");
    task::block_on(futures::future::pending::<()>());
    /*let (out_sender, out_receiver) = mpsc::unbounded();
    let (in_sender, in_receiver) = mpsc::unbounded();
    */
}
