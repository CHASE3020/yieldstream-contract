extern crate std;
use std::string::String;
use std::vec::Vec as StdVec;

use super::{build_test_tree, setup_test, SimpleClaimItem};
use crate::merkle::compute_leaf;
use crate::types::DistributionStatus;
use serde::{Deserialize, Serialize};
use soroban_sdk::{
    testutils::Address as _,
    Address, BytesN, Env, String as SorobanString, Vec,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ClaimVector {
    pub index: u32,
    pub recipient: String,
    pub amount: i128,
    pub leaf_hex: String,
    pub proof_hex: StdVec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TreeVector {
    pub name: String,
    pub distribution_id: u32,
    pub total_amount: i128,
    pub num_recipients: u32,
    pub root_hex: String,
    pub claims: StdVec<ClaimVector>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VectorFile {
    pub version: String,
    pub description: String,
    pub trees: StdVec<TreeVector>,
}

fn bytes32_to_hex(bytes: &BytesN<32>) -> String {
    let arr = bytes.to_array();
    hex::encode(arr)
}

fn hex_to_bytes32(env: &Env, hex_str: &str) -> BytesN<32> {
    let bytes = hex::decode(hex_str).expect("Valid hex string");
    assert_eq!(bytes.len(), 32, "Hex string must decode to 32 bytes");
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    BytesN::from_array(env, &arr)
}

fn address_to_std_string(addr: &Address) -> String {
    let s = addr.to_string();
    let mut buf = [0u8; 128];
    let len = s.len() as usize;
    s.copy_into_slice(&mut buf[..len]);
    String::from_utf8(buf[..len].to_vec()).expect("Valid UTF-8 address")
}

#[test]
fn generate_and_verify_vector_file() {
    let env = Env::default();
    let _fixture = setup_test(&env);

    // Generate distinct recipient addresses (contract & token are already registered)
    let rec0 = Address::generate(&env);
    let rec1 = Address::generate(&env);
    let rec2 = Address::generate(&env);
    let rec3 = Address::generate(&env);

    let items_4 = [
        SimpleClaimItem {
            index: 0,
            recipient: rec0.clone(),
            amount: 10_000,
        },
        SimpleClaimItem {
            index: 1,
            recipient: rec1.clone(),
            amount: 25_000,
        },
        SimpleClaimItem {
            index: 2,
            recipient: rec2.clone(),
            amount: 35_000,
        },
        SimpleClaimItem {
            index: 3,
            recipient: rec3.clone(),
            amount: 50_000,
        },
    ];

    let tree_4 = build_test_tree(&env, 1, &items_4);

    let mut claims_4 = StdVec::new();
    for (i, item) in items_4.iter().enumerate() {
        let leaf = &tree_4.leaves[i];
        let proof = &tree_4.proofs[i];
        let mut proof_hex = StdVec::new();
        for p in proof.iter() {
            proof_hex.push(bytes32_to_hex(&p));
        }

        claims_4.push(ClaimVector {
            index: item.index,
            recipient: address_to_std_string(&item.recipient),
            amount: item.amount,
            leaf_hex: bytes32_to_hex(leaf),
            proof_hex,
        });
    }

    let tree_vec_4 = TreeVector {
        name: "balanced_4_recipients".into(),
        distribution_id: 1,
        total_amount: 120_000,
        num_recipients: 4,
        root_hex: bytes32_to_hex(&tree_4.root),
        claims: claims_4,
    };

    // Generate 2-leaf tree
    let u0 = Address::generate(&env);
    let u1 = Address::generate(&env);

    let items_2 = [
        SimpleClaimItem {
            index: 0,
            recipient: u0.clone(),
            amount: 500_000,
        },
        SimpleClaimItem {
            index: 1,
            recipient: u1.clone(),
            amount: 750_000,
        },
    ];

    let tree_2 = build_test_tree(&env, 2, &items_2);

    let mut claims_2 = StdVec::new();
    for (i, item) in items_2.iter().enumerate() {
        let leaf = &tree_2.leaves[i];
        let proof = &tree_2.proofs[i];
        let mut proof_hex = StdVec::new();
        for p in proof.iter() {
            proof_hex.push(bytes32_to_hex(&p));
        }

        claims_2.push(ClaimVector {
            index: item.index,
            recipient: address_to_std_string(&item.recipient),
            amount: item.amount,
            leaf_hex: bytes32_to_hex(leaf),
            proof_hex,
        });
    }

    let tree_vec_2 = TreeVector {
        name: "balanced_2_recipients".into(),
        distribution_id: 2,
        total_amount: 1_250_000,
        num_recipients: 2,
        root_hex: bytes32_to_hex(&tree_2.root),
        claims: claims_2,
    };

    let vector_file = VectorFile {
        version: "v1.0.0".into(),
        description: "Deterministic Merkle test vectors for YieldStream Distributor contract".into(),
        trees: std::vec![tree_vec_4, tree_vec_2],
    };

    let json_output = serde_json::to_string_pretty(&vector_file).expect("Serialize to json");
    std::fs::write(
        "/home/gamp/yieldstream-contract/test-vectors/merkle_v1.json",
        json_output,
    )
    .expect("Write vector file");
}

#[test]
fn test_verify_merkle_test_vectors_e2e() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let raw_json = include_str!("../../../../test-vectors/merkle_v1.json");
    let vector_file: VectorFile = serde_json::from_str(raw_json).expect("Parse vector JSON");

    assert_eq!(vector_file.version, "v1.0.0");
    assert!(!vector_file.trees.is_empty(), "Must contain test trees");

    for tree in vector_file.trees.iter() {
        let expected_root = hex_to_bytes32(&env, &tree.root_hex);

        // Creator funds distribution
        let dist_id = fixture.client.create_distribution(
            &fixture.creator,
            &fixture.token_id,
            &expected_root,
            &tree.total_amount,
            &tree.num_recipients,
            &0,
        );

        assert_eq!(dist_id, tree.distribution_id);

        for claim in tree.claims.iter() {
            let soroban_str = SorobanString::from_str(&env, &claim.recipient);
            let recipient = Address::from_string(&soroban_str);

            // Verify off-chain computed leaf matches on-chain calculation
            let calculated_leaf = compute_leaf(
                &env,
                tree.distribution_id,
                claim.index,
                &recipient,
                claim.amount,
            );
            let expected_leaf = hex_to_bytes32(&env, &claim.leaf_hex);
            assert_eq!(
                calculated_leaf, expected_leaf,
                "Calculated leaf must match vector leaf for tree {}",
                tree.name
            );

            // Reconstruct proof
            let mut proof: Vec<BytesN<32>> = Vec::new(&env);
            for p_hex in claim.proof_hex.iter() {
                proof.push_back(hex_to_bytes32(&env, p_hex));
            }

            // Verify proof verification via contract helper
            let is_valid = fixture
                .client
                .verify_proof(&proof, &expected_root, &calculated_leaf);
            assert!(
                is_valid,
                "Proof must verify against root for recipient {}",
                claim.recipient
            );

            // Claim allocation on-chain
            let bal_before = fixture.token.balance(&recipient);
            fixture.client.claim(
                &dist_id,
                &claim.index,
                &recipient,
                &claim.amount,
                &proof,
            );
            let bal_after = fixture.token.balance(&recipient);
            assert_eq!(bal_after - bal_before, claim.amount);

            assert!(fixture.client.is_claimed(&dist_id, &claim.index));
        }

        // Entire distribution must now be marked Completed
        let dist = fixture.client.get_distribution(&dist_id);
        assert_eq!(dist.status, DistributionStatus::Completed);
        assert_eq!(dist.claimed_amount, tree.total_amount);
        assert_eq!(dist.claimed_count, tree.num_recipients);
    }
}
