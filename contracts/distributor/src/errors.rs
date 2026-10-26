use soroban_sdk::contracterror;

/// Contract error codes for the YieldStream Distributor contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// Contract instance is not initialized.
    NotInitialized = 1,
    /// Contract instance has already been initialized.
    AlreadyInitialized = 2,
    /// Caller is not authorized to perform the operation.
    Unauthorized = 3,
    /// Amount supplied or requested is zero or negative.
    InvalidAmount = 4,
    /// Specified leaf index is out of bounds for the distribution.
    InvalidLeafIndex = 5,
    /// Requested distribution ID does not exist.
    DistributionNotFound = 6,
    /// Distribution is not in Active status.
    DistributionNotActive = 7,
    /// Distribution has been cancelled by the creator.
    DistributionCancelled = 8,
    /// Distribution has expired and claims are closed.
    DistributionExpired = 9,
    /// Distribution has not yet reached its expiration time.
    DistributionNotExpired = 10,
    /// Specified leaf has already been claimed.
    AlreadyClaimed = 11,
    /// Provided Merkle inclusion proof is invalid.
    InvalidProof = 12,
    /// Arithmetic overflow or underflow occurred during computation.
    ArithmeticOverflow = 13,
    /// End timestamp must be greater than current ledger timestamp.
    InvalidEndTime = 14,
    /// No remaining tokens to sweep.
    NothingToSweep = 15,
    /// Recipient count must be greater than zero.
    InvalidRecipientCount = 16,
    /// Provided address cannot be zero or empty.
    ZeroAddress = 17,
}
