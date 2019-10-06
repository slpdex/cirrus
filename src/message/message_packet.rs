use crate::errors::{message::ErrorKind::*, ErrorKind, Result, ResultExt};
use crate::message::MessageHeader;
use cashcontracts::double_sha256;
use std::io;

#[derive(Clone, Debug)]
pub struct MessagePacket {
    header: MessageHeader,
    payload: Vec<u8>,
}

impl MessagePacket {
    fn _check_checksum(payload: &[u8], checksum: [u8; 4]) -> Result<()> {
        let hash = double_sha256(&payload);
        if hash[..4] != checksum {
            return Err(ErrorKind::Message(InvalidChecksum).into());
        }
        Ok(())
    }

    pub fn from_header_payload(header: MessageHeader, payload: Vec<u8>) -> Result<Self> {
        Self::_check_checksum(&payload, header.checksum())?;
        Ok(MessagePacket { header, payload })
    }

    pub fn from_payload(command: &[u8], payload: Vec<u8>) -> MessagePacket {
        use std::io::Write;
        let hash = double_sha256(&payload);
        let mut checksum = [0; 4];
        checksum.copy_from_slice(&hash[..4]);
        let mut command_padded = [0u8; 12];
        io::Cursor::new(&mut command_padded[..])
            .write_all(command)
            .unwrap();
        let header = MessageHeader::new(command_padded, payload.len() as u32, checksum);
        MessagePacket { header, payload }
    }

    pub async fn write_to_stream<W: async_std::io::Write + Unpin>(
        &self,
        write: &mut W,
    ) -> Result<()> {
        use async_std::prelude::*;
        write
            .write_all(&self.header.bytes()[..])
            .await
            .chain_err(|| IoError)?;
        write.write_all(&self.payload).await.chain_err(|| IoError)?;
        Ok(())
    }

    pub fn header(&self) -> &MessageHeader {
        &self.header
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

impl std::fmt::Display for MessagePacket {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.header)?;
        writeln!(f, "payload: {}", hex::encode(&self.payload))?;
        Ok(())
    }
}
