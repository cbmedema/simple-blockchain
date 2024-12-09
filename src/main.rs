use std::time::{SystemTime, UNIX_EPOCH};

use num_format::{Locale, ToFormattedString};
use rayon::prelude::*;

use blockchain::Blockchain;
use miner::Miner;
use wallet::Wallet;

use crate::block::Block;
use crate::global_utxos::GlobalUtxos;

mod transactions;
mod wallet;
mod input;
mod output;
mod block;
mod miner;
mod blockchain;
mod mempool;
mod global_utxos;

const BLOCKS : u64=100;
const WALLETS: u64 = 500;
const OUTS_PER_WALLET: usize = 2;
const BOB_TX_AMOUNT: u64 = 5000000 / (WALLETS+1);
const NON_BOB_TX_AMOUNT: u64 = 1;
const MINING_FEE: u64 = 10;
const TARGET: u64 = 2u64.pow(64-5);
fn main() {
    test();
}

fn test() {
    let genesis_block = Block { index: 0, hash: [0; 32], previous_hash: [0; 32], time: 3, target: 4, nonce: 5, transactions: Vec::new() };
    let mut chain = Blockchain::create_from_genesis(genesis_block);

    let mut bob = Wallet::new();
    let bob_miner = Miner { address: bob.address(), threads: 1 };

    let mut wallets = vec![];
    for _ in 0..WALLETS {
        wallets.push(Wallet::new());
    }
    let mut pool = mempool::Mempool::new();
    let mut utxo_generator = GlobalUtxos::new();
    let  blockchain_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let mut start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let mut end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();

    let mut block_times = vec![];
    let mut utxo_generation_times = vec![];
    let mut utxo_times = vec![];
    let mut mempool_times = vec![];

    for block in 0..BLOCKS - 1 {
        start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        chain.add_block(bob_miner.generate_candidate_block(chain.get_height() + 1, chain.get_current_hash(), TARGET, &mut pool, &chain));
        end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        block_times.push(end-start);
        println!("Block: {:<4} added to the chain! {:>10} nanos ", block,(end-start).to_formatted_string(&Locale::en));

        start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        utxo_generator.find_utxos(&chain);
        end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        utxo_generation_times.push(end-start);
        println!("UTXO Generation Time            {:>10} nanos",(end-start).to_formatted_string(&Locale::en));
        if block > 0 {
            wallets.iter().for_each(|wallet| {
                start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
                let utxos = utxo_generator.get_utxos(&wallet.address()).unwrap().clone();
                end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
                utxo_times.push(end-start);

                let amounts = vec![NON_BOB_TX_AMOUNT; OUTS_PER_WALLET-1];
                let mut addresses = vec![];
                wallets.iter().for_each(|w| if w.address() != wallet.address() && addresses.len() < OUTS_PER_WALLET-1  {addresses.push(w.address())});

                start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
                pool.add_tx(wallet.clone().send_amounts(amounts, MINING_FEE, addresses.clone(), &utxos).unwrap(),&chain,&utxos);
                end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
                mempool_times.push(end-start);
            });
            println!("Mempool usage:                {} / {}    {:.2}%\n",pool.get_size().to_formatted_string(&Locale::en),mempool::MAX_MEMPOOL_SIZE, pool.get_size() as f64 *100.0  / mempool::MAX_MEMPOOL_SIZE as f64);
        }
        else {
            let utxos = utxo_generator.get_utxos(&bob.address()).unwrap();
            let mut addresses = vec![];
            let amounts = vec![BOB_TX_AMOUNT;WALLETS as usize];
            wallets.iter().for_each(|w|addresses.push(w.address()));
            pool.add_tx(bob.send_amounts(amounts, MINING_FEE, addresses, utxos).unwrap(),&chain, utxos);
        }
    }
    chain.add_block(bob_miner.generate_candidate_block(chain.get_height() + 1, chain.get_current_hash(), TARGET, &mut pool, &chain));
    end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    //chain.chain.last().unwrap().print();
    utxo_generator.find_utxos(&chain);
    bob.calc_balance(utxo_generator.get_utxos(&bob.address()).unwrap());

    println!("\n\n\nTime to generate {} blocks {} nanos", BLOCKS,(end - blockchain_start).to_formatted_string(&Locale::en));
    println!("Total Wallets:   {}   \nOutputs per Wallet: {}", WALLETS, OUTS_PER_WALLET);
    println!("Average time per block:    {} nanos",((end-blockchain_start) / BLOCKS as u128).to_formatted_string(&Locale::en));
    let blockchain_size: u32 = chain.chain.iter().map(|block|block.get_size()).sum();
    println!("Total size of blockchain:  {} bytes ",blockchain_size.to_formatted_string(&Locale::en));
    println!("Average size of block:     {} bytes \n",(blockchain_size as u64 / BLOCKS ).to_formatted_string(&Locale::en));
    let transaction_count = chain.chain.iter().flat_map(|block| {
        block.transactions.iter().map(|tx|tx)
    }).count();
    let transaction_sizes: u32 = chain.chain.iter().flat_map(|block|{
        block.transactions.iter().map(|tx|tx.get_size())
    }).sum();
    println!("Transactions in blockchain: {}",transaction_count.to_formatted_string(&Locale::en));
    println!("Total size of transactions: {}",transaction_sizes.to_formatted_string(&Locale::en));
    println!("Average transaction size: {} Bytes\n",(transaction_sizes / transaction_count as u32).to_formatted_string(&Locale::en));
    println!("Bob's balance of {} equals {} - {} + {} = {}",
        bob.get_balance().to_formatted_string(&Locale::en),
        (BLOCKS*5000000).to_formatted_string(&Locale::en),
        (BOB_TX_AMOUNT*WALLETS).to_formatted_string(&Locale::en),
        (MINING_FEE*(transaction_count as u64 -BLOCKS-1)).to_formatted_string(&Locale::en),
        bob.get_balance() == BLOCKS*5000000-BOB_TX_AMOUNT*WALLETS+MINING_FEE*(transaction_count as u64 -BLOCKS-1));

    let min: u128 = utxo_generation_times.iter().cloned().min().unwrap();
    let sum: u128 = utxo_generation_times.iter().sum();
    let count: u128 = utxo_generation_times.iter().len() as u128;
    let avg: u128 = sum / count;
    let max: u128 = utxo_generation_times.iter().cloned().max().unwrap();
    println!("{:<24} {:<18} {:<18} {:<18}",
         "UTXO Generation Times:",
         format!("Min {}", min.to_formatted_string(&Locale::en)),
         format!("Avg {}", avg.to_formatted_string(&Locale::en)),
         format!("Max {}", max.to_formatted_string(&Locale::en)));

    let min: u128 = utxo_times.iter().cloned().min().unwrap();
    let sum: u128 = utxo_times.iter().sum();
    let count: u128 = utxo_times.iter().len() as u128;
    let avg: u128 = sum / count;
    let max: u128 = utxo_times.iter().cloned().max().unwrap();
    println!("{:<24} {:<18} {:<18} {:<18}",
         "UTXO access Times:",
         format!("Min {}", min.to_formatted_string(&Locale::en)),
         format!("Avg {}", avg.to_formatted_string(&Locale::en)),
         format!("Max {}", max.to_formatted_string(&Locale::en)));


    let min: u128 = mempool_times.iter().cloned().min().unwrap();
    let sum: u128 = mempool_times.iter().sum();
    let count: u128 = mempool_times.iter().len() as u128;
    let avg: u128 = sum  / count;
    let max: u128 = mempool_times.iter().cloned().max().unwrap();
    println!("{:<24} {:<18} {:<18} {:<18}",
             "Mempool Insertion Times:",
             format!("Min {}", min.to_formatted_string(&Locale::en)),
             format!("Avg {}", avg.to_formatted_string(&Locale::en)),
             format!("Max {}", max.to_formatted_string(&Locale::en)));


    println!("\nAverage Mempool update time per Block {} nanos",(sum/BLOCKS as u128).to_formatted_string(&Locale::en));
    println!("Mempool is handling around {} Txs per second",((transaction_count as u128-BLOCKS as u128) * 1000000000 / sum ).to_formatted_string(&Locale::en));

}
