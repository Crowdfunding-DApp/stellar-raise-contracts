//! Campaign goal minimum constants and validation helpers.

pub const MIN_GOAL_AMOUNT: i128 = 1;
pub const MIN_CONTRIBUTION_AMOUNT: i128 = 1;
pub const MAX_PLATFORM_FEE_BPS: u32 = 10_000;
pub const PROGRESS_BPS_SCALE: i128 = 10_000;
pub const MIN_DEADLINE_OFFSET: u64 = 60;
pub const MAX_PROGRESS_BPS: u32 = 10_000;

#[inline]
pub fn validate_goal(goal: i128) -> Result<(), &'static str> {
    if goal < MIN_GOAL_AMOUNT {
        return Err("goal must be at least MIN_GOAL_AMOUNT");
    }
    Ok(())
}

#[inline]
pub fn validate_min_contribution(min_contribution: i128) -> Result<(), &'static str> {
    if min_contribution < MIN_CONTRIBUTION_AMOUNT {
        return Err("min_contribution must be at least MIN_CONTRIBUTION_AMOUNT");
    }
    Ok(())
}

#[inline]
pub fn validate_deadline(now: u64, deadline: u64) -> Result<(), &'static str> {
    if deadline < now.saturating_add(MIN_DEADLINE_OFFSET) {
        return Err("deadline must be at least MIN_DEADLINE_OFFSET seconds in the future");
    }
    Ok(())
}

#[inline]
pub fn validate_platform_fee(fee_bps: u32) -> Result<(), &'static str> {
    if fee_bps > MAX_PLATFORM_FEE_BPS {
        return Err("platform fee cannot exceed MAX_PLATFORM_FEE_BPS (100%)");
    }
    Ok(())
}

#[inline]
pub fn compute_progress_bps(total_raised: i128, goal: i128) -> u32 {
    if goal <= 0 || total_raised <= 0 {
        return 0;
    }
    let scaled = total_raised
        .checked_mul(PROGRESS_BPS_SCALE)
        .unwrap_or(i128::MAX);
    let raw = scaled / goal;
    if raw >= PROGRESS_BPS_SCALE {
        MAX_PROGRESS_BPS
    } else {
        raw as u32
    }
}
