use cashcontracts::{tx_hash_to_hex, TxOutpoint};

#[derive(Clone, Debug)]
pub struct Utxo {
    pub outpoint: TxOutpoint,
    pub amount: u64,
    pub script: Vec<u8>,
    pub block_height: i32,
    pub flags: u8,
}

impl std::fmt::Display for Utxo {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> Result<(), std::fmt::Error> {
        writeln!(
            f,
            "Utxo: {}:{}",
            tx_hash_to_hex(&self.outpoint.tx_hash),
            self.outpoint.vout
        )?;
        writeln!(f, " amount:       {}", self.amount)?;
        writeln!(f, " script:       {}", hex::encode(&self.script))?;
        writeln!(f, " block_height: {}", self.block_height)?;
        writeln!(f, " flags:        {:x}", self.flags)?;
        Ok(())
    }
}
