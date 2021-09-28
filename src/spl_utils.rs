use {
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        program::{invoke},
        msg
    },
    spl_token::instruction::{
        transfer,
        initialize_account
    },
};
use crate::{
    state::TokenInitializeAccountParams,
    error::TokenError
};
pub fn spl_token_init_account(params: TokenInitializeAccountParams<'_>) -> ProgramResult {
    let TokenInitializeAccountParams {
        account,
        mint,
        owner,
        rent,
        token_program,
    } = params;
    let ix = spl_token::instruction::initialize_account(
        token_program.key,
        account.key,
        mint.key,
        owner.key,
    )?;
    let result = invoke(&ix, &[account, mint, owner, rent, token_program]);
    result.map_err(|_| TokenError::TimeEnd.into())
}
pub fn spl_token_transfer<'a>(
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    if amount > 0 {
        let ix = transfer(
            token_program.key,
            source.key,
            destination.key,
            owner.key,
            &[],
            amount,
        )?;
        msg!("let me in");
        invoke(
            &ix,
            &[
                source.clone(),
                destination.clone(),
                owner.clone(),
                token_program.clone(),
            ],
        )?;
    }
    Ok(())
}
