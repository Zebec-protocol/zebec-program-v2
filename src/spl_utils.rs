use solana_program::{
    pubkey::Pubkey,
    account_info::{AccountInfo},
    system_instruction,
    program::{invoke,invoke_signed},
    program_error::ProgramError,
    entrypoint::ProgramResult,
};
use super::error::TokenError;

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
    seeds: &[&[u8]]
)-> Result<(), ProgramError>{
    // Creating associated token for pda - Owner PDA
    invoke(
        &system_instruction::transfer(source_account_info.key,associated_token_address.key, rent_amount,), // SPL token space should be 165
        &[
            source_account_info.clone(),
            system_program.clone(),
            associated_token_address.clone()
        ]
    )?;
    invoke(
        &system_instruction::allocate(associated_token_address.key, 165 as u64),
        &[associated_token_address.clone(), system_program.clone()],
    )?;
    invoke(
        &system_instruction::assign(associated_token_address.key, token_program_info.key),
        &[associated_token_address.clone(), system_program.clone()]
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
) -> [&'a [u8]; 1] {
    [
        realm.as_ref(),
        // governing_token_mint.as_ref(),
    ]
}
pub fn assert_keys_equal(key1: Pubkey, key2: Pubkey) -> ProgramResult {
    if key1 != key2 {
        return Err(TokenError::PublicKeyMismatch.into())
    } else {
        Ok(())
    }
}