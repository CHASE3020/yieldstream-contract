use super::{build_test_tree, setup_test, SimpleClaimItem};
use crate::errors::ContractError;
use crate::types::DistributionStatus;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

#[test]
fn test_cancel_active_distribution_success() {
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
        &5_000,
    );

    // Cancel distribution
    fixture.client.cancel(&dist_id);

    let dist = fixture.client.get_distribution(&dist_id);
    assert_eq!(dist.status, DistributionStatus::Cancelled);

    // Claims must now be rejected with DistributionCancelled
    let res = fixture
        .client
        .try_claim(&dist_id, &0, &recipient, &amount, &tree.proofs[0]);
    assert_eq!(res, Err(Ok(ContractError::DistributionCancelled)));
}

#[test]
fn test_cannot_cancel_already_cancelled_or_completed() {
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
        &5_000,
    );

    fixture.client.cancel(&dist_id);

    // Cancelling a second time fails with DistributionNotActive
    let res = fixture.client.try_cancel(&dist_id);
    assert_eq!(res, Err(Ok(ContractError::DistributionNotActive)));
}

#[test]
fn test_sweep_after_cancellation() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let treasury = Address::generate(&env);

    let items = [
        SimpleClaimItem {
            index: 0,
            recipient: user_a.clone(),
            amount: 100_000,
        },
        SimpleClaimItem {
            index: 1,
            recipient: user_b.clone(),
            amount: 400_000,
        },
    ];
    let total = 500_000i128;
    let tree = build_test_tree(&env, 1, &items);

    let dist_id = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &tree.root,
        &total,
        &2,
        &10_000,
    );

    // User A claims 100,000
    fixture
        .client
        .claim(&dist_id, &0, &user_a, &100_000, &tree.proofs[0]);
    assert_eq!(fixture.token.balance(&user_a), 100_000);

    // Creator cancels distribution early
    fixture.client.cancel(&dist_id);

    // Creator sweeps remaining 400,000 to treasury
    let swept = fixture.client.sweep(&dist_id, &treasury);
    assert_eq!(swept, 400_000);
    assert_eq!(fixture.token.balance(&treasury), 400_000);
    assert_eq!(fixture.token.balance(&fixture.contract_id), 0);

    let dist = fixture.client.get_distribution(&dist_id);
    assert_eq!(dist.status, DistributionStatus::Completed);
    assert_eq!(dist.claimed_amount, total);

    // Sweeping again fails with NothingToSweep
    let res = fixture.client.try_sweep(&dist_id, &treasury);
    assert_eq!(res, Err(Ok(ContractError::NothingToSweep)));
}

#[test]
fn test_sweep_after_expiration_without_cancel() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);

    let items = [
        SimpleClaimItem {
            index: 0,
            recipient: user_a.clone(),
            amount: 150_000,
        },
        SimpleClaimItem {
            index: 1,
            recipient: user_b.clone(),
            amount: 350_000,
        },
    ];
    let total = 500_000i128;
    let end_time = 2_000u64;
    let tree = build_test_tree(&env, 1, &items);

    let dist_id = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &tree.root,
        &total,
        &2,
        &end_time,
    );

    // User A claims before expiration
    fixture
        .client
        .claim(&dist_id, &0, &user_a, &150_000, &tree.proofs[0]);

    // Advance time past expiration
    env.ledger().set_timestamp(end_time + 10);

    // Creator sweeps remaining 350_000 directly without calling cancel
    let balance_before = fixture.token.balance(&fixture.creator);
    let swept = fixture.client.sweep(&dist_id, &fixture.creator);
    assert_eq!(swept, 350_000);
    let balance_after = fixture.token.balance(&fixture.creator);
    assert_eq!(balance_after - balance_before, 350_000);
}

#[test]
fn test_cannot_sweep_active_unexpired_distribution() {
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
        &5_000,
    );

    // Attempting to sweep while active and unexpired fails
    let res = fixture.client.try_sweep(&dist_id, &fixture.creator);
    assert_eq!(res, Err(Ok(ContractError::DistributionNotExpired)));
}
