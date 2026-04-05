use ore_stake_api::prelude::*;
use steel::*;

/// Closes an empty stake account and returns rent to the payer.
pub fn process_close(accounts: &[AccountInfo<'_>]) -> ProgramResult {
    // Load accounts.
    let [signer_info, mint_info, recipient_info, stake_info, stake_tokens_info, system_program, token_program, associated_token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    recipient_info.is_writable()?;
    let stake = stake_info
        .as_account_mut::<Stake>(&ore_stake_api::ID)?
        .assert_mut(|s| s.authority == *signer_info.key)?
        .assert_mut(|s| s.balance == 0 && s.rewards == 0)?;
    let stake_tokens =
        stake_tokens_info.as_associated_token_account(stake_info.key, mint_info.key)?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;

    // Transfer any remaining ORE to recipient.
    if stake_tokens.amount() > 0 {
        // Open recipient token account.
        if recipient_info.data_is_empty() {
            create_associated_token_account(
                signer_info,
                signer_info,
                recipient_info,
                mint_info,
                system_program,
                token_program,
                associated_token_program,
            )?;
        } else {
            recipient_info.as_associated_token_account(&signer_info.key, &mint_info.key)?;
        }

        // Transfer ORE to recipient.
        transfer_signed(
            stake_info,
            stake_tokens_info,
            recipient_info,
            token_program,
            stake_tokens.amount(),
            &[STAKE, &stake.authority.to_bytes()],
        )?;
    }

    // Close stake token account.
    close_token_account_signed(
        stake_tokens_info,
        signer_info,
        stake_info,
        token_program,
        &[STAKE, &stake.authority.to_bytes()],
    )?;

    // Close stake account.
    stake_info.close(&signer_info)?;

    Ok(())
}
