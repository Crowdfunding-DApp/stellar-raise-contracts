use crate::DataKey;
use soroban_sdk::{Address, BytesN, Env};

/// Validates that the caller is the authorized admin for contract upgrades.
///
/// ### Security Note
/// This function uses `require_auth()` which ensures the transaction is
/// signed by the admin address stored during initialization.
pub fn validate_admin_upgrade(env: &Env) -> Address {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("Admin not initialized");

    admin.require_auth();
    admin
}

/// Stores the current WASM hash as the previous hash before performing an upgrade.
///
/// This creates a rollback point so that if the new WASM is broken, the admin can
/// restore the previous working implementation via `rollback_upgrade()`.
///
/// # Arguments
/// * `env` – The Soroban environment.
/// * `current_wasm_hash` – The 32-byte hash of the currently deployed WASM binary.
pub fn store_current_wasm_hash(env: &Env, current_wasm_hash: &BytesN<32>) {
    env.storage()
        .instance()
        .set(&DataKey::PreviousWasmHash, current_wasm_hash);
}

/// Retrieves the previously stored WASM hash for rollback purposes.
///
/// Returns `None` if no previous WASM hash has been stored (e.g., first upgrade).
pub fn get_previous_wasm_hash(env: &Env) -> Option<BytesN<32>> {
    env.storage().instance().get(&DataKey::PreviousWasmHash)
}

/// Executes the WASM update.
///
/// # Arguments
/// * `env` – The Soroban environment.
/// * `new_wasm_hash` – The 32-byte hash of the new WASM binary to deploy.
pub fn perform_upgrade(env: &Env, new_wasm_hash: BytesN<32>) {
    env.deployer().update_current_contract_wasm(new_wasm_hash);
}

/// Rollback the contract to the previously stored WASM implementation.
///
/// This function should only be called by the admin after a bad upgrade.
/// It restores the WASM hash stored in `DataKey::PreviousWasmHash`.
///
/// # Panics
/// * If no previous WASM hash is stored.
/// * If the caller is not the admin.
///
/// # Returns
/// The restored WASM hash on success.
pub fn rollback_upgrade(env: &Env) -> BytesN<32> {
    let previous_hash: BytesN<32> = env
        .storage()
        .instance()
        .get(&DataKey::PreviousWasmHash)
        .expect("No previous WASM hash available for rollback");

    env.deployer()
        .update_current_contract_wasm(previous_hash.clone());
    previous_hash
}
