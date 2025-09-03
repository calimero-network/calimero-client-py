#!/bin/bash

# Test script for crates.io API parsing
VERSION="0.2.2"

echo "Testing crates.io API for calimero-primitives version $VERSION"
echo "=========================================================="

# Get the API response
CRATES_RESPONSE=$(curl -s "https://crates.io/api/v1/crates/calimero-primitives/versions")

# Check if we got a response
if [ -z "$CRATES_RESPONSE" ]; then
    echo "❌ Error: No response from crates.io API"
    exit 1
fi

echo "✅ Got response from API (length: ${#CRATES_RESPONSE} characters)"

# Test the jq command
echo ""
echo "Testing jq parsing:"
PUBLISHED=$(echo "$CRATES_RESPONSE" | jq -r --arg ver "$VERSION" '.versions[] | select(.num == $ver) | .num // empty' 2>/dev/null || echo "")

if [ -z "$PUBLISHED" ]; then
    echo "❌ jq returned empty result"
    echo ""
    echo "Debugging:"
    echo "1. Checking if jq is available:"
    if command -v jq &> /dev/null; then
        echo "   ✅ jq is available"
    else
        echo "   ❌ jq is not available"
        exit 1
    fi
    
    echo ""
    echo "2. Checking API response structure:"
    echo "$CRATES_RESPONSE" | jq '.versions | length' 2>/dev/null || echo "Failed to parse versions array"
    
    echo ""
    echo "3. Checking first few versions:"
    echo "$CRATES_RESPONSE" | jq '.versions[0:3] | .[] | .num' 2>/dev/null || echo "Failed to parse version numbers"
    
    echo ""
    echo "4. Testing exact match:"
    echo "$CRATES_RESPONSE" | jq -r --arg ver "$VERSION" '.versions[] | select(.num == $ver) | .num' 2>/dev/null || echo "Failed to find exact match"
    
else
    echo "✅ jq returned: '$PUBLISHED'"
fi

echo ""
echo "5. Raw API response (first 500 chars):"
echo "$CRATES_RESPONSE" | head -c 500
echo "..."
