mod claim_yield;
mod compound_yield;
mod deposit;
mod distribute;
mod initialize;
mod log;
mod migrate_stake;
mod migrate_treasury;
mod withdraw;

use claim_yield::*;
use compound_yield::*;
use deposit::*;
use distribute::*;
use initialize::*;
use log::*;
use migrate_stake::*;
use migrate_treasury::*;
use withdraw::*;

use ore_stake_api::instruction::*;
use solana_security_txt::security_txt;
use steel::*;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let (ix, data) = parse_instruction(&ore_stake_api::ID, program_id, data)?;

    match ix {
        // Staker
        OreStakeInstruction::Deposit => process_deposit(accounts, data)?,
        OreStakeInstruction::Withdraw => process_withdraw(accounts, data)?,
        OreStakeInstruction::ClaimYield => process_claim_yield(accounts, data)?,
        OreStakeInstruction::CompoundYield => process_compound_yield(accounts, data)?,

        // Misc
        OreStakeInstruction::Initialize => process_initialize(accounts, data)?,
        OreStakeInstruction::Log => process_log(accounts, data)?,
        OreStakeInstruction::Distribute => process_distribute(accounts, data)?,

        // Migration
        OreStakeInstruction::MigrateStake => process_migrate_stake(accounts, data)?,
        OreStakeInstruction::MigrateTreasury => process_migrate_treasury(accounts, data)?,
    }

    Ok(())
}

entrypoint!(process_instruction);

security_txt! {
    name: "ORE Stake",
    project_url: "https://ore.supply",
    contacts: "email:hardhatchad@gmail.com,discord:hardhatchad",
    policy: "https://github.com/regolith-labs/ore-stake/blob/master/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/regolith-labs/ore-stake"
}
