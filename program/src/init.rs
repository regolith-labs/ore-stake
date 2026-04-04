use ore_stake_api::prelude::*;
use steel::*;

/// Initialize the program.
pub fn process_init(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, ore_mint_info, treasury_info, treasury_tokens_info, system_program, token_program, associated_token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    ore_mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    treasury_info.is_empty()?.is_writable()?;
    treasury_tokens_info.is_empty()?.is_writable()?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;

    // Create treasury.
    create_program_account::<Treasury>(
        treasury_info,
        system_program,
        signer_info,
        &ore_stake_api::ID,
        &[TREASURY],
    )?;

    // Create treasury tokens account.
    create_associated_token_account(
        signer_info,
        treasury_info,
        treasury_tokens_info,
        ore_mint_info,
        system_program,
        token_program,
        associated_token_program,
    )?;

    Ok(())
}
