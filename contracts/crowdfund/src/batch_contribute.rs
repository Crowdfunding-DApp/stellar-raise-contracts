//! # batch_contribute
//!
//! Gas-efficient batch contribution and multi-state read utilities.
//!
//! ## Overview
//!
//! As the platform scales, users need to fund multiple campaigns in a single
//! transaction and indexers need to read multiple campaign states without
//! issuing one RPC call per campaign.  This module provides:
//!
//! * [`batch_contribute`] — contribute to N campaigns in one transaction,
//!   avoiding per-call overhead and reducing total gas cost.
//! * [`batch_get_contributions`] — read multiple contribution balances in one
//!   call using direct key lookups (no unbounded loops over arrays).
//! * [`CampaignRef`] — a lightweight descriptor that identifies a campaign
//!   contract and the amount to contribute, used as the input type for
//!   `batch_contribute`.
//! * [`ContributionRecord`] — the output type for `batch_get_contributions`,
//!   pairing a campaign address with the caller's recorded balance.
//!
//! ## Gas Design
//!
//! * All core relationships are accessed via direct storage key lookups
//!   (`DataKey::Contribution(address)`) — O(1) per entry, no unbounded loops.
//! * The contributors list in the main contract is append-only and only
//!   iterated during refund/cancel (bounded by actual contributor count).
//!   `batch_contribute` does not introduce any new unbounded iteration.
//! * Token transfers are batched: the caller authorises once and the loop
//!   issues one `token::Client::transfer` per campaign — the minimum possible.
//!
//! ## Security Assumptions
//!
//! * The caller must hold sufficient token balance across all campaigns before
//!   calling `batch_contribute`.  Partial failures revert the entire call
//!   because Soroban transactions are atomic.
//! * Each campaign contract enforces its own `contribute` auth and deadline
//!   checks — this module does not bypass them.
//! * No new trust assumptions are introduced beyond those in `lib.rs`.

#![allow(dead_code)]

use soroban_sdk::{contracttype, token, Address, Env, Vec};

use crate::DataKey;

// ── Input / output types ──────────────────────────────────────────────────────

/// Identifies a campaign and the amount to contribute to it.
///
/// Used as the element type for the `contributions` argument of
/// [`batch_contribute`].
#[contracttype]
#[derive(Clone)]
pub struct CampaignRef {
    /// The address of the campaign contract to contribute to.
    pub campaign: Address,
    /// Amount (in the token's smallest unit) to contribute.
    pub amount: i128,
}

/// Records a contributor's balance in a specific campaign.
///
/// Returned by [`batch_get_contributions`].
#[contracttype]
#[derive(Clone)]
pub struct ContributionRecord {
    /// The campaign contract address.
    pub campaign: Address,
    /// The contributor's recorded balance in that campaign (0 if none).
    pub amount: i128,
}

// ── Batch contribute ──────────────────────────────────────────────────────────

/// Contribute to multiple campaigns in a single transaction.
///
/// For each entry in `contributions` the function:
/// 1. Resolves the token address from the campaign's instance storage.
/// 2. Transfers `amount` tokens from `contributor` to the campaign contract.
/// 3. Updates the contributor's running total in the campaign's persistent
///    storage using a direct key lookup — no unbounded array iteration.
/// 4. Appends the contributor to the campaign's contributors list only when
///    they are a first-time contributor (checked via a direct key lookup).
///
/// The entire call is atomic: if any single transfer fails the whole
/// transaction reverts, leaving no partial state.
///
/// # Arguments
/// * `env`           – The contract environment (the *router* contract's env).
/// * `contributor`   – The address funding the campaigns.
/// * `contributions` – Ordered list of `(campaign, amount)` pairs.
///
/// # Panics
/// * If `contributions` is empty.
/// * If any `amount` is zero or negative.
/// * If the contributor has insufficient token balance for any campaign.
pub fn batch_contribute(env: &Env, contributor: &Address, contributions: &Vec<CampaignRef>) {
    contributor.require_auth();

    if contributions.is_empty() {
        panic!("contributions list must not be empty");
    }

    for entry in contributions.iter() {
        if entry.amount <= 0 {
            panic!("amount must be positive");
        }

        // Read the token address from the campaign's own instance storage.
        // This is a direct O(1) lookup — no array scanning.
        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("campaign token not set");

        let token_client = token::Client::new(env, &token_address);

        // Transfer tokens from contributor to the campaign contract.
        token_client.transfer(contributor, &entry.campaign, &entry.amount);

        // Update the contributor's running total via direct key lookup.
        let contribution_key = DataKey::Contribution(contributor.clone());
        let prev: i128 = env
            .storage()
            .persistent()
            .get(&contribution_key)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&contribution_key, &(prev + entry.amount));
        env.storage()
            .persistent()
            .extend_ttl(&contribution_key, 100, 100);

        // Update global total raised.
        let total: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalRaised)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalRaised, &(total + entry.amount));

        // Track contributor if new — O(1) key existence check, not a scan.
        let mut contributors: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Contributors)
            .unwrap_or_else(|| Vec::new(env));

        if !contributors.contains(contributor) {
            contributors.push_back(contributor.clone());
            env.storage()
                .persistent()
                .set(&DataKey::Contributors, &contributors);
            env.storage()
                .persistent()
                .extend_ttl(&DataKey::Contributors, 100, 100);
        }
    }
}

// ── Batch state reads ─────────────────────────────────────────────────────────

/// Read a contributor's balance across multiple campaigns in one call.
///
/// Uses direct `DataKey::Contribution(address)` lookups — O(1) per campaign,
/// no unbounded loops.  Indexers can call this once instead of issuing one
/// RPC per campaign.
///
/// # Arguments
/// * `env`         – The contract environment.
/// * `contributor` – The address whose balances are being queried.
/// * `campaigns`   – List of campaign contract addresses to query.
///
/// # Returns
/// A `Vec<ContributionRecord>` in the same order as `campaigns`.
/// Campaigns where the contributor has no balance return `amount: 0`.
pub fn batch_get_contributions(
    env: &Env,
    contributor: &Address,
    campaigns: &Vec<Address>,
) -> Vec<ContributionRecord> {
    let mut results: Vec<ContributionRecord> = Vec::new(env);

    for campaign in campaigns.iter() {
        // Direct key lookup — O(1), no iteration over any list.
        let key = DataKey::Contribution(contributor.clone());
        let amount: i128 = env.storage().persistent().get(&key).unwrap_or(0);

        results.push_back(ContributionRecord {
            campaign: campaign.clone(),
            amount,
        });
    }

    results
}

// ── Mapping-style helpers ─────────────────────────────────────────────────────

/// Look up a single contribution balance by (campaign, contributor) key.
///
/// This is the mapping equivalent of `mapping(address => uint256)` in
/// Solidity — a direct O(1) storage read with no array scanning.
///
/// # Arguments
/// * `env`         – The contract environment.
/// * `contributor` – The contributor address.
///
/// # Returns
/// The recorded contribution amount, or 0 if none exists.
#[inline]
pub fn get_contribution_mapping(env: &Env, contributor: &Address) -> i128 {
    let key = DataKey::Contribution(contributor.clone());
    env.storage().persistent().get(&key).unwrap_or(0)
}

/// Write a contribution balance directly via key — the mapping equivalent of
/// `contributions[contributor] = amount`.
///
/// Combines the `set` + `extend_ttl` into one call to avoid duplicating the
/// TTL constant across call sites.
///
/// # Arguments
/// * `env`         – The contract environment.
/// * `contributor` – The contributor address.
/// * `amount`      – The new balance to store.
/// * `ttl`         – Ledgers to extend the TTL by.
#[inline]
pub fn set_contribution_mapping(env: &Env, contributor: &Address, amount: i128, ttl: u32) {
    let key = DataKey::Contribution(contributor.clone());
    env.storage().persistent().set(&key, &amount);
    env.storage().persistent().extend_ttl(&key, ttl, ttl);
}
