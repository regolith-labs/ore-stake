use ore_stake_api::prelude::*;
use steel::*;

/// No-op, use instruction data for logging w/o truncation.
pub fn process_log(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info
        .is_signer()?
        .as_account::<Treasury>(&ore_stake_api::ID)?;

    // For data integrity, only the treasury can log messages.

    Ok(())
}
