#!/bin/bash

# Example script for running the local Mysticeti network orchestrator
DIR="$( cd "$( dirname "$0" )" && pwd )"
set -e

echo "=== Mysticeti Local Network Example ==="

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

echo "Configure local network parameters:"
echo "=================================="

# Get the default docker-compose path (orchestrator directory)
DEFAULT_DOCKER_COMPOSE_PATH="${DIR}/docker-compose.yml"

# Prompt for parameters
prompt_with_default "Path to docker-compose.yml file" "$DEFAULT_DOCKER_COMPOSE_PATH" "DOCKER_COMPOSE_PATH"
prompt_with_default "Number of transactions to simulate" "1000" "NUM_TRANSACTIONS"
prompt_with_default "Transaction size in bytes" "512" "TRANSACTION_SIZE"
prompt_with_default "Transaction rate (tx/s)" "100" "TRANSACTION_RATE"
prompt_with_default "Startup wait time in seconds" "30" "STARTUP_WAIT"
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
echo "Docker-compose path: $DOCKER_COMPOSE_PATH"
echo "Number of transactions: $NUM_TRANSACTIONS"
echo "Transaction size: $TRANSACTION_SIZE bytes"
echo "Transaction rate: $TRANSACTION_RATE tx/s"
echo "Startup wait: $STARTUP_WAIT seconds"
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
# echo "Building local-network binary..."
# cargo build --release --bin local-network

# Run with user-specified parameters
echo "Starting local network with specified parameters..."
cargo run --release --bin local-network -- \
  --docker-compose-path "$DOCKER_COMPOSE_PATH" \
  --num-transactions "$NUM_TRANSACTIONS" \
  --transaction-size "$TRANSACTION_SIZE" \
  --transaction-rate "$TRANSACTION_RATE" \
  --startup-wait "$STARTUP_WAIT" \
  $CLEANUP_FLAG

echo "=== Example completed ===" 