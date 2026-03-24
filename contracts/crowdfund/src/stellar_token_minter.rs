//! # stellar_token_minter
//!
//! @title   Bounded NFT reward minting after successful `withdraw`
//!
//! @notice  After a successful campaign, `withdraw` may call an external NFT
//!          contract to mint contributor rewards.  All such mints in one
//!          `withdraw` invocation are **capped** at [`MAX_NFT_MINT_BATCH`] so gas
//!          and event emission stay bounded when the contributor list is large.
//!
//! @dev     This module holds **compile-time constants** and **pure helpers**
//!          only — no storage and no `Env`-dependent logic.  The crowdfund
//!          contract invokes `mint` via `invoke_contract` with the Soroban
//!          symbol [`NFT_MINT_FN_NAME`] and arguments `(to, token_id)`; that
//!          differs from the minimal `NftContract` client trait in this crate
//!          (single `to` argument).  Integrations must match the two-argument
//!          `mint(to, token_id)` ABI used in `lib.rs`.
//!
//! ## Security assumptions
//!
//! 1. **`MAX_NFT_MINT_BATCH`** — Caps cross-contract calls per `withdraw`,
//!    limiting worst-case gas and preventing unbounded loops over contributors.
//! 2. **Sequential `token_id`** — The contract assigns increasing ids starting
//!    at `1`; helpers use `saturating_add` so `u64::MAX` cannot wrap silently.
//! 3. **External NFT contract** — The reward contract is untrusted; only the
//!    mint method name and argument count are assumed; malicious NFT code can
//!    still revert or consume gas — operators should set a known-good WASM.

/// Maximum NFT `mint` invocations in a single `withdraw()` call.
///
/// @dev    Chosen to bound per-withdraw gas while still rewarding many
///         contributors in one transaction.  Contributors beyond this cap
///         are skipped until a future upgrade or off-chain compensation path.
pub const MAX_NFT_MINT_BATCH: u32 = 50;

/// Soroban `Symbol` name used when invoking the external NFT contract.
///
/// @dev    Must match the NFT contract’s exported entrypoint; keep in sync
///         with `invoke_contract(..., Symbol::new(&env, ...))` in `lib.rs`.
pub const NFT_MINT_FN_NAME: &str = "mint";

/// Returns `true` when no more mints should be performed in this `withdraw`.
///
/// @param  minted_so_far  Number of successful `mint` calls already executed
///                        in the current loop iteration.
#[inline]
pub fn mint_batch_is_full(minted_so_far: u32) -> bool {
    minted_so_far >= MAX_NFT_MINT_BATCH
}

/// Next sequential token id after a successful mint.
///
/// @param  current  Id that was just used for `mint`.
/// @return          Next id for the following contributor (saturating).
#[inline]
pub fn bump_token_id_after_mint(current: u64) -> u64 {
    current.saturating_add(1)
}
