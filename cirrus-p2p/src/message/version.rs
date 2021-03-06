use crate::message::Message;
use bitflags::bitflags;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use cashcontracts::serialize::{read_var_str, write_var_str};
use cirrus_peer::{
    errors::{message, Result, ResultExt},
    MessagePacket,
};
use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct VersionMessage {
    pub version: i32,
    pub services: NetworkServices,
    pub timestamp: i64,
    pub recv_services: NetworkServices,
    pub recv_addr: IpAddr,
    pub recv_port: u16,
    pub send_services: NetworkServices,
    pub send_addr: IpAddr,
    pub send_port: u16,
    pub nonce: u64,
    pub user_agent: Vec<u8>,
    pub start_height: i32,
    pub relay: bool,
}

bitflags! {
    #[derive(Default)]
    pub struct NetworkServices: u64 {
        const NETWORK = 1;
        const GETUTXO = 2;
        const BLOOM = 4;
        const NODE_BITCOIN_CASH = 0x20;
        const NETWORK_LIMITED = 0x400;
    }
}

fn ip_octets(ip: IpAddr) -> [u8; 16] {
    match ip {
        IpAddr::V4(ip) => ip.to_ipv6_mapped().octets(),
        IpAddr::V6(ip) => ip.octets(),
    }
}

impl VersionMessage {
    pub fn from_addrs(
        peer_addr: &SocketAddr,
        local_addr: &SocketAddr,
        requested_services: NetworkServices,
        provided_services: NetworkServices,
        user_agent: Vec<u8>,
        start_height: i32,
        relay: bool,
    ) -> Self {
        let unix_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        VersionMessage {
            version: 70015,
            services: requested_services,
            timestamp: unix_time as i64,
            recv_services: requested_services,
            recv_addr: peer_addr.ip(),
            recv_port: peer_addr.port(),
            send_services: provided_services,
            send_addr: local_addr.ip(),
            send_port: local_addr.port(),
            nonce: rand::random(),
            user_agent,
            start_height,
            relay,
        }
    }
}

impl Message for VersionMessage {
    fn command() -> &'static [u8] {
        b"version"
    }

    fn packet(&self) -> MessagePacket {
        let mut payload = Vec::new();
        payload.write_i32::<LittleEndian>(self.version).unwrap();
        payload
            .write_u64::<LittleEndian>(self.services.bits())
            .unwrap();
        payload.write_i64::<LittleEndian>(self.timestamp).unwrap();

        payload
            .write_u64::<LittleEndian>(self.recv_services.bits())
            .unwrap();
        payload.write_all(&ip_octets(self.recv_addr)).unwrap();
        payload.write_u16::<LittleEndian>(self.recv_port).unwrap();

        payload
            .write_u64::<LittleEndian>(self.send_services.bits())
            .unwrap();
        payload.write_all(&ip_octets(self.send_addr)).unwrap();
        payload.write_u16::<LittleEndian>(self.send_port).unwrap();

        payload.write_u64::<LittleEndian>(self.nonce).unwrap();
        write_var_str(&mut payload, &self.user_agent).unwrap();
        payload
            .write_i32::<LittleEndian>(self.start_height)
            .unwrap();
        payload.write_u8(if self.relay { 1 } else { 0 }).unwrap();

        MessagePacket::from_payload(Self::command(), payload)
    }

    fn from_payload(payload: &[u8]) -> Result<Self> {
        let io_err = || message::ErrorKind::IoError;
        let mut stream = io::Cursor::new(payload);
        let version = stream.read_i32::<LittleEndian>().chain_err(io_err)?;
        let services = stream.read_u64::<LittleEndian>().chain_err(io_err)?;
        let services = NetworkServices::from_bits(services)
            .chain_err(|| message::ErrorKind::InvalidNetworkServices)?;
        let timestamp = stream.read_i64::<LittleEndian>().chain_err(io_err)?;

        let recv_services = stream.read_u64::<LittleEndian>().chain_err(io_err)?;
        let recv_services = NetworkServices::from_bits(recv_services)
            .chain_err(|| message::ErrorKind::InvalidNetworkServices)?;
        let mut recv_addr_bytes = [0; 16];
        stream.read(&mut recv_addr_bytes).chain_err(io_err)?;
        let recv_addr = IpAddr::from(recv_addr_bytes);
        let recv_port = stream.read_u16::<LittleEndian>().chain_err(io_err)?;

        let send_services = stream.read_u64::<LittleEndian>().chain_err(io_err)?;
        let send_services = NetworkServices::from_bits(send_services)
            .chain_err(|| message::ErrorKind::InvalidNetworkServices)?;
        let mut send_addr_bytes = [0; 16];
        stream.read(&mut send_addr_bytes).chain_err(io_err)?;
        let send_addr = IpAddr::from(send_addr_bytes);
        let send_port = stream.read_u16::<LittleEndian>().chain_err(io_err)?;

        let nonce = stream.read_u64::<LittleEndian>().chain_err(io_err)?;
        let user_agent = read_var_str(&mut stream).chain_err(io_err)?;
        let start_height = stream.read_i32::<LittleEndian>().chain_err(io_err)?;
        let relay = stream.read_u8().chain_err(io_err)? > 0;
        Ok(VersionMessage {
            version,
            services,
            timestamp,
            recv_services,
            recv_addr,
            recv_port,
            send_services,
            send_addr,
            send_port,
            nonce,
            user_agent,
            start_height,
            relay,
        })
    }
}

pub struct VerackMessage;

impl Message for VerackMessage {
    fn command() -> &'static [u8] {
        b"verack"
    }

    fn packet(&self) -> MessagePacket {
        MessagePacket::from_payload(Self::command(), vec![])
    }

    fn from_payload(_payload: &[u8]) -> Result<Self> {
        Ok(VerackMessage)
    }
}
