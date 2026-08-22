//! Tests for `contract_state_size` — state-size limit enforcement.
//!
//! Coverage:
//! - Every `validate_*_capacity` helper returns `true` below the limit and
//!   `false` at or above the limit.
//! - Every `validate_*` string helper returns `true` at/below MAX_STRING_LEN
//!   and `false` one byte over.
//! - `validate_metadata_total_length` accepts combined lengths ≤ MAX_STRING_LEN * 5
//!   and rejects combined lengths above.
//! - Constants are set to their documented values.
//! - `validate_milestone_description` rejects empty strings and accepts
//!   non-empty strings up to MAX_STRING_LEN bytes.

#![cfg(test)]

use soroban_sdk::{Env, String};

use crate::contract_state_size::{
    validate_bonus_goal_description, validate_contributor_capacity, validate_description,
    validate_metadata_total_length, validate_milestone_capacity, validate_milestone_description,
    validate_pledger_capacity, validate_roadmap_capacity, validate_roadmap_description,
    validate_social_links, validate_stretch_goal_capacity, validate_title, MAX_CONTRIBUTORS,
    MAX_MILESTONES, MAX_STRING_LEN,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_env() -> Env {
    Env::default()
}

/// Build a `soroban_sdk::String` of exactly `n` ASCII 'a' bytes.
fn str_of_len(env: &Env, n: u32) -> String {
    let mut b = soroban_sdk::Bytes::new(env);
    for _ in 0..n {
        b.push_back(b'a');
    }
    // Safe for values up to 2304; tests here stay well within that range.
    let mut buf = [0u8; 2304];
    b.copy_into_slice(&mut buf[..n as usize]);
    String::from_bytes(env, &buf[..n as usize])
}

// ── Constant sanity checks ────────────────────────────────────────────────────

#[test]
fn constants_have_expected_values() {
    assert_eq!(MAX_STRING_LEN, 256);
    assert_eq!(MAX_CONTRIBUTORS, 1_000);
    assert_eq!(MAX_MILESTONES, 20);
}

// ── validate_title ────────────────────────────────────────────────────────────

#[test]
fn validate_title_empty_is_true() {
    let env = make_env();
    assert!(validate_title(&String::from_str(&env, "")));
}

#[test]
fn validate_title_at_limit_is_true() {
    let env = make_env();
    assert!(validate_title(&str_of_len(&env, MAX_STRING_LEN)));
}

#[test]
fn validate_title_over_limit_is_false() {
    let env = make_env();
    assert!(!validate_title(&str_of_len(&env, MAX_STRING_LEN + 1)));
}

// ── validate_description ──────────────────────────────────────────────────────

#[test]
fn validate_description_empty_is_true() {
    let env = make_env();
    assert!(validate_description(&String::from_str(&env, "")));
}

#[test]
fn validate_description_at_limit_is_true() {
    let env = make_env();
    assert!(validate_description(&str_of_len(&env, MAX_STRING_LEN)));
}

#[test]
fn validate_description_over_limit_is_false() {
    let env = make_env();
    assert!(!validate_description(&str_of_len(&env, MAX_STRING_LEN + 1)));
}

// ── validate_social_links ─────────────────────────────────────────────────────

#[test]
fn validate_social_links_empty_is_true() {
    let env = make_env();
    assert!(validate_social_links(&String::from_str(&env, "")));
}

#[test]
fn validate_social_links_at_limit_is_true() {
    let env = make_env();
    assert!(validate_social_links(&str_of_len(&env, MAX_STRING_LEN)));
}

#[test]
fn validate_social_links_over_limit_is_false() {
    let env = make_env();
    assert!(!validate_social_links(&str_of_len(
        &env,
        MAX_STRING_LEN + 1
    )));
}

// ── validate_roadmap_description ──────────────────────────────────────────────

#[test]
fn validate_roadmap_description_empty_is_true() {
    let env = make_env();
    assert!(validate_roadmap_description(&String::from_str(&env, "")));
}

#[test]
fn validate_roadmap_description_at_limit_is_true() {
    let env = make_env();
    assert!(validate_roadmap_description(&str_of_len(
        &env,
        MAX_STRING_LEN
    )));
}

#[test]
fn validate_roadmap_description_over_limit_is_false() {
    let env = make_env();
    assert!(!validate_roadmap_description(&str_of_len(
        &env,
        MAX_STRING_LEN + 1
    )));
}

// ── validate_bonus_goal_description ───────────────────────────────────────────

#[test]
fn validate_bonus_goal_description_empty_is_true() {
    let env = make_env();
    assert!(validate_bonus_goal_description(&String::from_str(&env, "")));
}

#[test]
fn validate_bonus_goal_description_at_limit_is_true() {
    let env = make_env();
    assert!(validate_bonus_goal_description(&str_of_len(
        &env,
        MAX_STRING_LEN
    )));
}

#[test]
fn validate_bonus_goal_description_over_limit_is_false() {
    let env = make_env();
    assert!(!validate_bonus_goal_description(&str_of_len(
        &env,
        MAX_STRING_LEN + 1
    )));
}

// ── validate_metadata_total_length ────────────────────────────────────────────

#[test]
fn validate_metadata_total_length_zero_is_true() {
    assert!(validate_metadata_total_length(0));
}

#[test]
fn validate_metadata_total_length_at_limit_is_true() {
    // MAX_STRING_LEN * 5 == 1280
    assert!(validate_metadata_total_length(MAX_STRING_LEN * 5));
}

#[test]
fn validate_metadata_total_length_one_over_limit_is_false() {
    assert!(!validate_metadata_total_length(MAX_STRING_LEN * 5 + 1));
}

#[test]
fn validate_metadata_total_length_large_value_is_false() {
    assert!(!validate_metadata_total_length(u32::MAX));
}

// ── validate_contributor_capacity ─────────────────────────────────────────────

#[test]
fn validate_contributor_capacity_zero_is_true() {
    assert!(validate_contributor_capacity(0));
}

#[test]
fn validate_contributor_capacity_one_below_max_is_true() {
    assert!(validate_contributor_capacity(MAX_CONTRIBUTORS - 1));
}

#[test]
fn validate_contributor_capacity_at_max_is_false() {
    // The guard is `len < MAX_CONTRIBUTORS`, so at exactly MAX it returns false.
    assert!(!validate_contributor_capacity(MAX_CONTRIBUTORS));
}

#[test]
fn validate_contributor_capacity_over_max_is_false() {
    assert!(!validate_contributor_capacity(MAX_CONTRIBUTORS + 10));
}

// ── validate_pledger_capacity ─────────────────────────────────────────────────

#[test]
fn validate_pledger_capacity_zero_is_true() {
    assert!(validate_pledger_capacity(0));
}

#[test]
fn validate_pledger_capacity_one_below_max_is_true() {
    // Pledgers share the same limit as contributors.
    assert!(validate_pledger_capacity(MAX_CONTRIBUTORS - 1));
}

#[test]
fn validate_pledger_capacity_at_max_is_false() {
    assert!(!validate_pledger_capacity(MAX_CONTRIBUTORS));
}

#[test]
fn validate_pledger_capacity_over_max_is_false() {
    assert!(!validate_pledger_capacity(MAX_CONTRIBUTORS + 5));
}

// ── validate_roadmap_capacity ─────────────────────────────────────────────────

#[test]
fn validate_roadmap_capacity_zero_is_true() {
    assert!(validate_roadmap_capacity(0));
}

#[test]
fn validate_roadmap_capacity_one_below_limit_is_true() {
    // Hard limit in implementation is < 20.
    assert!(validate_roadmap_capacity(19));
}

#[test]
fn validate_roadmap_capacity_at_limit_is_false() {
    assert!(!validate_roadmap_capacity(20));
}

#[test]
fn validate_roadmap_capacity_over_limit_is_false() {
    assert!(!validate_roadmap_capacity(25));
}

// ── validate_stretch_goal_capacity ────────────────────────────────────────────

#[test]
fn validate_stretch_goal_capacity_zero_is_true() {
    assert!(validate_stretch_goal_capacity(0));
}

#[test]
fn validate_stretch_goal_capacity_one_below_limit_is_true() {
    // Hard limit in implementation is < 10.
    assert!(validate_stretch_goal_capacity(9));
}

#[test]
fn validate_stretch_goal_capacity_at_limit_is_false() {
    assert!(!validate_stretch_goal_capacity(10));
}

#[test]
fn validate_stretch_goal_capacity_over_limit_is_false() {
    assert!(!validate_stretch_goal_capacity(15));
}

// ── validate_milestone_capacity ───────────────────────────────────────────────

#[test]
fn validate_milestone_capacity_zero_is_true() {
    assert!(validate_milestone_capacity(0));
}

#[test]
fn validate_milestone_capacity_one_below_max_is_true() {
    assert!(validate_milestone_capacity(MAX_MILESTONES - 1));
}

#[test]
fn validate_milestone_capacity_at_max_is_false() {
    assert!(!validate_milestone_capacity(MAX_MILESTONES));
}

#[test]
fn validate_milestone_capacity_over_max_is_false() {
    assert!(!validate_milestone_capacity(MAX_MILESTONES + 5));
}

// ── validate_milestone_description ────────────────────────────────────────────

#[test]
fn validate_milestone_description_non_empty_short_is_true() {
    let env = make_env();
    assert!(validate_milestone_description(&String::from_str(
        &env,
        "Q1 milestone"
    )));
}

#[test]
fn validate_milestone_description_empty_is_false() {
    let env = make_env();
    assert!(!validate_milestone_description(&String::from_str(&env, "")));
}

#[test]
fn validate_milestone_description_at_limit_is_true() {
    let env = make_env();
    assert!(validate_milestone_description(&str_of_len(
        &env,
        MAX_STRING_LEN
    )));
}

#[test]
fn validate_milestone_description_over_limit_is_false() {
    let env = make_env();
    assert!(!validate_milestone_description(&str_of_len(
        &env,
        MAX_STRING_LEN + 1
    )));
}
