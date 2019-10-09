use crate::errors::{peer::ErrorKind::*, Error, ErrorKind, Result, ResultExt};
use crate::message_header::{MessageHeader, HEADER_SIZE};
use crate::message_packet::MessagePacket;
use async_std::{net::TcpStream, prelude::*, task};
use futures::future::try_join3;
use futures::Stream;
use futures_channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use std::net::SocketAddr;

struct PeerStream {
    stream: TcpStream,
}

pub struct Peer {
    message_receiver: UnboundedReceiver<MessagePacket>,
    message_sender: UnboundedSender<MessagePacket>,
    shutdown_sender: UnboundedSender<()>,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
}

impl Peer {
    pub async fn start(addr: SocketAddr) -> Result<Peer> {
        let (outgoing_sender, outgoing_receiver) = mpsc::unbounded();
        let (incoming_sender, incoming_receiver) = mpsc::unbounded();
        let (shutdown_sender, shutdown_receiver) = mpsc::unbounded();
        let stream = TcpStream::connect(addr).await.chain_err(|| ConnectFailed)?;
        let peer_addr = stream.peer_addr().chain_err(|| HasNoPeerAddr)?;
        let local_addr = stream.local_addr().chain_err(|| HasNoLocalAddr)?;
        task::spawn(async move {
            if let Err(err) = Self::_start_peer_stream(
                stream,
                outgoing_receiver,
                incoming_sender,
                shutdown_receiver,
            )
            .await
            {
                eprintln!("Peer error: {}", err);
            }
        });
        Ok(Peer {
            message_receiver: incoming_receiver,
            message_sender: outgoing_sender,
            shutdown_sender,
            local_addr,
            peer_addr,
        })
    }

    async fn _start_peer_stream(
        stream: TcpStream,
        outgoing_receiver: UnboundedReceiver<MessagePacket>,
        incoming_sender: UnboundedSender<MessagePacket>,
        shutdown_receiver: UnboundedReceiver<()>,
    ) -> Result<()> {
        let mut peer_stream = PeerStream::new(stream);
        peer_stream
            .run(outgoing_receiver, incoming_sender, shutdown_receiver)
            .await
    }

    pub fn message_stream(&mut self) -> &mut impl Stream<Item = MessagePacket> {
        &mut self.message_receiver
    }

    pub fn send_message(&mut self, packet: MessagePacket) -> Result<()> {
        self.message_sender
            .unbounded_send(packet)
            .chain_err(|| ErrorKind::ChannelError)
    }

    pub fn shutdown(&mut self) -> Result<()> {
        self.shutdown_sender
            .unbounded_send(())
            .chain_err(|| ErrorKind::ChannelError)
    }

    pub fn local_addr(&self) -> &SocketAddr {
        &self.local_addr
    }

    pub fn peer_addr(&self) -> &SocketAddr {
        &self.peer_addr
    }
}

impl PeerStream {
    pub fn new(stream: TcpStream) -> Self {
        PeerStream { stream }
    }

    pub async fn run(
        &mut self,
        outgoing_receiver: UnboundedReceiver<MessagePacket>,
        incoming_sender: UnboundedSender<MessagePacket>,
        shutdown_receiver: UnboundedReceiver<()>,
    ) -> Result<()> {
        let result = try_join3(
            Self::handle_incoming(&self.stream, incoming_sender.clone()),
            Self::handle_outgoing(&self.stream, outgoing_receiver),
            Self::handle_shutdown(shutdown_receiver),
        )
        .await;
        incoming_sender.close_channel();
        self.stream
            .shutdown(std::net::Shutdown::Both)
            .chain_err(|| ShutdownFailed)?;
        if let Err(Error(ErrorKind::Peer(Shutdown), _)) = result {
            return Ok(());
        }
        result?;
        Ok(())
    }

    async fn handle_incoming(
        mut stream: &TcpStream,
        incoming_sender: UnboundedSender<MessagePacket>,
    ) -> Result<()> {
        let mut buf = [0; 0x10000];
        let mut remaining = Vec::with_capacity(0x10000);
        loop {
            let n_bytes = stream
                .read(&mut buf[..])
                .await
                .chain_err(|| ReadMessageFailed)?;
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
                    incoming_sender
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
        }
    }

    async fn handle_outgoing(
        mut stream: &TcpStream,
        mut outgoing_receiver: UnboundedReceiver<MessagePacket>,
    ) -> Result<()> {
        while let Some(packet) = outgoing_receiver.next().await {
            packet.write_to_stream(&mut stream).await?;
        }
        Ok(())
    }

    async fn handle_shutdown(mut shutdown_receiver: UnboundedReceiver<()>) -> Result<()> {
        shutdown_receiver.next().await;
        Err(ErrorKind::Peer(Shutdown).into())
    }
}
