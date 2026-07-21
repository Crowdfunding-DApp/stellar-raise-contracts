#![allow(missing_docs)]

//! # Pluggable KYC / AML Gate
//!
//! A threshold-based, jurisdiction-aware gate on `contribute`/`pledge`,
//! off by default and never baked into the base pledge flow.
//!
//! ## Why this shape
//!
//! A Soroban contract can't call out to an off-chain KYC provider directly,
//! and it has no way to know a pledger's real-world jurisdiction. So the
//! design leans on two things this codebase already has precedent for:
//!
//! - **Attestation contract, not inline logic.** Mirroring
//!   [`crate::NftContract`]/`NftContractClient` (an optional stored
//!   `Address` + a `#[contractclient]`-generated client, called only if
//!   configured), the gate stores an optional [`crate::KycGateConfig`]
//!   pointing at an external [`crate::KycVerifier`] contract. That verifier
//!   is populated off-chain by a KYC provider after *they* run identity
//!   verification; this contract only ever asks it a yes/no question and
//!   never touches personal data.
//! - **Early-return guard on existing entry points**, exactly like the
//!   `MilestoneModeActive` checks added to `withdraw`/`cancel`/
//!   `collect_pledges` for the milestone-release feature: read a piece of
//!   state, early-return a typed [`crate::ContractError`] if the gate
//!   condition fails. No trait objects, no generic middleware/hook system —
//!   just [`enforce_kyc_gate`] called at the top of `contribute`/`pledge`.
//!
//! ## Off by default, and who can turn it on
//!
//! [`crate::DataKey::KycGate`] is absent until `configure_kyc_gate` is
//! called; [`enforce_kyc_gate`] short-circuits to `Ok(())` whenever it's
//! absent or `enabled == false`. A campaign that never calls
//! `configure_kyc_gate` pays zero extra storage reads or friction on its
//! pledge flow beyond one `Option` read.
//!
//! Configuration is **admin-gated, not creator-gated**. The campaign
//! creator is the party motivated to accept large anonymous pledges, so
//! letting them enable/disable/loosen their own compliance gate would
//! defeat the point. The admin role (the same one that can upgrade the
//! contract) is expected to set `threshold`/`jurisdiction` per legal
//! guidance for this specific campaign — determining *which* jurisdictions
//! and *what* thresholds require KYC is a legal question, not an
//! engineering one; this module only enforces whatever value it's given.
//!
//! ## Threshold scope
//!
//! The threshold is evaluated against an address's **cumulative committed
//! amount on this campaign** — the sum of its running `Contribution` and
//! `Pledge` totals — not each call in isolation. Otherwise a backer could
//! stay under a $10k threshold forever by contributing $9,999 a hundred
//! times.

use soroban_sdk::{Address, Env, Symbol};

use crate::{ContractError, DataKey, KycGateConfig, KycVerifierClient};

/// Sum of `who`'s running `Contribution` and `Pledge` totals on this
/// campaign — the single number the gate evaluates, regardless of which
/// entry point (`contribute` or `pledge`) is being used.
fn cumulative_committed(env: &Env, who: &Address) -> i128 {
    let contributed: i128 = env
        .storage()
        .persistent()
        .get(&DataKey::Contribution(who.clone()))
        .unwrap_or(0);
    let pledged: i128 = env
        .storage()
        .persistent()
        .get(&DataKey::Pledge(who.clone()))
        .unwrap_or(0);
    contributed + pledged
}

/// Pure threshold comparison, split out so it can be unit-tested without an
/// `Env`.
fn crosses_threshold(prospective_cumulative: i128, threshold: i128) -> bool {
    prospective_cumulative >= threshold
}

/// Returns this campaign's KYC gate configuration, if one has been set.
pub fn kyc_gate_config(env: &Env) -> Option<KycGateConfig> {
    env.storage().instance().get(&DataKey::KycGate)
}

/// Enforces the KYC gate for a prospective contribution/pledge of `amount`
/// by `who`.
///
/// No-op (`Ok(())`) whenever the gate isn't configured, is disabled, or the
/// resulting cumulative total stays under the configured threshold — this
/// is what keeps the gate zero-friction for campaigns/jurisdictions that
/// don't need it. Only once the threshold is reached does this call out to
/// the configured [`crate::KycVerifier`] contract.
pub fn enforce_kyc_gate(env: &Env, who: &Address, amount: i128) -> Result<(), ContractError> {
    let Some(config) = kyc_gate_config(env) else {
        return Ok(());
    };
    if !config.enabled {
        return Ok(());
    }

    let prospective = cumulative_committed(env, who)
        .checked_add(amount)
        .ok_or(ContractError::Overflow)?;
    if !crosses_threshold(prospective, config.threshold) {
        return Ok(());
    }

    let verified = KycVerifierClient::new(env, &config.verifier).is_verified(who);
    if verified {
        Ok(())
    } else {
        Err(ContractError::KycRequired)
    }
}

/// Read-only preflight check a frontend can call before submitting a
/// `contribute`/`pledge` transaction, so it can prompt for KYC verification
/// ahead of time instead of the on-chain call failing with
/// [`ContractError::KycRequired`]. Returns `true` if the given prospective
/// amount would be allowed to proceed right now.
pub fn would_pass_kyc_gate(env: &Env, who: &Address, amount: i128) -> bool {
    enforce_kyc_gate(env, who, amount).is_ok()
}

pub fn execute_configure_kyc_gate(
    env: &Env,
    admin: Address,
    verifier: Address,
    threshold: i128,
    jurisdiction: Symbol,
) -> Result<(), ContractError> {
    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if admin != stored_admin {
        return Err(ContractError::Unauthorized);
    }
    admin.require_auth();

    if threshold <= 0 {
        return Err(ContractError::InvalidParameter);
    }

    let config = KycGateConfig {
        verifier,
        threshold,
        enabled: true,
        jurisdiction,
    };
    env.storage().instance().set(&DataKey::KycGate, &config);

    crate::withdraw_event_emission::emit_kyc_gate_configured(env, &config);
    Ok(())
}

pub fn execute_set_kyc_gate_enabled(
    env: &Env,
    admin: Address,
    enabled: bool,
) -> Result<(), ContractError> {
    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if admin != stored_admin {
        return Err(ContractError::Unauthorized);
    }
    admin.require_auth();

    let mut config = kyc_gate_config(env).ok_or(ContractError::KycGateNotConfigured)?;
    config.enabled = enabled;
    env.storage().instance().set(&DataKey::KycGate, &config);

    crate::withdraw_event_emission::emit_kyc_gate_toggled(env, enabled);
    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn crosses_threshold_below_is_false() {
        assert!(!crosses_threshold(999, 1_000));
    }

    #[test]
    fn crosses_threshold_at_exact_value_is_true() {
        assert!(crosses_threshold(1_000, 1_000));
    }

    #[test]
    fn crosses_threshold_above_is_true() {
        assert!(crosses_threshold(1_001, 1_000));
    }
}
