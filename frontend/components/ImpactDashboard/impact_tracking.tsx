import React, { useEffect, useState } from "react";
import { shortenAddress } from "../../utils/format";

// ── Types ─────────────────────────────────────────────────────────────────────

/** Campaign status as returned by the contract's `status()` function. */
export type CampaignStatus = "Active" | "Succeeded" | "Expired" | "Cancelled";

/** A single pledge event shown in the recent activity feed. */
export interface PledgeActivity {
  /** Full on-chain donor address. Truncated for display — no PII concern:
   *  Stellar addresses are public by design. */
  donor: string;
  /** Pledge amount in token base units (stroops / micro-units). */
  amount: number;
  /** Unix timestamp (seconds) when the pledge was recorded. */
  timestamp: number;
}

/** All campaign data needed by the dashboard. */
export interface CampaignData {
  totalRaised: number;
  goal: number;
  donorCount: number;
  /** Unix timestamp (seconds). */
  deadline: number;
  status: CampaignStatus;
  /** Up to 5 most recent pledges. */
  recentPledges: PledgeActivity[];
}

/**
 * Props for ImpactDashboard.
 *
 * @prop campaignId  - The on-chain contract address of the campaign.
 * @prop fetchData   - Async function that resolves campaign data from the
 *                    contract or an indexer. Injected so the component stays
 *                    testable without a live network.
 * @prop tokenSymbol - Display symbol for the token (default: "XLM").
 * @prop tokenDecimals - Decimal places for amount formatting (default: 7).
 */
export interface ImpactDashboardProps {
  campaignId: string;
  fetchData: (campaignId: string) => Promise<CampaignData>;
  tokenSymbol?: string;
  tokenDecimals?: number;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Clamp progress to [0, 100]. */
export function progressPercent(raised: number, goal: number): number {
  if (goal <= 0) return 0;
  return Math.min(100, Math.max(0, Math.round((raised / goal) * 100)));
}

/** Format a token amount from base units to a human-readable string. */
export function formatAmount(amount: number, decimals: number): string {
  const divisor = Math.pow(10, decimals);
  return (amount / divisor).toLocaleString(undefined, {
    minimumFractionDigits: 0,
    maximumFractionDigits: decimals,
  });
}

/** Return a human-readable time-remaining string, or "Ended" if past. */
export function timeRemaining(deadlineSeconds: number): string {
  const nowSeconds = Math.floor(Date.now() / 1000);
  const diff = deadlineSeconds - nowSeconds;
  if (diff <= 0) return "Ended";
  const days = Math.floor(diff / 86400);
  const hours = Math.floor((diff % 86400) / 3600);
  const minutes = Math.floor((diff % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h remaining`;
  if (hours > 0) return `${hours}h ${minutes}m remaining`;
  return `${minutes}m remaining`;
}

/** Map contract status to a display label and CSS class. */
export function statusLabel(status: CampaignStatus): { label: string; className: string } {
  switch (status) {
    case "Active":     return { label: "Active",     className: "status--active" };
    case "Succeeded":  return { label: "Successful", className: "status--success" };
    case "Expired":    return { label: "Failed",     className: "status--failed" };
    case "Cancelled":  return { label: "Cancelled",  className: "status--cancelled" };
  }
}

// ── Component ─────────────────────────────────────────────────────────────────

/**
 * @title ImpactDashboard
 * @notice Displays live campaign impact metrics fetched from the Stellar
 *         crowdfund contract. Shows funding progress, donor count, time
 *         remaining, campaign status, and recent pledge activity.
 *
 * @dev Data source: the `fetchData` prop wraps calls to the contract's
 *   `total_raised()`, `goal()`, `deadline()`, `status()`, and
 *   `contributors()` view functions (or an equivalent indexer endpoint).
 *   ⚠️  Verify these function names against the deployed contract ABI before
 *   wiring up a real implementation.
 *
 * @security Donor addresses are public on-chain — no PII concern. Addresses
 *   are truncated for display only (cosmetic). No user-supplied HTML is
 *   rendered; all values are inserted as React text nodes.
 *
 * @accessibility
 *   - Progress bar uses role="progressbar" with aria-valuenow/min/max.
 *   - Sections use role="region" with aria-label.
 *   - Status badge uses aria-label for screen readers.
 */
const ImpactDashboard: React.FC<ImpactDashboardProps> = ({
  campaignId,
  fetchData,
  tokenSymbol = "XLM",
  tokenDecimals = 7,
}) => {
  const [data, setData] = useState<CampaignData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetchData(campaignId)
      .then((result) => { if (!cancelled) { setData(result); setLoading(false); } })
      .catch((err) => { if (!cancelled) { setError(err?.message ?? "Failed to load campaign data"); setLoading(false); } });
    return () => { cancelled = true; };
  }, [campaignId, fetchData]);

  if (loading) {
    return (
      <div className="impact-dashboard impact-dashboard--loading" aria-busy="true" aria-label="Loading campaign data">
        <p>Loading campaign data…</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="impact-dashboard impact-dashboard--error" role="alert">
        <p>Error: {error}</p>
      </div>
    );
  }

  if (!data) return null;

  const pct = progressPercent(data.totalRaised, data.goal);
  const { label: sLabel, className: sClass } = statusLabel(data.status);
  const timeLeft = timeRemaining(data.deadline);

  return (
    <div className="impact-dashboard" data-testid="impact-dashboard">

      {/* ── Status ── */}
      <section role="region" aria-label="Campaign status">
        <span
          className={`status-badge ${sClass}`}
          aria-label={`Campaign status: ${sLabel}`}
          data-testid="status-badge"
        >
          {sLabel}
        </span>
      </section>

      {/* ── Funding progress ── */}
      <section role="region" aria-label="Funding progress">
        <h2>Funding Progress</h2>
        <p data-testid="progress-text">
          {formatAmount(data.totalRaised, tokenDecimals)} / {formatAmount(data.goal, tokenDecimals)} {tokenSymbol}
        </p>
        <div
          role="progressbar"
          aria-valuenow={pct}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-label={`${pct}% funded`}
          data-testid="progress-bar"
          className="progress-bar"
        >
          <div className="progress-bar__fill" style={{ width: `${pct}%` }} data-testid="progress-fill" />
        </div>
        <p data-testid="progress-pct">{pct}% funded</p>
      </section>

      {/* ── Stats ── */}
      <section role="region" aria-label="Campaign statistics">
        <dl>
          <div>
            <dt>Unique Donors</dt>
            <dd data-testid="donor-count">
              {data.donorCount === 0 ? "No donors yet" : data.donorCount.toLocaleString()}
            </dd>
          </div>
          <div>
            <dt>Time Remaining</dt>
            <dd data-testid="time-remaining">{timeLeft}</dd>
          </div>
        </dl>
      </section>

      {/* ── Recent activity ── */}
      <section role="region" aria-label="Recent pledge activity">
        <h2>Recent Pledges</h2>
        {data.recentPledges.length === 0 ? (
          <p data-testid="no-pledges">No pledges yet.</p>
        ) : (
          <ul aria-label="Recent pledge list">
            {data.recentPledges.slice(0, 5).map((pledge, i) => (
              <li key={i} data-testid={`pledge-item-${i}`}>
                <span data-testid={`pledge-donor-${i}`}>{shortenAddress(pledge.donor, 6, 4)}</span>
                {" — "}
                <span data-testid={`pledge-amount-${i}`}>
                  {formatAmount(pledge.amount, tokenDecimals)} {tokenSymbol}
                </span>
                {" · "}
                <time dateTime={new Date(pledge.timestamp * 1000).toISOString()}>
                  {new Date(pledge.timestamp * 1000).toLocaleString()}
                </time>
              </li>
            ))}
          </ul>
        )}
      </section>

    </div>
  );
};

export default ImpactDashboard;
