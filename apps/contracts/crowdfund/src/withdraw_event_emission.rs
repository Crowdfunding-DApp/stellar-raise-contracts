#![allow(missing_docs)]

use soroban_sdk::{Address, Env, Vec};

use crate::{DataKey, NftContractClient, MAX_NFT_MINT_BATCH};

// ── contributed ───────────────────────────────────────────────────────────────

pub fn emit_contributed(env: &Env, backer: &Address, amount: i128, total_raised: i128) {
    env.events()
        .publish(("crowdfund", "contributed"), (backer.clone(), amount, total_raised));
}

// ── goal_reached ──────────────────────────────────────────────────────────────

pub fn emit_goal_reached(env: &Env, total_raised: i128, goal: i128) {
    env.events()
        .publish(("crowdfund", "goal_reached"), (total_raised, goal));
}

// ── withdrawn ─────────────────────────────────────────────────────────────────

pub fn emit_withdrawn(env: &Env, creator: &Address, amount: i128, platform_fee: i128) {
    assert!(amount > 0, "withdrawn: amount must be positive");
    env.events()
        .publish(("crowdfund", "withdrawn"), (creator.clone(), amount, platform_fee));
}

// ── refunded ─────────────────────────────────────────────────────────────────

pub fn emit_refunded(env: &Env, backer: &Address, amount: i128) {
    env.events()
        .publish(("crowdfund", "refunded"), (backer.clone(), amount));
}

// ── cancelled ────────────────────────────────────────────────────────────────

pub fn emit_cancelled(env: &Env) {
    env.events().publish(("crowdfund", "cancelled"), ());
}

// ── fee_transferred ──────────────────────────────────────────────────────────

pub fn emit_fee_transferred(env: &Env, platform: &Address, fee: i128) {
    assert!(fee > 0, "fee_transferred: fee must be positive");
    env.events()
        .publish(("crowdfund", "fee_transferred"), (platform.clone(), fee));
}

// ── nft_batch_minted ─────────────────────────────────────────────────────────

pub fn emit_nft_batch_minted(env: &Env, minted_count: u32) {
    assert!(
        minted_count > 0,
        "nft_batch_minted: minted_count must be positive"
    );
    env.events()
        .publish(("crowdfund", "nft_batch_minted"), minted_count);
}

// ── NFT batch minting ─────────────────────────────────────────────────────────

/// Mint NFTs to eligible contributors, capped at `MAX_NFT_MINT_BATCH`.
/// Returns the number minted (0 if no NFT contract or no eligible contributors).
pub fn mint_nfts_in_batch(env: &Env, nft_contract: &Option<Address>) -> u32 {
    let Some(nft_addr) = nft_contract else {
        return 0;
    };

    let contributors: Vec<Address> = env
        .storage()
        .persistent()
        .get(&DataKey::Contributors)
        .unwrap_or_else(|| Vec::new(env));

    let client = NftContractClient::new(env, nft_addr);
    let mut minted: u32 = 0;

    for contributor in contributors.iter() {
        if minted >= MAX_NFT_MINT_BATCH {
            break;
        }
        let contribution: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Contribution(contributor.clone()))
            .unwrap_or(0);
        if contribution > 0 {
            client.mint(&contributor);
            minted += 1;
        }
    }

    if minted > 0 {
        emit_nft_batch_minted(env, minted);
    }

    minted
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    #[should_panic(expected = "fee_transferred: fee must be positive")]
    fn emit_fee_transferred_rejects_zero() {
        let env = Env::default();
        emit_fee_transferred(&env, &Address::generate(&env), 0);
    }

    #[test]
    #[should_panic(expected = "fee_transferred: fee must be positive")]
    fn emit_fee_transferred_rejects_negative() {
        let env = Env::default();
        emit_fee_transferred(&env, &Address::generate(&env), -1);
    }

    #[test]
    fn emit_fee_transferred_accepts_positive() {
        let env = Env::default();
        emit_fee_transferred(&env, &Address::generate(&env), 1);
    }

    #[test]
    #[should_panic(expected = "nft_batch_minted: minted_count must be positive")]
    fn emit_nft_batch_minted_rejects_zero() {
        let env = Env::default();
        emit_nft_batch_minted(&env, 0);
    }

    #[test]
    fn emit_nft_batch_minted_accepts_positive() {
        let env = Env::default();
        emit_nft_batch_minted(&env, 1);
    }

    #[test]
    #[should_panic(expected = "withdrawn: amount must be positive")]
    fn emit_withdrawn_rejects_zero_payout() {
        let env = Env::default();
        emit_withdrawn(&env, &Address::generate(&env), 0, 0);
    }

    #[test]
    #[should_panic(expected = "withdrawn: amount must be positive")]
    fn emit_withdrawn_rejects_negative_payout() {
        let env = Env::default();
        emit_withdrawn(&env, &Address::generate(&env), -100, 0);
    }

    #[test]
    fn emit_withdrawn_accepts_valid_args() {
        let env = Env::default();
        emit_withdrawn(&env, &Address::generate(&env), 1_000, 0);
    }

    #[test]
    fn emit_withdrawn_accepts_nonzero_fee() {
        let env = Env::default();
        emit_withdrawn(&env, &Address::generate(&env), 950, 50);
    }
}
