use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::sync::Arc;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rayon::prelude::*;

use crate::blockchain::Blockchain;

pub struct GlobalUtxos {
    // hash_table stores wallets addresses as keys, and utxos and values for quick lookup
    pub utxos: HashMap<[u8;32],Vec<(u64, [u8;32])>>,
    known_blockchain_height: u32,
    verifying_keys: Vec<[u8;32]>,
}

impl GlobalUtxos {
    pub fn new() -> GlobalUtxos {
        GlobalUtxos { utxos: HashMap::new(), verifying_keys: Vec::new(), known_blockchain_height: 0}
    }

    pub fn get_utxos(&mut self, address: &[u8;32]) -> Option<&Vec<(u64,[u8;32])>> { self.utxos.get(address) }

    pub fn find_utxos(&mut self, chain: &Blockchain){
        // updates verifying keys with all wallet addresses found in unknown part of blockchain
        chain.chain.iter().enumerate().filter(|(index,_)| *index as u32 > self.known_blockchain_height)
            .flat_map(|(_,block)| block.transactions.iter()).for_each(| tx| {
            tx.outputs.iter().for_each(|out| {
                self.utxos.entry(out.address)
                    .and_modify(|value| {
                        if !value.iter().any(|(_, txid)| *txid == tx.txid) {
                            value.push((out.amount, tx.txid));
                        }
                    })
                    .or_insert(vec![(out.amount, tx.txid)]);
                // if an output address is found that isn't known, it as added to the vector of keys
                if !self.verifying_keys.contains(&out.address) {
                    self.verifying_keys.push(out.address);
                }
            })
        });
        let utxos = Arc::new(Mutex::new(&mut self.utxos));
        chain.chain.iter().enumerate().filter(|(index, _)| *index as u32 > self.known_blockchain_height)
            .flat_map(|(_, block)| block.transactions.iter()).for_each(|tx| {
            tx.inputs.iter().for_each(|input| {
                // for each public key, we search for spent txs
                // only searches blockchain for blocks that are not known to optimize performance
                self.verifying_keys.par_iter().for_each(|key|
                {
                    // if an input signature is valid for a key, the transaction associated with it has been spent
                    // thus we remove it from the known unspent tx outputs
                    if VerifyingKey::from_bytes(&key).unwrap().verify(&input.txid, &Signature::from_bytes(&input.signature)).is_ok() {
                        let mut utxos_lock: MutexGuard<&mut HashMap<[u8;32],Vec<(u64, [u8;32])>>> = utxos.lock().unwrap();
                        utxos_lock.get_mut(key).unwrap().retain(|(_,txid)|*txid!=input.txid);
                    }
                })
            });
        });
        self.known_blockchain_height = chain.get_height();
    }
}

