use std::str::FromStr;

use ore_stake_api::prelude::*;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    client_error::{reqwest::StatusCode, ClientErrorKind},
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::pubkey;
use solana_sdk::{
    address_lookup_table::{state::AddressLookupTable, AddressLookupTableAccount},
    compute_budget::ComputeBudgetInstruction,
    message::{v0::Message, VersionedMessage},
    native_token::lamports_to_sol,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signature, Signer},
    transaction::{Transaction, VersionedTransaction},
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::amount_to_ui_amount;
use steel::{AccountDeserialize, AccountMeta, Clock, Discriminator, Instruction};

#[tokio::main]
async fn main() {
    // Read keypair from file
    let payer =
        read_keypair_file(&std::env::var("KEYPAIR").expect("Missing KEYPAIR env var")).unwrap();

    // Build transaction
    let rpc = RpcClient::new(std::env::var("RPC").expect("Missing RPC env var"));
    match std::env::var("COMMAND")
        .expect("Missing COMMAND env var")
        .as_str()
    {
        "initialize" => {
            initialize(&rpc, &payer).await.unwrap();
        }
        "treasury" => {
            log_treasury(&rpc).await.unwrap();
        }
        "stake" => {
            log_stake(&rpc, &payer).await.unwrap();
        }
        "ata" => {
            ata(&rpc, &payer).await.unwrap();
        }
        "lut" => {
            lut(&rpc, &payer).await.unwrap();
        }
        "validate" => {
            validate(&rpc, &payer).await.unwrap();
        }
        _ => panic!("Invalid command"),
    };
}

async fn validate(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let stakes = get_stakes(rpc).await?;
    let treasury = get_treasury(rpc).await?;
    let mut total_rewards = 0;
    let mut total_deposits = 0;
    for (i, (address, mut stake)) in stakes.iter().enumerate() {
        stake.update_rewards(&treasury);
        let ata = get_associated_token_address(&address, &MINT_ADDRESS);
        let balance = rpc.get_token_account_balance(&ata).await?;
        assert!(balance.amount.parse::<u64>().unwrap() >= stake.balance);
        total_rewards += stake.rewards;
        total_deposits += stake.balance;
        println!("✅ {}: {}", i, stake.authority);
    }

    let treasury_ata = get_associated_token_address(&treasury_pda().0, &MINT_ADDRESS);
    let treasury_balance = rpc.get_token_account_balance(&treasury_ata).await?;
    assert!(treasury_balance.amount.parse::<u64>().unwrap() >= total_rewards);
    assert_eq!(treasury.total_staked, total_deposits);
    println!(
        "✅ Treasury rewards: {} ORE ==  {} ORE",
        amount_to_ui_amount(total_rewards, TOKEN_DECIMALS),
        treasury_balance.ui_amount_string
    );
    println!(
        "✅ Treasury deposits: {} ORE ==  {} ORE",
        amount_to_ui_amount(total_deposits, TOKEN_DECIMALS),
        amount_to_ui_amount(treasury.total_staked, TOKEN_DECIMALS)
    );

    Ok(())
}

async fn initialize(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let ix = ore_stake_api::sdk::initialize(payer.pubkey());
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn lut(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let recent_slot = rpc.get_slot().await? - 4;
    let (ix, lut_address) = solana_address_lookup_table_interface::instruction::create_lookup_table(
        payer.pubkey(),
        payer.pubkey(),
        recent_slot,
    );
    let ex_ix = solana_address_lookup_table_interface::instruction::extend_lookup_table(
        lut_address,
        payer.pubkey(),
        Some(payer.pubkey()),
        vec![
            pubkey!("HNWhK5f8RMWBqcA7mXJPaxdTPGrha3rrqUrri7HSKb3T"),
            pubkey!("2wQ7J46uwK3VyrmAYe5E8KhCjTg8CTaFimh1ty2huuyY"),
            pubkey!("DJqfQWB8tZE6fzqWa8okncDh7ciTuD8QQKp1ssNETWee"),
            pubkey!("HLaJ3RiyoaxQzwJQbU2Gc5RTZtx8HKAMJgkf57qdgpFJ"),
            pubkey!("8yS5zJTZa1Q1zQ1jsEAUnjAyMZfsNwvrgbDQp1ky2dr"),
            pubkey!("7qBS6huLjjGyrnMMBNXpLZA73yiGc6ao9znj7f9RpF1L"),
            pubkey!("3Mt1bpU3fnSXyPEm66HKKXyQTpLWrwYziPLqwTqK4ZT7"),
            pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo"),
            pubkey!("oreoU2P8bN6jkk3jbaiVxYnG1dCXcYxwhwyK9jSybcp"),
            pubkey!("So11111111111111111111111111111111111111112"),
            pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
            pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"),
            pubkey!("D1ZN9Wj1fRSUQfCjhvnu1hqDMT7hzjzBBpi12nVniYD6"),
            pubkey!("8kqLv9cBUDCYEKCL3Dj2MkeXX3tdCqT8KZ3gpYp8BnGP"),
            pubkey!("H38TVzkjAiAhBZR5SksbW8XDXP3N1ez4Tuna7uAW1Tsw"),
            pubkey!("11111111111111111111111111111111"),
            pubkey!("SysvarRent111111111111111111111111111111111"),
        ],
    );
    let ix_1 = Instruction {
        program_id: ix.program_id,
        accounts: ix
            .accounts
            .iter()
            .map(|a| AccountMeta::new(a.pubkey, a.is_signer))
            .collect(),
        data: ix.data,
    };
    let ix_2 = Instruction {
        program_id: ex_ix.program_id,
        accounts: ex_ix
            .accounts
            .iter()
            .map(|a| AccountMeta::new(a.pubkey, a.is_signer))
            .collect(),
        data: ex_ix.data,
    };
    submit_transaction(rpc, payer, &[ix_1, ix_2]).await?;
    println!("LUT address: {}", lut_address);
    Ok(())
}

async fn log_stake(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").unwrap_or(payer.pubkey().to_string());
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let treasury = get_treasury(&rpc).await?;
    let staker_address = ore_stake_api::state::stake_pda(authority).0;
    let mut stake = get_stake(rpc, authority).await?;
    stake.update_rewards(&treasury);
    println!("Stake");
    println!("  address: {}", staker_address);
    println!("  authority: {}", authority);
    println!(
        "  balance: {} ORE",
        amount_to_ui_amount(stake.balance, TOKEN_DECIMALS)
    );
    println!(
        "  compound_fee_reserve: {} SOL",
        lamports_to_sol(stake.compound_fee_reserve)
    );
    println!("  last_claim_at: {}", stake.last_claim_at);
    println!("  last_deposit_at: {}", stake.last_deposit_at);
    println!("  last_withdraw_at: {}", stake.last_withdraw_at);
    println!(
        "  rewards_factor: {}",
        stake.rewards_factor.to_i80f48().to_string()
    );
    println!(
        "  rewards: {} ORE",
        amount_to_ui_amount(stake.rewards, TOKEN_DECIMALS)
    );
    println!(
        "  lifetime_rewards: {} ORE",
        amount_to_ui_amount(stake.lifetime_rewards, TOKEN_DECIMALS)
    );

    Ok(())
}

async fn ata(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let user = pubkey!("FgZFnb3bi7QexKCdXWPwWy91eocUD7JCFySHb83vLoPD");
    let token = pubkey!("8H8rPiWW4iTFCfEkSnf7jpqeNpFfvdH9gLouAL3Fe2Zx");
    let ata = get_associated_token_address(&user, &token);
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        &user,
        &token,
        &spl_token::ID,
    );
    submit_transaction(rpc, payer, &[ix]).await?;
    let account = rpc.get_account(&ata).await?;
    println!("ATA: {}", ata);
    println!("Account: {:?}", account);
    Ok(())
}

#[allow(dead_code)]
pub async fn get_address_lookup_table_accounts(
    rpc_client: &RpcClient,
    addresses: Vec<Pubkey>,
) -> Result<Vec<AddressLookupTableAccount>, anyhow::Error> {
    let mut accounts = Vec::new();
    for key in addresses {
        if let Ok(account) = rpc_client.get_account(&key).await {
            if let Ok(address_lookup_table_account) = AddressLookupTable::deserialize(&account.data)
            {
                accounts.push(AddressLookupTableAccount {
                    key,
                    addresses: address_lookup_table_account.addresses.to_vec(),
                });
            }
        }
    }
    Ok(accounts)
}
async fn log_treasury(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let treasury_address = ore_stake_api::state::treasury_pda().0;
    let treasury = get_treasury(rpc).await?;
    println!("Treasury");
    println!("  address: {}", treasury_address);
    println!(
        "  stake_rewards_factor: {}",
        treasury.stake_rewards_factor.to_i80f48().to_string()
    );
    println!(
        "  total_staked: {} ORE",
        amount_to_ui_amount(treasury.total_staked, TOKEN_DECIMALS)
    );
    Ok(())
}

async fn get_stakes(rpc: &RpcClient) -> Result<Vec<(Pubkey, Stake)>, anyhow::Error> {
    let stakes =
        get_program_accounts::<ore_stake_api::prelude::Stake>(rpc, ore_stake_api::ID, vec![])
            .await?;
    Ok(stakes)
}

async fn log_clock(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let clock = get_clock(&rpc).await?;
    println!("Clock");
    println!("  slot: {}", clock.slot);
    println!("  epoch_start_timestamp: {}", clock.epoch_start_timestamp);
    println!("  epoch: {}", clock.epoch);
    println!("  leader_schedule_epoch: {}", clock.leader_schedule_epoch);
    println!("  unix_timestamp: {}", clock.unix_timestamp);
    Ok(())
}

async fn get_treasury(rpc: &RpcClient) -> Result<Treasury, anyhow::Error> {
    let treasury_pda = ore_stake_api::state::treasury_pda();
    let account = rpc.get_account(&treasury_pda.0).await?;
    let treasury = Treasury::try_from_bytes(&account.data)?;
    Ok(*treasury)
}

async fn get_clock(rpc: &RpcClient) -> Result<Clock, anyhow::Error> {
    let data = rpc.get_account_data(&solana_sdk::sysvar::clock::ID).await?;
    let clock = bincode::deserialize::<Clock>(&data)?;
    Ok(clock)
}

async fn get_stake(rpc: &RpcClient, authority: Pubkey) -> Result<Stake, anyhow::Error> {
    let stake_pda = ore_stake_api::state::stake_pda(authority);
    let account = rpc.get_account(&stake_pda.0).await?;
    let stake = Stake::try_from_bytes(&account.data)?;
    Ok(*stake)
}

#[allow(dead_code)]
async fn simulate_transaction(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
) {
    let blockhash = rpc.get_latest_blockhash().await.unwrap();
    let x = rpc
        .simulate_transaction(&Transaction::new_signed_with_payer(
            instructions,
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        ))
        .await;
    println!("Simulation result: {:?}", x);
}

#[allow(dead_code)]
async fn simulate_transaction_with_address_lookup_tables(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
    address_lookup_table_accounts: Vec<AddressLookupTableAccount>,
) {
    let blockhash = rpc.get_latest_blockhash().await.unwrap();
    let tx = VersionedTransaction {
        signatures: vec![Signature::default()],
        message: VersionedMessage::V0(
            Message::try_compile(
                &payer.pubkey(),
                instructions,
                &address_lookup_table_accounts,
                blockhash,
            )
            .unwrap(),
        ),
    };
    let s = tx.sanitize();
    println!("Sanitize result: {:?}", s);
    s.unwrap();
    let x = rpc.simulate_transaction(&tx).await;
    println!("Simulation result: {:?}", x);
}

#[allow(unused)]
async fn submit_transaction_batches(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    mut ixs: Vec<solana_sdk::instruction::Instruction>,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    // Batch and submit the instructions.
    while !ixs.is_empty() {
        let batch = ixs
            .drain(..std::cmp::min(batch_size, ixs.len()))
            .collect::<Vec<Instruction>>();
        submit_transaction_no_confirm(rpc, payer, &batch).await?;
        // submit_transaction(rpc, payer, &batch).await?;
    }
    Ok(())
}

#[allow(unused)]
async fn simulate_transaction_batches(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    mut ixs: Vec<solana_sdk::instruction::Instruction>,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    // Batch and submit the instructions.
    while !ixs.is_empty() {
        let batch = ixs
            .drain(..std::cmp::min(batch_size, ixs.len()))
            .collect::<Vec<Instruction>>();
        simulate_transaction(rpc, payer, &batch).await;
    }
    Ok(())
}

async fn submit_transaction(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
) -> Result<solana_sdk::signature::Signature, anyhow::Error> {
    let blockhash = rpc.get_latest_blockhash().await?;
    let mut all_instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
        ComputeBudgetInstruction::set_compute_unit_price(1_000_000),
    ];
    all_instructions.extend_from_slice(instructions);
    let transaction = Transaction::new_signed_with_payer(
        &all_instructions,
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );

    match rpc.send_and_confirm_transaction(&transaction).await {
        Ok(signature) => {
            println!("Transaction submitted: {:?}", signature);
            Ok(signature)
        }
        Err(e) => {
            println!("Error submitting transaction: {:?}", e);
            Err(e.into())
        }
    }
}

async fn submit_transaction_no_confirm(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
) -> Result<solana_sdk::signature::Signature, anyhow::Error> {
    let blockhash = rpc.get_latest_blockhash().await?;
    let mut all_instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
        ComputeBudgetInstruction::set_compute_unit_price(1_000_000),
    ];
    all_instructions.extend_from_slice(instructions);
    let transaction = Transaction::new_signed_with_payer(
        &all_instructions,
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );

    match rpc.send_transaction(&transaction).await {
        Ok(signature) => {
            println!("Transaction submitted: {:?}", signature);
            Ok(signature)
        }
        Err(e) => {
            println!("Error submitting transaction: {:?}", e);
            Err(e.into())
        }
    }
}

pub async fn get_program_accounts<T>(
    client: &RpcClient,
    program_id: Pubkey,
    filters: Vec<RpcFilterType>,
) -> Result<Vec<(Pubkey, T)>, anyhow::Error>
where
    T: AccountDeserialize + Discriminator + Clone,
{
    let mut all_filters = vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        0,
        &T::discriminator().to_le_bytes(),
    ))];
    all_filters.extend(filters);
    let result = client
        .get_program_accounts_with_config(
            &program_id,
            RpcProgramAccountsConfig {
                filters: Some(all_filters),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await;

    match result {
        Ok(accounts) => {
            let accounts = accounts
                .into_iter()
                .filter_map(|(pubkey, account)| {
                    if let Ok(account) = T::try_from_bytes(&account.data) {
                        Some((pubkey, account.clone()))
                    } else {
                        None
                    }
                })
                .collect();
            Ok(accounts)
        }
        Err(err) => match err.kind {
            ClientErrorKind::Reqwest(err) => {
                if let Some(status_code) = err.status() {
                    if status_code == StatusCode::GONE {
                        panic!(
                                "\n{} Your RPC provider does not support the getProgramAccounts endpoint, needed to execute this command. Please use a different RPC provider.\n",
                                "ERROR"
                            );
                    }
                }
                return Err(anyhow::anyhow!("Failed to get program accounts: {}", err));
            }
            _ => return Err(anyhow::anyhow!("Failed to get program accounts: {}", err)),
        },
    }
}
