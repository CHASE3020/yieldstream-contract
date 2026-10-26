use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, Env, Vec};

/// Domain separation prefix for Merkle leaf hashes.
/// Prevents second-preimage collisions between leaves and interior nodes (RFC 6962 standard).
pub const LEAF_DOMAIN_PREFIX: u8 = 0x00;

/// Domain separation prefix for Merkle internal node hashes.
pub const NODE_DOMAIN_PREFIX: u8 = 0x01;

/// Computes the cryptographic leaf hash for a distribution allocation.
///
/// Format: `SHA256(0x00 || XDR(distribution_id) || XDR(index) || XDR(recipient) || XDR(amount))`
pub fn compute_leaf(
    env: &Env,
    distribution_id: u32,
    index: u32,
    recipient: &Address,
    amount: i128,
) -> BytesN<32> {
    let mut data = Bytes::new(env);
    data.push_back(LEAF_DOMAIN_PREFIX);
    data.append(&distribution_id.to_xdr(env));
    data.append(&index.to_xdr(env));
    data.append(&recipient.to_xdr(env));
    data.append(&amount.to_xdr(env));
    env.crypto().sha256(&data).into()
}

/// Hashes two sibling nodes with sorted pair ordering (OpenZeppelin / RFC 6962 pattern).
///
/// Sorting `(min(a, b), max(a, b))` ensures that proof verification does not require
/// explicit left/right sibling direction flags, preventing malleability attacks.
///
/// Format: `SHA256(0x01 || min(a, b) || max(a, b))`
pub fn hash_pair(env: &Env, a: &BytesN<32>, b: &BytesN<32>) -> BytesN<32> {
    let mut data = Bytes::new(env);
    data.push_back(NODE_DOMAIN_PREFIX);
    if a <= b {
        data.append(&a.clone().into());
        data.append(&b.clone().into());
    } else {
        data.append(&b.clone().into());
        data.append(&a.clone().into());
    }
    env.crypto().sha256(&data).into()
}

/// Verifies whether a given leaf belongs to the Merkle tree with the specified root.
///
/// Iteratively accumulates sibling hashes along the proof path up to the root.
pub fn verify_merkle_proof(
    env: &Env,
    proof: &Vec<BytesN<32>>,
    root: &BytesN<32>,
    leaf: &BytesN<32>,
) -> bool {
    let mut current = leaf.clone();
    for sibling in proof.iter() {
        current = hash_pair(env, &current, &sibling);
    }
    current == *root
}
