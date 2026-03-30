# ImpactDashboard — Campaign Impact Tracking

## Purpose

`ImpactDashboard` is a React component that displays live campaign impact metrics for a Stellar Raise crowdfunding campaign. It shows funding progress, donor count, time remaining, campaign status, and recent pledge activity.

## Usage

```tsx
import ImpactDashboard from "./components/ImpactDashboard/impact_tracking";

// Wire up a real data fetcher that calls the contract view functions
async function fetchCampaignData(campaignId: string) {
  // TODO: replace with real Stellar SDK / RPC calls
  // Contract functions called (verify against deployed ABI):
  //   total_raised(env) -> i128
  //   goal(env) -> i128
  //   deadline(env) -> u64
  //   status(env) -> Status
  //   contributors(env) -> Vec<Address>
  //   contribution(env, contributor) -> i128
  return { ... };
}

<ImpactDashboard
  campaignId="CCONTRACT_ADDRESS_HERE"
  fetchData={fetchCampaignData}
  tokenSymbol="XLM"
  tokenDecimals={7}
/>
```

## Props

| Prop | Type | Required | Description |
|---|---|---|---|
| `campaignId` | `string` | ✅ | On-chain contract address of the campaign |
| `fetchData` | `(campaignId: string) => Promise<CampaignData>` | ✅ | Async function that returns campaign data. Injected for testability. |
| `tokenSymbol` | `string` | ❌ | Display symbol for the token. Default: `"XLM"` |
| `tokenDecimals` | `number` | ❌ | Decimal places for amount display. Default: `7` (stroops) |

### `CampaignData` shape

| Field | Type | Description |
|---|---|---|
| `totalRaised` | `number` | Total raised in token base units |
| `goal` | `number` | Funding goal in token base units |
| `donorCount` | `number` | Number of unique donors |
| `deadline` | `number` | Unix timestamp (seconds) |
| `status` | `"Active" \| "Succeeded" \| "Expired" \| "Cancelled"` | Campaign status |
| `recentPledges` | `PledgeActivity[]` | Up to 5 most recent pledges |

### `PledgeActivity` shape

| Field | Type | Description |
|---|---|---|
| `donor` | `string` | Full on-chain donor address (truncated for display) |
| `amount` | `number` | Pledge amount in token base units |
| `timestamp` | `number` | Unix timestamp (seconds) |

## Data sources

The `fetchData` prop should call these contract view functions. **Verify names against the deployed contract ABI before wiring up:**

| Contract function | Returns | Used for |
|---|---|---|
| `total_raised()` | `i128` | Funding progress |
| `goal()` | `i128` | Funding goal |
| `deadline()` | `u64` | Time remaining |
| `status()` | `Status` enum | Campaign status badge |
| `contributors()` | `Vec<Address>` | Donor count |
| `contribution(contributor)` | `i128` | Recent pledge amounts |

## How to run the tests

```bash
# Run all tests
npm test

# Run only impact_tracking tests
npm test -- impact_tracking

# With coverage
npm run test:coverage -- impact_tracking
```

## Accessibility notes

- Progress bar uses `role="progressbar"` with `aria-valuenow`, `aria-valuemin`, `aria-valuemax`.
- Each section uses `role="region"` with `aria-label`.
- Status badge has `aria-label="Campaign status: <value>"` for screen readers.
- Error state uses `role="alert"` for immediate announcement.
- Loading state uses `aria-busy="true"`.
- Pledge timestamps use `<time dateTime="...">` with ISO 8601 `dateTime` attribute.
- No keyboard-interactive elements beyond native browser defaults.
