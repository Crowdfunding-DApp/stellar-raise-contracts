//! Tests for `admin_upgrade_mechanism` helpers.
//!
//! Covers every public function with normal, boundary, and edge-case inputs
//! to achieve ≥ 95 % line coverage.

#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

    use crate::{
        admin_upgrade_mechanism::{
            admin_is_set, get_admin, is_admin, log_upgrade, rotate_admin, set_admin,
            validate_upgrade,
        },
        CrowdfundContract, DataKey,
    };

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_env() -> (Env, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(CrowdfundContract, ());
        (env, contract_id)
    }

    fn zero_hash(env: &Env) -> BytesN<32> {
        BytesN::from_array(env, &[0u8; 32])
    }

    fn nonzero_hash(env: &Env) -> BytesN<32> {
        let mut bytes = [0u8; 32];
        bytes[0] = 0xde;
        bytes[31] = 0xad;
        BytesN::from_array(env, &bytes)
    }

    // ── admin_is_set ─────────────────────────────────────────────────────────

    #[test]
    fn test_admin_is_set_false_when_absent() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            assert!(!admin_is_set(&env));
        });
    }

    #[test]
    fn test_admin_is_set_true_after_set() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            set_admin(&env, &admin);
            assert!(admin_is_set(&env));
        });
    }

    // ── set_admin ────────────────────────────────────────────────────────────

    #[test]
    fn test_set_admin_stores_address() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            set_admin(&env, &admin);
            let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
            assert_eq!(stored, admin);
        });
    }

    #[test]
    #[should_panic(expected = "admin already set")]
    fn test_set_admin_panics_if_already_set() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            set_admin(&env, &admin);
            set_admin(&env, &admin); // second call must panic
        });
    }

    // ── get_admin ────────────────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "admin not set")]
    fn test_get_admin_panics_when_absent() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            get_admin(&env);
        });
    }

    #[test]
    fn test_get_admin_returns_stored_address() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            set_admin(&env, &admin);
            assert_eq!(get_admin(&env), admin);
        });
    }

    // ── is_admin ─────────────────────────────────────────────────────────────

    #[test]
    fn test_is_admin_false_when_no_admin_set() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let addr = Address::generate(&env);
            assert!(!is_admin(&env, &addr));
        });
    }

    #[test]
    fn test_is_admin_true_for_correct_address() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            set_admin(&env, &admin);
            assert!(is_admin(&env, &admin));
        });
    }

    #[test]
    fn test_is_admin_false_for_wrong_address() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            let other = Address::generate(&env);
            set_admin(&env, &admin);
            assert!(!is_admin(&env, &other));
        });
    }

    // ── validate_upgrade ─────────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "admin not set")]
    fn test_validate_upgrade_panics_when_no_admin() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let hash = nonzero_hash(&env);
            validate_upgrade(&env, &hash);
        });
    }

    #[test]
    #[should_panic(expected = "wasm hash must not be zero")]
    fn test_validate_upgrade_panics_on_zero_hash() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            set_admin(&env, &admin);
            let hash = zero_hash(&env);
            validate_upgrade(&env, &hash);
        });
    }

    #[test]
    fn test_validate_upgrade_succeeds_with_valid_inputs() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            set_admin(&env, &admin);
            let hash = nonzero_hash(&env);
            // mock_all_auths covers the require_auth inside validate_upgrade
            validate_upgrade(&env, &hash);
        });
    }

    #[test]
    fn test_validate_upgrade_accepts_max_hash() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            set_admin(&env, &admin);
            let hash = BytesN::from_array(&env, &[0xff; 32]);
            validate_upgrade(&env, &hash);
        });
    }

    #[test]
    fn test_validate_upgrade_accepts_single_nonzero_byte() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            set_admin(&env, &admin);
            // Only the last byte is non-zero — still valid.
            let mut bytes = [0u8; 32];
            bytes[31] = 1;
            let hash = BytesN::from_array(&env, &bytes);
            validate_upgrade(&env, &hash);
        });
    }

    // ── log_upgrade ──────────────────────────────────────────────────────────

    #[test]
    fn test_log_upgrade_does_not_panic() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let hash = nonzero_hash(&env);
            // Should complete without error.
            log_upgrade(&env, &hash);
        });
    }

    #[test]
    fn test_log_upgrade_with_zero_hash_does_not_panic() {
        // log_upgrade has no validation — it just records what was deployed.
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let hash = zero_hash(&env);
            log_upgrade(&env, &hash);
        });
    }

    // ── rotate_admin ─────────────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "admin not set")]
    fn test_rotate_admin_panics_when_no_admin() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let new_admin = Address::generate(&env);
            rotate_admin(&env, &new_admin);
        });
    }

    #[test]
    fn test_rotate_admin_updates_stored_address() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            let new_admin = Address::generate(&env);
            set_admin(&env, &admin);
            rotate_admin(&env, &new_admin);
            assert_eq!(get_admin(&env), new_admin);
        });
    }

    #[test]
    fn test_rotate_admin_old_admin_no_longer_matches() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            let new_admin = Address::generate(&env);
            set_admin(&env, &admin);
            rotate_admin(&env, &new_admin);
            assert!(!is_admin(&env, &admin));
            assert!(is_admin(&env, &new_admin));
        });
    }

    #[test]
    fn test_rotate_admin_to_same_address() {
        // Rotating to the same address is a no-op but must not panic.
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            set_admin(&env, &admin);
            rotate_admin(&env, &admin);
            assert_eq!(get_admin(&env), admin);
        });
    }

    #[test]
    fn test_rotate_admin_twice() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin1 = Address::generate(&env);
            let admin2 = Address::generate(&env);
            let admin3 = Address::generate(&env);
            set_admin(&env, &admin1);
            rotate_admin(&env, &admin2);
            rotate_admin(&env, &admin3);
            assert_eq!(get_admin(&env), admin3);
        });
    }

    // ── validate_upgrade + rotate_admin integration ───────────────────────────

    #[test]
    fn test_new_admin_can_validate_after_rotation() {
        let (env, contract_id) = make_env();
        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            let new_admin = Address::generate(&env);
            set_admin(&env, &admin);
            rotate_admin(&env, &new_admin);

            let hash = nonzero_hash(&env);
            // New admin should be able to validate an upgrade.
            validate_upgrade(&env, &hash);
        });
    }
}
