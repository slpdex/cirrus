use crate::errors::{peer::ErrorKind::*, Error, ErrorKind, Result, ResultExt};
use crate::message::{
    Message, MessageHeader, MessagePacket, PingMessage, PongMessage, VerackMessage, VersionMessage,
    HEADER_SIZE,
};
use async_std::{net::TcpStream, prelude::*, task};
use futures::future::try_join4;
use futures::Stream;
use futures_channel::mpsc::{self, UnboundedReceiver, UnboundedSender};

pub struct PeerStream {
    stream: TcpStream,
}

pub struct Peer {
    message_receiver: UnboundedReceiver<MessagePacket>,
    message_sender: UnboundedSender<MessagePacket>,
    shutdown_sender: UnboundedSender<()>,
}

impl Peer {
    pub async fn start(addr: std::net::SocketAddr) -> Result<Peer> {
        let (outgoing_sender, outgoing_receiver) = mpsc::unbounded();
        let (message_sender, message_receiver) = mpsc::unbounded();
        let (shutdown_sender, shutdown_receiver) = mpsc::unbounded();
        let outgoing_sender2 = outgoing_sender.clone();
        task::spawn(async move {
            if let Err(err) = Self::_start_peer_stream(
                addr,
                outgoing_receiver,
                outgoing_sender2,
                message_sender,
                shutdown_receiver,
            )
            .await
            {
                eprintln!("Peer error: {}", err);
            }
        });
        Ok(Peer {
            message_receiver,
            message_sender: outgoing_sender,
            shutdown_sender,
        })
    }

    async fn _start_peer_stream(
        addr: std::net::SocketAddr,
        outgoing_receiver: UnboundedReceiver<MessagePacket>,
        outgoing_sender: UnboundedSender<MessagePacket>,
        message_sender: UnboundedSender<MessagePacket>,
        shutdown_receiver: UnboundedReceiver<()>,
    ) -> Result<()> {
        let mut peer_stream = PeerStream::connect(addr).await?;
        peer_stream
            .run(
                outgoing_receiver,
                outgoing_sender,
                message_sender,
                shutdown_receiver,
            )
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
}

impl PeerStream {
    pub async fn connect(addr: std::net::SocketAddr) -> Result<PeerStream> {
        let stream = TcpStream::connect(addr).await.chain_err(|| ConnectFailed)?;
        Ok(PeerStream { stream })
    }

    pub async fn run(
        &mut self,
        outgoing_receiver: UnboundedReceiver<MessagePacket>,
        outgoing_sender: UnboundedSender<MessagePacket>,
        message_sender: UnboundedSender<MessagePacket>,
        shutdown_receiver: UnboundedReceiver<()>,
    ) -> Result<()> {
        let (incoming_sender, incoming_receiver) = mpsc::unbounded();
        Self::send_version(&self.stream).await?;
        let result = try_join4(
            Self::handle_incoming(&self.stream, incoming_sender),
            Self::handle_outgoing(&self.stream, outgoing_receiver),
            Self::handle_emit(
                outgoing_sender.clone(),
                incoming_receiver,
                message_sender.clone(),
            ),
            Self::handle_shutdown(shutdown_receiver),
        )
        .await;
        message_sender.close_channel();
        outgoing_sender.close_channel();
        self.stream
            .shutdown(std::net::Shutdown::Both)
            .chain_err(|| ShutdownFailed)?;
        if let Err(Error(ErrorKind::Peer(Shutdown), _)) = result {
            return Ok(())
        }
        result?;
        Ok(())
    }

    async fn send_version(mut stream: &TcpStream) -> Result<()> {
        VersionMessage::from_addrs(
            &stream.peer_addr().chain_err(|| HasNoPeerAddr)?,
            &stream.peer_addr().chain_err(|| HasNoLocalAddr)?,
        )
        .packet()
        .write_to_stream(&mut stream)
        .await?;
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

    async fn handle_emit(
        outgoing_sender: UnboundedSender<MessagePacket>,
        mut incoming_receiver: UnboundedReceiver<MessagePacket>,
        emit_sender: UnboundedSender<MessagePacket>,
    ) -> Result<()> {
        while let Some(packet) = incoming_receiver.next().await {
            match packet.header().command_name() {
                b"verack" => {
                    outgoing_sender
                        .unbounded_send(VerackMessage.packet())
                        .chain_err(|| ErrorKind::ChannelError)?;
                }
                b"ping" => {
                    let ping = PingMessage::from_payload(packet.payload())?;
                    outgoing_sender
                        .unbounded_send(PongMessage { nonce: ping.nonce }.packet())
                        .chain_err(|| ErrorKind::ChannelError)?;
                }
                _ => {
                    emit_sender
                        .unbounded_send(packet)
                        .chain_err(|| ErrorKind::ChannelError)?;
                }
            }
        }
        Ok(())
    }

    async fn handle_shutdown(mut shutdown_receiver: UnboundedReceiver<()>) -> Result<()> {
        shutdown_receiver.next().await;
        Err(ErrorKind::Peer(Shutdown).into())
    }
}
