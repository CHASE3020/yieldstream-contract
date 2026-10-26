use crate::errors::ContractError;
use crate::types::{DataKey, Distribution};
use soroban_sdk::{Address, Env};

/// Ledger constants for TTL management (~5 seconds per ledger on Stellar).
pub const DAY_IN_LEDGERS: u32 = 17_280;

/// Instance lifetime target: ~30 days.
pub const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
/// Instance threshold before extending TTL: ~29 days.
pub const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

/// Persistent storage lifetime target: ~120 days.
pub const PERSISTENT_BUMP_AMOUNT: u32 = 120 * DAY_IN_LEDGERS;
/// Persistent storage threshold before extending TTL: ~90 days.
pub const PERSISTENT_LIFETIME_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - (30 * DAY_IN_LEDGERS);

/// Extends the TTL of the contract instance storage.
pub fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

/// Extends the TTL of a specific distribution record in persistent storage.
pub fn bump_distribution(env: &Env, id: u32) {
    let key = DataKey::Distribution(id);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

/// Extends the TTL of a specific claimed record in persistent storage.
pub fn bump_claimed(env: &Env, id: u32, index: u32) {
    let key = DataKey::Claimed(id, index);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

/// Checks if an administrator address has been initialized.
pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

/// Retrieves the contract administrator address.
pub fn get_admin(env: &Env) -> Result<Address, ContractError> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(ContractError::NotInitialized)
}

/// Sets the contract administrator address.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
    bump_instance(env);
}

/// Gets the total number of distributions created so far.
pub fn get_distribution_count(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::DistributionCount)
        .unwrap_or(0)
}

/// Updates the total distribution count.
pub fn set_distribution_count(env: &Env, count: u32) {
    env.storage()
        .instance()
        .set(&DataKey::DistributionCount, &count);
    bump_instance(env);
}

/// Checks whether a distribution with the given ID exists.
pub fn has_distribution(env: &Env, id: u32) -> bool {
    env.storage().persistent().has(&DataKey::Distribution(id))
}

/// Retrieves the distribution record by ID, bumping its TTL if found.
pub fn get_distribution(env: &Env, id: u32) -> Result<Distribution, ContractError> {
    let key = DataKey::Distribution(id);
    let dist: Distribution = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(ContractError::DistributionNotFound)?;
    bump_distribution(env, id);
    Ok(dist)
}

/// Saves or updates a distribution record in persistent storage and extends its TTL.
pub fn set_distribution(env: &Env, id: u32, distribution: &Distribution) {
    let key = DataKey::Distribution(id);
    env.storage().persistent().set(&key, distribution);
    bump_distribution(env, id);
}

/// Checks if a particular leaf index for a distribution has already been claimed.
pub fn is_claimed(env: &Env, id: u32, index: u32) -> bool {
    let key = DataKey::Claimed(id, index);
    if env.storage().persistent().has(&key) {
        bump_claimed(env, id, index);
        true
    } else {
        false
    }
}

/// Marks a leaf index for a distribution as claimed and extends its TTL.
pub fn set_claimed(env: &Env, id: u32, index: u32) {
    let key = DataKey::Claimed(id, index);
    env.storage().persistent().set(&key, &true);
    bump_claimed(env, id, index);
}
