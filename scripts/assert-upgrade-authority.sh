#!/bin/bash
set -e

# Configuration
PROGRAM_ID="62syzcwvnS56yKHakNx2rr4JBd5BJmgJ7jDMK3SiipbM"
EXPECTED_AUTHORITY="${GOV_MULTISIG_ADDRESS:-<GOV_MULTISIG_ADDRESS>}"

echo "🔍 Checking upgrade authority for Hero-Reward Program"
echo "Program ID: $PROGRAM_ID"
echo "Expected Authority: $EXPECTED_AUTHORITY"

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    echo "❌ jq is required but not installed. Please install jq first."
    exit 1
fi

# Check if solana CLI is available
if ! command -v solana &> /dev/null; then
    echo "❌ Solana CLI is required but not installed. Please install Solana CLI first."
    exit 1
fi

# Get current upgrade authority
echo "📡 Fetching program information..."
CURRENT_AUTHORITY=$(solana program show $PROGRAM_ID --output json | jq -r '.upgradeAuthority // "null"')

if [ "$CURRENT_AUTHORITY" = "null" ]; then
    echo "❌ Failed to fetch program information or program not found"
    exit 1
fi

echo "Current Authority: $CURRENT_AUTHORITY"

# Check if upgrade authority matches
if [ "$CURRENT_AUTHORITY" = "$EXPECTED_AUTHORITY" ]; then
    echo "✅ Upgrade authority is correctly set to GOV multisig"
    exit 0
else
    echo "❌ Upgrade authority mismatch!"
    echo "Expected: $EXPECTED_AUTHORITY"
    echo "Current:  $CURRENT_AUTHORITY"
    echo ""
    echo "To fix this, run:"
    echo "solana program set-upgrade-authority $PROGRAM_ID --new-upgrade-authority $EXPECTED_AUTHORITY"
    exit 1
fi
