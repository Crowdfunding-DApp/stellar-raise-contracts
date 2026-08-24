#![no_std]
#![allow(deprecated)]
#![allow(clippy::too_many_arguments)]

use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, BytesN, Env, IntoVal, String, Symbol, Vec,
};

#[cfg(test)]
mod batch_contribute;
#[cfg(test)]
mod batch_contribute_tests;
#[cfg(test)]
mod test;

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Append-only list of all deployed campaign addresses, in deployment
    /// order. Never mutated or shrunk — see `CampaignStatus` for how
    /// individual entries get logically hidden without touching this log.
    Campaigns,
    /// Platform moderator address, set once via `initialize`.
    Admin,
    /// Per-campaign moderation status. Absent means `CampaignStatus::Active`.
    Status(Address),
    /// Number of campaigns a given creator has deployed through this
    /// factory. Feeds the deployment salt (see `create_campaign`) so the
    /// same creator can deploy more than one campaign — Soroban's deployer
    /// derives the deployed address from `(creator, salt)`, so a fixed salt
    /// per creator would permanently consume that address after the first
    /// deployment.
    CreatorCampaignCount(Address),
}

/// Mirrors `crowdfund::PlatformConfig` field-for-field so it round-trips
/// through the `initialize` cross-contract call without the factory crate
/// depending on the crowdfund crate directly (`#[contracttype]` structs
/// encode by field name, not by crate identity, so this only needs to match
/// field names and types, not share a type definition).
#[derive(Clone)]
#[contracttype]
pub struct PlatformConfig {
    pub address: Address,
    pub fee_bps: u32,
}

/// Moderation status of a registered campaign.
///
/// This lives in the factory's own storage, independent of whatever status
/// the deployed campaign contract itself reports — it lets the platform
/// moderator flag campaigns (cancelled, expired, or fraudulent) directly in
/// the registry, without needing per-campaign cross-contract calls to build
/// a filtered front-end list.
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum CampaignStatus {
    Active,
    Cancelled,
    Expired,
    Flagged,
}

#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {
    /// Deploy a new crowdfund campaign contract.
    ///
    /// Permissionless and repeatable: a creator may call this any number of
    /// times, including after an earlier campaign of theirs has completed,
    /// been cancelled, or is still active. Each call deploys to a fresh
    /// address, because the deployment salt is derived from a per-creator
    /// campaign counter rather than a fixed constant.
    ///
    /// # Arguments
    /// * `creator`                 – The campaign creator's address.
    /// * `token`                   – The token contract address used for contributions.
    /// * `goal`                    – The funding goal (in the token's smallest unit).
    /// * `deadline`                – The campaign deadline as a ledger timestamp.
    /// * `wasm_hash`                – The hash of the crowdfund contract WASM to deploy.
    /// * `min_contribution`        – Minimum single-contribution amount for the deployed campaign.
    /// * `platform_config`         – Optional platform fee configuration for the deployed campaign.
    /// * `bonus_goal`              – Optional stretch-goal amount for the deployed campaign.
    /// * `bonus_goal_description`  – Optional description for `bonus_goal`.
    ///
    /// # Returns
    /// The address of the newly deployed campaign contract.
    pub fn create_campaign(
        env: Env,
        creator: Address,
        token: Address,
        goal: i128,
        deadline: u64,
        wasm_hash: BytesN<32>,
        min_contribution: i128,
        platform_config: Option<PlatformConfig>,
        bonus_goal: Option<i128>,
        bonus_goal_description: Option<String>,
    ) -> Address {
        creator.require_auth();

        // Derive a salt from how many campaigns this creator has already
        // deployed through this factory. `env.deployer().with_address`
        // derives the deployed address from `(creator, salt)`, so varying
        // the salt per call (for a fixed creator) is what makes repeat
        // deployments land at distinct addresses instead of colliding.
        let campaign_count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CreatorCampaignCount(creator.clone()))
            .unwrap_or(0);
        let mut salt_bytes = [0u8; 32];
        salt_bytes[28..32].copy_from_slice(&campaign_count.to_be_bytes());
        let salt = BytesN::from_array(&env, &salt_bytes);

        let deployed_address = env
            .deployer()
            .with_address(creator.clone(), salt)
            .deploy_v2(wasm_hash, ());

        env.storage().instance().set(
            &DataKey::CreatorCampaignCount(creator.clone()),
            &campaign_count
                .checked_add(1)
                .expect("creator campaign count overflow"),
        );

        // Initialize the deployed contract with the caller-supplied parameters.
        // initialize() fail-fast-checks the token address against a caller-supplied
        // decimals value (audit: validate campaign amounts against token decimals).
        // The factory has no independent expectation of its own, so it simply
        // reads the token's own decimals and passes it straight through — the
        // check then only ever fails if `token` doesn't implement SEP-41 at all.
        let expected_token_decimals: u32 = token::Client::new(&env, &token).decimals();
        let _: () = env.invoke_contract(
            &deployed_address,
            &Symbol::new(&env, "initialize"),
            soroban_sdk::vec![
                &env,
                creator.clone().into_val(&env),
                creator.into_val(&env),
                token.into_val(&env),
                goal.into_val(&env),
                deadline.into_val(&env),
                min_contribution.into_val(&env),
                platform_config.into_val(&env),
                bonus_goal.into_val(&env),
                bonus_goal_description.into_val(&env),
                expected_token_decimals.into_val(&env)
            ],
        );

        // Add to registry.
        let mut campaigns: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Campaigns)
            .unwrap_or(Vec::new(&env));
        campaigns.push_back(deployed_address.clone());
        env.storage()
            .instance()
            .set(&DataKey::Campaigns, &campaigns);

        deployed_address
    }

    /// Returns the full, unfiltered list of all deployed campaign addresses.
    ///
    /// This is an append-only log — it includes campaigns flagged via
    /// `set_campaign_status` and grows without bound. Prefer
    /// `active_campaigns_page` for a front-end listing, or `campaigns_page`
    /// if you need the raw log without loading it all in one call.
    pub fn campaigns(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::Campaigns)
            .unwrap_or(Vec::new(&env))
    }

    /// Returns the total number of deployed campaigns (including flagged
    /// ones — see `campaigns`).
    pub fn campaign_count(env: Env) -> u32 {
        let campaigns: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Campaigns)
            .unwrap_or(Vec::new(&env));
        campaigns.len()
    }

    /// Returns a page of the raw campaign registry, in deployment order.
    ///
    /// `offset`/`limit` index into the same unfiltered list as `campaigns`
    /// (including flagged/cancelled/expired entries). Out-of-range offsets
    /// or a zero limit return an empty page rather than panicking.
    pub fn campaigns_page(env: Env, offset: u32, limit: u32) -> Vec<Address> {
        let campaigns: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Campaigns)
            .unwrap_or(Vec::new(&env));
        Self::bounded_slice(&campaigns, offset, limit)
    }

    /// Returns a page of `Active` campaigns only — entries the moderator has
    /// marked `Cancelled`, `Expired`, or `Flagged` via `set_campaign_status`
    /// are skipped.
    ///
    /// Unlike `campaigns_page`, `offset`/`limit` index into the *filtered*
    /// (active-only) result, so page boundaries stay stable as campaigns get
    /// flagged or unflagged over time. This scans the registry from the
    /// start on every call, so keep `offset` reasonably small.
    pub fn active_campaigns_page(env: Env, offset: u32, limit: u32) -> Vec<Address> {
        let campaigns: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Campaigns)
            .unwrap_or(Vec::new(&env));

        let mut result = Vec::new(&env);
        if limit == 0 {
            return result;
        }

        let mut skipped = 0u32;
        for campaign in campaigns.iter() {
            if Self::status_of(&env, &campaign) != CampaignStatus::Active {
                continue;
            }
            if skipped < offset {
                skipped += 1;
                continue;
            }
            result.push_back(campaign);
            if result.len() >= limit {
                break;
            }
        }
        result
    }

    /// Returns the moderation status of `campaign` (`Active` if it has never
    /// been flagged).
    ///
    /// # Panics
    /// * If `campaign` is not registered with this factory.
    pub fn campaign_status(env: Env, campaign: Address) -> CampaignStatus {
        let campaigns: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Campaigns)
            .unwrap_or(Vec::new(&env));
        if !campaigns.contains(&campaign) {
            panic!("campaign is not registered with this factory");
        }
        Self::status_of(&env, &campaign)
    }

    /// One-time setup of the platform moderator address.
    ///
    /// Campaign creation stays fully permissionless and is unaffected by
    /// this — `admin` only gains the ability to flag/unflag entries via
    /// `set_campaign_status`.
    ///
    /// # Panics
    /// * If the factory has already been initialized.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("factory already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Returns the platform moderator address.
    ///
    /// # Panics
    /// * If the factory has not been initialized.
    pub fn admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("factory not initialized"))
    }

    /// Sets the moderation status of a registered campaign — moderator-only.
    ///
    /// Lets the platform moderator flag campaigns that turn out to be
    /// cancelled, expired, or fraudulent (or reinstate one by setting it
    /// back to `Active`) without mutating the append-only `campaigns` log.
    /// Front-ends should prefer `active_campaigns_page` to avoid surfacing
    /// flagged campaigns.
    ///
    /// # Panics
    /// * If the factory has not been initialized.
    /// * If `admin` does not match the stored moderator, or fails auth.
    /// * If `campaign` is not registered with this factory.
    pub fn set_campaign_status(
        env: Env,
        admin: Address,
        campaign: Address,
        status: CampaignStatus,
    ) {
        let stored_admin = Self::admin(env.clone());
        if stored_admin != admin {
            panic!("unauthorized: caller is not the factory admin");
        }
        admin.require_auth();

        let campaigns: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Campaigns)
            .unwrap_or(Vec::new(&env));
        if !campaigns.contains(&campaign) {
            panic!("campaign is not registered with this factory");
        }

        env.storage()
            .instance()
            .set(&DataKey::Status(campaign.clone()), &status);
        env.events()
            .publish(("factory", "campaign_status_changed"), (campaign, status));
    }

    fn status_of(env: &Env, campaign: &Address) -> CampaignStatus {
        env.storage()
            .instance()
            .get(&DataKey::Status(campaign.clone()))
            .unwrap_or(CampaignStatus::Active)
    }

    /// Clamps `offset..offset+limit` to `items`' bounds and returns that
    /// slice, or an empty vec instead of panicking when out of range.
    fn bounded_slice(items: &Vec<Address>, offset: u32, limit: u32) -> Vec<Address> {
        let len = items.len();
        if offset >= len || limit == 0 {
            return Vec::new(items.env());
        }
        let end = offset.saturating_add(limit).min(len);
        items.slice(offset..end)
    }
}
