//! withdraw() event emission — schema, constants, and CI/CD helpers.
//!
//! Documents every event emitted by `withdraw()` so that off-chain indexers,
//! CI/CD pipelines, and monitoring tools can parse them without reading
//! contract source.
//!
//! # Events emitted by `withdraw()`
//!
//! | Topic 1    | Topic 2             | Data fields                              | Condition              |
//! |------------|---------------------|------------------------------------------|------------------------|
//! | "campaign" | "fee_transferred"   | `(platform_address: Address, fee: i128)` | platform fee configured |
//! | "campaign" | "nft_batch_minted"  | `minted_count: u32`                      | NFT contract set & ≥1 minted |
//! | "campaign" | "withdrawn"         | `(creator: Address, payout: i128, nft_minted_count: u32)` | always |
//!
//! # Security notes
//!
//! - Events are emitted **after** all state mutations and token transfers,
//!   so a missing event always means the transfer also did not happen.
//! - `nft_batch_minted` carries only the count, not individual addresses,
//!   to keep event size bounded regardless of contributor list length.
//! - `withdrawn` is always the last event; CI/CD pipelines can use its
//!   presence as a reliable success signal.

/// Topic strings for events emitted by `withdraw()`.
pub mod topics {
    /// Namespace shared by all crowdfund contract events.
    pub const CAMPAIGN: &str = "campaign";
    /// Emitted when a platform fee is deducted and transferred.
    pub const FEE_TRANSFERRED: &str = "fee_transferred";
    /// Emitted once when one or more NFT rewards are minted in a batch.
    pub const NFT_BATCH_MINTED: &str = "nft_batch_minted";
    /// Emitted on every successful withdrawal — always the final event.
    pub const WITHDRAWN: &str = "withdrawn";
}

/// Returns `true` if the given topic pair matches a `withdraw()` event.
///
/// Useful in test helpers and off-chain event filters.
pub fn is_withdraw_event(t1: &str, t2: &str) -> bool {
    t1 == topics::CAMPAIGN
        && matches!(
            t2,
            topics::FEE_TRANSFERRED | topics::NFT_BATCH_MINTED | topics::WITHDRAWN
        )
}
