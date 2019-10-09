use crate::message::Message;
use byteorder::{LittleEndian, WriteBytesExt};
use cashcontracts::serialize::write_var_int;
use cirrus_consensus::Bloom;
use cirrus_peer::{errors::Result, MessagePacket};
use std::io::Write;

#[derive(Clone, Debug)]
pub struct FilterLoadMessage {
    pub bloom: Bloom,
}

impl Message for FilterLoadMessage {
    fn command() -> &'static [u8] {
        b"filterload"
    }

    fn packet(&self) -> MessagePacket {
        let mut payload = Vec::new();
        write_var_int(&mut payload, self.bloom.filter_bits().len() as u64).unwrap();
        payload.write_all(self.bloom.filter_bits()).unwrap();
        payload
            .write_u32::<LittleEndian>(self.bloom.num_hash_funcs())
            .unwrap();
        payload
            .write_u32::<LittleEndian>(self.bloom.tweak())
            .unwrap();
        payload.write_u8(self.bloom.flags()).unwrap();
        MessagePacket::from_payload(Self::command(), payload)
    }

    fn from_payload(_payload: &[u8]) -> Result<Self> {
        unimplemented!()
    }
}
