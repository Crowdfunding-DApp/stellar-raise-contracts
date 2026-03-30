import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import ImpactDashboard, {
  progressPercent,
  formatAmount,
  timeRemaining,
  statusLabel,
  type CampaignData,
  type ImpactDashboardProps,
} from "./impact_tracking";

// ── Fixtures ──────────────────────────────────────────────────────────────────

const FUTURE_DEADLINE = Math.floor(Date.now() / 1000) + 86400; // 1 day from now
const PAST_DEADLINE   = Math.floor(Date.now() / 1000) - 3600;  // 1 hour ago

const baseCampaign: CampaignData = {
  totalRaised: 5_000_0000000, // 5000 XLM in stroops
  goal:        10_000_0000000, // 10000 XLM
  donorCount:  42,
  deadline:    FUTURE_DEADLINE,
  status:      "Active",
  recentPledges: [
    { donor: "GABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890ABCDEFGHIJKLMNOPQRSTU", amount: 100_0000000, timestamp: 1_700_000_000 },
    { donor: "GZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ", amount: 50_0000000,  timestamp: 1_700_000_100 },
  ],
};

function makeFetch(data: CampaignData) {
  return jest.fn().mockResolvedValue(data);
}

function makeFetchError(message = "Network error") {
  return jest.fn().mockRejectedValue(new Error(message));
}

function renderDashboard(overrides: Partial<ImpactDashboardProps> = {}) {
  return render(
    <ImpactDashboard
      campaignId="CTEST123"
      fetchData={makeFetch(baseCampaign)}
      {...overrides}
    />
  );
}

// ── Unit: pure helpers ────────────────────────────────────────────────────────

describe("progressPercent", () => {
  it("returns 50 for half-funded", () => expect(progressPercent(500, 1000)).toBe(50));
  it("clamps to 100 when over-funded", () => expect(progressPercent(2000, 1000)).toBe(100));
  it("returns 0 for zero goal", () => expect(progressPercent(100, 0)).toBe(0));
  it("returns 0 for zero raised", () => expect(progressPercent(0, 1000)).toBe(0));
});

describe("formatAmount", () => {
  it("formats stroops to XLM with 7 decimals", () =>
    expect(formatAmount(10_0000000, 7)).toBe("10"));
  it("handles zero", () => expect(formatAmount(0, 7)).toBe("0"));
});

describe("timeRemaining", () => {
  it("returns 'Ended' for past deadline", () =>
    expect(timeRemaining(PAST_DEADLINE)).toBe("Ended"));
  it("returns days remaining for future deadline", () =>
    expect(timeRemaining(FUTURE_DEADLINE)).toMatch(/\d+d/));
});

describe("statusLabel", () => {
  it("maps Active correctly",    () => expect(statusLabel("Active").label).toBe("Active"));
  it("maps Succeeded correctly", () => expect(statusLabel("Succeeded").label).toBe("Successful"));
  it("maps Expired correctly",   () => expect(statusLabel("Expired").label).toBe("Failed"));
  it("maps Cancelled correctly", () => expect(statusLabel("Cancelled").label).toBe("Cancelled"));
});

// ── Component: renders correctly with valid data ──────────────────────────────

describe("ImpactDashboard", () => {
  it("renders correctly with valid campaign data", async () => {
    renderDashboard();
    await waitFor(() => expect(screen.getByTestId("impact-dashboard")).toBeInTheDocument());
    expect(screen.getByTestId("status-badge")).toHaveTextContent("Active");
    expect(screen.getByTestId("donor-count")).toHaveTextContent("42");
    expect(screen.getByTestId("progress-pct")).toHaveTextContent("50%");
  });

  // ── Progress bar ────────────────────────────────────────────────────────────

  it("shows progress bar at correct percentage", async () => {
    renderDashboard();
    await waitFor(() => screen.getByTestId("progress-bar"));
    const bar = screen.getByTestId("progress-bar");
    expect(bar).toHaveAttribute("aria-valuenow", "50");
    const fill = screen.getByTestId("progress-fill");
    expect(fill).toHaveStyle({ width: "50%" });
  });

  it("clamps progress bar to 100% when over-funded", async () => {
    const data: CampaignData = { ...baseCampaign, totalRaised: 20_000_0000000 };
    renderDashboard({ fetchData: makeFetch(data) });
    await waitFor(() => screen.getByTestId("progress-bar"));
    expect(screen.getByTestId("progress-bar")).toHaveAttribute("aria-valuenow", "100");
  });

  // ── Zero donors ─────────────────────────────────────────────────────────────

  it("handles zero donors gracefully", async () => {
    const data: CampaignData = { ...baseCampaign, donorCount: 0, recentPledges: [] };
    renderDashboard({ fetchData: makeFetch(data) });
    await waitFor(() => screen.getByTestId("donor-count"));
    expect(screen.getByTestId("donor-count")).toHaveTextContent("No donors yet");
    expect(screen.getByTestId("no-pledges")).toBeInTheDocument();
  });

  // ── Campaign ended ──────────────────────────────────────────────────────────

  it("shows 'Ended' when deadline has passed", async () => {
    const data: CampaignData = { ...baseCampaign, deadline: PAST_DEADLINE, status: "Expired" };
    renderDashboard({ fetchData: makeFetch(data) });
    await waitFor(() => screen.getByTestId("time-remaining"));
    expect(screen.getByTestId("time-remaining")).toHaveTextContent("Ended");
    expect(screen.getByTestId("status-badge")).toHaveTextContent("Failed");
  });

  // ── Error state ─────────────────────────────────────────────────────────────

  it("shows error state when data fetch fails", async () => {
    renderDashboard({ fetchData: makeFetchError("Network error") });
    await waitFor(() => screen.getByRole("alert"));
    expect(screen.getByRole("alert")).toHaveTextContent("Network error");
  });

  it("shows generic error message when error has no message", async () => {
    const fetchData = jest.fn().mockRejectedValue({});
    renderDashboard({ fetchData });
    await waitFor(() => screen.getByRole("alert"));
    expect(screen.getByRole("alert")).toHaveTextContent("Failed to load campaign data");
  });

  // ── Address truncation ──────────────────────────────────────────────────────

  it("truncates long donor addresses correctly", async () => {
    renderDashboard();
    await waitFor(() => screen.getByTestId("pledge-donor-0"));
    const displayed = screen.getByTestId("pledge-donor-0").textContent ?? "";
    expect(displayed).toMatch(/^.{6}\.\.\.(.{4})$/);
    expect(displayed.length).toBeLessThan(baseCampaign.recentPledges[0].donor.length);
  });

  // ── Loading state ───────────────────────────────────────────────────────────

  it("shows loading state initially", () => {
    // fetchData never resolves during this test
    const fetchData = jest.fn().mockReturnValue(new Promise(() => {}));
    render(<ImpactDashboard campaignId="C1" fetchData={fetchData} />);
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  // ── Recent pledges capped at 5 ──────────────────────────────────────────────

  it("shows at most 5 recent pledges", async () => {
    const pledges = Array.from({ length: 8 }, (_, i) => ({
      donor: `G${"A".repeat(55)}${i}`,
      amount: 10_0000000,
      timestamp: 1_700_000_000 + i,
    }));
    const data: CampaignData = { ...baseCampaign, recentPledges: pledges };
    renderDashboard({ fetchData: makeFetch(data) });
    await waitFor(() => screen.getByTestId("pledge-item-0"));
    expect(screen.queryByTestId("pledge-item-5")).not.toBeInTheDocument();
    expect(screen.getByTestId("pledge-item-4")).toBeInTheDocument();
  });
});
