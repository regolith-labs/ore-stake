use ore_mint_api::consts::MINT_ADDRESS;
use ore_stake_api::prelude::*;
use solana_program::rent::Rent;
use steel::*;

/// Compounds yield from the staking contract.
pub fn process_compound(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let clock = Clock::get()?;
    let [signer_info, mint_info, stake_info, stake_tokens_info, treasury_info, treasury_tokens_info, vesting_info, system_program, token_program, ore_stake_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    let stake = stake_info
        .as_account_mut::<Stake>(&ore_stake_api::ID)?
        .assert_mut(|s| s.compound_fee > 0)?
        .assert_mut(|s| s.compound_fee_reserve >= s.compound_fee)?
        .assert_mut(|s| s.last_claim_at + ONE_DAY < clock.unix_timestamp)?;
    stake_tokens_info
        .is_writable()?
        .as_associated_token_account(stake_info.key, mint_info.key)?;
    let treasury = treasury_info
        .has_address(&treasury_pda().0)?
        .as_account_mut::<Treasury>(&ore_stake_api::ID)?;
    let treasury_tokens = treasury_tokens_info
        .is_writable()?
        .as_associated_token_account(&treasury_info.key, &mint_info.key)?;
    let vesting = vesting_info
        .has_address(&vesting_pda().0)?
        .as_account_mut::<Vesting>(&ore_stake_api::ID)?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    ore_stake_program.is_program(&ore_stake_api::ID)?;

    // Claim yield from stake account.
    let amount = stake.claim(u64::MAX, &clock, treasury, vesting);

    // If no yield to compound, return.
    if amount == 0 {
        return Ok(());
    }

    // Deposit into stake account.
    let amount = stake.deposit(amount, &clock, treasury, &treasury_tokens, vesting);

    // Transfer ORE rewards from treasury to stake deposits.
    transfer_signed(
        treasury_info,
        treasury_tokens_info,
        stake_tokens_info,
        token_program,
        amount,
        &[TREASURY],
    )?;

    // Safety check.
    let stake_tokens =
        stake_tokens_info.as_associated_token_account(stake_info.key, mint_info.key)?;
    if stake_tokens.amount() < stake.balance {
        return Err(OreStakeError::InsufficientReserves.into());
    }

    // Check for rent exemption.
    let minimum_rent = Rent::get()?.minimum_balance(stake_info.data_len());
    if stake_info.lamports() < minimum_rent + stake.compound_fee {
        return Err(ProgramError::InsufficientFunds);
    }

    // Deduct compound fee from stake account.
    stake.compound_fee_reserve -= stake.compound_fee;
    stake_info.send(stake.compound_fee, &signer_info);

    // Log event.
    program_log(
        &[treasury_info.clone()],
        CompoundEvent {
            disc: OreStakeEvent::Compound as u64,
            authority: stake.authority,
            amount,
            ts: clock.unix_timestamp,
        }
        .to_bytes(),
    )?;

    Ok(())
}
