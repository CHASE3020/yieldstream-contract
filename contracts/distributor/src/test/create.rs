use super::setup_test;
use crate::errors::ContractError;
use crate::types::DistributionStatus;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

#[test]
fn test_initialize_admin() {
    let env = Env::default();
    let fixture = setup_test(&env);

    fixture.client.initialize(&fixture.admin);
    assert_eq!(fixture.client.get_admin(), fixture.admin);

    // Re-initialization must fail
    let second_admin = Address::generate(&env);
    let res = fixture.client.try_initialize(&second_admin);
    assert_eq!(res, Err(Ok(ContractError::AlreadyInitialized)));
}

#[test]
fn test_create_distribution_success() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let dummy_root = BytesN::from_array(&env, &[7u8; 32]);
    let total_amount = 1_000_000i128;
    let num_recipients = 10u32;
    let end_time = 2_000u64;

    let balance_before = fixture.token.balance(&fixture.creator);
    let dist_id = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &dummy_root,
        &total_amount,
        &num_recipients,
        &end_time,
    );

    assert_eq!(dist_id, 1);
    assert_eq!(fixture.client.get_distribution_count(), 1);

    // Verify token transfer to contract
    let balance_after = fixture.token.balance(&fixture.creator);
    assert_eq!(balance_before - balance_after, total_amount);
    assert_eq!(
        fixture.token.balance(&fixture.contract_id),
        total_amount
    );

    // Verify stored distribution state
    let dist = fixture.client.get_distribution(&dist_id);
    assert_eq!(dist.id, 1);
    assert_eq!(dist.creator, fixture.creator);
    assert_eq!(dist.token, fixture.token_id);
    assert_eq!(dist.merkle_root, dummy_root);
    assert_eq!(dist.total_amount, total_amount);
    assert_eq!(dist.claimed_amount, 0);
    assert_eq!(dist.num_recipients, num_recipients);
    assert_eq!(dist.claimed_count, 0);
    assert_eq!(dist.created_at, 1_000);
    assert_eq!(dist.end_time, end_time);
    assert_eq!(dist.status, DistributionStatus::Active);
}

#[test]
fn test_multi_tenant_distributions() {
    let env = Env::default();
    let fixture = setup_test(&env);

    let tenant_b = Address::generate(&env);
    let tenant_c = Address::generate(&env);
    fixture.token_admin.mint(&tenant_b, &5_000_000);
    fixture.token_admin.mint(&tenant_c, &5_000_000);

    let root_a = BytesN::from_array(&env, &[1u8; 32]);
    let root_b = BytesN::from_array(&env, &[2u8; 32]);
    let root_c = BytesN::from_array(&env, &[3u8; 32]);

    let id_a = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &root_a,
        &100_000,
        &5,
        &0,
    );
    let id_b = fixture.client.create_distribution(
        &tenant_b,
        &fixture.token_id,
        &root_b,
        &200_000,
        &10,
        &5_000,
    );
    let id_c = fixture.client.create_distribution(
        &tenant_c,
        &fixture.token_id,
        &root_c,
        &300_000,
        &15,
        &10_000,
    );

    assert_eq!(id_a, 1);
    assert_eq!(id_b, 2);
    assert_eq!(id_c, 3);
    assert_eq!(fixture.client.get_distribution_count(), 3);

    let dist_a = fixture.client.get_distribution(&id_a);
    let dist_b = fixture.client.get_distribution(&id_b);
    let dist_c = fixture.client.get_distribution(&id_c);

    assert_eq!(dist_a.creator, fixture.creator);
    assert_eq!(dist_b.creator, tenant_b);
    assert_eq!(dist_c.creator, tenant_c);

    assert_eq!(
        fixture.token.balance(&fixture.contract_id),
        600_000
    );
}

#[test]
fn test_create_distribution_invalid_amount() {
    let env = Env::default();
    let fixture = setup_test(&env);
    let root = BytesN::from_array(&env, &[0u8; 32]);

    let res_zero = fixture.client.try_create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &root,
        &0,
        &5,
        &0,
    );
    assert_eq!(res_zero, Err(Ok(ContractError::InvalidAmount)));

    let res_neg = fixture.client.try_create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &root,
        &-100,
        &5,
        &0,
    );
    assert_eq!(res_neg, Err(Ok(ContractError::InvalidAmount)));
}

#[test]
fn test_create_distribution_zero_recipients() {
    let env = Env::default();
    let fixture = setup_test(&env);
    let root = BytesN::from_array(&env, &[0u8; 32]);

    let res = fixture.client.try_create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &root,
        &100_000,
        &0,
        &0,
    );
    assert_eq!(res, Err(Ok(ContractError::InvalidRecipientCount)));
}

#[test]
fn test_create_distribution_invalid_end_time() {
    let env = Env::default();
    let fixture = setup_test(&env);
    let root = BytesN::from_array(&env, &[0u8; 32]);

    // Current ledger timestamp is 1_000. End time 999 is in the past.
    let res_past = fixture.client.try_create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &root,
        &100_000,
        &5,
        &999,
    );
    assert_eq!(res_past, Err(Ok(ContractError::InvalidEndTime)));

    // End time 1_000 equals current timestamp (expired immediately)
    let res_eq = fixture.client.try_create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &root,
        &100_000,
        &5,
        &1_000,
    );
    assert_eq!(res_eq, Err(Ok(ContractError::InvalidEndTime)));
}

#[test]
fn test_create_distribution_without_expiration() {
    let env = Env::default();
    let fixture = setup_test(&env);
    let root = BytesN::from_array(&env, &[5u8; 32]);

    let dist_id = fixture.client.create_distribution(
        &fixture.creator,
        &fixture.token_id,
        &root,
        &50_000,
        &1,
        &0,
    );
    let dist = fixture.client.get_distribution(&dist_id);
    assert_eq!(dist.end_time, 0);
    assert_eq!(dist.status, DistributionStatus::Active);
}
