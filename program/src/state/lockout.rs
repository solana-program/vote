#[cfg(test)]
use arbitrary::Arbitrary;
use {
    serde::{Deserialize, Serialize},
    solana_frozen_abi_macro::AbiExample,
    solana_program::clock::Slot,
};

// Maximum number of votes to keep around, tightly coupled with
// epoch_schedule::MINIMUM_SLOTS_PER_EPOCH
pub const MAX_LOCKOUT_HISTORY: usize = 31;
pub const INITIAL_LOCKOUT: usize = 2;

#[derive(Serialize, Default, Deserialize, Debug, PartialEq, Eq, Copy, Clone, AbiExample)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Lockout {
    slot: Slot,
    confirmation_count: u32,
}

impl Lockout {
    pub fn new(slot: Slot) -> Self {
        Self::new_with_confirmation_count(slot, 1)
    }

    pub fn new_with_confirmation_count(slot: Slot, confirmation_count: u32) -> Self {
        Self {
            slot,
            confirmation_count,
        }
    }

    // The number of slots for which this vote is locked
    pub fn lockout(&self) -> u64 {
        (INITIAL_LOCKOUT as u64).pow(self.confirmation_count())
    }

    // The last slot at which a vote is still locked out. Validators should not
    // vote on a slot in another fork which is less than or equal to this slot
    // to avoid having their stake slashed.
    pub fn last_locked_out_slot(&self) -> Slot {
        self.slot.saturating_add(self.lockout())
    }

    pub fn is_locked_out_at_slot(&self, slot: Slot) -> bool {
        self.last_locked_out_slot() >= slot
    }

    pub fn slot(&self) -> Slot {
        self.slot
    }

    pub fn confirmation_count(&self) -> u32 {
        self.confirmation_count
    }

    pub fn increase_confirmation_count(&mut self, by: u32) {
        self.confirmation_count = self.confirmation_count.saturating_add(by)
    }
}

#[derive(Serialize, Default, Deserialize, Debug, PartialEq, Eq, Copy, Clone, AbiExample)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct LandedVote {
    // Latency is the difference in slot number between the slot that was voted on (lockout.slot)
    // and the slot in which the vote that added this Lockout landed.  For votes which were
    // cast before versions of the validator software which recorded vote latencies, latency is
    // recorded as 0.
    pub latency: u8,
    pub lockout: Lockout,
}

impl LandedVote {
    pub fn slot(&self) -> Slot {
        self.lockout.slot
    }

    pub fn confirmation_count(&self) -> u32 {
        self.lockout.confirmation_count
    }
}

impl From<LandedVote> for Lockout {
    fn from(landed_vote: LandedVote) -> Self {
        landed_vote.lockout
    }
}

impl From<Lockout> for LandedVote {
    fn from(lockout: Lockout) -> Self {
        Self {
            latency: 0,
            lockout,
        }
    }
}
