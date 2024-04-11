use {
    crate::state::lockout::Lockout,
    serde::{Deserialize, Serialize},
    solana_frozen_abi_macro::{frozen_abi, AbiExample},
    solana_program::{
        clock::{Slot, UnixTimestamp},
        hash::Hash,
    },
    std::collections::VecDeque,
};

#[frozen_abi(digest = "5VUusSTenF9vZ9eHiCprVe9ABJUHCubeDNCCDxykybZY")]
#[derive(Serialize, Default, Deserialize, Debug, PartialEq, Eq, Clone, AbiExample)]
pub struct TowerSync {
    /// The proposed tower
    pub lockouts: VecDeque<Lockout>,
    /// The proposed root
    pub root: Option<Slot>,
    /// signature of the bank's state at the last slot
    pub hash: Hash,
    /// processing timestamp of last slot
    pub timestamp: Option<UnixTimestamp>,
    /// the unique identifier for the chain up to and
    /// including this block. Does not require replaying
    /// in order to compute.
    pub block_id: Hash,
}

impl From<Vec<(Slot, u32)>> for TowerSync {
    fn from(recent_slots: Vec<(Slot, u32)>) -> Self {
        let lockouts: VecDeque<Lockout> = recent_slots
            .into_iter()
            .map(|(slot, confirmation_count)| {
                Lockout::new_with_confirmation_count(slot, confirmation_count)
            })
            .collect();
        Self {
            lockouts,
            root: None,
            hash: Hash::default(),
            timestamp: None,
            block_id: Hash::default(),
        }
    }
}

impl TowerSync {
    pub fn new(
        lockouts: VecDeque<Lockout>,
        root: Option<Slot>,
        hash: Hash,
        block_id: Hash,
    ) -> Self {
        Self {
            lockouts,
            root,
            hash,
            timestamp: None,
            block_id,
        }
    }

    pub fn slots(&self) -> Vec<Slot> {
        self.lockouts.iter().map(|lockout| lockout.slot()).collect()
    }

    pub fn last_voted_slot(&self) -> Option<Slot> {
        self.lockouts.back().map(|l| l.slot())
    }
}

pub mod serde_tower_sync {
    use {
        super::*,
        serde::{Deserialize, Deserializer, Serialize, Serializer},
        solana_program::{serde_varint, short_vec},
    };

    #[derive(Deserialize, Serialize, AbiExample)]
    struct LockoutOffset {
        #[serde(with = "serde_varint")]
        offset: Slot,
        confirmation_count: u8,
    }

    #[derive(Deserialize, Serialize)]
    struct CompactTowerSync {
        root: Slot,
        #[serde(with = "short_vec")]
        lockout_offsets: Vec<LockoutOffset>,
        hash: Hash,
        timestamp: Option<UnixTimestamp>,
        block_id: Hash,
    }

    pub fn serialize<S>(tower_sync: &TowerSync, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let lockout_offsets = tower_sync.lockouts.iter().scan(
            tower_sync.root.unwrap_or_default(),
            |slot, lockout| {
                let Some(offset) = lockout.slot().checked_sub(*slot) else {
                    return Some(Err(serde::ser::Error::custom("Invalid vote lockout")));
                };
                let Ok(confirmation_count) = u8::try_from(lockout.confirmation_count()) else {
                    return Some(Err(serde::ser::Error::custom("Invalid confirmation count")));
                };
                let lockout_offset = LockoutOffset {
                    offset,
                    confirmation_count,
                };
                *slot = lockout.slot();
                Some(Ok(lockout_offset))
            },
        );
        let compact_tower_sync = CompactTowerSync {
            root: tower_sync.root.unwrap_or(Slot::MAX),
            lockout_offsets: lockout_offsets.collect::<Result<_, _>>()?,
            hash: tower_sync.hash,
            timestamp: tower_sync.timestamp,
            block_id: tower_sync.block_id,
        };
        compact_tower_sync.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<TowerSync, D::Error>
    where
        D: Deserializer<'de>,
    {
        let CompactTowerSync {
            root,
            lockout_offsets,
            hash,
            timestamp,
            block_id,
        } = CompactTowerSync::deserialize(deserializer)?;
        let root = (root != Slot::MAX).then_some(root);
        let lockouts =
            lockout_offsets
                .iter()
                .scan(root.unwrap_or_default(), |slot, lockout_offset| {
                    *slot = match slot.checked_add(lockout_offset.offset) {
                        None => {
                            return Some(Err(serde::de::Error::custom("Invalid lockout offset")))
                        }
                        Some(slot) => slot,
                    };
                    let lockout = Lockout::new_with_confirmation_count(
                        *slot,
                        u32::from(lockout_offset.confirmation_count),
                    );
                    Some(Ok(lockout))
                });
        Ok(TowerSync {
            root,
            lockouts: lockouts.collect::<Result<_, _>>()?,
            hash,
            timestamp,
            block_id,
        })
    }
}
