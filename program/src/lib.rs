mod claim;
mod compound;
mod deposit;
mod distribute;
mod init;
mod log;
mod withdraw;

use claim::*;
use compound::*;
use deposit::*;
use distribute::*;
use init::*;
use log::*;
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
        OreStakeInstruction::Claim => process_claim(accounts, data)?,
        OreStakeInstruction::Compound => process_compound(accounts, data)?,

        // Misc
        OreStakeInstruction::Init => process_init(accounts, data)?,
        OreStakeInstruction::Log => process_log(accounts, data)?,
        OreStakeInstruction::Distribute => process_distribute(accounts, data)?,
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
