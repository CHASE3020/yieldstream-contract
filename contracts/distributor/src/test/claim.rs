use super::{build_test_tree, setup_test, SimpleClaimItem};
use crate::errors::ContractError;
use crate::types::DistributionStatus;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env,
};

#[test]
fn test_claim_single_recipient_success() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let recipient = Address::generate(&env);
    let amount = 250_000i128;

    let items = [SimpleClaimItem {
        index: 0,
        recipient: recipient.clone(),
        amount,
    }];

    let tree = build_test_tree(&env, 1, &items);

    let dist_id = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &tree.root,
        &amount,
        &1,
        &0,
    );

    assert_eq!(fixture.token.balance(&recipient), 0);
    assert!(!fixture.client.is_claimed(&dist_id, &0));

    fixture
        .client
        .claim(&dist_id, &0, &recipient, &amount, &tree.proofs[0]);

    assert_eq!(fixture.token.balance(&recipient), amount);
    assert!(fixture.client.is_claimed(&dist_id, &0));

    let dist = fixture.client.get_distribution(&dist_id);
    assert_eq!(dist.claimed_amount, amount);
    assert_eq!(dist.claimed_count, 1);
    assert_eq!(dist.status, DistributionStatus::Completed);
}

#[test]
fn test_claim_multi_recipient_flow() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let user_c = Address::generate(&env);
    let user_d = Address::generate(&env);

    let items = [
        SimpleClaimItem {
            index: 0,
            recipient: user_a.clone(),
            amount: 10_000,
        },
        SimpleClaimItem {
            index: 1,
            recipient: user_b.clone(),
            amount: 25_000,
        },
        SimpleClaimItem {
            index: 2,
            recipient: user_c.clone(),
            amount: 40_000,
        },
        SimpleClaimItem {
            index: 3,
            recipient: user_d.clone(),
            amount: 75_000,
        },
    ];

    let total = 10_000 + 25_000 + 40_000 + 75_000;
    let tree = build_test_tree(&env, 1, &items);

    let dist_id = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &tree.root,
        &total,
        &4,
        &5_000,
    );

    // User A claims
    fixture
        .client
        .claim(&dist_id, &0, &user_a, &10_000, &tree.proofs[0]);
    assert_eq!(fixture.token.balance(&user_a), 10_000);
    let d1 = fixture.client.get_distribution(&dist_id);
    assert_eq!(d1.claimed_amount, 10_000);
    assert_eq!(d1.claimed_count, 1);
    assert_eq!(d1.status, DistributionStatus::Active);

    // User C claims out of order
    fixture
        .client
        .claim(&dist_id, &2, &user_c, &40_000, &tree.proofs[2]);
    assert_eq!(fixture.token.balance(&user_c), 40_000);
    let d2 = fixture.client.get_distribution(&dist_id);
    assert_eq!(d2.claimed_amount, 50_000);
    assert_eq!(d2.claimed_count, 2);
    assert_eq!(d2.status, DistributionStatus::Active);

    // User B claims
    fixture
        .client
        .claim(&dist_id, &1, &user_b, &25_000, &tree.proofs[1]);
    assert_eq!(fixture.token.balance(&user_b), 25_000);

    // User D claims final allocation -> marks Completed
    fixture
        .client
        .claim(&dist_id, &3, &user_d, &75_000, &tree.proofs[3]);
    assert_eq!(fixture.token.balance(&user_d), 75_000);

    let d_final = fixture.client.get_distribution(&dist_id);
    assert_eq!(d_final.claimed_amount, total);
    assert_eq!(d_final.claimed_count, 4);
    assert_eq!(d_final.status, DistributionStatus::Completed);
    assert_eq!(fixture.token.balance(&fixture.contract_id), 0);
}

#[test]
fn test_double_claim_rejected() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let recipient = Address::generate(&env);
    let amount = 100_000i128;

    let items = [SimpleClaimItem {
        index: 0,
        recipient: recipient.clone(),
        amount,
    }];
    let tree = build_test_tree(&env, 1, &items);

    let dist_id = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &tree.root,
        &amount,
        &1,
        &0,
    );

    // First claim succeeds
    fixture
        .client
        .claim(&dist_id, &0, &recipient, &amount, &tree.proofs[0]);

    // Second claim must fail
    let res = fixture
        .client
        .try_claim(&dist_id, &0, &recipient, &amount, &tree.proofs[0]);
    assert_eq!(res, Err(Ok(ContractError::AlreadyClaimed)));
}

#[test]
fn test_invalid_proof_rejected() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);

    let items = [
        SimpleClaimItem {
            index: 0,
            recipient: user_a.clone(),
            amount: 50_000,
        },
        SimpleClaimItem {
            index: 1,
            recipient: user_b.clone(),
            amount: 50_000,
        },
    ];
    let tree = build_test_tree(&env, 1, &items);

    let dist_id = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &tree.root,
        &100_000,
        &2,
        &0,
    );

    // Corrupted proof sibling
    let mut bad_proof = tree.proofs[0].clone();
    bad_proof.set(0, BytesN::from_array(&env, &[99u8; 32]));

    let res = fixture
        .client
        .try_claim(&dist_id, &0, &user_a, &50_000, &bad_proof);
    assert_eq!(res, Err(Ok(ContractError::InvalidProof)));
}

#[test]
fn test_tampered_amount_or_recipient_rejected() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let recipient = Address::generate(&env);
    let attacker = Address::generate(&env);
    let amount = 50_000i128;

    let items = [SimpleClaimItem {
        index: 0,
        recipient: recipient.clone(),
        amount,
    }];
    let tree = build_test_tree(&env, 1, &items);

    let dist_id = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &tree.root,
        &amount,
        &1,
        &0,
    );

    // Tampered amount (trying to claim more than leaf allows)
    let res_amount =
        fixture
            .client
            .try_claim(&dist_id, &0, &recipient, &60_000, &tree.proofs[0]);
    assert_eq!(res_amount, Err(Ok(ContractError::InvalidProof)));

    // Tampered recipient (attacker trying to claim with recipient's proof)
    let res_attacker =
        fixture
            .client
            .try_claim(&dist_id, &0, &attacker, &amount, &tree.proofs[0]);
    assert_eq!(res_attacker, Err(Ok(ContractError::InvalidProof)));
}

#[test]
fn test_claim_out_of_bounds_index() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let recipient = Address::generate(&env);
    let amount = 50_000i128;

    let items = [SimpleClaimItem {
        index: 0,
        recipient: recipient.clone(),
        amount,
    }];
    let tree = build_test_tree(&env, 1, &items);

    let dist_id = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &tree.root,
        &amount,
        &1,
        &0,
    );

    // Index 1 is out of bounds for num_recipients = 1
    let res = fixture
        .client
        .try_claim(&dist_id, &1, &recipient, &amount, &tree.proofs[0]);
    assert_eq!(res, Err(Ok(ContractError::InvalidLeafIndex)));
}

#[test]
fn test_claim_expired_distribution() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let recipient = Address::generate(&env);
    let amount = 50_000i128;

    let items = [SimpleClaimItem {
        index: 0,
        recipient: recipient.clone(),
        amount,
    }];
    let tree = build_test_tree(&env, 1, &items);

    let end_time = 2_000u64;
    let dist_id = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &tree.root,
        &amount,
        &1,
        &end_time,
    );

    // Advance ledger timestamp to expiration
    env.ledger().set_timestamp(end_time);

    let res = fixture
        .client
        .try_claim(&dist_id, &0, &recipient, &amount, &tree.proofs[0]);
    assert_eq!(res, Err(Ok(ContractError::DistributionExpired)));
}
