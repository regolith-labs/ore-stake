use std::str::FromStr;

use ore_mint_api::consts::{MINT_ADDRESS, TOKEN_DECIMALS};
use ore_stake_api::prelude::*;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::program_pack::Pack;
use solana_sdk::{
    commitment_config::CommitmentConfig,
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
        "stakes" => {
            log_stakes_by_authority(&rpc, &payer).await.unwrap();
        }
        "audit" => {
            audit_reserves(&rpc).await.unwrap();
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

async fn log_stakes_by_authority(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").unwrap_or(payer.pubkey().to_string());
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let mut treasury = get_treasury(rpc).await?;
    let clock = get_clock(rpc).await?;
    let mut vesting = get_vesting(rpc).await?;
    let accounts = get_stakes_by_authority(rpc, authority).await?;

    println!(
        "Found {} stake account(s) for authority {}",
        accounts.len(),
        authority
    );
    let mut rewards = 0;
    for (address, mut stake) in accounts {
        stake.update_rewards(&clock, &mut treasury, &mut vesting);
        println!();
        println!("Stake");
        println!("  address: {}", address);
        println!("  authority: {}", stake.authority);
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

        rewards += stake.lifetime_rewards;
    }
    println!(
        "Total rewards: {} ORE",
        amount_to_ui_amount(rewards, TOKEN_DECIMALS)
    );

    Ok(())
}

async fn audit_reserves(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    // Fetch global state.
    let mut treasury = get_treasury(rpc).await?;
    let clock = get_clock(rpc).await?;
    let mut vesting = get_vesting(rpc).await?;

    // Fetch all stake accounts.
    let all_stakes = get_all_stakes(rpc).await?;
    println!("Found {} stake accounts", all_stakes.len());

    // Collect all stake token ATA addresses for batch fetch.
    let ata_addresses: Vec<Pubkey> = all_stakes
        .iter()
        .map(|(stake_address, _)| {
            spl_associated_token_account::get_associated_token_address(stake_address, &MINT_ADDRESS)
        })
        .collect();

    // Batch fetch all ATAs in chunks of 1000, aggregating results into a single Vec.
    let mut ata_accounts = Vec::with_capacity(ata_addresses.len());
    for chunk in ata_addresses.chunks(1000) {
        let chunk_accounts = rpc.get_multiple_accounts(chunk).await?;
        ata_accounts.extend(chunk_accounts);
    }

    // Audit each stake account.
    let mut total_balance: u64 = 0;
    let mut total_rewards: u64 = 0;
    let mut deposit_failures: Vec<(Pubkey, u64, u64)> = Vec::new();
    let mut pda_failures: Vec<(Pubkey, Pubkey)> = Vec::new();

    for (i, (stake_address, mut stake)) in all_stakes.into_iter().enumerate() {
        // Verify PDA derivation.
        let expected_pda = ore_stake_api::state::stake_pda(stake.authority).0;
        if stake_address != expected_pda {
            pda_failures.push((stake_address, stake.authority));
        }

        // Update rewards to get current owed amount.
        stake.update_rewards(&clock, &mut treasury, &mut vesting);
        total_balance += stake.balance;
        total_rewards += stake.rewards;

        // Check stake token ATA has sufficient funds and correct authority.
        let (ata_balance, ata_authority) = match &ata_accounts[i] {
            Some(account) => match spl_token::state::Account::unpack(&account.data) {
                Ok(token_account) => (
                    token_account.amount,
                    token_account.owner, // This is the owner field, but authority is the delegate/owner for ATA
                ),
                Err(_) => (0, Pubkey::default()),
            },
            None => (0, Pubkey::default()),
        };

        // Verify the stake token ATA is owned by the stake account PDA.
        let stake_ata_pda = spl_associated_token_account::get_associated_token_address(
            &stake_address,
            &MINT_ADDRESS,
        );
        let expected_owner = stake_address;
        if ata_authority != expected_owner {
            println!(
                "  WARNING: Token account at {} has incorrect owner: {}, expected: {}",
                stake_ata_pda, ata_authority, expected_owner
            );
        }

        if ata_balance < stake.balance {
            deposit_failures.push((stake_address, stake.balance, ata_balance));
        }

        // assert!(ata_balance >= stake.balance);
        // assert!(ata_authority == expected_owner);
    }

    // Fetch treasury token balance.
    let treasury_tokens_address = ore_stake_api::state::treasury_tokens_address();
    let treasury_token_balance = match rpc.get_account(&treasury_tokens_address).await {
        Ok(account) => spl_token::state::Account::unpack(&account.data)
            .map(|t| t.amount)
            .unwrap_or(0),
        Err(_) => 0,
    };

    // Print results.
    println!();
    println!("=== Deposit Reserve Audit ===");
    println!(
        "  Total staked (sum of balances): {} ORE",
        amount_to_ui_amount(total_balance, TOKEN_DECIMALS)
    );
    println!(
        "  Treasury total_staked field:    {} ORE",
        amount_to_ui_amount(treasury.total_staked, TOKEN_DECIMALS)
    );
    if total_balance == treasury.total_staked {
        println!("  PASS: total_staked matches sum of stake balances");
    } else {
        println!(
            "  FAIL: mismatch! diff = {} ORE",
            amount_to_ui_amount(
                total_balance.abs_diff(treasury.total_staked),
                TOKEN_DECIMALS
            )
        );
    }

    println!();
    if deposit_failures.is_empty() {
        println!(
            "  PASS: All {} stake token accounts have sufficient deposit reserves",
            ata_addresses.len()
        );
    } else {
        println!(
            "  FAIL: {} stake account(s) with insufficient deposit reserves:",
            deposit_failures.len()
        );
        for (address, balance, ata_balance) in &deposit_failures {
            println!(
                "    {} — balance: {} ORE, ATA holds: {} ORE, deficit: {} ORE",
                address,
                amount_to_ui_amount(*balance, TOKEN_DECIMALS),
                amount_to_ui_amount(*ata_balance, TOKEN_DECIMALS),
                amount_to_ui_amount(balance - ata_balance, TOKEN_DECIMALS),
            );
        }
    }

    println!();
    println!("=== PDA Derivation Audit ===");
    if pda_failures.is_empty() {
        println!("  PASS: All stake accounts match their expected PDA");
    } else {
        println!(
            "  FAIL: {} account(s) with mismatched PDA:",
            pda_failures.len()
        );
        for (address, authority) in &pda_failures {
            println!("    account {} — authority {}", address, authority);
        }
    }

    println!();
    println!("=== Rewards Reserve Audit ===");
    println!(
        "  Total rewards owed:      {} ORE",
        amount_to_ui_amount(total_rewards, TOKEN_DECIMALS)
    );
    println!(
        "  Treasury token balance:  {} ORE",
        amount_to_ui_amount(treasury_token_balance, TOKEN_DECIMALS)
    );
    if treasury_token_balance >= total_rewards {
        println!(
            "  PASS: Treasury can cover all rewards (surplus: {} ORE)",
            amount_to_ui_amount(treasury_token_balance - total_rewards, TOKEN_DECIMALS)
        );
    } else {
        println!(
            "  FAIL: Treasury cannot cover rewards (deficit: {} ORE)",
            amount_to_ui_amount(total_rewards - treasury_token_balance, TOKEN_DECIMALS)
        );
    }

    println!();
    let all_pass = deposit_failures.is_empty()
        && pda_failures.is_empty()
        && treasury_token_balance >= total_rewards
        && total_balance == treasury.total_staked;
    if all_pass {
        println!("=== ALL AUDITS PASSED ===");
    } else {
        println!("=== SOME AUDITS FAILED ===");
    }

    Ok(())
}

async fn get_all_stakes(rpc: &RpcClient) -> Result<Vec<(Pubkey, Stake)>, anyhow::Error> {
    let filters = vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
        0,
        vec![OreAccount::Stake as u8],
    ))];
    let config = RpcProgramAccountsConfig {
        filters: Some(filters),
        account_config: RpcAccountInfoConfig {
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        },
        ..Default::default()
    };
    let accounts = rpc
        .get_program_accounts_with_config(&ore_stake_api::ID, config)
        .await?;
    let stakes = accounts
        .into_iter()
        .filter_map(|(pubkey, account)| {
            Stake::try_from_bytes(&account.data)
                .ok()
                .map(|s| (pubkey, *s))
        })
        .collect();
    Ok(stakes)
}

async fn get_stakes_by_authority(
    rpc: &RpcClient,
    authority: Pubkey,
) -> Result<Vec<(Pubkey, Stake)>, anyhow::Error> {
    let filters = vec![
        // Filter by account discriminator (Steel uses first byte = enum variant)
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, vec![OreAccount::Stake as u8])),
        // Filter by authority pubkey at offset 8 (after 8-byte discriminator)
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(8, authority.to_bytes().to_vec())),
    ];
    let config = RpcProgramAccountsConfig {
        filters: Some(filters),
        account_config: RpcAccountInfoConfig {
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        },
        ..Default::default()
    };
    let accounts = rpc
        .get_program_accounts_with_config(&ore_stake_api::ID, config)
        .await?;
    let stakes = accounts
        .into_iter()
        .filter_map(|(pubkey, account)| {
            Stake::try_from_bytes(&account.data)
                .ok()
                .map(|s| (pubkey, *s))
        })
        .collect();
    Ok(stakes)
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
