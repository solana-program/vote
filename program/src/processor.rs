//! Vote Program processor.

use {
    crate::{
        instruction::{VoteAuthorize, VoteInit, VoteInstruction},
        state::{vote::Vote, vote_state_update::VoteStateUpdate},
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::Clock,
        entrypoint::ProgramResult,
        program_error::ProgramError,
        pubkey::Pubkey,
        rent::Rent,
        sysvar::Sysvar,
    },
    std::collections::HashSet,
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

fn get_signers(accounts: &[AccountInfo]) -> HashSet<Pubkey> {
    accounts
        .iter()
        .filter_map(|account| {
            if account.is_signer {
                Some(*account.key)
            } else {
                None
            }
        })
        .collect()
}

fn verify_authorized_signer(
    authorized: &Pubkey,
    signers: &HashSet<Pubkey>,
) -> Result<(), ProgramError> {
    if signers.contains(authorized) {
        Ok(())
    } else {
        Err(ProgramError::MissingRequiredSignature)
    }
}

// [Core BPF]: Feature `vote_state_add_vote_latency` is active on all clusters.
//   - `dev`: 597
//   - `tst`: 586
//   - `mnb`: 585
// The original implementation here was requiring the feature set in order to
// key on `vote_state_add_vote_latency` being active. Here we can just omit it.
//
// Updates the vote account state with a new VoteState instance.  This is required temporarily during the
// upgrade of vote account state from V1_14_11 to Current.
fn set_vote_account_state(
    vote_account: &AccountInfo,
    vote_state: VoteState,
    rent: &Rent,
) -> Result<(), ProgramError> {
    // [Core BPF]: This implementation looks a little different from the
    // original, since `AccountInfo` doesn't have methods like
    // `set_data_length`. However, the control flow is the same.
    //
    // If this conditional resolves to `true`, store the old vote state:
    //
    // ```
    // [ The account data is too small and needs to be reallocated ]
    //   - AND -
    // [
    //     The realloc will cause the account to no longer be rent-exempt
    //      - OR -
    //     The realloc failed for other reasons
    // ]
    // ```
    //
    // Otherwise store the new vote state.
    let vote_state_size = VoteStateVersions::vote_state_size_of(true);
    if (vote_account.data_len() < vote_state_size)
        && (!rent.is_exempt(vote_account.lamports(), vote_state_size)
            || vote_account.realloc(vote_state_size, false).is_err())
    {
        // Account cannot be resized to the size of a vote state as it will not be rent exempt, or failed to be
        // resized for other reasons.  So store the V1_14_11 version.
        bincode::serialize_into(
            &mut vote_account.try_borrow_mut_data()?[..],
            VoteState1_14_11::from(vote_state),
        )
        .map_err(|_| {
            // [Core BPF]: Original implementation was `InstructionError::GenericError`.
            ProgramError::InvalidAccountData
        })?;
        return Ok(());
    }

    // Vote account is large enough to store the newest version of vote state.
    bincode::serialize_into(
        &mut vote_account.try_borrow_mut_data()?[..],
        VoteStateVersions::new_current(vote_state),
    )
    .map_err(|_| {
        // [Core BPF]: Original implementation was `InstructionError::GenericError`.
        ProgramError::InvalidAccountData
    })?;

    Ok(())
}

fn process_initialize_account(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    vote_init: VoteInit,
) -> ProgramResult {
    let signers = get_signers(accounts);
    let accounts_iter = &mut accounts.iter();

    let vote_account = next_account_info(accounts_iter)?;
    let vote_account_data_len = vote_account.data_len();

    let clock = <Clock as Sysvar>::get()?;
    let rent = <Rent as Sysvar>::get()?;

    if !rent.is_exempt(vote_account.lamports(), vote_account_data_len) {
        return Err(ProgramError::InsufficientFunds);
    }

    // [Core BPF]: The original implementation was passing
    // `feature_set.is_active(&feature_set::vote_state_add_vote_latency::id())`
    // as the boolean argument for `VoteStateVersions::vote_state_size_of`.
    // Since this feature is active on all clusters, we just pass `true` here.
    if vote_account_data_len != VoteStateVersions::vote_state_size_of(true) {
        return Err(ProgramError::InvalidAccountData);
    }

    let vote_state: VoteStateVersions = bincode::deserialize(&vote_account.try_borrow_data()?)
        .map_err(|_| {
            // [Core BPF]: Original implementation was `InstructionError::GenericError`.
            ProgramError::InvalidAccountData
        })?;

    if !vote_state.is_uninitialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // node must agree to accept this vote account
    verify_authorized_signer(&vote_init.node_pubkey, &signers)?;

    set_vote_account_state(vote_account, VoteState::new(vote_init, clock), &rent)?;

    Ok(())
}

fn process_authorize(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    authorized: &Pubkey,
    vote_authorize: VoteAuthorize,
    signers: &HashSet<Pubkey>,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let vote_account = next_account_info(accounts_iter)?;
    let vote_account_data_len = vote_account.data_len();

    let mut vote_state: VoteState = bincode::deserialize(&vote_account.try_borrow_data()?)
        .map_err(|_| {
            // [Core BPF]: Original implementation was `InstructionError::GenericError`.
            ProgramError::InvalidAccountData
        })?
        .convert_to_current();

    let clock = <Clock as Sysvar>::get()?;
    let rent = <Rent as Sysvar>::get()?;

    match vote_authorize {
        VoteAuthorize::Voter => {
            let authorized_withdrawer_signer =
                verify_authorized_signer(&vote_state.authorized_withdrawer, &signers).is_ok();

            vote_state.set_new_authorized_voter(
                authorized,
                clock.epoch,
                clock
                    .leader_schedule_epoch
                    .checked_add(1)
                    .ok_or(ProgramError::InvalidAccountData)?,
                |epoch_authorized_voter| {
                    // current authorized withdrawer or authorized voter must say "yay"
                    if authorized_withdrawer_signer {
                        Ok(())
                    } else {
                        verify_authorized_signer(&epoch_authorized_voter, &signers)
                    }
                },
            )?;
        }
        VoteAuthorize::Withdrawer => {
            // current authorized withdrawer must say "yay"
            verify_authorized_signer(&vote_state.authorized_withdrawer, &signers)?;
            vote_state.authorized_withdrawer = *authorized;
        }
    }

    set_vote_account_state(vote_account, vote_state, &rent)?;

    Ok(())
}

fn process_authorize_checked(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    vote_authorize: VoteAuthorize,
) -> ProgramResult {
    let voter_pubkey = accounts[3].key; // TODO
    process_authorize(
        program_id,
        accounts,
        voter_pubkey,
        vote_authorize,
        &get_signers(accounts),
    )
}

fn process_authorize_with_seed(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    authorized: &Pubkey,
    authorization_type: VoteAuthorize,
    current_authority_derived_key_owner: &Pubkey,
    current_authority_derived_key_seed: &str,
) -> ProgramResult {
    let mut expected_authority_keys: HashSet<Pubkey> = HashSet::default();
    // TODO
    if accounts[2].is_signer {
        let base_pubkey = accounts[2].key; // TODO
        expected_authority_keys.insert(Pubkey::create_with_seed(
            base_pubkey,
            current_authority_derived_key_seed,
            current_authority_derived_key_owner,
        )?);
    };
    process_authorize(
        program_id,
        accounts,
        authorized,
        authorization_type,
        &expected_authority_keys,
    )
}

fn process_authorize_checked_with_seed(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    authorization_type: VoteAuthorize,
    current_authority_derived_key_owner: &Pubkey,
    current_authority_derived_key_seed: &str,
) -> ProgramResult {
    let new_authority = accounts[3].key; // TODO
    process_authorize_with_seed(
        program_id,
        accounts,
        new_authority,
        authorization_type,
        current_authority_derived_key_owner,
        current_authority_derived_key_seed,
    )
}

fn process_update_validator_identity(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let signers = get_signers(accounts);
    let accounts_iter = &mut accounts.iter();

    let vote_info = next_account_info(accounts_iter)?;
    let node_info = next_account_info(accounts_iter)?;

    let rent = <Rent as Sysvar>::get()?;

    let mut vote_state: VoteState = bincode::deserialize(&vote_info.try_borrow_data()?)
        .map_err(|_| {
            // [Core BPF]: Original implementation was `InstructionError::GenericError`.
            ProgramError::InvalidAccountData
        })?
        .convert_to_current();

    // current authorized withdrawer must say "yay"
    verify_authorized_signer(&vote_state.authorized_withdrawer, &signers)?;

    // new node must say "yay"
    verify_authorized_signer(node_info.key, &signers)?;

    vote_state.node_pubkey = *node_info.key;

    set_vote_account_state(vote_info, vote_state, &rent)?;

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
        VoteInstruction::Authorize(voter_pubkey, vote_authorize) => process_authorize(
            program_id,
            accounts,
            &voter_pubkey,
            vote_authorize,
            &get_signers(accounts),
        ),
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
