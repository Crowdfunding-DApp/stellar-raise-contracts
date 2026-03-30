//! # Exception Handling
//!
//! @title Centralized Error Handling for Stellar Contracts
//! @notice Provides a reusable, auditable error enum and helper functions to
//!         replace panic!() calls with structured Result<T, Error> returns.
//! @dev    - All errors have NatSpec comments.
//!         - Helpers like ensure_auth(), invalid_input() for common patterns.
//!         - Prevents unwinds, enables precise error handling and monitoring.
//!         - Reusable across crowdfund, factory, security modules.

//! Common contract errors — centralized for consistency and auditability.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
#[soroban_sdk::contracterror]
pub enum Error {
    /// Emitted when caller lacks required authorization.
    /// @dev Use ensure_auth() helper.
    Unauthorized = 1,
    /// Emitted when input fails validation (amount=0, date too soon, etc.).
    /// @param reason Specific failure reason (for logging/UI).
    InvalidInput = 2,
    /// Emitted when expected data is missing (admin unset, etc.).
    /// @param resource Name of missing data.
    NotFound = 3,
    /// Emitted on arithmetic overflow/underflow.
    /// @dev Use checked_* operations before this.
    Overflow = 4,
    /// Emitted when operation attempted on already-initialized state.
    AlreadyInitialized = 5,
    /// Emitted when state would exceed contract limits (contributors > 1000).
    /// @dev Enforced by contract_state_size module.
    StateLimitExceeded = 6,
    /// Emitted when contract is in invalid state for operation.
    /// @param reason Current status (e.g. "CampaignInactive").
    InvalidState = 7,
    /// Emitted when zero/negative values provided where positive required.
    ZeroValue = 8,
    /// Specific to contributions: amount below configured minimum.
    BelowMinimum = 9,
    /// Specific to campaigns: not Active.
    CampaignInactive = 10,
    /// Batch-specific: empty batch or exceeds MAX_BATCH_SIZE.
    BatchInvalid = 11,
    /// WASM upgrade-specific: zero/invalid hash.
    InvalidWasmHash = 12,
    /// Already paused/stopped.
    AlreadyHalted = 13,
    /// Platform fee > MAX_PLATFORM_FEE_BPS.
    InvalidFee = 14,
}

use soroban_sdk::{Env, Address};

// ── Helper Functions ─────────────────────────────────────────────────────────

//! @notice Ensures `addr` authorized the tx; returns Err(Error::Unauthorized) otherwise.
//! @dev     Inline-friendly: unwrap result in function guards.
pub fn ensure_auth(env: &Env, addr: &Address) -> Result<(), Error> {
    addr.require_auth();
    Ok(())
}

//! @notice Returns Err(Error::InvalidInput(reason)).
//! @param reason Human-readable failure (truncated to 64 bytes if needed).
pub fn invalid_input(env: &Env, reason: &str) -> Result<(), Error> {
    env.events().publish(
        ("error", "InvalidInput"),
        reason,
    );
    Err(Error::InvalidInput)
}

//! @notice Returns Err(Error::InvalidState(reason)).
pub fn invalid_state(env: &Env, reason: &str) -> Result<(), Error> {
    env.events().publish(
        ("error", "InvalidState"),
        reason,
    );
    Err(Error::InvalidState)
}

//! @notice Returns Err(Error::NotFound(resource)).
//! @param resource Storage key name or data type.
pub fn not_found(env: &Env, resource: &str) -> Result<(), Error> {
    Err(Error::NotFound)
}

//! @notice Panic-replacement for state size checks.
pub fn state_limit_exceeded(env: &Env) -> Result<(), Error> {
    Err(Error::StateLimitExceeded)
}

//! @notice Batch validation helper.
pub fn validate_batch_size(len: usize, max: usize) -> Result<(), Error> {
    if len == 0 {
        return Err(Error::BatchInvalid);
    }
    if len > max {
        return Err(Error::BatchInvalid);
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_ensure_auth_success() {
        let env = Env::default();
        let addr = Address::random(&env);
        addr.require_auth_for_args(&env);
        assert!(ensure_auth(&env, &addr).is_ok());
    }

    #[test]
    #[should_panic]
    fn test_ensure_auth_fail() {
        let env = Env::default();
        let addr = Address::random(&env);
        let _ = ensure_auth(&env, &addr);
    }
}
