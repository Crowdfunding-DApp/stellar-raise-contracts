//! # performance_profiling
//!
//! @title   PerformanceProfiling — Automated performance measurement for testing
//! @notice  Provides lightweight profiling primitives to measure and report
//!          execution metrics for the Stellar Raise crowdfund contract.
//! @dev     All functions are pure or read-only with respect to ledger state.
//!          No auth required — profiling is permissionless so automated
//!          tooling does not need a privileged key.
//!
//! ## Security Assumptions
//! 1. **Read-only**      — No function writes to contract storage.
//! 2. **No auth required** — Profiling is permissionless.
//! 3. **Deterministic**  — Same inputs always produce the same output.
//! 4. **Overflow-safe**  — All arithmetic uses `saturating_*` operations.
//! 5. **Bounded**        — Sample counts are bounded by `MAX_SAMPLES`.

#![allow(dead_code)]

use soroban_sdk::{Env, Symbol};

// ── Constants ─────────────────────────────────────────────────────────────────

/// @notice Maximum number of samples a `ProfileReport` will store.
pub const MAX_SAMPLES: usize = 1_000;

/// @notice Soroban instruction budget limit used for utilization calculations.
pub const BUDGET_INSTRUCTION_LIMIT: u64 = 100_000_000;

// ── ProfileSample ─────────────────────────────────────────────────────────────

/// @notice A single profiling measurement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileSample {
    pub label: &'static str,
    pub instructions: u64,
    pub memory_bytes: u64,
}

impl ProfileSample {
    pub fn new(label: &'static str, instructions: u64, memory_bytes: u64) -> Self {
        Self { label, instructions, memory_bytes }
    }

    /// @notice Returns `true` when instructions are within the budget limit.
    pub fn is_within_budget(&self) -> bool {
        self.instructions <= BUDGET_INSTRUCTION_LIMIT
    }
}

// ── ProfileReport ─────────────────────────────────────────────────────────────

/// @notice Aggregated profiling report for a set of operations.
#[derive(Debug, Default)]
pub struct ProfileReport {
    pub samples: Vec<ProfileSample>,
    pub total_instructions: u64,
    pub peak_memory_bytes: u64,
}

impl ProfileReport {
    /// @notice Creates an empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// @notice Adds a sample if the sample limit has not been reached.
    ///
    /// Updates `total_instructions` (saturating) and `peak_memory_bytes`.
    pub fn add_sample(&mut self, sample: ProfileSample) {
        if self.samples.len() >= MAX_SAMPLES {
            return;
        }
        self.total_instructions =
            self.total_instructions.saturating_add(sample.instructions);
        if sample.memory_bytes > self.peak_memory_bytes {
            self.peak_memory_bytes = sample.memory_bytes;
        }
        self.samples.push(sample);
    }

    /// @notice Returns the number of stored samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// @notice Returns the mean instruction count, or `0` for an empty report.
    pub fn average_instructions(&self) -> u64 {
        let count = self.samples.len() as u64;
        if count == 0 {
            return 0;
        }
        self.total_instructions / count
    }

    /// @notice Returns budget utilization in basis points (0–10 000).
    ///
    /// Capped at 10 000 bps (100%) even when total exceeds the limit.
    pub fn budget_utilization_bps(&self) -> u32 {
        let limit = BUDGET_INSTRUCTION_LIMIT.max(1);
        let bps = self.total_instructions.saturating_mul(10_000) / limit;
        bps.min(10_000) as u32
    }
}

// ── Functions ─────────────────────────────────────────────────────────────────

/// @notice Constructs a `ProfileSample` for the given operation.
pub fn profile_operation(
    label: &'static str,
    instructions: u64,
    memory_bytes: u64,
) -> ProfileSample {
    ProfileSample::new(label, instructions, memory_bytes)
}

/// @notice Returns `true` when the sample is within the instruction budget.
pub fn check_budget(sample: &ProfileSample) -> bool {
    sample.is_within_budget()
}

/// @notice Emits a profiling event on-chain for off-chain monitors.
pub fn emit_profile_event(env: &Env, _label: &'static str, instructions: u64) {
    env.events()
        .publish((Symbol::new(env, "profile"),), (instructions,));
}
