Try this code in a new crate and see the inv messages rain:

## Cargo.toml
```toml
[dependencies]
cirrus-peer = {git="https://github.com/slpdex/cirrus"}
cirrus-p2p = {git="https://github.com/slpdex/cirrus"}
async-std = "0.99.8"
```

## main.rs
```rust
use async_std::{prelude::*, task};
use cirrus_p2p::{
    Message, NetworkServices, PingMessage, PongMessage, VerackMessage, VersionMessage,
};
use cirrus_peer::Peer;

async fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut peer = Peer::start("137.74.30.99:8333".parse().unwrap()).await?;
    peer.send_message(
        VersionMessage::from_addrs(
            peer.peer_addr(),
            peer.local_addr(),
            NetworkServices::NETWORK,
            NetworkServices::default(),
            b"/cirrus:0.0.1/".to_vec(),
            0,
            true,
        )
        .packet(),
    )?;
    while let Some(packet) = peer.message_stream().next().await {
        println!("msg: {}", packet);
        match packet.header().command_name() {
            b"version" => {
                println!("{:?}", VersionMessage::from_payload(packet.payload())?);
            }
            b"verack" => {
                peer.send_message(VerackMessage.packet())?;
            }
            b"ping" => {
                let ping = PingMessage::from_payload(packet.payload())?;
                peer.send_message(PongMessage { nonce: ping.nonce }.packet())?;
            }
            _ => {}
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
}
````
