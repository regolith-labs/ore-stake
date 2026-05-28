use std::str::FromStr;

use ore_mint_api::consts::TOKEN_DECIMALS;
use ore_stake_api::prelude::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    native_token::lamports_to_sol,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use spl_token::amount_to_ui_amount;
use steel::{AccountDeserialize, Clock};

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
        "init" => {
            init(&rpc, &payer).await.unwrap();
        }
        "treasury" => {
            log_treasury(&rpc).await.unwrap();
        }
        "stake" => {
            log_stake(&rpc, &payer).await.unwrap();
        }
        _ => panic!("Invalid command"),
    };
}

async fn init(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let ix = ore_stake_api::sdk::init(payer.pubkey());
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn log_stake(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").unwrap_or(payer.pubkey().to_string());
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let mut treasury = get_treasury(&rpc).await?;
    let clock = get_clock(rpc).await?;
    let mut vesting = get_vesting(rpc).await?;
    let staker_address = ore_stake_api::state::stake_pda(authority).0;
    let mut stake = get_stake(rpc, authority).await?;
    stake.update_rewards(&clock, &mut treasury, &mut vesting);
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

async fn log_treasury(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let treasury_address = ore_stake_api::state::treasury_pda().0;
    let treasury = get_treasury(rpc).await?;
    println!("Treasury");
    println!("  address: {}", treasury_address);
    println!(
        "  rewards_factor: {}",
        treasury.rewards_factor.to_i80f48().to_string()
    );
    println!(
        "  total_staked: {} ORE",
        amount_to_ui_amount(treasury.total_staked, TOKEN_DECIMALS)
    );
    Ok(())
}

async fn get_clock(rpc: &RpcClient) -> Result<Clock, anyhow::Error> {
    let data = rpc.get_account_data(&solana_sdk::sysvar::clock::ID).await?;
    let clock = bincode::deserialize::<Clock>(&data)?;
    Ok(clock)
}

async fn get_treasury(rpc: &RpcClient) -> Result<Treasury, anyhow::Error> {
    let treasury_pda = ore_stake_api::state::treasury_pda();
    let account = rpc.get_account(&treasury_pda.0).await?;
    let treasury = Treasury::try_from_bytes(&account.data)?;
    Ok(*treasury)
}

async fn get_stake(rpc: &RpcClient, authority: Pubkey) -> Result<Stake, anyhow::Error> {
    let stake_pda = ore_stake_api::state::stake_pda(authority);
    let account = rpc.get_account(&stake_pda.0).await?;
    let stake = Stake::try_from_bytes(&account.data)?;
    Ok(*stake)
}

async fn get_vesting(rpc: &RpcClient) -> Result<Vesting, anyhow::Error> {
    let vesting_pda = ore_stake_api::state::vesting_pda();
    let account = rpc.get_account(&vesting_pda.0).await?;
    let vesting = Vesting::try_from_bytes(&account.data)?;
    Ok(*vesting)
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
