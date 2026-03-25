# Role-Based Access Control

Defines the three distinct roles used across the crowdfund contract system.

## Roles

### DEFAULT_ADMIN_ROLE
The highest-privilege role. Responsible for contract-level administration.

- Assign and revoke roles for other addresses
- Trigger contract upgrades via `upgrade()`
- Update platform configuration (fee address, fee basis points)
- Pause and unpause the contract (can act as PAUSER)

Only one address should hold this role at any time. Transferring it requires explicit revocation of the previous holder.

---

### CAMPAIGN_CREATOR
Granted to the address that initializes a campaign. Scoped to campaign-level operations.

- Call `initialize()` to create a new campaign
- Call `withdraw()` to claim funds after a successful campaign
- Call `cancel()` to cancel an active campaign and trigger refunds
- Call `update_metadata()` to update title, description, and social links
- Call `add_roadmap_item()` to append milestones to the campaign timeline
- Call `add_stretch_goal()` to add stretch goal milestones

This role is set at initialization and cannot be transferred to another address mid-campaign.

---

### PAUSER
Authorized to halt campaign activity in an emergency without needing full admin privileges.

- Call `pause()` to suspend contributions and withdrawals
- Call `unpause()` to resume normal operation

The PAUSER role is intended for a multisig or operations address separate from DEFAULT_ADMIN_ROLE, following the principle of least privilege.

---

## Role Hierarchy

```
DEFAULT_ADMIN_ROLE
├── can grant / revoke CAMPAIGN_CREATOR
├── can grant / revoke PAUSER
└── inherits PAUSER permissions
```

## Summary Table

| Role | Upgrade | Pause | Campaign Ops | Metadata | Role Mgmt |
|---|---|---|---|---|---|
| DEFAULT_ADMIN_ROLE | yes | yes | no | no | yes |
| CAMPAIGN_CREATOR | no | no | yes | yes | no |
| PAUSER | no | yes | no | no | no |
