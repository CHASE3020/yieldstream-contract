mod cancel_sweep;
mod claim;
mod create;
mod vectors;

extern crate std;
use std::vec::Vec as StdVec;

use crate::merkle::{compute_leaf, hash_pair};
use crate::{YieldStreamDistributor, YieldStreamDistributorClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env, Vec,
};

#[allow(dead_code)]
pub struct TestFixture<'a> {
    pub env: Env,
    pub contract_id: Address,
    pub client: YieldStreamDistributorClient<'a>,
    pub admin: Address,
    pub creator: Address,
    pub token_id: Address,
    pub token: token::Client<'a>,
    pub token_admin: token::StellarAssetClient<'a>,
}

pub fn setup_test<'a>(env: &'a Env) -> TestFixture<'a> {
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(YieldStreamDistributor, ());
    let client = YieldStreamDistributorClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let creator = Address::generate(env);

    let token_admin_address = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin_address.clone());
    let token_id = token_contract.address();
    let token = token::Client::new(env, &token_id);
    let token_admin = token::StellarAssetClient::new(env, &token_id);

    // Fund creator with 10,000,000 tokens
    token_admin.mint(&creator, &10_000_000);

    TestFixture {
        env: env.clone(),
        contract_id,
        client,
        admin,
        creator,
        token_id,
        token,
        token_admin,
    }
}

pub struct SimpleClaimItem {
    pub index: u32,
    pub recipient: Address,
    pub amount: i128,
}

pub struct GeneratedTree {
    pub root: BytesN<32>,
    pub leaves: StdVec<BytesN<32>>,
    pub proofs: StdVec<Vec<BytesN<32>>>,
}

/// Builds a balanced binary Merkle tree using the contract's domain-separated hash rules
/// and returns the root, leaf hashes, and inclusion proofs for each item.
pub fn build_test_tree(
    env: &Env,
    distribution_id: u32,
    items: &[SimpleClaimItem],
) -> GeneratedTree {
    let mut leaves: StdVec<BytesN<32>> = StdVec::new();
    for item in items.iter() {
        let leaf = compute_leaf(env, distribution_id, item.index, &item.recipient, item.amount);
        leaves.push(leaf);
    }

    let n = leaves.len();
    assert!(n > 0, "Leaves cannot be empty");

    // Pad to next power of 2 by duplicating last leaf if necessary
    let mut current_level = leaves.clone();
    let target_size = n.next_power_of_two();
    while current_level.len() < target_size {
        let last = current_level.last().cloned().unwrap();
        current_level.push(last);
    }

    let mut tree_levels: StdVec<StdVec<BytesN<32>>> = StdVec::new();
    tree_levels.push(current_level.clone());

    while current_level.len() > 1 {
        let mut next_level = StdVec::new();
        for chunk in current_level.chunks(2) {
            let left = &chunk[0];
            let right = &chunk[1];
            next_level.push(hash_pair(env, left, right));
        }
        tree_levels.push(next_level.clone());
        current_level = next_level;
    }

    let root = current_level[0].clone();

    // Generate proofs for the original items
    let mut proofs = StdVec::new();
    for i in 0..n {
        let mut proof: Vec<BytesN<32>> = Vec::new(env);
        let mut idx = i;
        for level in tree_levels.iter().take(tree_levels.len() - 1) {
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            proof.push_back(level[sibling_idx].clone());
            idx /= 2;
        }
        proofs.push(proof);
    }

    GeneratedTree {
        root,
        leaves,
        proofs,
    }
}
