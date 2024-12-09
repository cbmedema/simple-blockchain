use std::time::{SystemTime, UNIX_EPOCH};

use rand::random;

use block::Block;

use crate::block;
use crate::blockchain::Blockchain;
use crate::input::Input;
use crate::mempool::Mempool;
use crate::output::Output;
use crate::transactions::Tx;

pub struct Miner {
    pub address: [u8;32],
    pub threads: u8,
}
impl Miner {

    pub fn generate_candidate_block(&self, index: u32, previous_hash: [u8;32], target: u64, pool: &mut Mempool, chain: &Blockchain) -> Block {
        let (hash,nonce) = Miner::gen_valid_hash(index,previous_hash, target);
        let (mut transactions, fees) = pool.calc_valid_tx_pool_and_fees(&chain);
        transactions.insert(0,self.generate_coinbase(fees));
        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut candidate =Block { index, hash, previous_hash, time, target, nonce, transactions };
        candidate
    }
    pub fn generate_coinbase(&self, fees: u64) -> Tx {
        const REWARD: u64 = 5000000;
        let mut inputs = vec![];
        let mut outputs = vec![];
        // inputs aren't important to coinbase Tx, however random signature given to prevent duplicate txid hash
        let mut signature: [u8; 64] = [0; 64];
        signature.iter_mut().for_each(|elm| *elm = random());


        let coinbase_input = Input { txid: [0; 32], signature,};
        let coinbase_output = Output { amount: REWARD+fees, address: self.address };


        inputs.push(coinbase_input);
        outputs.push(coinbase_output);
        let txid = Tx::generate_txid(&inputs, &outputs);
        Tx { txid, inputs, outputs }
    }

    fn gen_valid_hash(index: u32, previous_hash: [u8;32], target: u64) -> ([u8;32],u64) {
        let (mut hash, mut nonce) = Miner::gen_hash_nonce(index,previous_hash);
        while Miner::h2_u64(hash) > target {
            (hash, nonce) = Miner::gen_hash_nonce(index,previous_hash);
        }

        (hash, nonce)
    }

    fn gen_hash_nonce(index: u32, previous_hash: [u8;32]) -> ([u8;32],u64) {
        let mut hasher = blake3::Hasher::new();
        let nonce: u64 = random();
        hasher.update(&index.to_be_bytes());
        hasher.update(&previous_hash);
        hasher.update(&nonce.to_be_bytes());
        (*hasher.finalize().as_bytes(),nonce)
    }

    fn h2_u64(hash: [u8; 32]) -> u64 {
        let mut value: u64 = 0;
        for i in 0..8 {
            value |= (hash[i] as u64) << (8 * (7 - i)); // Shifting from the most significant byte to the least
        }
        value
    }
}
