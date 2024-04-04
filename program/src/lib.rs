//! Vote Program.
// [Core BPF]: Required for `solana-frozen-abi-macro` to work.
#![allow(incomplete_features)]
#![cfg_attr(RUSTC_WITH_SPECIALIZATION, feature(specialization))]

#[cfg(all(target_os = "solana", feature = "bpf-entrypoint"))]
mod entrypoint;
pub mod processor;

// [Core BPF]: TODO: Program-test will not overwrite existing built-ins.
// See https://github.com/solana-labs/solana/pull/35233.
// solana_program::declare_id!("Vote111111111111111111111111111111111111111");
solana_program::declare_id!("4YV59ZoDVJPAaAPth6pqWM1SX1FaztT4jxC8m56WMpa5");
