# Mysticeti Benchmark Scripts

This directory contains comprehensive benchmark scripts for running Mysticeti performance tests with support for both local and remote networks.

## Overview

The comprehensive benchmark runner provides:

- **Dual Network Support**: Run benchmarks on both local and remote networks
- **Console Output**: Real-time benchmark results displayed in the terminal
- **File Output**: Detailed results saved to JSON and human-readable text files
- **Comparison Analysis**: Automatic comparison between local and remote network performance
- **Flexible Configuration**: Customizable parameters for different network types

## Features

### Network Types

- **Local Network**: Benchmarks run on local infrastructure using Docker containers
- **Remote Network**: Benchmarks run on cloud infrastructure (AWS, Vultr, etc.)

### Real Network Execution

The benchmark runner now **actually starts networks and simulates transactions** instead of just printing mock constants:

- **Local Network**: Uses Docker Compose to start Mysticeti nodes locally
- **Remote Network**: Uses SSH to deploy and manage Mysticeti nodes on remote servers
- **Transaction Simulation**: Sends real transactions to the running networks
- **Metrics Collection**: Parses actual throughput, latency, and success rates from network output

### Output Formats

- **Console**: Real-time progress and results with formatted tables
- **JSON**: Structured data for programmatic analysis
- **Text Summary**: Human-readable summary files

### Metrics Collected

- Throughput (transactions per second)
- Average latency
- Latency standard deviation
- Input load vs actual throughput
- Network comparison statistics
- Success/failure rates
- Network efficiency

## Usage

### Basic Usage

```bash
# Local network benchmark with default settings
cargo run --bin benchmark -- --network-type local

# Remote network benchmark with default settings
cargo run --bin benchmark -- --network-type remote

# Custom loads for local network
cargo run --bin benchmark -- --network-type local --local-loads 100,500,1000

# Custom loads for remote network
cargo run --bin benchmark -- --network-type remote --remote-loads 50,100,200
```

### Advanced Usage

```bash
# Local network with custom parameters
cargo run --bin benchmark -- \
  --network-type local \
  --local-loads 100,500,1000 \
  --duration 300 \
  --transaction-size 1024 \
  --cleanup

# Remote network with custom parameters
cargo run --bin benchmark -- \
  --network-type remote \
  --remote-loads 50,100,200 \
  --duration 180 \
  --committee 4 \
  --cleanup
```

### Command Line Options

- `--network-type`: Type of network to benchmark (`local` or `remote`)
- `--output-dir`: Directory to save benchmark results (default: `./benchmarks`)
- `--console-output`: Whether to print results to console (default: `true`)
- `--file-output`: Whether to save results to file (default: `true`)
- `--committee`: Number of nodes in the committee (default: `4`)
- `--duration`: Duration of each benchmark in seconds (default: `180`)
- `--transaction-size`: Size of transactions in bytes (default: `512`)
- `--local-loads`: Comma-separated list of loads for local network (default: `100,200,500`)
- `--remote-loads`: Comma-separated list of loads for remote network (default: `50,100,200`)
- `--docker-compose-path`: Path to docker-compose.yml for local network (default: `./docker-compose.yml`)
- `--startup-wait`: Wait time for network startup in seconds (default: `30`)
- `--cleanup`: Whether to clean up containers after completion (default: `false`)

## Prerequisites

### Local Network Benchmarks

- Docker and Docker Compose installed and running
- `docker-compose.yml` file in the orchestrator directory
- Sufficient system resources to run multiple Mysticeti nodes

### Remote Network Benchmarks

- SSH access to remote servers
- Environment variables set for remote node hosts:
  - `MYSTICETI_NODE0_HOST`
  - `MYSTICETI_NODE1_HOST`
  - `MYSTICETI_NODE2_HOST`
  - `MYSTICETI_NODE3_HOST`
- SSH key configured for passwordless access to remote servers

## Output

### Console Output

The benchmark runner provides real-time feedback:

```
Comprehensive Benchmark Runner
=============================

Configuration:
  Network type: local
  Committee size: 4
  Duration: 180s
  Transaction size: 512 bytes
  ...

Starting local network benchmark with load: 100 tx/s
Building local-network binary...
Executing local network command: ...
Local network output: ...
Successful transactions: 18000
Failed transactions: 0
Actual rate: 95.2 tx/s

BENCHMARK RESULT #1
============================================================
Network Type: local
Input Load: 100 tx/s
Duration: 180s

RESULTS:
  Throughput: 95 tx/s
  Average Latency: 45.2 ms
  Latency Std Dev: 12.1 ms
  Successful Transactions: 18000
  Failed Transactions: 0
  Efficiency: 95.0%
============================================================
```

### File Output

Results are saved as JSON files with detailed metrics:

```json
{
  "network_type": "local",
  "benchmark_number": 1,
  "parameters": {
    "nodes": 4,
    "load": 100,
    "duration": 180,
    "transaction_size": 512,
    "faults": 0,
    "crash_recovery": false,
    "crash_interval": 60
  },
  "results": {
    "throughput": 95,
    "avg_latency_ms": 45.2,
    "latency_std_dev_ms": 12.1,
    "duration_secs": 180,
    "successful_transactions": 18000,
    "failed_transactions": 0
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

## Scripts Overview

### 1. `run_benchmark.sh` - Interactive Benchmark Runner

An interactive script that prompts for all benchmark parameters with sensible defaults.

**Usage:**

```bash
./run_benchmark.sh
```

**Features:**

- Interactive parameter configuration
- Default values for all parameters
- Confirmation before execution
- Automatic binary building
- Comprehensive output

### 2. `run_benchmark_auto.sh` - Automated Benchmark Runner

A script that accepts command line arguments for automation and CI/CD pipelines.

**Usage:**

```bash
# Basic usage with defaults
./run_benchmark_auto.sh

# Custom parameters
./run_benchmark_auto.sh --local-committee 8 --duration 300 --local-loads "100,500,1000"

# Show help
./run_benchmark_auto.sh --help
```

## Implementation Details

### Network Execution

The benchmark runner integrates with the existing `local-network` and `remote-network` binaries:

1. **Binary Building**: Automatically builds the required binaries before execution
2. **Network Startup**: Starts the appropriate network type (local Docker or remote SSH)
3. **Transaction Simulation**: Sends real transactions to the running networks
4. **Metrics Collection**: Parses actual results from network output
5. **Cleanup**: Optionally cleans up resources after benchmarks

### Error Handling

- **Timeout Protection**: Commands have reasonable timeouts to prevent hanging
- **Environment Validation**: Checks for required environment variables
- **Build Verification**: Ensures binaries are built before execution
- **Network Health Checks**: Validates network readiness before testing

### Result Parsing

The benchmark runner parses actual metrics from network output:

- **Throughput**: Extracted from "Actual rate" output
- **Success/Failure Counts**: Parsed from transaction results
- **Duration**: Measured from network execution time
- **Fallback Values**: Used when parsing fails (with appropriate warnings)

## Troubleshooting

### Common Issues

1. **Docker not running**: Ensure Docker is running for local benchmarks
2. **Missing environment variables**: Set required host variables for remote benchmarks
3. **SSH connection issues**: Verify SSH keys and connectivity for remote servers
4. **Build failures**: Check that all dependencies are installed

### Debug Mode

Enable verbose logging by setting the `RUST_LOG` environment variable:

```bash
RUST_LOG=debug cargo run --bin benchmark -- --network-type local
```

## Performance Considerations

- **Local Network**: Limited by local system resources
- **Remote Network**: Limited by network latency and remote server performance
- **Transaction Rate**: Adjust based on network capacity
- **Duration**: Longer durations provide more stable results but take more time
