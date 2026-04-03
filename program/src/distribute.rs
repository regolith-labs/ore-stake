use ore_stake_api::prelude::*;
use steel::*;

/// Distributes ORE from the treasury to the sender.
pub fn process_distribute(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = Distribute::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer_info, sender_info, ore_mint_info, treasury_info, treasury_tokens_info, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    sender_info
        .as_associated_token_account(&signer_info.key, &MINT_ADDRESS)?
        .assert_mut(|s| s.amount() >= amount)?;
    ore_mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    let treasury = treasury_info.as_account_mut::<Treasury>(&ore_stake_api::ID)?;
    treasury_tokens_info.as_associated_token_account(&treasury_info.key, &MINT_ADDRESS)?;
    token_program.is_program(&spl_token::ID)?;

    // Update treasury.
    if treasury.total_staked > 0 {
        treasury.stake_rewards_factor += Numeric::from_fraction(amount, treasury.total_staked);
    }

    // Create treasury.
    transfer(
        signer_info,
        sender_info,
        treasury_tokens_info,
        token_program,
        amount,
    )?;

    Ok(())
}
