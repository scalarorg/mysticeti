#!/bin/bash

# Comprehensive Benchmark Runner Script for Mysticeti
# This script runs the benchmark binary with full parameter configuration
DIR="$( cd "$( dirname "$0" )" && pwd )"
set -e

# Global variables for cleanup
BENCHMARK_PID=""
CLEANUP_FLAG=""
CLEANUP_THOROUGH_FLAG=""
NETWORK_TYPE=""

# Signal handler for graceful shutdown
cleanup_and_exit() {
    local signal_name="$1"
    echo ""
    echo "=== Received $signal_name signal ==="
    echo "Initiating graceful shutdown..."
    
    # Kill the benchmark process if it's running
    if [ -n "$BENCHMARK_PID" ] && kill -0 "$BENCHMARK_PID" 2>/dev/null; then
        echo "Terminating benchmark process (PID: $BENCHMARK_PID)..."
        kill -TERM "$BENCHMARK_PID" 2>/dev/null || true
        
        # Wait for the process to finish gracefully
        local count=0
        while kill -0 "$BENCHMARK_PID" 2>/dev/null && [ $count -lt 10 ]; do
            sleep 1
            count=$((count + 1))
        done
        
        # Force kill if still running
        if kill -0 "$BENCHMARK_PID" 2>/dev/null; then
            echo "Force killing benchmark process..."
            kill -KILL "$BENCHMARK_PID" 2>/dev/null || true
        fi
    fi
    
    # Perform cleanup for local networks on signal interruption
    if [ "$NETWORK_TYPE" = "local" ]; then
        if [ "$CLEANUP_THOROUGH_FLAG" = "--cleanup-thorough" ]; then
            echo "Performing thorough cleanup of Docker containers and volumes..."
            cd "$DIR/.." && docker-compose down -v --remove-orphans 2>/dev/null || true
        elif [ "$CLEANUP_FLAG" = "--cleanup" ]; then
            echo "Cleaning up Docker containers..."
            cd "$DIR/.." && docker-compose down 2>/dev/null || true
        else
            # Even if cleanup is not explicitly requested, we should stop containers on signal
            echo "Stopping Docker containers due to signal interruption..."
            cd "$DIR/.." && docker-compose down 2>/dev/null || true
        fi
    fi
    
    echo "Cleanup completed. Exiting..."
    exit 130  # Standard exit code for script terminated by Ctrl+C
}

# Set up signal traps
trap 'cleanup_and_exit "SIGINT (Ctrl+C)"' INT
trap 'cleanup_and_exit "SIGTERM"' TERM

echo "=== Mysticeti Comprehensive Benchmark Runner ==="
echo "Note: You can press Ctrl+C at any time to gracefully stop the benchmark"
echo ""

# Function to check if Docker is installed and running
check_docker() {
    echo "Checking Docker installation and status..."
    
    # Check if Docker is installed
    if ! command -v docker &> /dev/null; then
        echo "Error: Docker is not installed."
        echo "Please install Docker from https://docs.docker.com/get-docker/"
        echo ""
        echo "Installation instructions:"
        echo "  macOS: Download Docker Desktop from https://www.docker.com/products/docker-desktop"
        echo "  Ubuntu: sudo apt-get update && sudo apt-get install docker.io"
        echo "  CentOS: sudo yum install docker"
        exit 1
    fi
    
    # Check if Docker daemon is running
    if ! docker info &> /dev/null; then
        echo "Error: Docker is installed but not running."
        echo "Please start Docker and try again."
        echo ""
        echo "To start Docker:"
        echo "  macOS: Open Docker Desktop application"
        echo "  Linux: sudo systemctl start docker"
        echo "  Windows: Start Docker Desktop"
        exit 1
    fi
    
    # Check Docker Compose
    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        echo "Error: Docker Compose is not installed."
        echo "Please install Docker Compose to run local network benchmarks."
        echo ""
        echo "Installation instructions:"
        echo "  macOS: Docker Compose is included with Docker Desktop"
        echo "  Linux: sudo curl -L \"https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)\" -o /usr/local/bin/docker-compose && sudo chmod +x /usr/local/bin/docker-compose"
        exit 1
    fi
    
    echo "✓ Docker is installed and running"
    echo "✓ Docker Compose is available"
    echo ""
}

# Check Docker before proceeding
check_docker

# Function to check for docker-compose.yml file
check_docker_compose_file() {
    local compose_file="$1"
    
    if [ ! -f "$compose_file" ]; then
        echo "Error: docker-compose.yml not found at $compose_file"
        echo "Please ensure you're running from the orchestrator directory and the file exists."
        echo ""
        echo "Expected location: $(pwd)/$compose_file"
        echo "Current directory: $(pwd)"
        exit 1
    fi
    
    echo "✓ docker-compose.yml found at $compose_file"
    echo ""
}

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

echo "Configure benchmark parameters:"
echo "=============================="

# Get the default output directory
DEFAULT_OUTPUT_DIR="${DIR}/../benchmarks"

# Prompt for parameters
prompt_with_default "Output directory for results" "$DEFAULT_OUTPUT_DIR" "OUTPUT_DIR"
prompt_yes_no "Print results to console" "y" "CONSOLE_OUTPUT"
prompt_yes_no "Save results to file" "y" "FILE_OUTPUT"
prompt_with_default "Committee size" "4" "COMMITTEE"
prompt_with_default "Number of faulty nodes" "0" "FAULTS"
prompt_yes_no "Crash recovery enabled" "n" "CRASH_RECOVERY"
prompt_with_default "Crash interval (seconds)" "60" "CRASH_INTERVAL"
prompt_with_default "Startup waint (seconds)" "60" "STARTUP_WAIT"
prompt_with_default "Benchmark duration (seconds)" "180" "DURATION"
prompt_with_default "Network loads (comma-separated)" "100,200,500" "NETWORK_LOADS"
prompt_with_default "Network type (local or remote)" "local" "NETWORK_TYPE"
prompt_with_default "Transaction size in bytes" "512" "TRANSACTION_SIZE"

# Add cleanup options
echo ""
echo "Cleanup options:"
echo "  Regular cleanup: Stops containers but preserves volumes"
echo "  Thorough cleanup: Removes containers and volumes completely"
prompt_yes_no "Enable regular cleanup" "n" "CLEANUP"
prompt_yes_no "Enable thorough cleanup (overrides regular cleanup)" "n" "CLEANUP_THOROUGH"

# Convert boolean to flag
if [ "$CONSOLE_OUTPUT" = "true" ]; then
    CONSOLE_OUTPUT_FLAG="--console-output"
else
    CONSOLE_OUTPUT_FLAG=""
fi

if [ "$FILE_OUTPUT" = "true" ]; then
    FILE_OUTPUT_FLAG="--file-output"
else
    FILE_OUTPUT_FLAG=""
fi

if [ "$CRASH_RECOVERY" = "true" ]; then
    CRASH_RECOVERY_FLAG="--crash-recovery"
else
    CRASH_RECOVERY_FLAG=""
fi

if [ "$CLEANUP" = "true" ]; then
    CLEANUP_FLAG="--cleanup"
else
    CLEANUP_FLAG=""
fi

if [ "$CLEANUP_THOROUGH" = "true" ]; then
    CLEANUP_THOROUGH_FLAG="--cleanup-thorough"
else
    CLEANUP_THOROUGH_FLAG=""
fi

# Store network type for cleanup handler
NETWORK_TYPE="$NETWORK_TYPE"

echo ""
echo "Configuration Summary:"
echo "====================="
echo "Output directory: $OUTPUT_DIR"
echo "Console output: $CONSOLE_OUTPUT"
echo "File output: $FILE_OUTPUT"
echo "Committee size: $COMMITTEE"
echo "Faults: $FAULTS"
echo "Crash recovery: $CRASH_RECOVERY"
echo "Crash interval: $CRASH_INTERVAL seconds"
echo "Startup wait: $STARTUP_WAIT seconds"
echo "Duration: $DURATION seconds"
echo "Network loads: $NETWORK_LOADS"
echo "Network type: $NETWORK_TYPE"
echo "Transaction size: $TRANSACTION_SIZE bytes"
echo "Cleanup: $CLEANUP"
echo "Thorough cleanup: $CLEANUP_THOROUGH"
echo ""

# Confirm before proceeding
read -p "Proceed with these settings? [Y/n]: " confirm
if [[ $confirm =~ ^[Nn]$ ]]; then
    echo "Aborted."
    exit 0
fi

echo ""

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Check for docker-compose.yml if using local network
if [ "$NETWORK_TYPE" = "local" ]; then
    check_docker_compose_file "../docker-compose.yml"
fi

# Build the binary if needed
# echo "Building benchmark binary..."
# cargo build --release --bin benchmark

echo ""

# Run the benchmark with all parameters
echo "Starting comprehensive benchmark with specified parameters..."
echo "Signal handling: You can press Ctrl+C to gracefully stop the benchmark"
echo ""

# Add network loads based on network type
if [ "$NETWORK_TYPE" = "local" ]; then
    cargo run --release --bin benchmark -- \
      --output-dir "$OUTPUT_DIR" \
      $CONSOLE_OUTPUT_FLAG \
      $FILE_OUTPUT_FLAG \
      --committee "$COMMITTEE" \
      --faults "$FAULTS" \
      $CRASH_RECOVERY_FLAG \
      --crash-interval "$CRASH_INTERVAL" \
      --startup-wait  "$STARTUP_WAIT" \
      --duration "$DURATION" \
      --network-type "$NETWORK_TYPE" \
      --local-loads "$NETWORK_LOADS" \
      --transaction-size "$TRANSACTION_SIZE" \
      $CLEANUP_FLAG \
      $CLEANUP_THOROUGH_FLAG &
else
    cargo run --release --bin benchmark -- \
      --output-dir "$OUTPUT_DIR" \
      $CONSOLE_OUTPUT_FLAG \
      $FILE_OUTPUT_FLAG \
      --committee "$COMMITTEE" \
      --faults "$FAULTS" \
      $CRASH_RECOVERY_FLAG \
      --crash-interval "$CRASH_INTERVAL" \
      --startup-wait  "$STARTUP_WAIT" \
      --duration "$DURATION" \
      --network-type "$NETWORK_TYPE" \
      --remote-loads "$NETWORK_LOADS" \
      --transaction-size "$TRANSACTION_SIZE" \
      $CLEANUP_FLAG \
      $CLEANUP_THOROUGH_FLAG &
fi

# Store the benchmark process PID
BENCHMARK_PID=$!
echo "Benchmark process started with PID: $BENCHMARK_PID"

# Wait for the benchmark to complete
wait $BENCHMARK_PID
BENCHMARK_EXIT_CODE=$?

# Clear the PID since the process has finished
BENCHMARK_PID=""

# Check exit code
if [ $BENCHMARK_EXIT_CODE -eq 0 ]; then
    echo ""
    echo "=== Benchmark completed successfully ==="
elif [ $BENCHMARK_EXIT_CODE -eq 130 ]; then
    echo ""
    echo "=== Benchmark was interrupted but completed gracefully ==="
else
    echo ""
    echo "=== Benchmark exited with code: $BENCHMARK_EXIT_CODE ==="
fi

echo "Results saved to: $OUTPUT_DIR"
echo "Check the output directory for detailed benchmark results and summaries." 