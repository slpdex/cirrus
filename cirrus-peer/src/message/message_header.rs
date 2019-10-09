use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

use crate::errors::{message::ErrorKind::*, ErrorKind, Result, ResultExt};

#[derive(Clone, Debug)]
pub struct MessageHeader {
    command: [u8; 12],
    payload_size: u32,
    checksum: [u8; 4],
}

pub const MESSAGE_MAGIC: &[u8] = b"\xe3\xe1\xf3\xe8";
pub const HEADER_SIZE: usize = 4 + 12 + 4 + 4;

impl MessageHeader {
    pub fn new(command: [u8; 12], payload_size: u32, checksum: [u8; 4]) -> Self {
        MessageHeader {
            command,
            payload_size,
            checksum,
        }
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        let mut magic = [0; 4];
        let mut command = [0; 12];
        let mut checksum = [0; 4];
        let mut cur = io::Cursor::new(bytes);
        cur.read_exact(&mut magic).chain_err(|| IoError)?;
        if &magic[..] != MESSAGE_MAGIC {
            return Err(ErrorKind::Message(WrongMagic(magic.to_vec())).into());
        }
        cur.read_exact(&mut command).chain_err(|| IoError)?;
        let payload_size = cur.read_u32::<LittleEndian>().chain_err(|| IoError)?;
        cur.read_exact(&mut checksum).chain_err(|| IoError)?;
        Ok(MessageHeader {
            command,
            payload_size,
            checksum,
        })
    }

    pub fn bytes(&self) -> [u8; HEADER_SIZE] {
        let mut header = [0u8; HEADER_SIZE];
        let mut cur = io::Cursor::new(&mut header[..]);
        cur.write_all(MESSAGE_MAGIC).unwrap();
        cur.write_all(&self.command).unwrap();
        cur.write_u32::<LittleEndian>(self.payload_size).unwrap();
        cur.write_all(&self.checksum).unwrap();
        header
    }

    pub fn payload_size(&self) -> u32 {
        self.payload_size
    }

    pub fn checksum(&self) -> [u8; 4] {
        self.checksum
    }

    pub fn command_name(&self) -> &[u8] {
        let len = self
            .command
            .iter()
            .position(|b| *b == 0)
            .unwrap_or_else(|| self.command.len());
        &self.command[..len]
    }
}

impl std::fmt::Display for MessageHeader {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::result::Result<(), std::fmt::Error> {
        writeln!(f, "command: {}", String::from_utf8_lossy(&self.command))?;
        writeln!(f, "payload size: {}", self.payload_size)?;
        writeln!(f, "checksum: {}", hex::encode(&self.checksum))?;
        Ok(())
    }
}
