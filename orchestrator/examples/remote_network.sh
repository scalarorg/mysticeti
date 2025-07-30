#!/bin/bash

# Example script for running the remote Mysticeti network orchestrator

set -e

echo "=== Mysticeti Remote Network Example ==="

# Function to prompt for input with default value
prompt_with_default() {
    local prompt="$1"
    local default="$2"
    local var_name="$3"
    
    if [ -z "$default" ]; then
        read -p "$prompt: " input
    else
        read -p "$prompt [$default]: " input
    fi
    
    # Use default if input is empty
    if [ -z "$input" ]; then
        input="$default"
    fi
    
    eval "$var_name=\"$input\""
}

# Function to prompt for yes/no with default
prompt_yes_no() {
    local prompt="$1"
    local default="$2"
    local var_name="$3"
    
    while true; do
        if [ "$default" = "y" ]; then
            read -p "$prompt [Y/n]: " input
        else
            read -p "$prompt [y/N]: " input
        fi
        
        # Use default if input is empty
        if [ -z "$input" ]; then
            input="$default"
        fi
        
        case $input in
            [Yy]* ) eval "$var_name=true"; break;;
            [Nn]* ) eval "$var_name=false"; break;;
            * ) echo "Please answer y or n.";;
        esac
    done
}

# Check if environment variables are set
if [ -z "$MYSTICETI_NODE0_HOST" ] || [ -z "$MYSTICETI_NODE1_HOST" ] || [ -z "$MYSTICETI_NODE2_HOST" ] || [ -z "$MYSTICETI_NODE3_HOST" ]; then
    echo "Error: Required environment variables not set!"
    echo "Please set the following environment variables:"
    echo "  MYSTICETI_NODE0_HOST"
    echo "  MYSTICETI_NODE1_HOST" 
    echo "  MYSTICETI_NODE2_HOST"
    echo "  MYSTICETI_NODE3_HOST"
    echo ""
    echo "Example:"
    echo "  export MYSTICETI_NODE0_HOST=\"192.168.1.10\""
    echo "  export MYSTICETI_NODE1_HOST=\"192.168.1.11\""
    echo "  export MYSTICETI_NODE2_HOST=\"192.168.1.12\""
    echo "  export MYSTICETI_NODE3_HOST=\"192.168.1.13\""
    exit 1
fi

echo "Using remote nodes:"
echo "  Node 0: $MYSTICETI_NODE0_HOST"
echo "  Node 1: $MYSTICETI_NODE1_HOST"
echo "  Node 2: $MYSTICETI_NODE2_HOST"
echo "  Node 3: $MYSTICETI_NODE3_HOST"

echo ""
echo "Configure remote network parameters:"
echo "==================================="

# Prompt for parameters
prompt_with_default "Number of transactions to simulate" "2000" "NUM_TRANSACTIONS"
prompt_with_default "Transaction size in bytes" "1024" "TRANSACTION_SIZE"
prompt_with_default "Transaction rate (tx/s)" "200" "TRANSACTION_RATE"
prompt_with_default "Startup wait time in seconds" "90" "STARTUP_WAIT"
prompt_with_default "SSH timeout in seconds" "30" "SSH_TIMEOUT"
prompt_yes_no "Clean up containers after completion" "y" "CLEANUP"

# Convert boolean to flag
if [ "$CLEANUP" = "true" ]; then
    CLEANUP_FLAG="--cleanup"
else
    CLEANUP_FLAG=""
fi

echo ""
echo "Configuration Summary:"
echo "====================="
echo "Number of transactions: $NUM_TRANSACTIONS"
echo "Transaction size: $TRANSACTION_SIZE bytes"
echo "Transaction rate: $TRANSACTION_RATE tx/s"
echo "Startup wait: $STARTUP_WAIT seconds"
echo "SSH timeout: $SSH_TIMEOUT seconds"
echo "Cleanup: $CLEANUP"
echo ""

# Confirm before proceeding
read -p "Proceed with these settings? [Y/n]: " confirm
if [[ $confirm =~ ^[Nn]$ ]]; then
    echo "Aborted."
    exit 0
fi

echo ""

# Build the binary
echo "Building remote-network binary..."
cargo build --release --bin remote-network

# Run with user-specified parameters
echo "Starting remote network with specified parameters..."
cargo run --release --bin remote-network -- \
  --num-transactions "$NUM_TRANSACTIONS" \
  --transaction-size "$TRANSACTION_SIZE" \
  --transaction-rate "$TRANSACTION_RATE" \
  --startup-wait "$STARTUP_WAIT" \
  --ssh-timeout "$SSH_TIMEOUT" \
  $CLEANUP_FLAG

echo "=== Example completed ===" 