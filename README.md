# YieldStream: Multi-Tenant Merkle Distributor

[![CI](https://github.com/CHASE3020/yieldstream-contract/actions/workflows/ci.yml/badge.svg)](https://github.com/CHASE3020/yieldstream-contract)
[![Stellar Protocol](https://img.shields.io/badge/Stellar-Protocol%2022%2F25-blue)](https://stellar.org)
[![Target](https://img.shields.io/badge/target-wasm32--v1--none-green)](https://github.com/stellar/rs-soroban-sdk)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

**YieldStream** is a multi-tenant, high-performance Merkle distributor smart contract engineered for the Stellar Soroban ecosystem. It enables protocols, yield aggregators, DAOs, and projects to distribute rewards, token incentives, retroactive yield, and airdrops to thousands of recipients efficiently using cryptographic inclusion proofs.

---

## Key Highlights

- **Multi-Tenant Architecture**: Any project or tenant can create independent distribution campaigns with customized token addresses, Merkle roots, allocation budgets, and expiration deadlines on a single deployed contract.
- **Cryptographic Security & RFC 6962 Compliance**:
  - Domain separation tags: `0x00` prefix for leaves, `0x01` prefix for internal nodes to prevent second-preimage / leaf-node ambiguity attacks.
  - Sorted sibling pairs (`min(a, b) || max(a, b)`): Eliminates proof malleability and avoids needing left/right proof direction flags.
- **State & TTL Optimization**:
  - Instance storage for global counters and administrative authority.
  - Persistent storage for distribution metadata and claimed leaf records.
  - Automatic TTL extensions on every interaction (`extend_ttl`) guaranteeing state longevity.
- **Capital Safety & Sweeping**:
  - Creator cancellation: Creators can cancel an active campaign if needed.
  - Clawback & Sweep: After expiration or cancellation, unclaimed tokens can be swept back to a designated treasury address.
- **Deterministic Test Vectors**: Validated with verifiable JSON Merkle test vectors in `test-vectors/merkle_v1.json`.

---

## Directory Structure

```text
yieldstream-contract/
├── Cargo.toml                      # Cargo workspace configuration
├── rust-toolchain.toml             # Pinned Rust toolchain and WASM targets
├── scripts/
│   └── deploy.sh                   # Automated deployment and setup script
├── test-vectors/
│   └── merkle_v1.json              # Canonical test vectors with proofs
└── contracts/distributor/
    ├── Cargo.toml                  # Distributor contract manifest
    └── src/
        ├── lib.rs                  # Contract entrypoints and public interface
        ├── types.rs                # Data structures (Distribution, DataKey, Status)
        ├── errors.rs               # ContractError error codes
        ├── storage.rs              # Instance and persistent storage helpers + TTL
        ├── merkle.rs               # Domain-separated Merkle proof verification
        ├── events.rs               # Structured contract lifecycle events
        └── test/
            ├── mod.rs              # Test harness and tree generator
            ├── create.rs           # Creation and multi-tenant test suite
            ├── claim.rs            # Claim verification and double-claim suite
            ├── cancel_sweep.rs     # Cancellation and treasury sweep suite
            └── vectors.rs          # Merkle vector conformance test suite
```

---

## Cryptographic Design

### 1. Leaf Hashing
Leaves commit to the distribution ID, unique index, recipient address, and token amount:
$$\text{leaf} = \text{SHA-256}(0\text{x}00 \mathbin{\Vert} \text{XDR}(\text{id}) \mathbin{\Vert} \text{XDR}(\text{index}) \mathbin{\Vert} \text{XDR}(\text{recipient}) \mathbin{\Vert} \text{XDR}(\text{amount}))$$

### 2. Sorted Node Hashing
To prevent proof manipulation, sibling pairs are sorted lexicographically:
$$\text{parent} = \text{SHA-256}(0\text{x}01 \mathbin{\Vert} \min(a, b) \mathbin{\Vert} \max(a, b))$$

---

## Smart Contract Interface

| Function | Access | Description |
|---|---|---|
| `initialize(admin)` | Admin Auth | Sets the initial contract administrator. |
| `create_distribution(creator, token, merkle_root, total_amount, num_recipients, end_time)` | Creator Auth | Creates and funds a new distribution campaign. |
| `claim(distribution_id, index, recipient, amount, proof)` | Recipient Auth | Verifies proof and transfers tokens to recipient. |
| `cancel(distribution_id)` | Creator Auth | Cancels active distribution, halting new claims. |
| `sweep(distribution_id, to)` | Creator Auth | Sweeps remaining unclaimed tokens if expired or cancelled. |
| `get_distribution(distribution_id)` | Read-only | Returns distribution details and current status. |
| `is_claimed(distribution_id, index)` | Read-only | Checks if a leaf index has already been claimed. |
| `compute_leaf(distribution_id, index, recipient, amount)` | Pure Helper | Computes leaf hash according to contract format. |
| `verify_proof(proof, root, leaf)` | Pure Helper | Verifies Merkle inclusion against root. |

---

## Quickstart

### Prerequisites
- [Rust](https://rustup.rs/) (1.84+)
- WASM target: `rustup target add wasm32v1-none`
- [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/install) (v22+)

### Run Test Suite
```bash
cargo test
```

### Build Contract WASM
```bash
stellar contract build
```
Optimized WASM will be output at:
`target/wasm32v1-none/release/yieldstream_distributor.wasm`

### Deploy
```bash
./scripts/deploy.sh --network testnet --source <IDENTITY>
```
EOF
