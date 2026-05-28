use ore_mint_api::consts::MINT_ADDRESS;
use ore_stake_api::prelude::*;
use steel::*;

/// Claims yield from the staking contract.
pub fn process_claim(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = Claim::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let clock = Clock::get()?;
    let [signer_info, mint_info, recipient_info, stake_info, treasury_info, treasury_tokens_info, vesting_info, system_program, token_program, associated_token_program, ore_stake_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    recipient_info.is_writable()?;
    let stake = stake_info
        .as_account_mut::<Stake>(&ore_stake_api::ID)?
        .assert_mut(|s| s.authority == *signer_info.key)?;
    let treasury = treasury_info
        .has_address(&treasury_pda().0)?
        .as_account_mut::<Treasury>(&ore_stake_api::ID)?;
    treasury_tokens_info
        .is_writable()?
        .as_associated_token_account(&treasury_info.key, &mint_info.key)?;
    let vesting = vesting_info
        .has_address(&vesting_pda().0)?
        .as_account_mut::<Vesting>(&ore_stake_api::ID)?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;
    ore_stake_program.is_program(&ore_stake_api::ID)?;

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

    // Claim yield from stake account.
    let amount = stake.claim(amount, &clock, treasury, vesting);

    // Transfer ORE to recipient.
    transfer_signed(
        treasury_info,
        treasury_tokens_info,
        recipient_info,
        token_program,
        amount,
        &[TREASURY],
    )?;

    // Log event.
    program_log(
        &[treasury_info.clone()],
        ClaimEvent {
            disc: OreStakeEvent::Claim as u64,
            authority: *signer_info.key,
            amount,
            ts: clock.unix_timestamp,
        }
        .to_bytes(),
    )?;

    Ok(())
}
