import React from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import RewardDistribution, {
  computeClaimableAmount,
  sanitizeRewardString,
  MAX_REWARD_NAME_LENGTH,
  RewardTier,
  RewardDistributionProps,
} from "./reward_distribution";

// ── Fixtures ──────────────────────────────────────────────────────────────────

const baseTiers: RewardTier[] = [
  { milestone: 25, label: "Early Bird", rewardAmount: 10, claimed: false },
  { milestone: 50, label: "Halfway", rewardAmount: 20, claimed: false },
  { milestone: 75, label: "Almost There", rewardAmount: 30, claimed: false },
  { milestone: 100, label: "Goal Reached", rewardAmount: 50, claimed: false },
];

const defaultProps: RewardDistributionProps = {
  campaignName: "Test Campaign",
  totalRaised: 500,
  goal: 1000,
  rewardPool: 200,
  tiers: baseTiers,
  onClaim: jest.fn(),
};

function renderComponent(props: Partial<RewardDistributionProps> = {}) {
  return render(<RewardDistribution {...defaultProps} {...props} />);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("RewardDistribution", () => {
  beforeEach(() => jest.clearAllMocks());

  it("renders campaign name", () => {
    renderComponent();
    expect(screen.getByTestId("campaign-name")).toHaveTextContent("Test Campaign");
  });

  it("renders progress bar with correct aria attributes", () => {
    renderComponent({ totalRaised: 500, goal: 1000 });
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("aria-valuenow", "50");
    expect(bar).toHaveAttribute("aria-valuemin", "0");
    expect(bar).toHaveAttribute("aria-valuemax", "100");
  });

  it("renders all reward tiers", () => {
    renderComponent();
    for (const tier of baseTiers) {
      expect(screen.getByTestId(`tier-${tier.milestone}`)).toBeInTheDocument();
    }
  });

  it("disables claim buttons when progress is below milestone", () => {
    renderComponent({ totalRaised: 0, goal: 1000 });
    for (const tier of baseTiers) {
      expect(screen.getByTestId(`claim-btn-${tier.milestone}`)).toBeDisabled();
    }
  });

  it("enables claim buttons when progress meets milestone", () => {
    renderComponent({ totalRaised: 1000, goal: 1000 });
    for (const tier of baseTiers) {
      expect(screen.getByTestId(`claim-btn-${tier.milestone}`)).not.toBeDisabled();
    }
  });

  it("disables claim button for already claimed tier", () => {
    const tiers: RewardTier[] = [
      { milestone: 25, label: "Early Bird", rewardAmount: 10, claimed: true },
    ];
    renderComponent({ totalRaised: 1000, goal: 1000, tiers });
    expect(screen.getByTestId("claim-btn-25")).toBeDisabled();
  });

  it("calls onClaim with correct milestone value", () => {
    const onClaim = jest.fn();
    renderComponent({ totalRaised: 1000, goal: 1000, onClaim });
    fireEvent.click(screen.getByTestId("claim-btn-25"));
    expect(onClaim).toHaveBeenCalledWith(25);
  });

  it("sanitizes long campaign name", () => {
    const longName = "A".repeat(MAX_REWARD_NAME_LENGTH + 20);
    renderComponent({ campaignName: longName });
    const displayed = screen.getByTestId("campaign-name").textContent ?? "";
    expect(displayed.length).toBeLessThanOrEqual(MAX_REWARD_NAME_LENGTH);
  });

  it("handles zero goal gracefully without crashing", () => {
    renderComponent({ goal: 0, totalRaised: 100 });
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("aria-valuenow", "0");
  });

  it("clamps negative totalRaised to zero", () => {
    renderComponent({ totalRaised: -500, goal: 1000 });
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("aria-valuenow", "0");
  });
});

// ── Unit: computeClaimableAmount ──────────────────────────────────────────────

describe("computeClaimableAmount", () => {
  const tier: RewardTier = {
    milestone: 50,
    label: "Halfway",
    rewardAmount: 20,
    claimed: false,
  };

  it("returns 0 for claimed tier", () => {
    expect(computeClaimableAmount({ ...tier, claimed: true }, 100)).toBe(0);
  });

  it("returns 0 for empty pool", () => {
    expect(computeClaimableAmount(tier, 0)).toBe(0);
  });

  it("returns 0 for negative pool", () => {
    expect(computeClaimableAmount(tier, -50)).toBe(0);
  });

  it("returns rewardAmount for valid unclaimed tier", () => {
    expect(computeClaimableAmount(tier, 100)).toBe(20);
  });

  it("returns 0 for non-finite pool", () => {
    expect(computeClaimableAmount(tier, NaN)).toBe(0);
    expect(computeClaimableAmount(tier, Infinity)).toBe(0);
  });
});

// ── Unit: sanitizeRewardString ────────────────────────────────────────────────

describe("sanitizeRewardString", () => {
  it("trims whitespace", () => {
    expect(sanitizeRewardString("  hello  ", 20)).toBe("hello");
  });

  it("truncates to maxLen", () => {
    expect(sanitizeRewardString("abcdef", 3)).toBe("abc");
  });

  it("handles empty string", () => {
    expect(sanitizeRewardString("", 10)).toBe("");
  });
});
