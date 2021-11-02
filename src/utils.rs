use solana_program::{
    pubkey::Pubkey,
    account_info::{AccountInfo},
    system_instruction,
    program::{invoke_signed},
    program_error::ProgramError,
    entrypoint::ProgramResult,
};
use super::error::TokenError;

use crate::{
    PREFIX,
    PREFIX_ASSOCIATED
};
#[allow(clippy::too_many_arguments)]
pub fn initialize_token_account <'a>(
    token_program_info: &AccountInfo<'a>,
    token_mint_info: &AccountInfo<'a>,
    source_account_info: &AccountInfo<'a>,
    associated_token_address:&AccountInfo<'a>,
    rent_amount: u64,
    rent: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    seeds: &[&[u8]],
)-> Result<(), ProgramError>{
    // Creating associated token for pda - Owner PDA
    invoke_signed(
        &system_instruction::transfer(source_account_info.key,associated_token_address.key, rent_amount,), // SPL token space should be 165
        &[
            source_account_info.clone(),
            system_program.clone(),
            associated_token_address.clone()
        ],&[&seeds[..]],
    )?;
    invoke_signed(
        &system_instruction::allocate(associated_token_address.key, 165 as u64),
        &[associated_token_address.clone(), system_program.clone()],&[&seeds[..]],
    )?;
    invoke_signed(
        &system_instruction::assign(associated_token_address.key, token_program_info.key),
        &[associated_token_address.clone(), system_program.clone()],&[&seeds[..]],
    )?;    
    invoke_signed(
        &spl_token::instruction::initialize_account(
            token_program_info.key,
            associated_token_address.key,
            token_mint_info.key,
            owner.key,
        )?,
        &[
            associated_token_address.clone(),
            token_program_info.clone(),
            owner.clone(),
            rent.clone(),
            token_mint_info.clone(),
        ],&[&seeds[..]],
    )?;
    Ok(())
}



/// Returns Realm Token Holding PDA seeds
pub fn get_seeds<'a>(
    realm: &'a Pubkey,
) -> [&'a [u8]; 2] {
    [
        PREFIX.as_bytes(),
        realm.as_ref(),
        // governing_token_mint.as_ref(),
    ]
}

pub fn get_recipient_seeds<'a>(
    sender: &'a Pubkey,
    recipient: &'a Pubkey,
) -> [&'a [u8]; 2] {
    [
        sender.as_ref(),
        recipient.as_ref(),
    ]
}
pub fn get_account_address_and_bump_seed_internal(
    sender: &Pubkey,
    program_id: &Pubkey,
    recipient: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &sender.to_bytes(),
            &recipient.to_bytes(),
        ],
        program_id,
    )
}

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
pub fn get_account_token_address_and_bump_seed_internal(
    sender: &Pubkey,
    program_id: &Pubkey,
    recipient: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            PREFIX.as_bytes(),
            &sender.to_bytes(),
            &recipient.to_bytes(),
        ],
        program_id,
    )
}
pub fn get_master_token_address_and_bump_seed(
    sender: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            PREFIX.as_bytes(),
            &sender.to_bytes(),
        ],
        program_id,
    )
}
pub fn get_account_associated_token_address_and_bump_seed_internal(
    sender: &Pubkey,
    program_id: &Pubkey,
    recipient: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            PREFIX_ASSOCIATED.as_bytes(),
            &sender.to_bytes(),
            &recipient.to_bytes(),
        ],
        program_id,
    )
}
pub fn assert_keys_equal(key1: Pubkey, key2: Pubkey) -> ProgramResult {
    if key1 != key2 {
        return Err(TokenError::PublicKeyMismatch.into())
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
    new_pda_signer_seeds: &[&[u8]],
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
            &[new_pda_signer_seeds],
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