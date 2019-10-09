use fasthash::murmur3;
use std::f64::consts::LN_2;

#[derive(Clone, Debug)]
pub struct Bloom {
    filter_bits: Vec<u8>,
    num_hash_funcs: u32,
    tweak: u32,
    flags: u8,
}

const LN2SQUARED: f64 = LN_2 * LN_2;
const MAX_BLOOM_FILTER_SIZE: usize = 36000;
const MAX_HASH_FUNCS: u32 = 50;
const HASH_SEED_FACTOR: u32 = 0xFBA4_C795;

impl Bloom {
    pub fn from_num_elements(num_elements: usize, fp_rate: f64, tweak: u32, flags: u8) -> Self {
        let num_elements = num_elements as f64;
        let num_bits = -1.0 / LN2SQUARED * num_elements * fp_rate.ln();
        let num_bytes = (num_bits as usize / 8).min(MAX_BLOOM_FILTER_SIZE);
        let num_hash_funcs = (num_bytes as f64 * 8.0) / num_elements * LN_2;
        Bloom {
            filter_bits: vec![0; num_bytes],
            num_hash_funcs: (num_hash_funcs as u32).min(MAX_HASH_FUNCS),
            tweak,
            flags,
        }
    }

    pub fn insert(&mut self, data: &[u8]) {
        for i in 0..self.num_hash_funcs {
            let idx = self.hash(data, i);
            self.filter_bits[idx as usize / 8] |= 1 << (idx as u8 & 0x7);
        }
    }

    pub fn contains(&self, data: &[u8]) -> bool {
        for i in 0..self.num_hash_funcs {
            let idx = self.hash(data, i);
            if self.filter_bits[idx as usize / 8] & 1 << (idx as u8 & 0x7) == 0 {
                return false;
            }
        }
        true
    }

    fn hash(&self, data: &[u8], hash_idx: u32) -> u32 {
        let seed = hash_idx
            .wrapping_mul(HASH_SEED_FACTOR)
            .wrapping_add(self.tweak);
        murmur3::hash32_with_seed(data, seed) % (self.filter_bits.len() as u32 * 8)
    }

    pub fn filter_bits(&self) -> &[u8] {
        &self.filter_bits
    }

    pub fn num_hash_funcs(&self) -> u32 {
        self.num_hash_funcs
    }

    pub fn tweak(&self) -> u32 {
        self.tweak
    }

    pub fn flags(&self) -> u8 {
        self.flags
    }
}

#[test]
fn test_bloom() {
    use hex_literal::hex;
    let mut filter = Bloom::from_num_elements(3, 0.01, 0, 0);
    let data = hex!("99108ad8ed9bb6274d3980bab5a85c048f0950c8");
    filter.insert(&data);
    assert!(filter.contains(&data));
}
