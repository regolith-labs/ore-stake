use ore_stake_api::prelude::*;
use steel::*;

/// Initialize the program.
pub fn process_init(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, ore_mint_info, treasury_info, treasury_tokens_info, vesting_info, system_program, token_program, associated_token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    ore_mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    treasury_info.is_empty()?.is_writable()?;
    treasury_tokens_info.is_empty()?.is_writable()?;
    vesting_info.is_empty()?.is_writable()?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;

    // Create treasury.
    if treasury_info.data_is_empty() {
        create_program_account::<Treasury>(
            treasury_info,
            system_program,
            signer_info,
            &ore_stake_api::ID,
            &[TREASURY],
        )?;
        let treasury = treasury_info.as_account_mut::<Treasury>(&ore_stake_api::ID)?;
        treasury.rewards_factor = Numeric::ZERO;
        treasury.total_staked = 0;
    } else {
        treasury_info.as_account_mut::<Treasury>(&ore_stake_api::ID)?;
    }

    // Create treasury tokens account.
    if treasury_tokens_info.data_is_empty() {
        create_associated_token_account(
            signer_info,
            treasury_info,
            treasury_tokens_info,
            ore_mint_info,
            system_program,
            token_program,
            associated_token_program,
        )?;
    } else {
        treasury_tokens_info.as_associated_token_account(&treasury_info.key, &ore_mint_info.key)?;
    }

    // Create vesting account.
    if vesting_info.data_is_empty() {
        create_program_account::<Vesting>(
            vesting_info,
            system_program,
            signer_info,
            &ore_stake_api::ID,
            &[VESTING],
        )?;
        let vesting = vesting_info.as_account_mut::<Vesting>(&ore_stake_api::ID)?;
        vesting.initial_amount = 0;
        vesting.start_time = 0;
        vesting.total_vested = 0;
    } else {
        vesting_info.as_account_mut::<Vesting>(&ore_stake_api::ID)?;
    }

    Ok(())
}
