use solana_program::{
    pubkey::Pubkey,
    account_info::{AccountInfo},
    system_instruction,
    program::{invoke_signed,invoke},
    entrypoint::ProgramResult,
};
use super::error::TokenError;

pub fn get_master_address_and_bump_seed(
    sender: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &sender.to_bytes(),
        ],
        program_id,
    )
}

pub fn assert_keys_equal(key1: Pubkey, key2: Pubkey) -> ProgramResult {
    if key1 != key2 {
        Err(TokenError::PublicKeyMismatch.into())
    } else {
        Ok(())
    }
}
pub fn create_pda_account<'a>(
    payer: &AccountInfo<'a>,
    amount: u64,
    space: usize,
    owner: &Pubkey,
    system_program: &AccountInfo<'a>,
    new_pda_account: &AccountInfo<'a>,
) -> ProgramResult {
        invoke(
            &system_instruction::create_account(
                payer.key,
                new_pda_account.key,
                amount,
                space as u64,
                owner,
            ),
            &[
                payer.clone(),
                new_pda_account.clone(),
                system_program.clone(),
            ],
        )
    }

pub fn create_pda_account_signed<'a>(
    payer: &AccountInfo<'a>,
    amount: u64,
    space: usize,
    owner: &Pubkey,
    system_program: &AccountInfo<'a>,
    new_pda_account: &AccountInfo<'a>,
    seeds: &[&[u8]],
) -> ProgramResult {
        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                new_pda_account.key,
                amount,
                space as u64,
                owner,
            ),
            &[
                payer.clone(),
                new_pda_account.clone(),
                system_program.clone(),
            ],
            &[seeds],
        )
    }
pub fn create_transfer<'a>(
    sender: &AccountInfo<'a>,
    receiver: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    amount: u64,
    seeds: &[&[u8]],
) -> ProgramResult {
    invoke_signed(
        &system_instruction::transfer(
            sender.key,
            receiver.key,
            amount
        ),
        &[
            sender.clone(),
            receiver.clone(),
            system_program.clone()
        ],
        &[seeds],
    )
}

pub fn create_transfer_unsigned<'a>(
    sender: &AccountInfo<'a>,
    receiver: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    invoke(
        &system_instruction::transfer(
            sender.key,
            receiver.key,
            amount
        ),
        &[
            sender.clone(),
            receiver.clone(),
            system_program.clone()
        ],
    )
}
pub fn get_withdraw_data_and_bump_seed(
    prefix: &str,
    sender: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            prefix.as_bytes(),
            &sender.to_bytes(),
        ],
        program_id,
    )
}

pub fn get_multisig_data_and_bump_seed(
    sender: &Pubkey,
    pda: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &sender.to_bytes(),
            &pda.to_bytes()
        ],
        program_id,
    )
}
