use steel::*;

/// Migrates the treasury from the old staking contract to the new one.
pub fn process_migrate_treasury(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, old_treasury_info, treasury_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    let old_treasury = old_treasury_info.as_account::<ore_api::prelude::Treasury>(&ore_api::ID)?;
    let treasury =
        treasury_info.as_account_mut::<ore_stake_api::prelude::Treasury>(&ore_stake_api::ID)?;

    treasury.total_staked = old_treasury.total_staked;
    treasury.stake_rewards_factor = old_treasury.stake_rewards_factor;

    Ok(())
}
