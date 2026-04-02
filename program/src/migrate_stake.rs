use steel::*;

/// Migrates ORE from the old staking contract to the new one.
pub fn process_migrate_stake(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [payer_info, mint_info, old_stake_info, old_stake_tokens_info, stake_info, stake_tokens_info, system_program, token_program, associated_token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    payer_info.is_signer()?;
    mint_info
        .has_address(&ore_api::prelude::MINT_ADDRESS)?
        .as_mint()?;
    let old_stake = old_stake_info
        .is_signer()?
        .as_account_mut::<ore_api::prelude::Stake>(&ore_api::ID)?;
    old_stake_tokens_info.as_associated_token_account(old_stake_info.key, mint_info.key)?;
    stake_info.is_writable()?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;

    // Create stake account.
    let stake = if stake_info.data_is_empty() {
        create_program_account::<ore_stake_api::prelude::Stake>(
            stake_info,
            system_program,
            &payer_info,
            &ore_stake_api::ID,
            &[
                ore_stake_api::prelude::STAKE,
                &old_stake.authority.to_bytes(),
            ],
        )?;
        let stake =
            stake_info.as_account_mut::<ore_stake_api::prelude::Stake>(&ore_stake_api::ID)?;
        stake
    } else {
        stake_info
            .as_account_mut::<ore_stake_api::prelude::Stake>(&ore_stake_api::ID)?
            .assert_mut(|s| s.authority == old_stake.authority)?
    };

    // Copy old stake data to new stake account.
    stake.authority = old_stake.authority;
    stake.balance = old_stake.balance;
    stake.compound_fee_reserve = old_stake.compound_fee_reserve;
    stake.last_claim_at = old_stake.last_claim_at;
    stake.last_deposit_at = old_stake.last_deposit_at;
    stake.last_withdraw_at = old_stake.last_withdraw_at;
    stake.rewards_factor = old_stake.rewards_factor;
    stake.rewards = old_stake.rewards;
    stake.lifetime_rewards = old_stake.lifetime_rewards;

    // Create stake tokens account.
    if stake_tokens_info.data_is_empty() {
        create_associated_token_account(
            payer_info,
            stake_info,
            stake_tokens_info,
            mint_info,
            system_program,
            token_program,
            associated_token_program,
        )?;
    } else {
        stake_tokens_info.as_associated_token_account(stake_info.key, mint_info.key)?;
    }

    Ok(())
}
