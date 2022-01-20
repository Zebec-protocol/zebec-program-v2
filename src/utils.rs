use solana_program::{
    pubkey::Pubkey,
    account_info::{AccountInfo},
    system_instruction,
    program::{invoke_signed,invoke},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::Pack
};
use super::error::TokenError;
use arrayref::array_ref;

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

pub fn get_token_withdraw_data_and_bump_seed(
    prefix: &str,
    sender: &Pubkey,
    token_mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            prefix.as_bytes(),
            &sender.to_bytes(),
            &token_mint.to_bytes()
        ],
        program_id,
    )
}
pub fn get_multisig_data_and_bump_seed(
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
pub fn check_data_len(data: &[u8], min_len: usize) -> Result<(), ProgramError> {
    if data.len() < min_len {
        Err(ProgramError::AccountDataTooSmall)
    } else {
        Ok(())
    }
}
pub fn get_token_balance(token_account: &AccountInfo) -> Result<u64, ProgramError> {
    let data = token_account.try_borrow_data()?;
    check_data_len(&data, spl_token::state::Account::get_packed_len())?;
    let amount = array_ref![data, 64, 8];
    Ok(u64::from_le_bytes(*amount))
}