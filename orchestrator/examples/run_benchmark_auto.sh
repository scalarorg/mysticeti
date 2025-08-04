#!/bin/bash

# Automated Benchmark Runner Script for Mysticeti
# This script runs the benchmark binary with command line arguments for automation
# Usage: ./run_benchmark_auto.sh [OPTIONS]
# 
# Options:
#   --output-dir DIR          Output directory for results (default: ./benchmarks)
#   --console-output BOOL     Print results to console (default: true)
#   --file-output BOOL        Save results to file (default: true)
#   --committee N             Committee size (default: 4)
#   --faults N               Number of faulty nodes (default: 0)
#   --crash-recovery BOOL    Crash recovery enabled (default: false)
#   --crash-interval N       Crash interval in seconds (default: 60)
#   --duration N             Benchmark duration in seconds (default: 180)
#   --network-type TYPE      Network type to benchmark (local or remote) (default: local)
#   --transaction-size SIZE  Transaction size in bytes (default: 512)
#   --help                   Show this help message

DIR="$( cd "$( dirname "$0" )" && pwd )"
set -e

# Default values
OUTPUT_DIR="./benchmarks"
CONSOLE_OUTPUT="true"
FILE_OUTPUT="true"
COMMITTEE="4"
FAULTS="0"
CRASH_RECOVERY="false"
CRASH_INTERVAL="60"
DURATION="180"
LOCAL_LOADS="100,200,500"
REMOTE_LOADS="50,100,200"
NETWORK_TYPE="local"
TRANSACTION_SIZE="512"

# Function to show help
show_help() {
    cat << EOF
Automated Benchmark Runner Script for Mysticeti

Usage: $0 [OPTIONS]

Options:
  --output-dir DIR          Output directory for results (default: $OUTPUT_DIR)
  --console-output BOOL     Print results to console (default: $CONSOLE_OUTPUT)
  --file-output BOOL        Save results to file (default: $FILE_OUTPUT)
  --committee N             Committee size (default: $COMMITTEE)
  --faults N               Number of faulty nodes (default: $FAULTS)
  --crash-recovery BOOL    Crash recovery enabled (default: $CRASH_RECOVERY)
  --crash-interval N       Crash interval in seconds (default: $CRASH_INTERVAL)
  --duration N             Benchmark duration in seconds (default: $DURATION)
  --network-type TYPE      Network type to benchmark (local or remote) (default: $NETWORK_TYPE)
  --transaction-size SIZE  Transaction size in bytes (default: $TRANSACTION_SIZE)
  --help                   Show this help message

Examples:
  $0 --committee 8 --duration 300
  $0 --network-type local --transaction-size 1024
  $0 --faults 1 --crash-recovery true --crash-interval 30

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --console-output)
            CONSOLE_OUTPUT="$2"
            shift 2
            ;;
        --file-output)
            FILE_OUTPUT="$2"
            shift 2
            ;;
        --committee)
            COMMITTEE="$2"
            if ! [[ "$COMMITTEE" =~ ^[0-9]+$ ]] || [ "$COMMITTEE" -lt 1 ]; then
                echo "Error: --committee must be a positive integer, got '$COMMITTEE'"
                exit 1
            fi
            shift 2
            ;;
        --faults)
            FAULTS="$2"
            if ! [[ "$FAULTS" =~ ^[0-9]+$ ]]; then
                echo "Error: --faults must be a non-negative integer, got '$FAULTS'"
                exit 1
            fi
            shift 2
            ;;
        --crash-recovery)
            CRASH_RECOVERY="$2"
            shift 2
            ;;
        --crash-interval)
            CRASH_INTERVAL="$2"
            shift 2
            ;;
        --duration)
            DURATION="$2"
            shift 2
            ;;
        --local-loads)
            LOCAL_LOADS="$2"
            shift 2
            ;;
        --remote-loads)
            REMOTE_LOADS="$2"
            shift 2
            ;;
        --network-type)
            NETWORK_TYPE="$2"
            if [[ "$NETWORK_TYPE" != "local" && "$NETWORK_TYPE" != "remote" ]]; then
                echo "Error: --network-type must be 'local' or 'remote', got '$NETWORK_TYPE'"
                exit 1
            fi
            shift 2
            ;;
        --transaction-size)
            TRANSACTION_SIZE="$2"
            
            shift 2
            ;;
        --help)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

echo "=== Mysticeti Automated Benchmark Runner ==="
echo "Configuration:"
echo "  Output directory: $OUTPUT_DIR"
echo "  Console output: $CONSOLE_OUTPUT"
echo "  File output: $FILE_OUTPUT"
echo "  Committee size: $COMMITTEE"
echo "  Faults: $FAULTS"
echo "  Crash recovery: $CRASH_RECOVERY"
echo "  Crash interval: $CRASH_INTERVAL seconds"
echo "  Duration: $DURATION seconds"
echo "  Network type: $NETWORK_TYPE"
echo "  Transaction size: $TRANSACTION_SIZE bytes"
echo ""

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

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Build the binary if needed
# echo "Building benchmark binary..."
# cargo build --release --bin benchmark

echo ""

# Run the benchmark with all parameters
echo "Starting automated benchmark with specified parameters..."
cargo run --release --bin benchmark -- \
  --output-dir "$OUTPUT_DIR" \
  $CONSOLE_OUTPUT_FLAG \
  $FILE_OUTPUT_FLAG \
  --committee "$COMMITTEE" \
  --faults "$FAULTS" \
  $CRASH_RECOVERY_FLAG \
  --crash-interval "$CRASH_INTERVAL" \
  --duration "$DURATION" \
  --network-type "$NETWORK_TYPE" \
  --transaction-size "$TRANSACTION_SIZE"

echo ""
echo "=== Automated benchmark completed ==="
echo "Results saved to: $OUTPUT_DIR" 