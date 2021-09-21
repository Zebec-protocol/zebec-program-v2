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
pub fn spl_initialize<'a>(
    token_program: &AccountInfo<'a>,
    new_account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    rent: &AccountInfo<'a>,
) -> ProgramResult {
    let ix = initialize_account(token_program.key, new_account.key, mint.key, authority.key)?;
    msg!("token Program:{:?}, new_account:{:?}, {:?}, {:?},{:?}",token_program,new_account,mint,authority,rent);
    invoke(
        &ix,
        &[
            new_account.clone(),
            mint.clone(),
            authority.clone(),
            rent.clone(),
            token_program.clone(),
        ],
    )?;

    // msg!("token Program:{:?}, new_account:{:?}, {:?}, {:?},{:?}",token_program,new_account,mint,authority,rent);
    Ok(())
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
