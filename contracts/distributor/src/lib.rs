#![no_std]

pub mod errors;
pub mod events;
pub mod merkle;
pub mod storage;
pub mod types;

#[cfg(test)]
mod test;

use errors::ContractError;
use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, Vec};
use types::{Distribution, DistributionStatus};

#[contract]
pub struct YieldStreamDistributor;

#[contractimpl]
impl YieldStreamDistributor {
    /// Initializes the contract with an administrative address.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if storage::has_admin(&env) {
            return Err(ContractError::AlreadyInitialized);
        }
        admin.require_auth();
        storage::set_admin(&env, &admin);
        Ok(())
    }

    /// Creates a new distribution campaign, transferring total tokens to the contract.
    pub fn create_distribution(
        env: Env,
        creator: Address,
        token: Address,
        merkle_root: BytesN<32>,
        total_amount: i128,
        num_recipients: u32,
        end_time: u64,
    ) -> Result<u32, ContractError> {
        creator.require_auth();

        if total_amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }
        if num_recipients == 0 {
            return Err(ContractError::InvalidRecipientCount);
        }
        if end_time != 0 && end_time <= env.ledger().timestamp() {
            return Err(ContractError::InvalidEndTime);
        }

        let next_id = storage::get_distribution_count(&env)
            .checked_add(1)
            .ok_or(ContractError::ArithmeticOverflow)?;
        storage::set_distribution_count(&env, next_id);

        let contract_address = env.current_contract_address();
        token::Client::new(&env, &token).transfer(&creator, &contract_address, &total_amount);

        let distribution = Distribution {
            id: next_id,
            creator: creator.clone(),
            token: token.clone(),
            merkle_root: merkle_root.clone(),
            total_amount,
            claimed_amount: 0,
            num_recipients,
            claimed_count: 0,
            created_at: env.ledger().timestamp(),
            end_time,
            status: DistributionStatus::Active,
        };

        storage::set_distribution(&env, next_id, &distribution);
        events::distribution_created(
            &env,
            next_id,
            &creator,
            &token,
            &merkle_root,
            total_amount,
            end_time,
        );

        Ok(next_id)
    }

    /// Claims an allocated share of a distribution using a Merkle inclusion proof.
    pub fn claim(
        env: Env,
        distribution_id: u32,
        index: u32,
        recipient: Address,
        amount: i128,
        proof: Vec<BytesN<32>>,
    ) -> Result<(), ContractError> {
        recipient.require_auth();
        storage::bump_instance(&env);

        let mut distribution = storage::get_distribution(&env, distribution_id)?;

        if amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }
        if index >= distribution.num_recipients {
            return Err(ContractError::InvalidLeafIndex);
        }
        if distribution.status == DistributionStatus::Cancelled {
            return Err(ContractError::DistributionCancelled);
        }
        if distribution.status != DistributionStatus::Active {
            return Err(ContractError::DistributionNotActive);
        }
        if distribution.end_time > 0 && env.ledger().timestamp() >= distribution.end_time {
            return Err(ContractError::DistributionExpired);
        }
        if storage::is_claimed(&env, distribution_id, index) {
            return Err(ContractError::AlreadyClaimed);
        }

        let leaf = merkle::compute_leaf(&env, distribution_id, index, &recipient, amount);
        if !merkle::verify_merkle_proof(&env, &proof, &distribution.merkle_root, &leaf) {
            return Err(ContractError::InvalidProof);
        }

        storage::set_claimed(&env, distribution_id, index);

        distribution.claimed_amount = distribution
            .claimed_amount
            .checked_add(amount)
            .ok_or(ContractError::ArithmeticOverflow)?;
        distribution.claimed_count = distribution
            .claimed_count
            .checked_add(1)
            .ok_or(ContractError::ArithmeticOverflow)?;

        if distribution.claimed_count == distribution.num_recipients
            || distribution.claimed_amount == distribution.total_amount
        {
            distribution.status = DistributionStatus::Completed;
        }

        storage::set_distribution(&env, distribution_id, &distribution);

        let contract_address = env.current_contract_address();
        token::Client::new(&env, &distribution.token).transfer(
            &contract_address,
            &recipient,
            &amount,
        );

        events::distribution_claimed(&env, distribution_id, index, &recipient, amount);

        Ok(())
    }

    /// Cancels an active distribution, halting future claims.
    pub fn cancel(env: Env, distribution_id: u32) -> Result<(), ContractError> {
        storage::bump_instance(&env);
        let mut distribution = storage::get_distribution(&env, distribution_id)?;
        distribution.creator.require_auth();

        if distribution.status != DistributionStatus::Active {
            return Err(ContractError::DistributionNotActive);
        }

        distribution.status = DistributionStatus::Cancelled;
        let remaining_amount = distribution
            .total_amount
            .checked_sub(distribution.claimed_amount)
            .ok_or(ContractError::ArithmeticOverflow)?;

        storage::set_distribution(&env, distribution_id, &distribution);
        events::distribution_cancelled(
            &env,
            distribution_id,
            &distribution.creator,
            remaining_amount,
        );

        Ok(())
    }

    /// Sweeps remaining unclaimed tokens to a recipient address if cancelled or expired.
    pub fn sweep(env: Env, distribution_id: u32, to: Address) -> Result<i128, ContractError> {
        storage::bump_instance(&env);
        let mut distribution = storage::get_distribution(&env, distribution_id)?;
        distribution.creator.require_auth();

        let is_cancelled = distribution.status == DistributionStatus::Cancelled;
        let is_expired =
            distribution.end_time > 0 && env.ledger().timestamp() >= distribution.end_time;

        if !is_cancelled && !is_expired {
            return Err(ContractError::DistributionNotExpired);
        }

        let remaining_amount = distribution
            .total_amount
            .checked_sub(distribution.claimed_amount)
            .ok_or(ContractError::ArithmeticOverflow)?;

        if remaining_amount <= 0 {
            return Err(ContractError::NothingToSweep);
        }

        distribution.status = DistributionStatus::Completed;
        distribution.claimed_amount = distribution.total_amount;
        storage::set_distribution(&env, distribution_id, &distribution);

        let contract_address = env.current_contract_address();
        token::Client::new(&env, &distribution.token).transfer(
            &contract_address,
            &to,
            &remaining_amount,
        );

        events::distribution_swept(
            &env,
            distribution_id,
            &distribution.creator,
            &to,
            remaining_amount,
        );

        Ok(remaining_amount)
    }

    /// View: Retrieves distribution metadata by ID.
    pub fn get_distribution(env: Env, distribution_id: u32) -> Result<Distribution, ContractError> {
        storage::get_distribution(&env, distribution_id)
    }

    /// View: Checks whether a specific leaf index has been claimed.
    pub fn is_claimed(env: Env, distribution_id: u32, index: u32) -> bool {
        storage::is_claimed(&env, distribution_id, index)
    }

    /// View: Gets the total number of distributions created.
    pub fn get_distribution_count(env: Env) -> u32 {
        storage::get_distribution_count(&env)
    }

    /// View: Gets the initialized contract administrator.
    pub fn get_admin(env: Env) -> Result<Address, ContractError> {
        storage::get_admin(&env)
    }

    /// Helper: Computes the leaf hash for given distribution claim parameters.
    pub fn compute_leaf(
        env: Env,
        distribution_id: u32,
        index: u32,
        recipient: Address,
        amount: i128,
    ) -> BytesN<32> {
        merkle::compute_leaf(&env, distribution_id, index, &recipient, amount)
    }

    /// Helper: Verifies a Merkle proof against a root and leaf.
    pub fn verify_proof(
        env: Env,
        proof: Vec<BytesN<32>>,
        root: BytesN<32>,
        leaf: BytesN<32>,
    ) -> bool {
        merkle::verify_merkle_proof(&env, &proof, &root, &leaf)
    }
}
