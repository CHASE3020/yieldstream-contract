use soroban_sdk::{symbol_short, Address, BytesN, Env};

/// Emits an event when a new distribution is successfully created and funded.
pub fn distribution_created(
    env: &Env,
    distribution_id: u32,
    creator: &Address,
    token: &Address,
    merkle_root: &BytesN<32>,
    total_amount: i128,
    end_time: u64,
) {
    let topics = (symbol_short!("created"), distribution_id, creator.clone());
    let data = (token.clone(), merkle_root.clone(), total_amount, end_time);
    env.events().publish(topics, data);
}

/// Emits an event when a recipient claims their allocation.
pub fn distribution_claimed(
    env: &Env,
    distribution_id: u32,
    index: u32,
    recipient: &Address,
    amount: i128,
) {
    let topics = (symbol_short!("claimed"), distribution_id, recipient.clone());
    let data = (index, amount);
    env.events().publish(topics, data);
}

/// Emits an event when a distribution is cancelled by its creator.
pub fn distribution_cancelled(
    env: &Env,
    distribution_id: u32,
    creator: &Address,
    remaining_amount: i128,
) {
    let topics = (symbol_short!("cancel"), distribution_id, creator.clone());
    let data = remaining_amount;
    env.events().publish(topics, data);
}

/// Emits an event when remaining unclaimed tokens are swept.
pub fn distribution_swept(
    env: &Env,
    distribution_id: u32,
    creator: &Address,
    recipient: &Address,
    swept_amount: i128,
) {
    let topics = (symbol_short!("swept"), distribution_id, creator.clone());
    let data = (recipient.clone(), swept_amount);
    env.events().publish(topics, data);
}
