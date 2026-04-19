use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Hello, Solana from Anza!");
    msg!("Program ID: {}", program_id);
    msg!("Accounts count: {}", accounts.len());
    msg!("Instruction data length: {}", instruction_data.len());
    
    Ok(())
}
