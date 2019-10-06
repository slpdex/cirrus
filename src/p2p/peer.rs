use crate::errors::{peer::ErrorKind::*, ErrorKind, Result, ResultExt};
use crate::message::{
    Message, MessageHeader, MessagePacket, PingMessage, PongMessage, VerackMessage, VersionMessage,
    HEADER_SIZE,
};
use async_std::{
    net::{TcpStream, ToSocketAddrs},
    prelude::*,
};
use futures::future::{select, Either};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};

pub async fn connect_peer(
    addr: impl ToSocketAddrs,
    out_messages: UnboundedSender<MessagePacket>,
    mut in_messages: UnboundedReceiver<MessagePacket>,
) -> Result<()> {
    let stream = TcpStream::connect(addr).await.chain_err(|| ConnectFailed)?;
    let (reader, writer) = &mut (&stream, &stream);
    VersionMessage::from_addrs(
        &stream.peer_addr().chain_err(|| HasNoPeerAddr)?,
        &stream.peer_addr().chain_err(|| HasNoLocalAddr)?,
    )
    .packet()
    .write_to_stream(writer)
    .await?;
    let mut buf = [0; 0x10000];
    let mut remaining = Vec::with_capacity(0x10000);
    let mut incoming_fut = reader.read(&mut buf[..]);
    let mut outgoing_fut = in_messages.next();
    loop {
        let result = select(&mut incoming_fut, &mut outgoing_fut).await;
        match result {
            Either::Left((n_bytes, _)) => {
                let n_bytes = n_bytes.chain_err(|| ReadMessageFailed)?;
                if n_bytes == 0 {
                    return Err(ErrorKind::Peer(Disconnected).into());
                }
                remaining.extend_from_slice(&buf[..n_bytes]);
                let mut i = 0;
                while remaining.len() >= i + HEADER_SIZE {
                    let header = MessageHeader::from_slice(&remaining[i..i + HEADER_SIZE])?;
                    let start = i + HEADER_SIZE;
                    let end = start + header.payload_size() as usize;
                    if remaining.len() >= end {
                        let payload = remaining[start..end].to_vec();
                        let packet = MessagePacket::from_header_payload(header, payload)?;
                        out_messages
                            .unbounded_send(packet)
                            .chain_err(|| ErrorKind::ChannelError)?;
                        i = end;
                    } else {
                        break;
                    }
                }
                if i == remaining.len() {
                    remaining.clear();
                } else {
                    remaining.drain(..i);
                }
                incoming_fut = reader.read(&mut buf[..]);
            }
            Either::Right((packet, _)) => match packet {
                Some(packet) => {
                    packet.write_to_stream(writer).await?;
                    outgoing_fut = in_messages.next();
                }
                None => return Ok(()),
            },
        }
    }
}

pub async fn handle_peer(
    out_messages: UnboundedSender<MessagePacket>,
    mut in_messages: UnboundedReceiver<MessagePacket>,
) -> Result<()> {
    while let Some(packet) = in_messages.next().await {
        println!("packet: {}", packet);
        match packet.header().command_name() {
            b"verack" => {
                out_messages
                    .unbounded_send(VerackMessage.packet())
                    .chain_err(|| ErrorKind::ChannelError)?;
            }
            b"ping" => {
                let ping = PingMessage::from_payload(packet.payload())?;
                out_messages
                    .unbounded_send(PongMessage { nonce: ping.nonce }.packet())
                    .chain_err(|| ErrorKind::ChannelError)?;
            }
            _ => {}
        }
    }
    Ok(())
}
