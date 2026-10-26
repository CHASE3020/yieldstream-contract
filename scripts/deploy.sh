#!/usr/bin/env bash
# ==============================================================================
# YieldStream Multi-Tenant Merkle Distributor - Deployment Script
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

NETWORK="${NETWORK:-testnet}"
SOURCE_IDENTITY="${SOURCE_IDENTITY:-alice}"
ADMIN_ADDRESS="${ADMIN_ADDRESS:-}"

function usage() {
  cat << EOF
Usage: $0 [OPTIONS]

Deploys and initializes the YieldStream Distributor contract on Stellar.

Options:
  -n, --network NETWORK         Network to deploy to: testnet, standalone, mainnet (default: testnet)
  -s, --source IDENTITY         Identity alias or secret key used to sign transactions (default: alice)
  -a, --admin ADDRESS           Admin address to initialize the contract with (default: source identity address)
  -h, --help                    Display this help message

Environment Variables:
  NETWORK                       Network override
  SOURCE_IDENTITY               Source identity override
  ADMIN_ADDRESS                 Admin address override

Examples:
  ./scripts/deploy.sh --network testnet --source alice
  ./scripts/deploy.sh -n standalone -s bob -a G...
EOF
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -n|--network)
      NETWORK="$2"
      shift 2
      ;;
    -s|--source)
      SOURCE_IDENTITY="$2"
      shift 2
      ;;
    -a|--admin)
      ADMIN_ADDRESS="$2"
      shift 2
      ;;
    -h|--help)
      usage
      ;;
    *)
      echo "Unknown option: $1"
      usage
      ;;
  esac
done

echo "======================================================================"
echo "🚀 Deploying YieldStream Distributor to Stellar [${NETWORK}]"
echo "======================================================================"
echo "Source Identity: ${SOURCE_IDENTITY}"

cd "${ROOT_DIR}"

# Step 1: Ensure toolchain is ready
if ! command -v stellar &> /dev/null; then
  echo "❌ Error: 'stellar' CLI is not installed or not in PATH."
  echo "Please install it: cargo install --locked stellar-cli --features opt"
  exit 1
fi

# Step 2: Build contract WASM
echo "📦 Building contract WASM..."
stellar contract build

WASM_FILE="${ROOT_DIR}/target/wasm32v1-none/release/yieldstream_distributor.wasm"
if [ ! -f "${WASM_FILE}" ]; then
  # Fallback for wasm32-unknown-unknown target directory
  WASM_FILE="${ROOT_DIR}/target/wasm32-unknown-unknown/release/yieldstream_distributor.wasm"
fi

if [ ! -f "${WASM_FILE}" ]; then
  echo "❌ Error: Built WASM file not found at ${WASM_FILE}"
  exit 1
fi

WASM_SIZE=$(wc -c < "${WASM_FILE}")
echo "✅ Built WASM: ${WASM_FILE} (${WASM_SIZE} bytes)"

# Step 3: Install contract WASM
echo "📤 Installing WASM bytecode on ${NETWORK}..."
WASM_HASH=$(stellar contract install \
  --wasm "${WASM_FILE}" \
  --network "${NETWORK}" \
  --source "${SOURCE_IDENTITY}" \
  2>&1 | tail -n 1 | tr -d '\r\n')

echo "✅ WASM Hash: ${WASM_HASH}"

# Step 4: Deploy contract instance
echo "🏗️  Deploying contract instance..."
CONTRACT_ID=$(stellar contract deploy \
  --wasm-hash "${WASM_HASH}" \
  --network "${NETWORK}" \
  --source "${SOURCE_IDENTITY}" \
  2>&1 | tail -n 1 | tr -d '\r\n')

echo "✅ Contract Deployed! Contract ID: ${CONTRACT_ID}"

# Step 5: Resolve admin address if not provided
if [ -z "${ADMIN_ADDRESS}" ]; then
  echo "🔍 Resolving address for source identity '${SOURCE_IDENTITY}'..."
  ADMIN_ADDRESS=$(stellar keys address "${SOURCE_IDENTITY}" 2>/dev/null || echo "")
fi

# Step 6: Initialize contract if admin address is available
if [ -n "${ADMIN_ADDRESS}" ]; then
  echo "⚙️  Initializing contract with admin: ${ADMIN_ADDRESS}..."
  stellar contract invoke \
    --id "${CONTRACT_ID}" \
    --network "${NETWORK}" \
    --source "${SOURCE_IDENTITY}" \
    -- \
    initialize \
    --admin "${ADMIN_ADDRESS}" > /dev/null
  echo "✅ Contract initialized successfully."
else
  echo "⚠️  Admin address not resolved; skipping contract initialization."
  echo "You can initialize it manually with:"
  echo "  stellar contract invoke --id ${CONTRACT_ID} --network ${NETWORK} --source ${SOURCE_IDENTITY} -- initialize --admin <ADMIN_ADDRESS>"
fi

echo "======================================================================"
echo "🎉 Deployment Summary"
echo "======================================================================"
echo "Network:        ${NETWORK}"
echo "WASM Hash:      ${WASM_HASH}"
echo "Contract ID:    ${CONTRACT_ID}"
echo "Admin Address:  ${ADMIN_ADDRESS:-'(uninitialized)'}"
echo ""
echo "Copy-pasteable environment export:"
echo "----------------------------------------------------------------------"
echo "export YIELDSTREAM_DISTRIBUTOR_ID=\"${CONTRACT_ID}\""
echo "export YIELDSTREAM_WASM_HASH=\"${WASM_HASH}\""
echo "export YIELDSTREAM_NETWORK=\"${NETWORK}\""
echo "======================================================================"
