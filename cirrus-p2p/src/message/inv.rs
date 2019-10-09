use crate::message::Message;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use cashcontracts::serialize::{read_var_int, write_var_int};
use cashcontracts::tx_hash_to_hex;
use cirrus_peer::{
    errors::{message::ErrorKind::IoError, Result, ResultExt},
    MessagePacket,
};
use std::io::{Cursor, Read, Write};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectType {
    #[allow(dead_code)]
    Error = 0,
    Tx = 1,
    Block = 2,
    #[allow(dead_code)]
    FilteredBlock = 3,
    #[allow(dead_code)]
    CmpctBlock = 4,
}

#[derive(Clone, Debug)]
pub struct InvVector {
    pub type_id: ObjectType,
    pub hash: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct InvMessage {
    pub inv_vectors: Vec<InvVector>,
}

impl Message for InvMessage {
    fn command() -> &'static [u8] {
        b"inv"
    }

    fn packet(&self) -> MessagePacket {
        let mut payload = Vec::new();
        write_var_int(&mut payload, self.inv_vectors.len() as u64).unwrap();
        for inv_vector in self.inv_vectors.iter() {
            payload
                .write_u32::<LittleEndian>(inv_vector.type_id as u32)
                .unwrap();
            payload.write_all(&inv_vector.hash).unwrap();
        }
        MessagePacket::from_payload(Self::command(), payload)
    }

    fn from_payload(payload: &[u8]) -> Result<Self> {
        let mut cur = Cursor::new(payload);
        let n_inv = read_var_int(&mut cur).chain_err(|| IoError)?;
        let mut inv_vectors = Vec::new();
        for _ in 0..n_inv {
            let type_id = match cur.read_u32::<LittleEndian>().chain_err(|| IoError)? {
                1 => ObjectType::Tx,
                2 => ObjectType::Block,
                _ => continue,
            };
            let mut hash = [0; 32];
            cur.read_exact(&mut hash).chain_err(|| IoError)?;
            inv_vectors.push(InvVector { type_id, hash });
        }
        Ok(InvMessage { inv_vectors })
    }
}

impl std::fmt::Display for InvMessage {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::result::Result<(), std::fmt::Error> {
        writeln!(f, "num of invs: {}", self.inv_vectors.len())?;
        for inv_vector in self.inv_vectors.iter() {
            writeln!(
                f,
                "{:?}\t{}",
                inv_vector.type_id,
                tx_hash_to_hex(&inv_vector.hash)
            )?;
        }
        Ok(())
    }
}
