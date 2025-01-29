//! Vote Program.

#[cfg(all(target_os = "solana", feature = "bpf-entrypoint"))]
mod entrypoint;
pub mod processor;

solana_program::declare_id!("Vote111111111111111111111111111111111111111");
