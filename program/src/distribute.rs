use ore_stake_api::prelude::*;
use steel::*;

/// Distributes ORE from the sender to the treasury for distribution to stakers.
pub fn process_distribute(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = Distribute::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);
    if amount == 0 {
        return Err(OreStakeError::AmountZero.into());
    }

    // Load accounts.
    let clock = Clock::get()?;
    let [signer_info, sender_info, ore_mint_info, treasury_info, treasury_tokens_info, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    sender_info
        .as_associated_token_account(&signer_info.key, &MINT_ADDRESS)?
        .assert(|s| s.amount() >= amount)?;
    ore_mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    let treasury = treasury_info
        .as_account_mut::<Treasury>(&ore_stake_api::ID)?
        .assert_mut_err(|t| t.total_staked > 0, OreStakeError::NoDeposits.into())?;
    treasury_tokens_info.as_associated_token_account(&treasury_info.key, &MINT_ADDRESS)?;
    token_program.is_program(&spl_token::ID)?;

    // Update rewards factor.
    let total_staked = treasury.total_staked;
    treasury.rewards_factor += Numeric::from_fraction(amount, total_staked);

    // Transfer tokens to treasury for distribution to stakers
    transfer(
        signer_info,
        sender_info,
        treasury_tokens_info,
        token_program,
        amount,
    )?;

    // Log event.
    program_log(
        &[treasury_info.clone()],
        DistributeEvent {
            disc: 2,
            amount,
            total_staked,
            ts: clock.unix_timestamp,
        }
        .to_bytes(),
    )?;

    Ok(())
}
