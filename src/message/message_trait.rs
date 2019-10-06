use crate::errors::Result;
use crate::message::MessagePacket;

pub trait Message: Sized {
    fn command() -> &'static [u8];
    fn packet(&self) -> MessagePacket;
    fn from_payload(payload: &[u8]) -> Result<Self>;
}
