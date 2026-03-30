import React, { useMemo } from "react";

/**
 * @title RewardDistribution
 * @notice Campaign milestone reward distribution UI for the Stellar Raise
 *         crowdfunding dApp. Displays claimable rewards per milestone tier
 *         and allows contributors to claim their rewards.
 *
 * @dev Security assumptions:
 *   - No dangerouslySetInnerHTML — all content rendered as React text nodes.
 *   - All user-supplied strings pass through sanitizeRewardString before render.
 *   - totalRaised and goal are clamped to non-negative values before use.
 *   - All numeric inputs are validated with Number.isFinite before use.
 *   - onClaim callback is only invoked for enabled (claimable) tiers.
 *
 * @custom:accessibility
 *   - role="region" with aria-label on the root element.
 *   - role="progressbar" with aria-valuenow/min/max on the funding bar.
 *   - Claim buttons carry descriptive aria-label attributes.
 *   - Disabled buttons use the disabled attribute for assistive technology.
 */

// ── Constants ─────────────────────────────────────────────────────────────────

/** Milestone thresholds as funding percentages. */
export const REWARD_TIERS = [25, 50, 75, 100] as const;
export type RewardMilestone = (typeof REWARD_TIERS)[number];

/** Maximum characters for a reward tier label. */
export const MAX_REWARD_NAME_LENGTH = 60;

/** Default reward pool when none is provided. */
export const DEFAULT_REWARD_POOL = 0;

// ── Types ─────────────────────────────────────────────────────────────────────

/** A single reward tier tied to a milestone percentage. */
export interface RewardTier {
  /** Milestone percentage threshold (25 | 50 | 75 | 100). */
  milestone: number;
  /** Display label — sanitized before render. */
  label: string;
  /** Token amount distributed when this tier is claimed. */
  rewardAmount: number;
  /** Whether this tier has already been claimed. */
  claimed: boolean;
}

/** Props for the RewardDistribution component. */
export interface RewardDistributionProps {
  /** Campaign display name — sanitized before render. */
  campaignName: string;
  /** Tokens raised so far (clamped to ≥ 0). */
  totalRaised: number;
  /** Campaign funding goal (clamped to ≥ 0). */
  goal: number;
  /** Total token pool available for rewards. */
  rewardPool: number;
  /** Ordered list of reward tiers. */
  tiers: RewardTier[];
  /** Called with the milestone value when a contributor claims a reward. */
  onClaim: (milestone: number) => void;
  /** Optional test identifier. */
  "data-testid"?: string;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/**
 * @notice Sanitizes a string for safe display by trimming whitespace and
 *         truncating to maxLen characters.
 * @dev    Never returns undefined or null — always a plain string.
 */
export function sanitizeRewardString(s: string, maxLen: number): string {
  return String(s).trim().slice(0, maxLen);
}

/**
 * @notice Computes the claimable token amount for a reward tier.
 * @dev    Returns 0 if the tier is already claimed, the pool is empty,
 *         or any input fails Number.isFinite validation.
 */
export function computeClaimableAmount(
  tier: RewardTier,
  rewardPool: number
): number {
  if (!Number.isFinite(rewardPool) || rewardPool <= 0) return 0;
  if (!Number.isFinite(tier.rewardAmount) || tier.claimed) return 0;
  return tier.rewardAmount;
}

// ── Component ─────────────────────────────────────────────────────────────────

const RewardDistribution: React.FC<RewardDistributionProps> = ({
  campaignName,
  totalRaised,
  goal,
  rewardPool,
  tiers,
  onClaim,
  "data-testid": testId = "reward-distribution",
}) => {
  const safeName = sanitizeRewardString(campaignName, MAX_REWARD_NAME_LENGTH);

  const safeRaised = Number.isFinite(totalRaised) ? Math.max(0, totalRaised) : 0;
  const safeGoal = Number.isFinite(goal) ? Math.max(0, goal) : 0;

  const progressPct = useMemo(
    () => Math.min(100, safeGoal > 0 ? (safeRaised / safeGoal) * 100 : 0),
    [safeRaised, safeGoal]
  );

  return (
    <section
      role="region"
      aria-label={`Reward distribution for ${safeName}`}
      data-testid={testId}
    >
      <h2 data-testid="campaign-name">{safeName}</h2>

      {/* Funding progress bar */}
      <div
        role="progressbar"
        aria-valuenow={Math.round(progressPct)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label="Funding progress"
        data-testid="progress-bar"
        style={{ width: `${progressPct}%` }}
      />

      {/* Reward tiers */}
      <ul data-testid="reward-tiers">
        {tiers.map((tier) => {
          const isClaimable = !tier.claimed && progressPct >= tier.milestone;
          const claimable = computeClaimableAmount(tier, rewardPool);
          const safeLabel = sanitizeRewardString(tier.label, MAX_REWARD_NAME_LENGTH);

          return (
            <li key={tier.milestone} data-testid={`tier-${tier.milestone}`}>
              <span data-testid={`tier-label-${tier.milestone}`}>{safeLabel}</span>
              <span data-testid={`tier-amount-${tier.milestone}`}>
                {claimable} tokens
              </span>
              <span data-testid={`tier-status-${tier.milestone}`}>
                {tier.claimed ? "Claimed" : "Unclaimed"}
              </span>
              <button
                disabled={!isClaimable}
                aria-label={`Claim reward for ${safeLabel} milestone`}
                data-testid={`claim-btn-${tier.milestone}`}
                onClick={() => onClaim(tier.milestone)}
              >
                Claim
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
};

export default RewardDistribution;
