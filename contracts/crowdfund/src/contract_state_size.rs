<<<<<<< HEAD
#![allow(missing_docs)]
||||||| a43ed59f
#![no_std]
use soroban_sdk::{Env, String};

#![allow(missing_docs)]
=======
//! Contract State Size Limits
//!
//! @title   ContractStateSize — On-chain size-limit constants and enforcement helpers.
//! @notice  Defines upper bounds for every unbounded collection and user-supplied
//!          string stored in the crowdfund contract's ledger state.
//! @dev     All `check_*` helpers follow a checks-before-effects pattern: they
//!          read current state and return a typed `StateSizeError` **before** any
//!          mutation occurs in the calling function.
//!
//! ## Security Rationale
//!
//! Without these limits an adversary could:
//! - Flood `Contributors` / `Pledgers` until iteration in `withdraw` / `refund` /
//!   `collect_pledges` exceeds Soroban's per-transaction resource budget.
//! - Supply oversized `String` values that push a ledger entry past the host's
//!   hard serialisation cap, causing a host panic.
//!
//! ## Limits
//!
//! | Constant            | Value | Applies to                                      |
//! |---------------------|-------|-------------------------------------------------|
//! | `MAX_CONTRIBUTORS`  |   128 | `Contributors` list, `Pledgers` list            |
//! | `MAX_ROADMAP_ITEMS` |    32 | `Roadmap` list (`add_roadmap_item`)             |
//! | `MAX_STRETCH_GOALS` |    32 | `StretchGoals` list (`add_stretch_goal`)        |
//! | `MAX_STRING_LEN`    |   256 | title, description, social links, roadmap desc  |

use soroban_sdk::{contract, contractimpl, contracterror, Env, String, Vec};
>>>>>>> origin/main

<<<<<<< HEAD
use soroban_sdk::{Env, String, Vec};

use crate::DataKey;
||||||| a43ed59f
use soroban_sdk::{contracterror, String, Vec};
=======
// ── Error type ────────────────────────────────────────────────────────────────
>>>>>>> origin/main

/// Typed errors returned by state-size enforcement helpers.
///
/// @dev Discriminants start at 100 to avoid collisions with `ContractError` (1–17).
///      Do **not** renumber these — they are stable across contract upgrades.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum StateSizeError {
    /// The `Contributors` or `Pledgers` list has reached `MAX_CONTRIBUTORS`.
    ContributorLimitExceeded = 100,
    /// The `Roadmap` list has reached `MAX_ROADMAP_ITEMS`.
    RoadmapLimitExceeded = 101,
    /// The `StretchGoals` list has reached `MAX_STRETCH_GOALS`.
    StretchGoalLimitExceeded = 102,
    /// A string field exceeds `MAX_STRING_LEN` bytes.
    StringTooLong = 103,
}

<<<<<<< HEAD
/// Maximum number of unique contributors tracked on-chain.
pub const MAX_CONTRIBUTORS: u32 = 1_000;

/// Maximum number of unique pledgers tracked on-chain.
pub const MAX_PLEDGERS: u32 = 1_000;

/// Maximum number of roadmap items stored in instance storage.
||||||| a43ed59f
/// Maximum number of unique contributors tracked on-chain.
pub const MAX_CONTRIBUTORS: u32 = 128;

/// Maximum number of unique pledgers tracked on-chain.
pub const MAX_PLEDGERS: u32 = 128;

/// Maximum number of unique pledgers tracked on-chain.
pub const MAX_PLEDGERS: u32 = 1_000;

/// Maximum number of roadmap items stored in instance storage.
=======
// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum number of unique contributors (and pledgers) tracked per campaign.
pub const MAX_CONTRIBUTORS: u32 = 128;

/// Maximum number of roadmap milestones stored per campaign.
>>>>>>> origin/main
pub const MAX_ROADMAP_ITEMS: u32 = 32;

/// Maximum number of stretch-goal milestones stored per campaign.
pub const MAX_STRETCH_GOALS: u32 = 32;

<<<<<<< HEAD
/// Maximum campaign title length in bytes.
pub const MAX_TITLE_LENGTH: u32 = 128;
/// Maximum campaign description length in bytes.
pub const MAX_DESCRIPTION_LENGTH: u32 = 2_048;
/// Maximum social-links payload length in bytes.
pub const MAX_SOCIAL_LINKS_LENGTH: u32 = 512;
/// Maximum bonus-goal description length in bytes.
pub const MAX_BONUS_GOAL_DESCRIPTION_LENGTH: u32 = 500;
/// Maximum roadmap item description length in bytes.
pub const MAX_ROADMAP_DESCRIPTION_LENGTH: u32 = 500;
/// Maximum combined metadata budget (`title + description + socials`) in bytes.
pub const MAX_METADATA_TOTAL_LENGTH: u32 = 4_000;
/// Backward-compatible generic string limit used by legacy tests/helpers.
||||||| a43ed59f
/// Maximum campaign title length in bytes.
pub const MAX_TITLE_LENGTH: u32 = 128;
/// Maximum campaign description length in bytes.
pub const MAX_DESCRIPTION_LENGTH: u32 = 2_048;
/// Maximum social-links payload length in bytes.
pub const MAX_SOCIAL_LINKS_LENGTH: u32 = 512;
/// Maximum bonus-goal description length in bytes.
pub const MAX_BONUS_GOAL_DESCRIPTION_LENGTH: u32 = 280;
/// Maximum roadmap item description length in bytes.
pub const MAX_ROADMAP_DESCRIPTION_LENGTH: u32 = 280;
/// Maximum combined metadata budget (`title + description + socials`) in bytes.
pub const MAX_METADATA_TOTAL_LENGTH: u32 = 2_304;
/// Backward-compatible generic string limit used by legacy tests/helpers.
=======
/// Maximum byte length for any user-supplied string field.
>>>>>>> origin/main
pub const MAX_STRING_LEN: u32 = 256;

<<<<<<< HEAD
// ── Error type ────────────────────────────────────────────────────────────────
||||||| a43ed59f
/// Maximum byte length of title field.
pub const MAX_TITLE_LENGTH: u32 = 100;
=======
// ── Standalone helpers (called from lib.rs) ───────────────────────────────────
>>>>>>> origin/main

<<<<<<< HEAD
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StateSizeError {
    ContributorLimitExceeded,
    RoadmapLimitExceeded,
    StretchGoalLimitExceeded,
    StringTooLong,
||||||| a43ed59f
/// Maximum byte length of description field.
pub const MAX_DESCRIPTION_LENGTH: u32 = 2000;
=======
/// Returns `Ok(())` if `s.len() <= MAX_STRING_LEN`, else `Err(StateSizeError::StringTooLong)`.
#[inline]
pub fn check_string_len(s: &String) -> Result<(), StateSizeError> {
    if s.len() > MAX_STRING_LEN {
        Err(StateSizeError::StringTooLong)
    } else {
        Ok(())
    }
>>>>>>> origin/main
}

<<<<<<< HEAD
impl core::fmt::Display for StateSizeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StateSizeError::ContributorLimitExceeded => write!(f, "contributor limit exceeded"),
            StateSizeError::RoadmapLimitExceeded => write!(f, "roadmap limit exceeded"),
            StateSizeError::StretchGoalLimitExceeded => write!(f, "stretch goal limit exceeded"),
            StateSizeError::StringTooLong => write!(f, "string too long"),
        }
    }
||||||| a43ed59f
/// Maximum byte length of bonus goal description field.
pub const MAX_BONUS_GOAL_DESCRIPTION_LENGTH: u32 = 500;

/// Maximum byte length of roadmap description field.
pub const MAX_ROADMAP_DESCRIPTION_LENGTH: u32 = 500;

/// Maximum byte length of social links field.
pub const MAX_SOCIAL_LINKS_LENGTH: u32 = 300;

/// Maximum total byte length of all metadata fields combined.
pub const MAX_METADATA_TOTAL_LENGTH: u32 = 4000;
=======
/// Returns `Ok(())` if `count < MAX_CONTRIBUTORS`, else `Err(ContributorLimitExceeded)`.
#[inline]
pub fn validate_contributor_capacity(count: u32) -> Result<(), StateSizeError> {
    if count >= MAX_CONTRIBUTORS {
        Err(StateSizeError::ContributorLimitExceeded)
    } else {
        Ok(())
    }
}

/// Reads the `Contributors` list length from persistent storage and enforces the cap.
#[inline]
pub fn check_contributor_limit(env: &Env) -> Result<(), StateSizeError> {
    let count: u32 = env
        .storage()
        .persistent()
        .get::<_, Vec<soroban_sdk::Address>>(&crate::DataKey::Contributors)
        .map(|v| v.len())
        .unwrap_or(0);
    validate_contributor_capacity(count)
}

/// Returns `Ok(())` if `count < MAX_CONTRIBUTORS`, else `Err(ContributorLimitExceeded)`.
#[inline]
pub fn validate_pledger_capacity(count: u32) -> Result<(), StateSizeError> {
    validate_contributor_capacity(count)
}

/// Reads the `Pledgers` list length from persistent storage and enforces the cap.
#[inline]
pub fn check_pledger_limit(env: &Env) -> Result<(), StateSizeError> {
    let count: u32 = env
        .storage()
        .persistent()
        .get::<_, Vec<soroban_sdk::Address>>(&crate::DataKey::Pledgers)
        .map(|v| v.len())
        .unwrap_or(0);
    validate_contributor_capacity(count)
>>>>>>> origin/main
}

/// Returns `Ok(())` if `count < MAX_ROADMAP_ITEMS`, else `Err(RoadmapLimitExceeded)`.
#[inline]
pub fn validate_roadmap_capacity(count: u32) -> Result<(), StateSizeError> {
    if count >= MAX_ROADMAP_ITEMS {
        Err(StateSizeError::RoadmapLimitExceeded)
    } else {
        Ok(())
    }
}

<<<<<<< HEAD
pub fn validate_title(title: &String) -> Result<(), &'static str> {
    if title.len() > MAX_TITLE_LENGTH {
        return Err("title exceeds MAX_TITLE_LENGTH bytes");
    }
    Ok(())
}

pub fn validate_description(value: &String) -> Result<(), &'static str> {
    if value.len() > MAX_DESCRIPTION_LENGTH {
        return Err("description exceeds MAX_DESCRIPTION_LENGTH bytes");
    }
    Ok(())
}

pub fn validate_social_links(value: &String) -> Result<(), &'static str> {
    if value.len() > MAX_SOCIAL_LINKS_LENGTH {
        return Err("social links exceed MAX_SOCIAL_LINKS_LENGTH bytes");
    }
    Ok(())
}

pub fn validate_bonus_goal_description(value: &String) -> Result<(), &'static str> {
    if value.len() > MAX_BONUS_GOAL_DESCRIPTION_LENGTH {
        return Err("bonus goal description exceeds MAX_BONUS_GOAL_DESCRIPTION_LENGTH bytes");
    }
    Ok(())
}

pub fn validate_roadmap_description(value: &String) -> Result<(), &'static str> {
    if value.len() > MAX_ROADMAP_DESCRIPTION_LENGTH {
        return Err("roadmap description exceeds MAX_ROADMAP_DESCRIPTION_LENGTH bytes");
    }
    Ok(())
}

||||||| a43ed59f
/// Validates that a title does not exceed MAX_TITLE_LENGTH bytes.
///
/// @param title The title string to validate.
/// @return Ok(()) if the title is within limits, Err with descriptive message otherwise.
/// @notice Callers should treat errors as permanent rejections; the limit
///         will not change without a contract upgrade.
pub fn validate_title(title: &String) -> Result<(), &'static str> {
    if title.len() > MAX_TITLE_LENGTH {
        return Err("title exceeds MAX_TITLE_LENGTH bytes");
    }
    Ok(())
}

impl core::fmt::Display for StateSizeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StateSizeError::ContributorLimitExceeded => {
                f.write_str("contributor limit exceeded")
            }
            StateSizeError::RoadmapLimitExceeded => f.write_str("roadmap limit exceeded"),
            StateSizeError::StretchGoalLimitExceeded => {
                f.write_str("stretch goal limit exceeded")
            }
            StateSizeError::StringTooLong => f.write_str("string too long"),
        }
    }
}

impl core::fmt::Display for StateSizeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StateSizeError::ContributorLimitExceeded => {
                f.write_str("contributor limit exceeded")
            }
            StateSizeError::RoadmapLimitExceeded => f.write_str("roadmap limit exceeded"),
            StateSizeError::StretchGoalLimitExceeded => {
                f.write_str("stretch goal limit exceeded")
            }
            StateSizeError::StringTooLong => f.write_str("string too long"),
        }
    }
}

impl core::fmt::Display for StateSizeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StateSizeError::ContributorLimitExceeded => {
                write!(f, "contributor limit exceeded")
            }
            StateSizeError::RoadmapLimitExceeded => {
                write!(f, "roadmap limit exceeded")
            }
            StateSizeError::StretchGoalLimitExceeded => {
                write!(f, "stretch goal limit exceeded")
            }
            StateSizeError::StringTooLong => {
                write!(f, "string too long")
            }
        }
    }
}

// ── Validation helpers ────────────────────────────────────────────────────────

/// Validate title length.
pub fn validate_title(value: &String) -> Result<(), &'static str> {
    if value.len() > MAX_TITLE_LENGTH {
        return Err("title exceeds MAX_TITLE_LENGTH bytes".into());
    }
    Ok(())
}

/// Validate description length.
pub fn validate_description(value: &String) -> Result<(), &'static str> {
    if value.len() > MAX_DESCRIPTION_LENGTH {
        return Err("description exceeds MAX_DESCRIPTION_LENGTH bytes".into());
    }
    Ok(())
}

/// Validate social links length.
pub fn validate_social_links(value: &String) -> Result<(), &'static str> {
    if value.len() > MAX_SOCIAL_LINKS_LENGTH {
        return Err("social links exceed MAX_SOCIAL_LINKS_LENGTH bytes".into());
    }
    Ok(())
}

/// Validate bonus goal description length.
pub fn validate_bonus_goal_description(value: &String) -> Result<(), &'static str> {
    if value.len() > MAX_BONUS_GOAL_DESCRIPTION_LENGTH {
        return Err(
            "bonus goal description exceeds MAX_BONUS_GOAL_DESCRIPTION_LENGTH bytes".into(),
        );
    }
    Ok(())
}

/// Validate roadmap item description length.
pub fn validate_roadmap_description(value: &String) -> Result<(), &'static str> {
    if value.len() > MAX_ROADMAP_DESCRIPTION_LENGTH {
        return Err("roadmap description exceeds MAX_ROADMAP_DESCRIPTION_LENGTH bytes".into());
    }
    Ok(())
}

/// Validate metadata aggregate length.
=======
/// Reads the `Roadmap` list length from instance storage and enforces the cap.
#[inline]
pub fn check_roadmap_limit(env: &Env) -> Result<(), StateSizeError> {
    let count: u32 = env
        .storage()
        .instance()
        .get::<_, Vec<crate::RoadmapItem>>(&crate::DataKey::Roadmap)
        .map(|v| v.len())
        .unwrap_or(0);
    validate_roadmap_capacity(count)
}

/// Validates a roadmap item description length (delegates to `check_string_len`).
#[inline]
pub fn validate_roadmap_description(desc: &String) -> Result<(), StateSizeError> {
    check_string_len(desc)
}

/// Returns `Ok(())` if `count < MAX_STRETCH_GOALS`, else `Err(StretchGoalLimitExceeded)`.
#[inline]
pub fn validate_stretch_goal_capacity(count: u32) -> Result<(), StateSizeError> {
    if count >= MAX_STRETCH_GOALS {
        Err(StateSizeError::StretchGoalLimitExceeded)
    } else {
        Ok(())
    }
}

/// Reads the `StretchGoals` list length from instance storage and enforces the cap.
#[inline]
pub fn check_stretch_goal_limit(env: &Env) -> Result<(), StateSizeError> {
    let count: u32 = env
        .storage()
        .instance()
        .get::<_, Vec<i128>>(&crate::DataKey::StretchGoals)
        .map(|v| v.len())
        .unwrap_or(0);
    validate_stretch_goal_capacity(count)
}

/// Validates a title string length.
#[inline]
pub fn validate_title(title: &String) -> Result<(), StateSizeError> {
    check_string_len(title)
}

/// Validates a description string length.
#[inline]
pub fn validate_description(desc: &String) -> Result<(), StateSizeError> {
    check_string_len(desc)
}

/// Validates a social-links string length.
#[inline]
pub fn validate_social_links(links: &String) -> Result<(), StateSizeError> {
    check_string_len(links)
}

/// Validates the aggregate metadata length across title, description, and social links.
///
/// @param title_len       Byte length of the title field.
/// @param description_len Byte length of the description field.
/// @param socials_len     Byte length of the social-links field.
/// @return `Ok(())` if the sum is within the aggregate limit.
#[inline]
>>>>>>> origin/main
pub fn validate_metadata_total_length(
    title_len: u32,
    description_len: u32,
    socials_len: u32,
<<<<<<< HEAD
) -> Result<(), &'static str> {
    let sum = title_len
        .checked_add(description_len)
        .and_then(|v| v.checked_add(socials_len))
        .ok_or("metadata exceeds MAX_METADATA_TOTAL_LENGTH bytes")?;
    if sum > MAX_METADATA_TOTAL_LENGTH {
        return Err("metadata exceeds MAX_METADATA_TOTAL_LENGTH bytes");
||||||| a43ed59f
) -> Result<(), &'static str> {
    let sum = title_len
        .checked_add(description_len)
        .and_then(|v| v.checked_add(socials_len))
        .ok_or("metadata exceeds MAX_METADATA_TOTAL_LENGTH bytes")?;
    if sum > MAX_METADATA_TOTAL_LENGTH {
        return Err("metadata exceeds MAX_METADATA_TOTAL_LENGTH bytes".into());
=======
) -> Result<(), StateSizeError> {
    const AGGREGATE_LIMIT: u32 = MAX_STRING_LEN * 3;
    if title_len.saturating_add(description_len).saturating_add(socials_len) > AGGREGATE_LIMIT {
        Err(StateSizeError::StringTooLong)
    } else {
        Ok(())
>>>>>>> origin/main
    }
}

<<<<<<< HEAD
pub fn validate_contributor_capacity(len: u32) -> Result<(), &'static str> {
    if len >= MAX_CONTRIBUTORS {
        return Err("contributors exceed MAX_CONTRIBUTORS");
    }
    Ok(())
}
||||||| a43ed59f
/// Validate contributor index capacity before append.
pub fn validate_contributor_capacity(len: u32) -> Result<(), &'static str> {
    if len >= MAX_CONTRIBUTORS {
        return Err("contributors exceed MAX_CONTRIBUTORS".into());
    }
    Ok(())
}
=======
// ── Standalone contract (exposes constants on-chain) ─────────────────────────
>>>>>>> origin/main

<<<<<<< HEAD
pub fn validate_pledger_capacity(len: u32) -> Result<(), &'static str> {
    if len >= MAX_PLEDGERS {
        return Err("pledgers exceed MAX_PLEDGERS");
    }
    Ok(())
}

pub fn validate_roadmap_capacity(len: u32) -> Result<(), &'static str> {
    if len >= MAX_ROADMAP_ITEMS {
        return Err("roadmap exceeds MAX_ROADMAP_ITEMS");
    }
    Ok(())
}

pub fn validate_stretch_goal_capacity(len: u32) -> Result<(), &'static str> {
    if len >= MAX_STRETCH_GOALS {
        return Err("stretch goals exceed MAX_STRETCH_GOALS");
    }
    Ok(())
}

pub fn check_string_len(s: &String) -> Result<(), StateSizeError> {
    if s.len() > MAX_STRING_LEN {
        return Err(StateSizeError::StringTooLong);
||||||| a43ed59f
/// Validate pledger index capacity before append.
pub fn validate_pledger_capacity(len: u32) -> Result<(), &'static str> {
    if len >= MAX_PLEDGERS {
        return Err("pledgers exceed MAX_PLEDGERS".into());
    }
    Ok(())
}

/// Validate roadmap capacity before append.
pub fn validate_roadmap_capacity(len: u32) -> Result<(), &'static str> {
    if len >= MAX_ROADMAP_ITEMS {
        return Err("roadmap exceeds MAX_ROADMAP_ITEMS".into());
    }
    Ok(())
}

/// Validate stretch-goal capacity before append.
pub fn validate_stretch_goal_capacity(len: u32) -> Result<(), &'static str> {
    if len >= MAX_STRETCH_GOALS {
        return Err("stretch goals exceed MAX_STRETCH_GOALS".into());
    }
    Ok(())
}

/// Assert that `s` does not exceed [`MAX_STRING_LEN`] bytes.
///
/// @param s The string to validate.
/// @return Ok(()) if within limits, Err with StateSizeError otherwise.
/// @deprecated Use validate_title, validate_description, or validate_social_links instead.
pub fn check_string_len(s: &String) -> Result<(), StateSizeError> {
    if s.len() > MAX_STRING_LEN {
        return Err(StateSizeError::StringTooLong);
=======
/// On-chain contract that exposes state-size constants and validation functions.
///
/// @notice Frontend UIs can call these view functions to retrieve the current
///         limits without hard-coding them, ensuring UI validation stays in sync
///         with the contract after upgrades.
#[contract]
pub struct ContractStateSize;

#[contractimpl]
impl ContractStateSize {
    /// Returns the maximum allowed byte length for any string field.
    pub fn max_string_len(_env: Env) -> u32 {
        MAX_STRING_LEN
>>>>>>> origin/main
    }

<<<<<<< HEAD
pub fn check_contributor_limit(env: &Env) -> Result<(), StateSizeError> {
    let contributors: Vec<soroban_sdk::Address> = env
        .storage()
        .persistent()
        .get(&DataKey::Contributors)
        .unwrap_or_else(|| Vec::new(env));
    if contributors.len() >= MAX_CONTRIBUTORS {
        return Err(StateSizeError::ContributorLimitExceeded);
||||||| a43ed59f
/// Legacy function for checking contributor limit.
///
/// @param env Soroban environment reference.
/// @return Ok(()) if within limits, Err with StateSizeError otherwise.
/// @deprecated Use validate_contributor_capacity instead.
pub fn check_contributor_limit(env: &Env) -> Result<(), StateSizeError> {
    let contributors: Vec<soroban_sdk::Address> = env
        .storage()
        .persistent()
        .get(&DataKey::Contributors)
        .unwrap_or_else(|| Vec::new(env));

    if contributors.len() >= MAX_CONTRIBUTORS {
        return Err(StateSizeError::ContributorLimitExceeded);
=======
    /// Returns the maximum number of contributors per campaign.
    pub fn max_contributors(_env: Env) -> u32 {
        MAX_CONTRIBUTORS
>>>>>>> origin/main
    }

<<<<<<< HEAD
pub fn check_pledger_limit(env: &Env) -> Result<(), StateSizeError> {
    let pledgers: Vec<soroban_sdk::Address> = env
        .storage()
        .persistent()
        .get(&DataKey::Pledgers)
        .unwrap_or_else(|| Vec::new(env));
    if pledgers.len() >= MAX_PLEDGERS {
        return Err(StateSizeError::ContributorLimitExceeded);
||||||| a43ed59f
/// Legacy function for checking pledger limit.
///
/// @param env Soroban environment reference.
/// @return Ok(()) if within limits, Err with StateSizeError otherwise.
/// @deprecated Use validate_pledger_capacity instead.
pub fn check_pledger_limit(env: &Env) -> Result<(), StateSizeError> {
    let pledgers: Vec<soroban_sdk::Address> = env
        .storage()
        .persistent()
        .get(&DataKey::Pledgers)
        .unwrap_or_else(|| Vec::new(env));

    if pledgers.len() >= MAX_PLEDGERS {
        return Err(StateSizeError::ContributorLimitExceeded);
=======
    /// Returns the maximum number of roadmap items.
    pub fn max_roadmap_items(_env: Env) -> u32 {
        MAX_ROADMAP_ITEMS
>>>>>>> origin/main
    }

<<<<<<< HEAD
pub fn check_roadmap_limit(env: &Env) -> Result<(), StateSizeError> {
    let roadmap: Vec<crate::RoadmapItem> = env
        .storage()
        .instance()
        .get(&DataKey::Roadmap)
        .unwrap_or_else(|| Vec::new(env));
    if roadmap.len() >= MAX_ROADMAP_ITEMS {
        return Err(StateSizeError::RoadmapLimitExceeded);
||||||| a43ed59f
/// Legacy function for checking roadmap limit.
///
/// @param env Soroban environment reference.
/// @return Ok(()) if within limits, Err with StateSizeError otherwise.
/// @deprecated Use validate_roadmap_capacity instead.
pub fn check_roadmap_limit(env: &Env) -> Result<(), StateSizeError> {
    let roadmap: Vec<crate::RoadmapItem> = env
        .storage()
        .instance()
        .get(&DataKey::Roadmap)
        .unwrap_or_else(|| Vec::new(env));

    if roadmap.len() >= MAX_ROADMAP_ITEMS {
        return Err(StateSizeError::RoadmapLimitExceeded);
=======
    /// Returns the maximum number of stretch goals.
    pub fn max_stretch_goals(_env: Env) -> u32 {
        MAX_STRETCH_GOALS
>>>>>>> origin/main
    }

<<<<<<< HEAD
pub fn check_stretch_goal_limit(env: &Env) -> Result<(), StateSizeError> {
    let goals: Vec<i128> = env
        .storage()
        .instance()
        .get(&DataKey::StretchGoals)
        .unwrap_or_else(|| Vec::new(env));
    if goals.len() >= MAX_STRETCH_GOALS {
        return Err(StateSizeError::StretchGoalLimitExceeded);
||||||| a43ed59f
/// Legacy function for checking stretch goal limit.
///
/// @param env Soroban environment reference.
/// @return Ok(()) if within limits, Err with StateSizeError otherwise.
/// @deprecated Use validate_stretch_goal_capacity instead.
pub fn check_stretch_goal_limit(env: &Env) -> Result<(), StateSizeError> {
    let goals: Vec<i128> = env
        .storage()
        .instance()
        .get(&DataKey::StretchGoals)
        .unwrap_or_else(|| Vec::new(env));

    if goals.len() >= MAX_STRETCH_GOALS {
        return Err(StateSizeError::StretchGoalLimitExceeded);
=======
    /// Returns `true` if `s.len() <= MAX_STRING_LEN`.
    pub fn validate_string(_env: Env, s: String) -> bool {
        s.len() <= MAX_STRING_LEN
>>>>>>> origin/main
    }
}
