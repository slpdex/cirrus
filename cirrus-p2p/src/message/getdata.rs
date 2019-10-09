use super::inv::InvVector;

use crate::message::Message;
use byteorder::{LittleEndian, WriteBytesExt};
use cashcontracts::serialize::write_var_int;
use cirrus_peer::{errors::Result, MessagePacket};
use std::io::Write;

#[derive(Clone, Debug)]
pub struct GetDataMessage {
    pub inv_vectors: Vec<InvVector>,
}

impl Message for GetDataMessage {
    fn command() -> &'static [u8] {
        b"getdata"
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

    fn from_payload(_payload: &[u8]) -> Result<Self> {
        unimplemented!()
    }
}
