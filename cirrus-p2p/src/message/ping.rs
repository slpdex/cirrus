use crate::message::Message;
use byteorder::{LittleEndian, ReadBytesExt};
use cirrus_peer::{
    errors::{message::ErrorKind::IoError, Result, ResultExt},
    MessagePacket,
};
use std::io;

#[derive(Clone, Debug)]
pub struct PingMessage {
    pub nonce: u64,
}

impl Message for PingMessage {
    fn command() -> &'static [u8] {
        b"ping"
    }

    fn packet(&self) -> MessagePacket {
        MessagePacket::from_payload(Self::command(), self.nonce.to_le_bytes().to_vec())
    }

    fn from_payload(payload: &[u8]) -> Result<Self> {
        Ok(PingMessage {
            nonce: io::Cursor::new(payload)
                .read_u64::<LittleEndian>()
                .chain_err(|| IoError)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct PongMessage {
    pub nonce: u64,
}

impl Message for PongMessage {
    fn command() -> &'static [u8] {
        b"pong"
    }

    fn packet(&self) -> MessagePacket {
        MessagePacket::from_payload(Self::command(), self.nonce.to_le_bytes().to_vec())
    }

    fn from_payload(payload: &[u8]) -> Result<Self> {
        Ok(PongMessage {
            nonce: io::Cursor::new(payload)
                .read_u64::<LittleEndian>()
                .chain_err(|| IoError)?,
        })
    }
}
