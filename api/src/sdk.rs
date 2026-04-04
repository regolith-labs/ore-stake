use solana_program::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use steel::*;

use crate::{
    consts::{MINT_ADDRESS, TREASURY},
    instruction::*,
    state::*,
};

pub fn log(signer: Pubkey, msg: &[u8]) -> Instruction {
    let mut data = Log {}.to_bytes();
    data.extend_from_slice(msg);
    Instruction {
        program_id: crate::ID,
        accounts: vec![AccountMeta::new(signer, true)],
        data: data,
    }
}

pub fn program_log(accounts: &[AccountInfo], msg: &[u8]) -> Result<(), ProgramError> {
    invoke_signed(
        &log(*accounts[0].key, msg),
        accounts,
        &crate::ID,
        &[TREASURY],
    )
}

pub fn deposit(signer: Pubkey, payer: Pubkey, amount: u64, compound_fee: u64) -> Instruction {
    let mint_address = MINT_ADDRESS;
    let stake_address = stake_pda(signer).0;
    let stake_tokens_address = get_associated_token_address(&stake_address, &MINT_ADDRESS);
    let sender_address = get_associated_token_address(&signer, &MINT_ADDRESS);
    let treasury_address = treasury_pda().0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(payer, true),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(sender_address, false),
            AccountMeta::new(stake_address, false),
            AccountMeta::new(stake_tokens_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: Deposit {
            amount: amount.to_le_bytes(),
            compound_fee: compound_fee.to_le_bytes(),
        }
        .to_bytes(),
    }
}

// let [signer_info, mint_info, recipient_info, stake_info, stake_tokens_info, treasury_info, system_program, token_program, associated_token_program] =

pub fn withdraw(signer: Pubkey, amount: u64) -> Instruction {
    let stake_address = stake_pda(signer).0;
    let stake_tokens_address = get_associated_token_address(&stake_address, &MINT_ADDRESS);
    let mint_address = MINT_ADDRESS;
    let recipient_address = get_associated_token_address(&signer, &MINT_ADDRESS);
    let treasury_address = treasury_pda().0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(recipient_address, false),
            AccountMeta::new(stake_address, false),
            AccountMeta::new(stake_tokens_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: Withdraw {
            amount: amount.to_le_bytes(),
        }
        .to_bytes(),
    }
}

pub fn claim(signer: Pubkey, amount: u64) -> Instruction {
    let stake_address = stake_pda(signer).0;
    let mint_address = MINT_ADDRESS;
    let recipient_address = get_associated_token_address(&signer, &MINT_ADDRESS);
    let treasury_address = treasury_pda().0;
    let treasury_tokens_address = treasury_tokens_address();
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(recipient_address, false),
            AccountMeta::new(stake_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new(treasury_tokens_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: Claim {
            amount: amount.to_le_bytes(),
        }
        .to_bytes(),
    }
}

pub fn compound(signer: Pubkey) -> Instruction {
    let stake_address = stake_pda(signer).0;
    let mint_address = MINT_ADDRESS;
    let stake_tokens_address = get_associated_token_address(&stake_address, &MINT_ADDRESS);
    let treasury_address = treasury_pda().0;
    let treasury_tokens_address = treasury_tokens_address();
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(stake_address, false),
            AccountMeta::new(stake_tokens_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new(treasury_tokens_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
        data: Compound {}.to_bytes(),
    }
}

pub fn init(signer: Pubkey) -> Instruction {
    let mint_address = MINT_ADDRESS;
    let treasury_address = treasury_pda().0;
    let treasury_tokens_info = treasury_tokens_address();
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new(treasury_tokens_info, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: Init {}.to_bytes(),
    }
}

pub fn distribute(signer: Pubkey, amount: u64) -> Instruction {
    let sender_address = get_associated_token_address(&signer, &MINT_ADDRESS);
    let ore_mint_info = MINT_ADDRESS;
    let treasury_info = treasury_pda().0;
    let treasury_tokens_info = treasury_tokens_address();
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(sender_address, false),
            AccountMeta::new(ore_mint_info, false),
            AccountMeta::new(treasury_info, false),
            AccountMeta::new(treasury_tokens_info, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
        data: Distribute {
            amount: amount.to_le_bytes(),
        }
        .to_bytes(),
    }
}
