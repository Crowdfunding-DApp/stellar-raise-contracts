# Milestone Reward Distribution

Frontend UI component for displaying and claiming milestone-based rewards in
the Stellar Raise crowdfunding dApp.

## Overview

`RewardDistribution` is a React/TypeScript component that renders a campaign's
reward tiers alongside a live funding progress bar. Contributors can claim
token rewards for each milestone tier once the campaign reaches the required
funding percentage.

The component is stateless — all data flows in via props and mutations are
communicated upward through the `onClaim` callback.

## Security Assumptions

1. **No `dangerouslySetInnerHTML`** — all content is rendered as React text nodes.
2. **Sanitized strings** — `campaignName` and tier labels pass through
   `sanitizeRewardString` before render, preventing XSS via injected markup.
3. **Validated numerics** — `totalRaised`, `goal`, and `rewardPool` are
   validated with `Number.isFinite` and clamped to non-negative values.
4. **Non-negative clamp** — negative `totalRaised` or `goal` values are
   treated as `0` rather than producing negative progress percentages.
5. **Claim guard** — `onClaim` is only invoked when the button is enabled
   (progress ≥ milestone and tier not yet claimed).

## Component API

| Prop | Type | Description |
|:-----|:-----|:------------|
| `campaignName` | `string` | Campaign display name (sanitized) |
| `totalRaised` | `number` | Tokens raised so far |
| `goal` | `number` | Campaign funding goal |
| `rewardPool` | `number` | Total token pool for rewards |
| `tiers` | `RewardTier[]` | Ordered list of reward tiers |
| `onClaim` | `(milestone: number) => void` | Claim callback |
| `data-testid` | `string?` | Optional test identifier |

### RewardTier

```ts
interface RewardTier {
  milestone: number;   // Percentage threshold (25 | 50 | 75 | 100)
  label: string;       // Display label
  rewardAmount: number;// Token amount for this tier
  claimed: boolean;    // Whether already claimed
}
```

## Reward Tiers

The default milestone thresholds are `[25, 50, 75, 100]` (exported as
`REWARD_TIERS`). A tier's claim button is enabled only when:

- The campaign's funding progress ≥ `tier.milestone` percent, **and**
- `tier.claimed === false`

## Usage Example

```tsx
import RewardDistribution, { RewardTier } from "./reward_distribution";

const tiers: RewardTier[] = [
  { milestone: 25, label: "Early Bird",   rewardAmount: 10, claimed: false },
  { milestone: 50, label: "Halfway",      rewardAmount: 20, claimed: false },
  { milestone: 75, label: "Almost There", rewardAmount: 30, claimed: false },
  { milestone: 100, label: "Goal Reached",rewardAmount: 50, claimed: false },
];

<RewardDistribution
  campaignName="My Campaign"
  totalRaised={750}
  goal={1000}
  rewardPool={110}
  tiers={tiers}
  onClaim={(milestone) => console.log("Claiming", milestone)}
/>
```

## Test Coverage

The test suite (`reward_distribution.test.tsx`) covers:

- Campaign name rendering and sanitization
- Progress bar aria attributes
- All four tiers rendered
- Claim buttons disabled below milestone threshold
- Claim buttons enabled at 100% progress
- Claimed tier button remains disabled
- `onClaim` called with correct milestone value
- Long name truncation
- Zero goal edge case
- Negative `totalRaised` clamping
- `computeClaimableAmount` — claimed, empty pool, negative pool, valid, non-finite
- `sanitizeRewardString` — trim, truncate, empty string

Target: ≥ 95% of all code paths covered.

## Related Files

- [`frontend/components/reward_distribution.tsx`](../frontend/components/reward_distribution.tsx) — Component
- [`frontend/components/reward_distribution.test.tsx`](../frontend/components/reward_distribution.test.tsx) — Tests
- [`frontend/components/milestone_dashboard.tsx`](../frontend/components/milestone_dashboard.tsx) — Related dashboard component
