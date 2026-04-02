mod claim_yield;
mod compound_yield;
mod deposit;
mod log;
mod withdraw;

use claim_yield::*;
use compound_yield::*;
use deposit::*;
use log::*;
use withdraw::*;

use ore_api::instruction::*;
use solana_security_txt::security_txt;
use steel::*;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let (ix, data) = parse_instruction(&ore_api::ID, program_id, data)?;

    match ix {
        // Staker
        OreInstruction::Deposit => process_deposit(accounts, data)?,
        OreInstruction::Withdraw => process_withdraw(accounts, data)?,
        OreInstruction::ClaimYield => process_claim_yield(accounts, data)?,
        OreInstruction::CompoundYield => process_compound_yield(accounts, data)?,

        // Misc
        OreInstruction::Log => process_log(accounts, data)?,
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
