//! Vote Program processor.

use {
    crate::{
        instruction::{VoteAuthorize, VoteInit, VoteInstruction},
        state::{vote::Vote, vote_state_update::VoteStateUpdate},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

// [Core BPF]: Locally-implemented
// `solana_sdk::program_utils::limited_deserialize`.
fn limited_deserialize<T>(input: &[u8]) -> Result<T, ProgramError>
where
    T: serde::de::DeserializeOwned,
{
    solana_program::program_utils::limited_deserialize(
        input, 1232, // [Core BPF]: See `solana_sdk::packet::PACKET_DATA_SIZE`
    )
    .map_err(|_| ProgramError::InvalidInstructionData)
}

fn process_initialize_account(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _vote_init: VoteInit,
) -> ProgramResult {
    Ok(())
}

fn process_authorize(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _authorized: &Pubkey,
    _vote_authorize: VoteAuthorize,
) -> ProgramResult {
    Ok(())
}

fn process_authorize_checked(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _vote_authorize: VoteAuthorize,
) -> ProgramResult {
    Ok(())
}

fn process_authorize_with_seed(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _authorized: &Pubkey,
    _authorization_type: VoteAuthorize,
    _current_authority_derived_key_owner: &Pubkey,
    _current_authority_derived_key_seed: &str,
) -> ProgramResult {
    Ok(())
}

fn process_authorize_checked_with_seed(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _authorization_type: VoteAuthorize,
    _current_authority_derived_key_owner: &Pubkey,
    _current_authority_derived_key_seed: &str,
) -> ProgramResult {
    Ok(())
}

fn process_update_validator_identity(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
) -> ProgramResult {
    Ok(())
}

fn process_update_commission(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _commission: u8,
) -> ProgramResult {
    Ok(())
}

fn process_vote(_program_id: &Pubkey, _accounts: &[AccountInfo], _vote: Vote) -> ProgramResult {
    Ok(())
}

fn process_update_vote_state(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _vote_state: VoteStateUpdate,
) -> ProgramResult {
    Ok(())
}

fn process_withdraw(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _lamports: u64,
) -> ProgramResult {
    Ok(())
}

pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match limited_deserialize(instruction_data)? {
        VoteInstruction::InitializeAccount(vote_init) => {
            process_initialize_account(program_id, accounts, vote_init)
        }
        VoteInstruction::Authorize(voter_pubkey, vote_authorize) => {
            process_authorize(program_id, accounts, &voter_pubkey, vote_authorize)
        }
        VoteInstruction::AuthorizeChecked(vote_authorize) => {
            process_authorize_checked(program_id, accounts, vote_authorize)
        }
        VoteInstruction::AuthorizeWithSeed(args) => process_authorize_with_seed(
            program_id,
            accounts,
            &args.new_authority,
            args.authorization_type,
            &args.current_authority_derived_key_owner,
            &args.current_authority_derived_key_seed,
        ),
        VoteInstruction::AuthorizeCheckedWithSeed(args) => process_authorize_checked_with_seed(
            program_id,
            accounts,
            args.authorization_type,
            &args.current_authority_derived_key_owner,
            &args.current_authority_derived_key_seed,
        ),
        VoteInstruction::UpdateValidatorIdentity => {
            process_update_validator_identity(program_id, accounts)
        }
        VoteInstruction::UpdateCommission(commission) => {
            process_update_commission(program_id, accounts, commission)
        }
        VoteInstruction::Vote(vote) | VoteInstruction::VoteSwitch(vote, _) => {
            process_vote(program_id, accounts, vote)
        }
        VoteInstruction::UpdateVoteState(vote_state_update)
        | VoteInstruction::UpdateVoteStateSwitch(vote_state_update, _) => {
            process_update_vote_state(program_id, accounts, vote_state_update)
        }
        VoteInstruction::CompactUpdateVoteState(vote_state_update)
        | VoteInstruction::CompactUpdateVoteStateSwitch(vote_state_update, _) => {
            process_update_vote_state(program_id, accounts, vote_state_update)
        }
        VoteInstruction::TowerSync(_tower_sync)
        | VoteInstruction::TowerSyncSwitch(_tower_sync, _) => {
            // This branch is currently unimplemented.
            //  - [FEATURE GATE]: `enable_tower_sync_ix`.
            Err(ProgramError::InvalidInstructionData)
        }
        VoteInstruction::Withdraw(lamports) => process_withdraw(program_id, accounts, lamports),
    }
}
