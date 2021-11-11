//! Program state processor
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{AccountInfo,next_account_info},
    program_error::{PrintProgramError,ProgramError},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    system_instruction::{create_account},
    program::{invoke,invoke_signed},
    pubkey::Pubkey,
    sysvar::{rent::Rent,fees::Fees,clock::Clock,Sysvar},
    msg,
    system_program,
};
use num_traits::FromPrimitive;
use crate::{
    instruction::{
        TokenInstruction,
        ProcessSolStream,
        ProcessSolWithdrawStream,
        ProcessTokenStream,
        ProcessTokenWithdrawStream,
        ProcessDepositSol,
        ProcessDepositToken,
        ProcessFundSol,
        ProcessFundToken,
        ProcessWithdrawToken,
        ProcessWithdrawSol
    },
    state::{Escrow,TokenEscrow},
    error::TokenError,
    utils::{
        assert_keys_equal,
        create_pda_account,
        get_master_address_and_bump_seed,
        create_transfer,
    }
};
use spl_associated_token_account::get_associated_token_address;
/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Function to initilize a solana
    pub fn process_sol_stream(program_id: &Pubkey, accounts: &[AccountInfo], start_time: u64, end_time: u64, amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  //sender
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda = next_account_info(account_info_iter)?; // master pda
        let pda_data = next_account_info(account_info_iter)?; // pda data storage
        let system_program = next_account_info(account_info_iter)?; // system program

        // Get the rent sysvar via syscall
        let rent = Rent::get()?; //

        // Since we are performing system_instruction source account must be signer.
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        // current time in unix time
        let now = Clock::get()?.unix_timestamp as u64; 
        if now > end_time{
            msg!("End time is already passed Now:{} End_time:{}",now,end_time);
            return Err(TokenError::TimeEnd.into());
        }

        assert_keys_equal(system_program::id(), *system_program.key)?;

        let (account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        assert_keys_equal(account_address, *pda.key)?;

        if !pda_data.data_is_empty(){
            return Err(TokenError::StreamAlreadyCreated.into());
        }

        let transfer_amount =  rent.minimum_balance(std::mem::size_of::<Escrow>());
        // Sending transaction fee to recipient. So, he can withdraw the streamed fund
        let fees = Fees::get()?;
        create_pda_account( 
            pda,
            transfer_amount,
            std::mem::size_of::<Escrow>(),
            program_id,
            system_program,
            pda_data,
            pda_signer_seeds
        )?;
        create_transfer(
            pda,
            dest_account_info,
            system_program,
            fees.fee_calculator.lamports_per_signature * 2,
            pda_signer_seeds
        )?;
        let mut escrow = Escrow::try_from_slice(&pda_data.data.borrow())?;
        escrow.start_time = start_time;
        escrow.end_time = end_time;
        escrow.paused = 0;
        escrow.withdraw_limit = 0;
        escrow.sender = *source_account_info.key;
        escrow.recipient = *dest_account_info.key;
        escrow.amount = amount;
        msg!("{:?}",escrow);
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to withdraw solana
    fn process_sol_withdraw_stream(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?; // stream initiator address
        let dest_account_info = next_account_info(account_info_iter)?; // stream receiver
        let pda = next_account_info(account_info_iter)?; // locked fund
        let pda_data = next_account_info(account_info_iter)?; // stored data 
        let system_program = next_account_info(account_info_iter)?; // system program id 

        let mut escrow = Escrow::try_from_slice(&pda_data.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        if now <= escrow.start_time {
            return Err(TokenError::StreamNotStarted.into());
        }
        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = (((now - escrow.start_time) as f64) / ((escrow.end_time - escrow.start_time) as f64) * escrow.amount as f64) as u64;
        if now >= escrow.end_time {
            allowed_amt = escrow.amount;
        }
        msg!("You can withdraw {}",allowed_amt);
        if !dest_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *dest_account_info.key != escrow.recipient {
            return Err(TokenError::EscrowMismatch.into());
        }
        // Checking if amount is greater than allowed amount
        if amount>allowed_amt {
            return Err(ProgramError::InsufficientFunds);
        }
        // Checking if paused stream is greater than withdraw limit
        if escrow.paused == 1 && amount > escrow.withdraw_limit {
            return Err(ProgramError::InsufficientFunds);
        }
        let (_account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        create_transfer(
            pda,
            dest_account_info,
            system_program,
            amount,
            pda_signer_seeds
        )?;
        if escrow.paused == 1{
            msg!("{}{}",escrow.withdraw_limit,amount);
            escrow.withdraw_limit = escrow.withdraw_limit-amount
        }
        escrow.amount = escrow.amount-amount;

        // Closing account to send rent to sender
        if escrow.amount == 0 { 
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
     /// Function to cancel solana streaming
     fn process_cancel_sol_stream(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda = next_account_info(account_info_iter)?; // locked fund
        let pda_data = next_account_info(account_info_iter)?; // stored data 
        let system_program = next_account_info(account_info_iter)?; // system program id 
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = Escrow::try_from_slice(&pda_data.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        // Amount that recipient should receive.  
        let mut allowed_amt = (((now - escrow.start_time) as f64) / ((escrow.end_time - escrow.start_time) as f64) * escrow.amount as f64) as u64;
        if now >= escrow.end_time {
            msg!("Stream already completed");
            return Err(TokenError::StreamNotStarted.into());
        }
        if now < escrow.start_time {
            allowed_amt = escrow.amount;
        }
        if *source_account_info.key != escrow.sender {
            return Err(TokenError::OwnerMismatch.into());
        }
        let dest_account_amount = escrow.amount-allowed_amt;
        let (_account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        // Sending streamed payment to receiver 
        create_transfer(
            pda,
            dest_account_info,
            system_program,
            dest_account_amount,
            pda_signer_seeds
        )?;
        // We don't need to send remaining funds to sender, its already in sender master pda account which he can withdraw with withdraw function
        // Closing account to send rent to sender
        let dest_starting_lamports = source_account_info.lamports();
        **source_account_info.lamports.borrow_mut() = dest_starting_lamports
            .checked_add(pda_data.lamports())
            .ok_or(TokenError::Overflow)?;

        **pda_data.lamports.borrow_mut() = 0;
        escrow.amount = 0;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    //Function to pause solana stream
    fn process_pause_sol_stream(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;

        let mut escrow = Escrow::try_from_slice(&pda_data.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        let allowed_amt = (((now - escrow.start_time) as f64) / ((escrow.end_time - escrow.start_time) as f64) * escrow.amount as f64) as u64;
        if now >= escrow.end_time {
            msg!("End time is already passed");
            return Err(TokenError::TimeEnd.into());
        }
        // Both sender and receiver can pause / resume stream
        if !source_account_info.is_signer && !dest_account_info.is_signer{ 
            return Err(ProgramError::MissingRequiredSignature); 
        }

        if *source_account_info.key != escrow.sender || *dest_account_info.key != escrow.recipient { 
            return Err(TokenError::EscrowMismatch.into());
        }
        if escrow.paused ==1{
            return Err(TokenError::AlreadyPaused.into());
        }
        escrow.paused = 1;
        escrow.withdraw_limit = allowed_amt;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    //Function to resume solana stream
    fn process_resume_sol_stream(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;

        let now = Clock::get()?.unix_timestamp as u64;
        let mut escrow = Escrow::try_from_slice(&pda_data.data.borrow())?;
        // Both sender and receiver can pause / resume stream
        if !source_account_info.is_signer && !dest_account_info.is_signer{ 
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *source_account_info.key != escrow.sender || *dest_account_info.key != escrow.recipient {
            return Err(TokenError::EscrowMismatch.into());
        }
        if escrow.paused ==0{
            return Err(TokenError::AlreadyResumed.into());
        }
        escrow.paused = 0;
        escrow.start_time =  now;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    // Function to initilize token streaming 
    fn process_token_stream(program_id: &Pubkey, accounts: &[AccountInfo], start_time: u64, end_time: u64, amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda = next_account_info(account_info_iter)?; // master pda
        let pda_data = next_account_info(account_info_iter)?; // Program pda to store data
        let token_program_info = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let system_program = next_account_info(account_info_iter)?; // system address
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 

        // Get the rent sysvar via syscall
        let rent = Rent::get()?; //
        if token_program_info.key != &spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }    
        // Since we are performing system_instruction source account must be signer
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        // current time in unix time
        let now = Clock::get()?.unix_timestamp as u64; 
        if now > end_time{
            return Err(TokenError::TimeEnd.into());
        }
        let space_size = std::mem::size_of::<TokenEscrow>() as u64;

        let (_account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];

        if !pda_data.data_is_empty(){
            return Err(TokenError::StreamAlreadyCreated.into());
        }
        let create_account_instruction = create_account(
            pda.key,
            pda_data.key,
            rent.minimum_balance(std::mem::size_of::<TokenEscrow>()),
            space_size,
            program_id,
        );
        invoke_signed(
            &create_account_instruction,
            &[
                pda.clone(),
                pda_data.clone(),
                system_program.clone(),
            ],&[&pda_signer_seeds[..]]
        )?;
        let mut escrow = TokenEscrow::try_from_slice(&pda_data.data.borrow())?;
        escrow.start_time = start_time;
        escrow.end_time = end_time;
        escrow.paused = 0;
        escrow.withdraw_limit = 0;
        escrow.sender = *source_account_info.key;
        escrow.recipient = *dest_account_info.key;
        escrow.amount = amount;
        escrow.token_mint = *token_mint_info.key;
        msg!("{:?}",escrow);
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    // Function to withdraw from  token streaming 
    fn process_token_withdraw_stream(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda = next_account_info(account_info_iter)?; // master pda
        let pda_data = next_account_info(account_info_iter)?; // Program pda to store data
        let token_program_info = next_account_info(account_info_iter)?; // {TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA}
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of pda
        let receiver_associated_info = next_account_info(account_info_iter)?; // Associated token of receiver
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let system_program = next_account_info(account_info_iter)?;
        if token_program_info.key != &spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }    
        // Since we are performing system_instruction source account must be signer
        if !dest_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = TokenEscrow::try_from_slice(&pda_data.data.borrow())?;
        assert_keys_equal(escrow.token_mint, *token_mint_info.key)?;
        let now = Clock::get()?.unix_timestamp as u64;
        if now <= escrow.start_time {
            msg!("Stream has not been started");
            return Err(TokenError::StreamNotStarted.into());
        }
        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = (((now - escrow.start_time) as f64) / ((escrow.end_time - escrow.start_time) as f64) * escrow.amount as f64) as u64;
        if now >= escrow.end_time {
            msg!("Stream has been successfully completed");
            allowed_amt = escrow.amount;
        }
        // let rent = &Rent::from_account_info(dest_account_info)?;
        msg!("{} allowed_amt",allowed_amt);
        if *dest_account_info.key != escrow.recipient {
            return Err(TokenError::EscrowMismatch.into());
        }
        if amount>allowed_amt {
            msg!("{} is not yet streamlined.",amount);
            return Err(ProgramError::InsufficientFunds);
        }
        msg!("{}",amount);
        if escrow.paused == 1 && amount > escrow.withdraw_limit {
            msg!("{} is your withdraw limit",escrow.withdraw_limit);
            return Err(ProgramError::InsufficientFunds);
        }

        let (account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        let pda_associated_token = spl_associated_token_account::get_associated_token_address(&account_address,&escrow.token_mint);
        assert_keys_equal(pda_associated_token, *pda_associated_info.key)?;
        if receiver_associated_info.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    dest_account_info.key,
                    dest_account_info.key,
                    token_mint_info.key,
                ),&[
                    dest_account_info.clone(),
                    receiver_associated_info.clone(),
                    dest_account_info.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                receiver_associated_info.key,
                pda.key,
                &[pda.key],
                amount
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                receiver_associated_info.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds[..]],
        )?;
        if escrow.paused == 1{
            msg!("{}{}",escrow.withdraw_limit,amount);
            escrow.withdraw_limit = escrow.withdraw_limit-amount
        }
        escrow.amount = escrow.amount-amount;
        // Closing account to send rent to sender 
        if escrow.amount == 0 { 
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to cancel token streaming
    fn process_token_cancel_stream(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda = next_account_info(account_info_iter)?; // master pda
        let pda_data = next_account_info(account_info_iter)?; // Program pda to store data
        let token_program_info = next_account_info(account_info_iter)?; // {TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA}
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let receiver_associated_info = next_account_info(account_info_iter)?; // Associated token of receiver
        let pda_associated_info = next_account_info(account_info_iter)?; // pda associated token info 
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let system_program = next_account_info(account_info_iter)?; // system program id
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut escrow = TokenEscrow::try_from_slice(&pda_data.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;

        // Amount that recipient should receive.  
        let mut allowed_amt = (((now - escrow.start_time) as f64) / ((escrow.end_time - escrow.start_time) as f64) * escrow.amount as f64) as u64;

        if now < escrow.start_time {
            allowed_amt = escrow.amount;
        }
        if now >= escrow.end_time {
            msg!("Stream already completed");
            return Err(TokenError::TimeEnd.into());
        }
        if *source_account_info.key != escrow.sender {
            return Err(TokenError::OwnerMismatch.into());
        }
        let dest_account_amount = escrow.amount-allowed_amt;

        assert_keys_equal(*token_mint_info.key, escrow.token_mint)?;

        let receiver_associated_account_check = get_associated_token_address(dest_account_info.key,&escrow.token_mint);

        assert_keys_equal(receiver_associated_account_check, *receiver_associated_info.key)?;

        // Sending pending streaming payment to sender 
        let (_account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        if receiver_associated_info.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    source_account_info.key,
                    dest_account_info.key,
                    token_mint_info.key,
                ),&[
                    source_account_info.clone(),
                    receiver_associated_info.clone(),
                    dest_account_info.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                receiver_associated_info.key,
                pda.key,
                &[pda.key],
                dest_account_amount
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                receiver_associated_info.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds[..]],
        )?;
        // We don't need to send tkens to sender wallet since tokens are already stored in master pda associated token account
        // Sending pda rent to sender account
        let dest_starting_lamports = source_account_info.lamports();
        **source_account_info.lamports.borrow_mut() = dest_starting_lamports
            .checked_add(pda_data.lamports())
            .ok_or(TokenError::Overflow)?;

        **pda_data.lamports.borrow_mut() = 0;

        escrow.amount = 0;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to pause token streaming
    fn process_pause_token_stream(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = TokenEscrow::try_from_slice(&pda_data.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        let allowed_amt = (((now - escrow.start_time) as f64) / ((escrow.end_time - escrow.start_time) as f64) * escrow.amount as f64) as u64;
        if now >= escrow.end_time {
            msg!("End time is already passed");
            return Err(TokenError::TimeEnd.into());
        }
        if !source_account_info.is_signer && !dest_account_info.is_signer{ // Both sender and receiver can pause / resume stream
            return Err(ProgramError::MissingRequiredSignature); 
        }

        if *source_account_info.key != escrow.sender || *dest_account_info.key != escrow.recipient { //Sender and Recipient both can pause or resume any transaction
            return Err(TokenError::EscrowMismatch.into());
        }
        if escrow.paused ==1{
            return Err(TokenError::AlreadyPaused.into());
        }
        escrow.paused = 1;
        escrow.withdraw_limit = allowed_amt;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to resume token streaming
    fn process_resume_token_stream(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let now = Clock::get()?.unix_timestamp as u64;
        let mut escrow = TokenEscrow::try_from_slice(&pda_data.data.borrow())?;
        if !source_account_info.is_signer && !dest_account_info.is_signer{ // Both sender and receiver can pause / resume stream
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *source_account_info.key != escrow.sender || *dest_account_info.key != escrow.recipient { //Sender and Recipient both can pause or resume any transaction
            return Err(TokenError::EscrowMismatch.into());
        }
        if escrow.paused ==0{
            return Err(TokenError::AlreadyResumed.into());
        }
        escrow.paused = 0;
        escrow.start_time =  now;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to deposit solana
    fn process_deposit_sol(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let pda = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        let (account_address, _bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        assert_keys_equal(account_address, *pda.key)?;
        assert_keys_equal(system_program::id(), *system_program.key)?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        invoke(
            &solana_program::system_instruction::transfer(
                source_account_info.key,
                pda.key,
                amount
            ),
            &[
                source_account_info.clone(),
                pda.clone(),
                system_program.clone()
            ],
        )?;
        Ok(())
    }
    /// Function to deposit token
    fn process_deposit_token(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let pda = next_account_info(account_info_iter)?; // pda
        let token_program_info = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let token_mint_info = next_account_info(account_info_iter)?; // token mint
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let associated_token_address = next_account_info(account_info_iter)?; // sender associated token address of token you are initializing 
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of pda
        let system_program = next_account_info(account_info_iter)?;
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}

        let (account_address, _bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );

        let pda_associated_token = spl_associated_token_account::get_associated_token_address(&account_address,token_mint_info.key);
        assert_keys_equal(spl_token::id(), *token_program_info.key)?;
        assert_keys_equal(pda_associated_token, *pda_associated_info.key)?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_associated_info.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    source_account_info.key,
                    pda.key,
                    token_mint_info.key,
                ),&[
                    source_account_info.clone(),
                    pda_associated_info.clone(),
                    pda.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        invoke(
            &spl_token::instruction::transfer(
                token_program_info.key,
                associated_token_address.key,
                pda_associated_info.key,
                source_account_info.key,
                &[source_account_info.key],
                amount
            )?,
            &[
                token_program_info.clone(),
                associated_token_address.clone(),
                pda_associated_info.clone(),
                source_account_info.clone(),
                system_program.clone()
            ],
        )?;
        Ok(())
    }
    /// Function to fund ongoing solana streaming
    fn process_fund_sol(accounts: &[AccountInfo],end_time: u64, amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let  source_account_info = next_account_info(account_info_iter)?;  //sender
        let pda_data = next_account_info(account_info_iter)?;  //pda
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount.into());
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut escrow = Escrow::try_from_slice(&pda_data.data.borrow())?;
        if *source_account_info.key != escrow.sender {
            return Err(TokenError::OwnerMismatch.into());
        }
        escrow.end_time = end_time;
        escrow.amount = escrow.amount+amount;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to fund ongoing token streaming
    fn process_fund_token(accounts: &[AccountInfo],end_time: u64, amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let  source_account_info = next_account_info(account_info_iter)?;  //sender
        let pda_data = next_account_info(account_info_iter)?;  //sender

        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount.into());
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut escrow = TokenEscrow::try_from_slice(&pda_data.data.borrow())?;
        if *source_account_info.key != escrow.sender {
            return Err(TokenError::OwnerMismatch.into());
        }
        escrow.end_time = end_time;
        escrow.amount = amount;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to deposit solana
    fn process_withdraw_sol(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let pda = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        let (account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        assert_keys_equal(account_address, *pda.key)?;
        assert_keys_equal(system_program::id(), *system_program.key)?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        invoke_signed(
            &solana_program::system_instruction::transfer(
                pda.key,
                source_account_info.key,
                amount
            ),
            &[
                pda.clone(),
                source_account_info.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds[..]],
        )?;
        Ok(())
    }
    /// Function to deposit token
    fn process_withdraw_token(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let token_mint_info = next_account_info(account_info_iter)?; // token mint
        let associated_token_address = next_account_info(account_info_iter)?; // sender associated token address
        let pda = next_account_info(account_info_iter)?; // pda
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of pda
        let system_program = next_account_info(account_info_iter)?; // system program 

        let (account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        let pda_associated_token = spl_associated_token_account::get_associated_token_address(&account_address,token_mint_info.key);
        let source_associated_token = spl_associated_token_account::get_associated_token_address(&source_account_info.key,token_mint_info.key);
        assert_keys_equal(source_associated_token, *associated_token_address.key)?;
        assert_keys_equal(spl_token::id(), *token_program_info.key)?;
        assert_keys_equal(account_address, *pda.key)?;
        assert_keys_equal(pda_associated_token, *pda_associated_info.key)?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                associated_token_address.key,
                pda.key,
                &[pda.key],
                amount
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                associated_token_address.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds[..]],
        )?;
        Ok(())
    }
    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = TokenInstruction::unpack(input)?;
        match instruction {
            TokenInstruction::ProcessSolStream (ProcessSolStream{
                start_time,
                end_time,
                amount,
            }) => {
                msg!("Instruction: Sol Stream");
                Self::process_sol_stream(program_id,accounts,start_time, end_time, amount)
            }
            TokenInstruction::ProcessSolWithdrawStream(ProcessSolWithdrawStream {
                amount,
            }) => {
                msg!("Instruction: Sol Withdraw");
                Self::process_sol_withdraw_stream(program_id,accounts, amount)
            }
            TokenInstruction::ProcessCancelSolStream => {
                msg!("Instruction: Sol Cancel");
                Self::process_cancel_sol_stream(program_id,accounts)
            }
            TokenInstruction::ProcessTokenStream(ProcessTokenStream {
                start_time,
                end_time,
                amount,
            }) => {
                msg!("Instruction: Token Stream");
                Self::process_token_stream(program_id,accounts,start_time, end_time, amount)
            }
            TokenInstruction::ProcessPauseSolStream => {
                msg!("Instruction: Stream pause");
                Self::process_pause_sol_stream(accounts)
            }
            TokenInstruction::ProcessResumeSolStream=> {
                msg!("Instruction: Stream Resume ");
                Self::process_resume_sol_stream(accounts)
            }
            TokenInstruction::ProcessTokenWithdrawStream(ProcessTokenWithdrawStream {
                amount,
            }) => {
                msg!("Instruction: Token Withdraw");
                Self::process_token_withdraw_stream(program_id,accounts, amount)
            }
            TokenInstruction::ProcessDepositSol(ProcessDepositSol {
                amount,
            }) => {
                msg!("Instruction: Deposit Sol");
                Self::process_deposit_sol(program_id,accounts, amount)
            }
            TokenInstruction::ProcessCancelTokenStream => {
                msg!("Instruction: Cancel Token Stream");
                Self::process_token_cancel_stream(program_id,accounts)
            }
            TokenInstruction::ProcessPauseTokenStream => {
                msg!("Instruction: Token Stream Pause");
                Self::process_pause_token_stream(accounts)
            }
            TokenInstruction::ProcessResumeTokenStream => {
                msg!("Instruction:  Token Stream Resume");
                Self::process_resume_token_stream(accounts)
            }
            TokenInstruction::ProcessDepositToken(ProcessDepositToken {
                amount,
            }) => {
                msg!("Instruction: Deposite token");
                Self::process_deposit_token(program_id,accounts,amount) 
            }
            TokenInstruction::ProcessFundSol(ProcessFundSol {
                end_time,
                amount,
            }) => {
                msg!("Instruction: Fund Solana");
                Self::process_fund_sol(accounts,end_time,amount) 
            }
            TokenInstruction::ProcessFundToken(ProcessFundToken {
                end_time,
                amount,
            }) => {
                msg!("Instruction: Fund token");
                Self::process_fund_token(accounts,end_time,amount) 
            }
            TokenInstruction::ProcessWithdrawSol(ProcessWithdrawSol {
                amount,
            }) => {
                msg!("Instruction: Withdraw Sol");
                Self::process_withdraw_sol(program_id,accounts,amount) 
            }
            TokenInstruction::ProcessWithdrawToken(ProcessWithdrawToken {
                amount,
            }) => {
                msg!("Instruction: Withdraw token");
                Self::process_withdraw_token(program_id,accounts,amount) 
            }
        }
    }
}
impl PrintProgramError for TokenError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            TokenError::TimeEnd => msg!("Error: Time has already passed"),
            TokenError::OwnerMismatch => msg!("Error: Owner does not match"),
            TokenError::NotRentExempt => msg!("Error: Lamport balance below rent-exempt threshold"),
            TokenError::EscrowMismatch => msg!("Error: Account not associated with this Escrow"),
            TokenError::InvalidInstruction => msg!("Error: Invalid instruction"),
            TokenError::AlreadyCancel => msg!("Error: Invalid instruction"),
            TokenError::AlreadyWithdrawn => msg!("Error: Paused stream, streamed amount already withdrawn"),
            TokenError::Overflow => msg!("Error: Operation overflowed"),
            TokenError::PublicKeyMismatch => msg!("Error: Public key mismatched"),
            TokenError::AlreadyPaused=> msg!("Error: Transaction is already paused"),
            TokenError::AlreadyResumed=>msg!("Error: Transaction is not paused"),
            TokenError::StreamAlreadyCreated=>msg!("Stream Already Created"),
            TokenError::StreamNotStarted=>msg!("Stream has not been started")
        }
    }
}