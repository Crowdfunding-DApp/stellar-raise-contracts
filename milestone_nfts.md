# Campaign Milestone Celebration NFTs

## Overview

This module introduces a secure, frontend-ready NFT milestone workflow for campaign celebrations. It computes deterministic collectible metadata from campaign progress and exposes a React panel that previews unlock state.

## Files

| File | Purpose |
|------|---------|
| `milestone_nfts.tsx` | Core types, security helpers, NFT mint plan computation, and `MilestoneNftPanel` UI |
| `milestone_nfts.test.tsx` | Unit + component tests for logic, security checks, and rendering |
| `milestone_nfts.md` | Implementation and security notes |

## What It Adds

- Calculates funded percent and milestone unlocks at `25%`, `50%`, `75%`, and `100%`
- Produces deterministic metadata payloads for collectible mint flows
- Maps milestone tiers to rarity (`common`, `rare`, `epic`, `legendary`)
- Validates and sanitizes user-controlled fields before rendering or metadata output
- Renders a reviewable panel for achieved/pending milestones and generated NFT entries

## API

### `computeCampaignMilestoneNftPlan(input)`

Returns a `MilestoneNftMintPlan` with:

- `percentFunded`
- `achievedMilestones`
- `pendingMilestones`
- `metadataToMint`
- `securityChecks`

If security checks fail, `metadataToMint` is intentionally empty.

### `MilestoneNftSecurity`

- `sanitizeText(input)` strips control chars/angle fragments and caps length (`NFT_MAX_TEXT_LENGTH`)
- `sanitizeHttpUri(uri)` permits only `http://` or `https://` and normalizes trailing slash
- `isSafeCampaignId(campaignId)` enforces `[a-zA-Z0-9_-]{1,64}`
- `clampNonNegative(value)` protects numeric calculations from negative/non-finite input

### `MilestoneNftPanel`

Frontend component that:

- Shows campaign funded percent
- Lists achieved and pending milestone thresholds
- Lists minted metadata previews when input is secure
- Displays an alert when security assumptions are violated

## Security Assumptions and Notes

1. **No unsafe HTML rendering**: component renders plain text only and does not use `dangerouslySetInnerHTML`.
2. **URI safety**: metadata and image links must be trusted `http(s)` endpoints; non-http(s) schemes are rejected.
3. **Deterministic token IDs**: token IDs follow `<campaignId>-<milestonePercent>` to prevent ambiguity in frontend and mint pipelines.
4. **Input hardening**: non-finite and negative numeric values are clamped to safe defaults.
5. **Fail-safe behavior**: invalid campaign IDs, invalid URIs, or zero goals produce explicit `securityChecks` and block metadata generation.

## Testing

Run:

```bash
npx jest milestone_nfts.test.tsx
```

Coverage includes:

- Sanitization and strict validation branches
- Milestone unlock and rarity assignment logic
- Invalid URI, invalid ID, and zero-goal fail-safe behavior
- Rendering for both success and security-alert UI states
