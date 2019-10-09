
use crate::utxo::Utxo;
use futures::Stream;
use hyper::Client;
use crate::errors::{ErrorKind::*, Result, ResultExt};
use futures_channel::mpsc::UnboundedSender;
use std::future::Future;
use std::io::{Read, Cursor};
use byteorder::{LittleEndian, ReadBytesExt};
use cashcontracts::TxOutpoint;


pub struct UtxoStreamIpfs {
    link: String
}

const UTXO_MIN_SIZE: usize = 32 + 4 + 4 + 8 + 4;

impl UtxoStreamIpfs {
    pub fn new() -> Self {
        UtxoStreamIpfs {
            link: "http://ipfs.greyh.at/ipfs/QmXkBQJrMKkCKNbwv4m5xtnqwU9Sq7kucPigvZW8mWxcrv".to_string(),
        }
    }

    pub async fn stream_to(&self, mut utxo_sender: UnboundedSender<Utxo>) -> Result<()> {
        let response = Client::new()
            .get(self.link.parse().unwrap())
            .await.chain_err(|| ConnectionError)?;
        let mut body = response.into_body();
        let mut remaining_bytes = Vec::new();
        while let Some(chunk) = body.next().await {
            remaining_bytes.extend_from_slice(
                chunk.chain_err(|| ConnectionError)?.as_ref()
            );
            let mut i = 0;
            while remaining_bytes.len() >= i {
                let num_remaining = remaining_bytes.len() - i;
                if num_remaining < UTXO_MIN_SIZE {
                    break;
                }
                let mut cur = Cursor::new(&remaining_bytes[i..]);
                let mut tx_hash = [0; 32];
                cur.read_exact(&mut tx_hash).unwrap();
                let vout = cur.read_u32::<LittleEndian>().unwrap();
                let height_flagged = cur.read_i32::<LittleEndian>().unwrap();
                let flags = ((height_flagged & 0x0100_0000) >> 24) as u8;
                let block_height = height_flagged & 0x00ff_ffff;
                let amount = cur.read_u64::<LittleEndian>().unwrap();
                let script_len = cur.read_u32::<LittleEndian>().unwrap() as usize;
                if num_remaining < UTXO_MIN_SIZE + script_len {
                    break;
                }
                let mut script = vec![0; script_len];
                cur.read_exact(&mut script).unwrap();
                utxo_sender.unbounded_send(Utxo {
                    outpoint: TxOutpoint { tx_hash, vout },
                    amount,
                    script,
                    block_height,
                    flags,
                }).chain_err(|| ChannelError)?;
            }
        }
        Ok(())
    }
}
