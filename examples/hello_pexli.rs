//! A minimal Pexli v2 / Solana-style program.
//!
//! Drop this single file into PexliFormation and click "Convert to SBF" to get
//! a `hello_pexli.so` — the SBF program the chain actually runs.

use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    msg!("Hello from PexliFormation — running on Pexli v2 as SBF!");
    Ok(())
}
