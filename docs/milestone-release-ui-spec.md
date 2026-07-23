# Milestone-Gated Partial Release — UI & Transaction Flow Spec

Handoff spec for the frontend team (frontend lives in a separate repo). Describes the
screens, states, and exact contract calls needed to support milestone-gated partial fund
release, on top of the existing funded/refundable all-or-nothing flow.

Governance model: **backer-majority vote, weighted by contribution amount** — not a
platform arbiter, not creator self-attestation. A milestone releases only once backers
holding more than half of the total contributed capital vote to approve it.

Contract: `apps/contracts/crowdfund/src/lib.rs` (this repo). All calls below are Soroban
contract invocations against the campaign's `CrowdfundContract` instance, signed by the
connected wallet.

## Data model the frontend reads

```
Milestone {
  id: u32
  description: string
  amount: i128
  status: "Pending" | "Approved" | "Rejected" | "Released"
  yes_weight: i128
  no_weight: i128
  voting_deadline: u64        // unix seconds
}
```

Read via:
- `milestones() -> Vec<Milestone>` — full schedule (empty if none proposed yet)
- `milestone(milestone_id) -> Option<Milestone>` — single row
- `milestone_basis() -> i128` — the frozen `total_raised` snapshot the schedule is voted/reconciled against (denominator for the progress bars below); `0` if no schedule exists
- `has_voted_milestone(milestone_id, address) -> bool`
- `has_claimed_milestone_refund(milestone_id, address) -> bool`
- `status() -> "Active" | "Successful" | "Refunded" | "Cancelled"`
- `contribution(address) -> i128` — also the backer's vote weight
- `total_raised()`, `goal()` — existing getters, unchanged

## Screen states

### 1. Campaign detail page — mode gate

Existing "Withdraw Funds" button (creator-only, shown once `total_raised() >= goal()` and
deadline passed) becomes conditional:

- `milestones().length == 0` → show the existing all-or-nothing UI unchanged (creator sees
  "Withdraw Funds" calling `withdraw()`).
- `milestones().length > 0` → hide "Withdraw Funds" entirely (the contract now rejects it
  with `MilestoneModeActive`) and show the **Milestone Release Plan** panel (§3) instead.

### 2. Mode-choice panel (creator only, one-time)

Shown when: `status() == "Active"`, deadline has passed, `total_raised() >= goal()`,
`milestones().length == 0`.

Two actions, presented as a real fork (copy should warn this choice is permanent for the
campaign):
- **"Withdraw all at once"** → calls `withdraw()` (existing, unchanged).
- **"Propose a milestone release schedule"** → opens the Propose Schedule form (§2a).

If the campaign used the pledge flow (`pledge()`/pledgers exist), prompt the creator to
call `collect_pledges()` **before** proposing a schedule — once milestones exist,
`collect_pledges()` is blocked (`MilestoneModeActive`), and any pledges not yet collected
would never be reconcilable against the frozen basis.

### 2a. Propose Schedule form (creator only)

- Dynamic rows: `{ description: string, amount: number }`, max 20 rows.
- Live "remaining to allocate" = `total_raised() - sum(entered row amounts)`.
- Submit button disabled until remaining is **exactly 0** — the contract rejects any
  schedule that doesn't sum exactly to `total_raised()` (`InvalidMilestoneSchedule`).
- On submit: `propose_milestones(creator, milestones: [{description, amount}, ...])`.
- Errors to surface: `MilestonesAlreadyProposed` (schedule already exists — refresh and
  show the milestone list instead), `InvalidMilestoneSchedule` (re-show the form with the
  reconciliation error), `GoalNotReached` / `CampaignStillActive` (gate mismatch — refresh
  campaign state).

### 3. Milestone list (visible to everyone once `milestones().length > 0`)

One card per milestone from `milestones()`:

- Description, amount, and amount as a % of `milestone_basis()`.
- Status badge: `Pending` (neutral) / `Approved` (positive, awaiting release) / `Rejected`
  (negative) / `Released` (positive, terminal).
- Three-segment progress bar for `Pending`/resolved milestones: yes % / no % / undecided %,
  each computed as `weight / milestone_basis()`.
- Countdown to `voting_deadline`, shown only while `status == "Pending"`.

### 4. Vote controls (any connected wallet)

Shown on `Pending` cards only. Two buttons, **Approve** / **Reject**.

- Enabled only if: `contribution(wallet) > 0 && !has_voted_milestone(id, wallet) && now < voting_deadline`.
- Disabled state copy: "Only backers who contributed can vote" (zero contribution) /
  "You already voted on this milestone" / "Voting has closed — awaiting resolution".
- Call: `vote_milestone(voter: wallet, milestone_id, approve: true|false)`.
- After a successful call, immediately re-fetch the milestone (its `status`/weights may
  have just flipped to `Approved`/`Rejected` as a side effect of this vote).
- Errors to surface: `AlreadyVoted`, `NoContributionWeight`, `MilestoneNotPending` (window
  closed or already resolved — refresh).

### 5. Finalize control (permissionless)

Shown on any `Pending` card whose `voting_deadline` has passed (can also be triggered
automatically by the frontend the first time such a card is rendered, no wallet prompt
needed beyond the transaction signature — any connected wallet can submit it, or the
frontend can run it as a scheduled/background transaction).

- Call: `finalize_milestone_vote(milestone_id)`.
- No contribution/creator gating — this only resolves an already-public tally against a
  public deadline. Silence resolves to **Rejected**, not Approved.

### 6. Release control (creator only)

Shown on `Approved` cards, enabled only if the connected wallet is the campaign creator.

- Call: `release_milestone(creator: wallet, milestone_id)`.
- On success, funds (minus any platform fee) move to the creator and the card flips to
  `Released`. Re-fetch `status()` too — if this was the last unsettled milestone, the
  campaign as a whole flips to `Successful`.

### 7. Claim Refund control (backers only, on `Rejected` cards)

- Enabled only if: `contribution(wallet) > 0 && !has_claimed_milestone_refund(id, wallet)`.
- Call: `claim_milestone_refund(contributor: wallet, milestone_id)`.
- Payout is pro-rata: `milestone.amount * contribution(wallet) / milestone_basis()`. Note
  small amounts can round down to a zero-share error (`NothingToRefund`) — disable the
  button and show "your share of this rejected milestone rounds to zero" if the frontend
  can precompute this (it can, using the same integer-division formula).

### 8. Campaign-complete banner

Once `status() == "Successful"` (via the new getter), show the existing "campaign
successful" banner regardless of whether it got there via `withdraw()` or via every
milestone reaching `Released`/`Rejected`. No milestone-specific copy needed here — the
milestone cards above already show the full settlement history.

## Reconciliation display (for trust/transparency, not gating)

Show a summary strip above the milestone list:

```
Total raised: {milestone_basis()}
Released to creator: sum(milestone.amount where status == Released)
Refunded (rejected milestones): sum(milestone.amount where status == Rejected)
Still pending: sum(milestone.amount where status in {Pending, Approved})
```

These four numbers always sum to `milestone_basis()` by construction (the contract
enforces `sum(schedule amounts) == total_raised` at proposal time), so this is purely
informational — it should never need client-side correction.
