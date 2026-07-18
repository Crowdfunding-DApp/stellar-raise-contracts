# Spam Prevention and Quality Contributions Plan

## Anti-Spam Measures

- **Minimum contribution threshold** — Each crowdfund campaign sets a `min_contribution` parameter, preventing dust-spam attacks that clog the contract state.
- **Contribution caps** — Per-wallet and per-campaign limits prevent a single bad actor from dominating or manipulating funding totals.
- **Deadline enforcement** — Contributions are only accepted before the campaign deadline; after expiry, the contract locks and no new pledges can be made, eliminating post-deadline spam.

## Quality Contribution Mechanisms

- **Factory-created campaigns** — Only whitelisted or verified creators can deploy new campaigns via the factory contract, preventing arbitrary spam campaigns.
- **Refundable contributions** — If a campaign fails to meet its goal, backers can reclaim their funds via a built-in refund mechanism, ensuring contributors only pay for successful campaigns and discouraging frivolous pledges.
- **On-chain transparency** — All contributions are recorded on Stellar's public ledger, making it easy to audit activity and flag suspicious patterns off-chain via analytics.

These measures ensure the platform remains useful for legitimate creators and backers while minimizing abuse.
