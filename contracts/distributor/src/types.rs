use soroban_sdk::{contracttype, Address, BytesN};

/// Status lifecycle for a distribution campaign.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DistributionStatus {
    /// Active and accepting claims.
    Active = 0,
    /// Cancelled early by creator; claims halted.
    Cancelled = 1,
    /// All allocations claimed or remaining balance swept.
    Completed = 2,
    /// Past end_time without being cancelled; claims halted.
    Expired = 3,
}

/// Metadata and state for an individual Merkle distribution.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Distribution {
    /// Unique sequential distribution ID.
    pub id: u32,
    /// Tenant / creator address who funded the distribution.
    pub creator: Address,
    /// Stellar asset / token contract address.
    pub token: Address,
    /// 32-byte Merkle root committing to valid claims.
    pub merkle_root: BytesN<32>,
    /// Total amount of tokens deposited into the distribution.
    pub total_amount: i128,
    /// Cumulative tokens claimed so far.
    pub claimed_amount: i128,
    /// Total number of recipient leaves in the Merkle tree.
    pub num_recipients: u32,
    /// Count of leaves claimed so far.
    pub claimed_count: u32,
    /// Ledger timestamp when the distribution was created.
    pub created_at: u64,
    /// Ledger timestamp after which claims are closed (0 = no expiration).
    pub end_time: u64,
    /// Current status of the distribution.
    pub status: DistributionStatus,
}

/// Storage keys for contract instance and persistent state.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Instance key: contract administrator address.
    Admin,
    /// Instance key: total distributions count (u32).
    DistributionCount,
    /// Persistent key: distribution metadata by ID.
    Distribution(u32),
    /// Persistent key: claimed leaf record (distribution_id, leaf_index) -> bool.
    Claimed(u32, u32),
}
