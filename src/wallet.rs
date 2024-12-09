use ed25519_dalek::{Signer, SigningKey, };
use ed25519_dalek::ed25519::signature::Keypair;
use ed25519_dalek::ed25519::SignatureEncoding;
use rand::rngs::OsRng;

use crate::input::Input;
use crate::output::Output;
use crate::transactions::{Tx, TxError};

#[derive(Clone)]
pub struct Wallet {
    signing_key: SigningKey,
    utxos: Vec<(u64,[u8;32])>,
    balance: u64,
}

impl Wallet {
    pub fn new() -> Self {
        let mut csprng = OsRng;  // Initialize random number generator
        let signing_key = SigningKey::generate(&mut csprng);
        Wallet { signing_key, utxos: Vec::new(), balance: 0}
    }

    pub fn address(&self) -> [u8;32] { self.signing_key.verifying_key().to_bytes() }

    pub fn get_balance(&self) -> u64 { self.balance }

    pub fn calc_balance(&mut self, updated_utxos: &Vec<(u64, [u8;32])>){
        self.utxos = updated_utxos.clone();
        self.balance = self.utxos.iter().map(|(amount,_)|*amount).sum();
    }

    fn generate_signature(&self, txid: &[u8;32]) -> [u8;64] {
        let message: &[u8] = txid;
        self.signing_key.sign(message).to_bytes()
    }

    pub fn send_amount(&mut self, amount: u64, mining_fee: u64, address: [u8;32], updated_utxos: &Vec<(u64, [u8;32])>) -> Result<Tx,TxError> {
        self.calc_balance(updated_utxos); // updates the wallets balance and finds correct utxos
        if self.balance >= amount + mining_fee {
            let mut inputs = vec![];
            let mut outputs = vec![];
            let mut sum = 0;
            let mut utxos_needed = 0;
            // calculates how many utxos are needed to have enough total value to complete the transaction
            for (index, (tx_amount, _)) in self.utxos.iter().enumerate() {
                sum += *tx_amount;
                if sum >= amount + mining_fee {
                    utxos_needed = index + 1;
                    break;
                }
            }
            // generates inputs for transaction sent to recipient address
            for (_, (_,txid)) in self.utxos.iter().enumerate().filter(|(i,_)| *i < utxos_needed) {
                let transaction_input = Input {
                    txid: *txid,
                    signature: self.generate_signature(&txid),
                };
                inputs.push(transaction_input);
            }

            outputs.push(Output { amount, address });
            // generates change if any exist
            if sum > amount {
                outputs.push(Output { amount: sum - amount - mining_fee, address: self.address() });
            }
            let txid = Tx::generate_txid(&inputs, &outputs);
            Ok(Tx { txid, inputs, outputs })
        }
        else {
            Err(TxError::InsufficientBalance)
        }
    }

    pub fn send_amounts(&mut self, amounts: Vec<u64>, mining_fee: u64, addresses: Vec<[u8;32]>, updated_utxos: &Vec<(u64, [u8;32])>) -> Result<Tx,TxError> {
        self.calc_balance(updated_utxos);
        let total_amount: u64 = amounts.iter().map(|amount|*amount).sum();
        if self.balance >= total_amount + mining_fee && (amounts.len() == addresses.len()){
            let num_addresses = amounts.len();
            let mut inputs = vec![];
            let mut outputs = vec![];
            let mut utxos_needed = 0;
            let mut sum_of_inputs = 0;
            // calculates how many utxos are needed to have enough total value to complete the transaction
            for (index, (tx_amount, _)) in self.utxos.iter().enumerate() {
                sum_of_inputs += *tx_amount;
                if sum_of_inputs >= total_amount + mining_fee {
                    utxos_needed = index + 1;
                    break;
                }
            }
            // generates inputs for transaction sent to recipient address
            for (_, (_,txid)) in self.utxos.iter().enumerate().filter(|(i,_)| *i < utxos_needed) {
                let transaction_input = Input {
                    txid: *txid,
                    signature: self.generate_signature(&txid),
                };
                inputs.push(transaction_input);
            }
            for i in 0..num_addresses{
                outputs.push(Output { amount: amounts[i], address: addresses[i] });
            }
            // final output is change back to sender, if change exists
            if sum_of_inputs > total_amount + mining_fee {
                outputs.push(Output { amount: sum_of_inputs - total_amount - mining_fee, address: self.address() });
            }
            // txid is simply hash of all inputs and outputs
            let txid = Tx::generate_txid(&inputs, &outputs);
            Ok(Tx { txid, inputs, outputs })

        }
        else{
            Err(TxError::InsufficientBalance)
        }
    }
}

