/// # privilege_escalation tests
///
/// @title   PrivilegeEscalation Test Suite
/// @notice  Comprehensive tests for the two-step, time-locked privilege escalation module.
/// @dev     All tests use `env.mock_all_auths()` so that Soroban auth checks do not
///          interfere with the unit under test.  Auth correctness is validated
///          separately in the auth_tests module.
///
/// ## Test output
/// Run with:
///   cargo test -p crowdfund privilege_escalation -- --nocapture
///
/// ## Security notes
/// - Two-step model: nominate → accept prevents single-transaction privilege grabs.
/// - Time-lock: nominations expire after ESCALATION_ACCEPTANCE_WINDOW seconds.
/// - Prerequisite chain: DEFAULT_ADMIN nomination requires nominee to hold PAUSER_ROLE.
/// - Admin rotation guard: acceptance fails if the nominating admin was replaced.
/// - Replay prevention: pending nomination is cleared on acceptance.
/// - Revocation: admin can cancel a pending nomination before acceptance.

#[cfg(test)]
mod privilege_escalation_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        token, Address, Env,
    };

    use crate::{
        privilege_escalation::{
            accept_role_default_admin, accept_role_governance, accept_role_pauser,
            get_pending_nomination, has_role, nominate_default_admin, nominate_governance,
            nominate_pauser, revoke_nomination, ESCALATION_ACCEPTANCE_WINDOW, ROLE_DEFAULT_ADMIN,
            ROLE_GOVERNANCE, ROLE_PAUSER,
        },
        CrowdfundContract, CrowdfundContractClient,
    };

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Spin up a fresh environment and register the crowdfund contract.
    fn setup() -> (
        Env,
        CrowdfundContractClient<'static>,
        Address, // admin
        Address, // creator
        Address, // token_address
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(CrowdfundContract, ());
        let client = CrowdfundContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let creator = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_contract_id.address();

        token::StellarAssetClient::new(&env, &token_address).mint(&creator, &10_000_000);

        // Initialize the campaign so roles are stored
        let deadline = env.ledger().timestamp() + 7_200;
        client.initialize(
            &admin,
            &creator,
            &token_address,
            &1_000_000i128,
            &deadline,
            &1_000i128,
            &None,
            &None,
            &None,
            &None,
        );

        (env, client, admin, creator, token_address)
    }

    /// Advance the ledger timestamp by `seconds`.
    fn advance_time(env: &Env, seconds: u64) {
        let current = env.ledger().timestamp();
        env.ledger().set(LedgerInfo {
            timestamp: current + seconds,
            protocol_version: env.ledger().protocol_version(),
            sequence_number: env.ledger().sequence(),
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 100_000,
            max_entry_ttl: 10_000_000,
        });
    }

    // ── nominate_pauser ───────────────────────────────────────────────────────

    /// @test Admin can nominate a new pauser.
    #[test]
    fn test_nominate_pauser_by_admin_succeeds() {
        let (env, client, admin, _, _) = setup();
        let nominee = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee);
            let pending = get_pending_nomination(&env, ROLE_PAUSER);
            assert!(pending.is_some());
            assert_eq!(pending.unwrap().nominee, nominee);
        });
    }

    /// @test Non-admin cannot nominate a pauser.
    #[test]
    #[should_panic(expected = "not DEFAULT_ADMIN_ROLE")]
    fn test_nominate_pauser_by_non_admin_panics() {
        let (env, client, _, _, _) = setup();
        let attacker = Address::generate(&env);
        let nominee = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &attacker, &nominee);
        });
    }

    // ── accept_role_pauser ────────────────────────────────────────────────────

    /// @test Nominee can accept a valid pauser nomination.
    #[test]
    fn test_accept_role_pauser_succeeds() {
        let (env, client, admin, _, _) = setup();
        let nominee = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee);
            accept_role_pauser(&env, &nominee).expect("accept should succeed");

            // Role must be updated
            assert!(has_role(&env, &nominee, ROLE_PAUSER));
            // Pending nomination must be cleared
            assert!(get_pending_nomination(&env, ROLE_PAUSER).is_none());
        });
    }

    /// @test Wrong address cannot accept a pauser nomination.
    #[test]
    #[should_panic(expected = "not the nominee")]
    fn test_accept_role_pauser_wrong_caller_panics() {
        let (env, client, admin, _, _) = setup();
        let nominee = Address::generate(&env);
        let impostor = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee);
            accept_role_pauser(&env, &impostor).unwrap();
        });
    }

    /// @test Acceptance fails after the window expires.
    #[test]
    #[should_panic(expected = "nomination expired")]
    fn test_accept_role_pauser_expired_panics() {
        let (env, client, admin, _, _) = setup();
        let nominee = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee);
        });

        // Advance past the acceptance window
        advance_time(&env, ESCALATION_ACCEPTANCE_WINDOW + 1);

        env.as_contract(&client.address, || {
            accept_role_pauser(&env, &nominee).unwrap();
        });
    }

    /// @test Acceptance fails when no nomination is pending.
    #[test]
    fn test_accept_role_pauser_no_nomination_returns_error() {
        let (env, client, _, _, _) = setup();
        let nominee = Address::generate(&env);

        env.as_contract(&client.address, || {
            let result = accept_role_pauser(&env, &nominee);
            assert!(result.is_err());
        });
    }

    /// @test Nomination is cleared after acceptance — replay is prevented.
    #[test]
    fn test_accept_role_pauser_replay_prevention() {
        let (env, client, admin, _, _) = setup();
        let nominee = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee);
            accept_role_pauser(&env, &nominee).expect("first accept should succeed");

            // Second accept must fail — nomination was cleared
            let result = accept_role_pauser(&env, &nominee);
            assert!(result.is_err(), "replay must be rejected");
        });
    }

    // ── nominate_governance / accept_role_governance ──────────────────────────

    /// @test Admin can nominate and governance nominee can accept.
    #[test]
    fn test_nominate_and_accept_governance_succeeds() {
        let (env, client, admin, _, _) = setup();
        let new_gov = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_governance(&env, &admin, &new_gov);
            accept_role_governance(&env, &new_gov).expect("accept should succeed");

            assert!(has_role(&env, &new_gov, ROLE_GOVERNANCE));
            assert!(get_pending_nomination(&env, ROLE_GOVERNANCE).is_none());
        });
    }

    /// @test Non-admin cannot nominate governance.
    #[test]
    #[should_panic(expected = "not DEFAULT_ADMIN_ROLE")]
    fn test_nominate_governance_by_non_admin_panics() {
        let (env, client, _, _, _) = setup();
        let attacker = Address::generate(&env);
        let nominee = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_governance(&env, &attacker, &nominee);
        });
    }

    // ── nominate_default_admin / accept_role_default_admin ────────────────────

    /// @test Admin can nominate current pauser for DEFAULT_ADMIN and pauser can accept.
    #[test]
    fn test_nominate_and_accept_default_admin_succeeds() {
        let (env, client, admin, _, _) = setup();

        // The current pauser is the admin (set at initialize); promote a new pauser first
        let new_pauser = Address::generate(&env);
        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &new_pauser);
            accept_role_pauser(&env, &new_pauser).expect("pauser accept should succeed");

            // Now nominate the new pauser for DEFAULT_ADMIN
            nominate_default_admin(&env, &admin, &new_pauser);
            accept_role_default_admin(&env, &new_pauser).expect("admin accept should succeed");

            assert!(has_role(&env, &new_pauser, ROLE_DEFAULT_ADMIN));
        });
    }

    /// @test Nominating an address that does not hold PAUSER_ROLE for DEFAULT_ADMIN panics.
    #[test]
    #[should_panic(expected = "nominee must hold PAUSER_ROLE first")]
    fn test_nominate_default_admin_without_pauser_role_panics() {
        let (env, client, admin, _, _) = setup();
        let cold_wallet = Address::generate(&env); // never held PAUSER_ROLE

        env.as_contract(&client.address, || {
            nominate_default_admin(&env, &admin, &cold_wallet);
        });
    }

    /// @test Non-admin cannot nominate for DEFAULT_ADMIN.
    #[test]
    #[should_panic(expected = "not DEFAULT_ADMIN_ROLE")]
    fn test_nominate_default_admin_by_non_admin_panics() {
        let (env, client, admin, _, _) = setup();
        let attacker = Address::generate(&env);
        let nominee = Address::generate(&env);

        // Give nominee PAUSER_ROLE so the prerequisite check passes (attacker check comes first)
        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee);
            accept_role_pauser(&env, &nominee).unwrap();
            // Now attacker tries to nominate — must fail
            nominate_default_admin(&env, &attacker, &nominee);
        });
    }

    // ── Admin rotation guard ──────────────────────────────────────────────────

    /// @test Acceptance fails if the nominating admin was replaced before acceptance.
    #[test]
    #[should_panic(expected = "nominator is no longer admin")]
    fn test_accept_pauser_fails_if_nominator_replaced() {
        let (env, client, admin, _, _) = setup();
        let nominee_pauser = Address::generate(&env);
        let new_admin_candidate = Address::generate(&env);

        env.as_contract(&client.address, || {
            // Nominate nominee_pauser for PAUSER_ROLE
            nominate_pauser(&env, &admin, &nominee_pauser);

            // Rotate admin: promote nominee_pauser to pauser first, then to admin
            accept_role_pauser(&env, &nominee_pauser).unwrap();

            // Nominate new_admin_candidate for DEFAULT_ADMIN (nominee_pauser is now pauser)
            nominate_default_admin(&env, &admin, &nominee_pauser);
            accept_role_default_admin(&env, &nominee_pauser).unwrap();
            // admin is now replaced by nominee_pauser

            // Now try to accept a *new* pauser nomination that was issued by the old admin
            nominate_pauser(&env, &admin, &new_admin_candidate);
            // admin is no longer the current admin — acceptance must fail
            accept_role_pauser(&env, &new_admin_candidate).unwrap();
        });
    }

    // ── revoke_nomination ─────────────────────────────────────────────────────

    /// @test Admin can revoke a pending nomination.
    #[test]
    fn test_revoke_nomination_by_admin_succeeds() {
        let (env, client, admin, _, _) = setup();
        let nominee = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee);
            assert!(get_pending_nomination(&env, ROLE_PAUSER).is_some());

            revoke_nomination(&env, &admin, ROLE_PAUSER).expect("revoke should succeed");
            assert!(get_pending_nomination(&env, ROLE_PAUSER).is_none());
        });
    }

    /// @test Non-admin cannot revoke a nomination.
    #[test]
    #[should_panic(expected = "not DEFAULT_ADMIN_ROLE")]
    fn test_revoke_nomination_by_non_admin_panics() {
        let (env, client, admin, _, _) = setup();
        let nominee = Address::generate(&env);
        let attacker = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee);
            revoke_nomination(&env, &attacker, ROLE_PAUSER).unwrap();
        });
    }

    /// @test Revoking when no nomination exists returns an error.
    #[test]
    fn test_revoke_nomination_no_pending_returns_error() {
        let (env, client, admin, _, _) = setup();

        env.as_contract(&client.address, || {
            let result = revoke_nomination(&env, &admin, ROLE_PAUSER);
            assert!(result.is_err());
        });
    }

    /// @test After revocation the nominee cannot accept.
    #[test]
    fn test_revoked_nomination_cannot_be_accepted() {
        let (env, client, admin, _, _) = setup();
        let nominee = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee);
            revoke_nomination(&env, &admin, ROLE_PAUSER).unwrap();

            let result = accept_role_pauser(&env, &nominee);
            assert!(result.is_err(), "revoked nomination must not be accepted");
        });
    }

    // ── has_role / get_pending_nomination ─────────────────────────────────────

    /// @test has_role returns false for an unknown role tag.
    #[test]
    fn test_has_role_unknown_tag_returns_false() {
        let (env, client, admin, _, _) = setup();

        env.as_contract(&client.address, || {
            assert!(!has_role(&env, &admin, "UNKNOWN_ROLE"));
        });
    }

    /// @test get_pending_nomination returns None when no nomination is pending.
    #[test]
    fn test_get_pending_nomination_none_when_absent() {
        let (env, client, _, _, _) = setup();

        env.as_contract(&client.address, || {
            assert!(get_pending_nomination(&env, ROLE_PAUSER).is_none());
            assert!(get_pending_nomination(&env, ROLE_GOVERNANCE).is_none());
            assert!(get_pending_nomination(&env, ROLE_DEFAULT_ADMIN).is_none());
        });
    }

    /// @test Nomination metadata (nominee, nominator, timestamp) is stored correctly.
    #[test]
    fn test_nomination_metadata_stored_correctly() {
        let (env, client, admin, _, _) = setup();
        let nominee = Address::generate(&env);
        let ts_before = env.ledger().timestamp();

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee);
            let pending = get_pending_nomination(&env, ROLE_PAUSER).unwrap();

            assert_eq!(pending.nominee, nominee);
            assert_eq!(pending.nominator, admin);
            assert!(pending.nominated_at >= ts_before);
        });
    }

    // ── Edge cases ────────────────────────────────────────────────────────────

    /// @test A nomination can be overwritten by a new nomination from the admin.
    #[test]
    fn test_overwrite_pending_nomination() {
        let (env, client, admin, _, _) = setup();
        let nominee_a = Address::generate(&env);
        let nominee_b = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee_a);
            // Overwrite with a new nomination
            nominate_pauser(&env, &admin, &nominee_b);

            let pending = get_pending_nomination(&env, ROLE_PAUSER).unwrap();
            assert_eq!(pending.nominee, nominee_b, "latest nomination must win");

            // nominee_a can no longer accept
            let result = accept_role_pauser(&env, &nominee_a);
            assert!(
                result.is_err() || {
                    // If it didn't error, the role must belong to nominee_b (overwrite succeeded)
                    has_role(&env, &nominee_b, ROLE_PAUSER)
                }
            );
        });
    }

    /// @test Acceptance exactly at the boundary of the window succeeds.
    #[test]
    fn test_accept_at_window_boundary_succeeds() {
        let (env, client, admin, _, _) = setup();
        let nominee = Address::generate(&env);

        env.as_contract(&client.address, || {
            nominate_pauser(&env, &admin, &nominee);
        });

        // Advance to exactly the last valid second
        advance_time(&env, ESCALATION_ACCEPTANCE_WINDOW);

        env.as_contract(&client.address, || {
            accept_role_pauser(&env, &nominee).expect("acceptance at boundary should succeed");
        });
    }
}
