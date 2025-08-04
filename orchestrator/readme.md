# Mysticeti Network Orchestration & Benchmarking

This comprehensive guide covers the Mysticeti network orchestration system, including binaries, scripts, and benchmarking capabilities for both local and remote networks.

## Table of Contents

1. [Overview](#overview)
2. [Network Orchestration Binaries](#network-orchestration-binaries)
3. [Benchmark System](#benchmark-system)
4. [Interactive Scripts](#interactive-scripts)
5. [Implementation Details](#implementation-details)
6. [Usage Examples](#usage-examples)
7. [Prerequisites](#prerequisites)
8. [Troubleshooting](#troubleshooting)
9. [Future Enhancements](#future-enhancements)

## Overview

The orchestrator package provides comprehensive tools for orchestrating Mysticeti networks and running performance benchmarks:

### Network Orchestration
- **`local-network`** - Starts a local docker-compose network with 4 Mysticeti nodes and simulates transactions
- **`remote-network`** - Connects to 4 remote nodes, starts Mysticeti containers, and simulates transactions

### Benchmarking System
- **Dual Network Support**: Run benchmarks on both local and remote networks
- **Console Output**: Real-time benchmark results displayed in the terminal
- **File Output**: Detailed results saved to JSON and human-readable text files
- **Comparison Analysis**: Automatic comparison between local and remote network performance
- **Flexible Configuration**: Customizable parameters for different network types

### Local vs Remote Network Comparison

| Aspect | Remote Network | Local Network |
|--------|---------------|---------------|
| **Infrastructure** | AWS/Vultr instances | Docker containers |
| **Connection** | SSH to remote hosts | Local Docker commands |
| **Network** | Real network latency | Local network (minimal latency) |
| **Setup** | Cloud provider setup | Docker Compose |
| **Scaling** | Limited by cloud instances | Limited by local resources |
| **Latency** | ~50-200ms | ~1-5ms |
| **Throughput** | Lower (network overhead) | Higher (no network overhead) |
| **Cost** | Cloud provider costs | Free (local resources) |

## Network Orchestration Binaries

### Local Network Binary

#### Usage

```bash
# Basic usage with default parameters
cargo run --bin local-network

# Custom parameters
cargo run --bin local-network \
  --docker-compose-path ./docker-compose.yml \
  --num-transactions 2000 \
  --transaction-size 1024 \
  --transaction-rate 200 \
  --startup-wait 45 \
  --cleanup

# With thorough cleanup (removes volumes and containers completely)
cargo run --bin local-network \
  --docker-compose-path ./docker-compose.yml \
  --num-transactions 2000 \
  --transaction-size 1024 \
  --transaction-rate 200 \
  --startup-wait 45 \
  --cleanup-thorough
```

#### Parameters

- `--docker-compose-path`: Path to docker-compose.yml file (default: ./docker-compose.yml)
- `--num-transactions`: Number of transactions to simulate (default: 1000)
- `--transaction-size`: Transaction size in bytes (default: 512)
- `--transaction-rate`: Transaction rate in tx/s (default: 100)
- `--startup-wait`: Wait time for network startup in seconds (default: 30)
- `--cleanup`: Whether to clean up containers after completion (default: false)
- `--cleanup-thorough`: Whether to perform thorough cleanup (remove volumes and containers completely) (default: false)

#### Docker Compose Configuration

The binary uses a docker-compose file with 4 Mysticeti nodes:

- Node 0: RPC port 26657, ABCI port 26670
- Node 1: RPC port 26658, ABCI port 26671  
- Node 2: RPC port 26659, ABCI port 26672
- Node 3: RPC port 26660, ABCI port 26673

### Remote Network Binary

#### Usage

```bash
# Set environment variables for remote nodes
export MYSTICETI_NODE0_HOST="192.168.1.10"
export MYSTICETI_NODE1_HOST="192.168.1.11"
export MYSTICETI_NODE2_HOST="192.168.1.12"
export MYSTICETI_NODE3_HOST="192.168.1.13"

# Optional: Set SSH configuration
export MYSTICETI_NODE0_SSH_PORT="22"
export MYSTICETI_NODE0_SSH_USER="ubuntu"
export MYSTICETI_NODE0_SSH_KEY="~/.ssh/my-key"

# Run the binary
cargo run --bin remote-network

# With custom parameters
cargo run --bin remote-network \
  --num-transactions 5000 \
  --transaction-size 2048 \
  --transaction-rate 500 \
  --startup-wait 120 \
  --cleanup
```

#### Required Environment Variables

For each node (0-3), you must set:

- `MYSTICETI_NODE{n}_HOST`: IP address or hostname of the remote node

Optional environment variables (with defaults):

- `MYSTICETI_NODE{n}_SSH_PORT`: SSH port (default: 22)
- `MYSTICETI_NODE{n}_SSH_USER`: SSH username (default: ubuntu)
- `MYSTICETI_NODE{n}_SSH_KEY`: Path to SSH private key (default: ~/.ssh/id_rsa)
- `SSH_TIMEOUT`: SSH connection timeout in seconds (default: 30)

## Benchmark System

### Features

#### Network Types

- **Local Network**: Benchmarks run on local infrastructure using Docker containers
- **Remote Network**: Benchmarks run on cloud infrastructure (AWS, Vultr, etc.)

#### Real Network Execution

The benchmark runner **actually starts networks and simulates transactions** instead of just printing mock constants:

- **Local Network**: Uses Docker Compose to start Mysticeti nodes locally
- **Remote Network**: Uses SSH to deploy and manage Mysticeti nodes on remote servers
- **Transaction Simulation**: Sends real transactions to the running networks
- **Metrics Collection**: Parses actual throughput, latency, and success rates from network output

#### Output Formats

- **Console**: Real-time progress and results with formatted tables
- **JSON**: Structured data for programmatic analysis
- **Text Summary**: Human-readable summary files

#### Metrics Collected

- Throughput (transactions per second)
- Average latency
- Latency standard deviation
- Input load vs actual throughput
- Network comparison statistics
- Success/failure rates
- Network efficiency

### Benchmark Usage

#### Basic Usage

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

#### Advanced Usage

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

# Local network with thorough cleanup
cargo run --bin benchmark -- \
  --network-type local \
  --local-loads 100,500,1000 \
  --duration 300 \
  --transaction-size 1024 \
  --cleanup-thorough
```

#### Command Line Options

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
- `--cleanup-thorough`: Whether to perform thorough cleanup (remove volumes and containers completely) (default: `false`)

## Interactive Scripts

### Overview

The `examples/` directory contains interactive bash scripts that provide user-friendly parameter configuration for both local and remote network testing.

### Available Scripts

#### 1. `local_network.sh` - Local Network Setup

**Features:**
- Interactive parameter configuration with sensible defaults
- Configuration summary before execution
- Confirmation step to prevent accidental execution
- Comprehensive error handling

**Usage:**
```bash
chmod +x examples/local_network.sh
./examples/local_network.sh
```

#### 2. `remote_network.sh` - Remote Network Setup

**Features:**
- Environment variable validation
- Interactive parameter configuration
- SSH timeout configuration
- Comprehensive error handling

**Usage:**
```bash
# Set environment variables
export MYSTICETI_NODE0_HOST="192.168.1.10"
export MYSTICETI_NODE1_HOST="192.168.1.11"
export MYSTICETI_NODE2_HOST="192.168.1.12"
export MYSTICETI_NODE3_HOST="192.168.1.13"

chmod +x examples/remote_network.sh
./examples/remote_network.sh
```

#### 3. `run_benchmark.sh` - Interactive Benchmark Runner

**Features:**
- Interactive parameter configuration
- Default values for all parameters
- Confirmation before execution
- Automatic binary building
- Comprehensive output

**Usage:**
```bash
./examples/run_benchmark.sh
```

#### 4. `run_benchmark_auto.sh` - Automated Benchmark Runner

**Features:**
- Command line arguments for automation
- CI/CD pipeline support
- Help documentation

**Usage:**
```bash
# Basic usage with defaults
./examples/run_benchmark_auto.sh

# Custom parameters
./examples/run_benchmark_auto.sh --local-committee 8 --duration 300 --local-loads "100,500,1000"

# Show help
./examples/run_benchmark_auto.sh --help
```

### Script Features

#### Input Validation
- Accepts empty input to use defaults
- Validates yes/no responses with retry logic
- Clear error messages for invalid inputs

#### User Experience
- Clear section headers and formatting
- Helpful default values for all parameters
- Configuration summary before execution
- Confirmation step to prevent accidental execution

#### Error Handling
- Checks for required environment variables (remote script)
- Graceful exit on user cancellation
- Clear error messages for missing dependencies

## Implementation Details

### Transaction Simulation

Both binaries implement transaction simulation with:

- **Configurable parameters**: transaction count, size, and rate
- **Round-robin distribution**: distributes load across all nodes
- **Rate limiting**: maintains specified transaction rate
- **Error tracking**: monitors successful and failed transactions
- **Statistics reporting**: provides detailed performance metrics

#### Transaction Format

Transactions are sent as JSON-RPC requests:

```json
{
  "jsonrpc": "2.0",
  "id": <transaction_id>,
  "method": "broadcast_tx_async",
  "params": {
    "tx": "<base64_encoded_transaction_data>"
  }
}
```

### Local Network System

#### Docker Setup

The local network uses a `docker-compose.yml` file that defines 4 Mysticeti validator nodes with proper networking and port configuration.

#### Container Management

The `LocalNetworkOrchestrator` provides methods for:

- **Container Status**: Check if containers are running
- **Container Logs**: Get logs for debugging
- **Network Operations**: Start, stop, and cleanup networks
- **Health Monitoring**: Validate network readiness

#### Transaction Simulation

1. **Round-robin distribution**: Sends transactions to different nodes in rotation
2. **Rate limiting**: Controls transaction rate using delays
3. **HTTP requests**: Uses JSON-RPC endpoints exposed by containers
4. **Base64 encoding**: Encodes transaction data for transmission

### Remote Network System

- `RemoteNode` struct for node configuration
- `RemoteNetworkOrchestrator` for managing remote nodes
- SSH command execution for remote operations
- Docker installation and container management
- Transaction simulation with distributed load

## Usage Examples

### Local Network Examples

#### Quick Local Test

```bash
# Start a local network with 1000 transactions at 100 tx/s
cargo run --bin local-network --cleanup
```

#### Interactive Script Example

```bash
$ ./examples/local_network.sh

=== Mysticeti Local Network Example ===

Configure local network parameters:
==================================
Path to docker-compose.yml file [./docker-compose.yml]: 
Number of transactions to simulate [1000]: 2000
Transaction size in bytes [512]: 1024
Transaction rate (tx/s) [100]: 150
Startup wait time in seconds [30]: 45
Clean up containers after completion [Y/n]: y

Configuration Summary:
=====================
Docker-compose path: ./docker-compose.yml
Number of transactions: 2000
Transaction size: 1024 bytes
Transaction rate: 150 tx/s
Startup wait: 45 seconds
Cleanup: true

Proceed with these settings? [Y/n]: y

Building local-network binary...
Starting local network with specified parameters...
=== Example completed ===
```

### Remote Network Examples

#### Remote Network Test

```bash
# Set up environment variables
export MYSTICETI_NODE0_HOST="10.0.1.10"
export MYSTICETI_NODE1_HOST="10.0.1.11"
export MYSTICETI_NODE2_HOST="10.0.1.12"
export MYSTICETI_NODE3_HOST="10.0.1.13"

# Run remote network test
cargo run --bin remote-network \
  --num-transactions 2000 \
  --transaction-rate 300 \
  --cleanup
```

### Benchmark Examples

#### Console Output

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

#### File Output

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

## Prerequisites

### Local Network

- Docker and docker-compose installed
- Existing `docker-compose.yml` file (specified by `--docker-compose-path`)
- Sufficient disk space for container data
- Ports 26657-26660 and 26670-26673 available

### Remote Network

- SSH access to all remote nodes
- SSH keys configured for passwordless access
- Remote nodes have internet access for Docker image download
- Ports 26657 and 26670-26673 available on remote nodes
- Sudo access on remote nodes (for Docker installation)

## Troubleshooting

### Local Network Issues

1. **Docker not running**: Ensure Docker daemon is started
   ```bash
   sudo systemctl start docker
   ```

2. **Port conflicts**: Check if ports 26657-26660 are already in use
   ```bash
   netstat -tulpn | grep :26657
   ```

3. **Permission issues**: Ensure user has Docker permissions
   ```bash
   sudo usermod -aG docker $USER
   ```

4. **Docker-compose file not found**: Check the path specified by `--docker-compose-path`

5. **Container connectivity issues**
   ```bash
   # Test node endpoints
   curl http://localhost:26657/health
   ```

### Remote Network Issues

1. **SSH connection failures**: Check SSH keys and network connectivity
   ```bash
   ssh -i ~/.ssh/id_rsa user@remote-host
   ```

2. **Docker installation failures**: Check if remote nodes have sudo access
   ```bash
   sudo apt-get update
   ```

3. **Container startup failures**: Check if ports are available on remote nodes
   ```bash
   netstat -tulpn | grep :26657
   ```

4. **Transaction failures**: Check if nodes are properly configured and running

### Script Issues

1. **Permission Denied**: Make sure scripts are executable
   ```bash
   chmod +x examples/*.sh
   ```

2. **Environment Variables Not Set**: For remote network, ensure all required variables are set
   ```bash
   export MYSTICETI_NODE0_HOST="your-node-ip"
   # ... set other variables
   ```

### Debug Mode

Enable verbose logging by setting the `RUST_LOG` environment variable:

```bash
RUST_LOG=debug cargo run --bin local-network
RUST_LOG=debug cargo run --bin benchmark -- --network-type local
```

### Container Debugging

Use enhanced container management methods:

```rust
// Check network status
orchestrator.get_network_status()?;

// Get container logs
let logs = orchestrator.get_container_logs("mysticeti-node0")?;
println!("Node 0 logs: {}", logs);
```

## Building

To build the binaries:

```bash
# Build all binaries
cargo build --release

# Build specific binary
cargo build --release --bin local-network
cargo build --release --bin remote-network
cargo build --release --bin benchmark
```

## Monitoring

Both binaries and benchmark system provide detailed logging of:

- Network startup progress
- Transaction submission status
- Success/failure statistics
- Performance metrics
- Container health status

## Future Enhancements

### Planned Improvements

1. **Real metrics collection**: Collect actual Prometheus metrics from containers
2. **Container monitoring**: Real-time container health monitoring
3. **Load generation**: More sophisticated load generation patterns
4. **Fault injection**: Simulate node failures and recovery
5. **Network simulation**: Add network latency and packet loss simulation
6. **Support for different transaction types**
7. **Integration with monitoring tools**
8. **Support for larger network sizes**
9. **Configuration file support**
10. **Real-time monitoring dashboard**

### Integration Opportunities

- Use the same metrics collection across local and remote systems
- Support the same benchmark parameters for both network types
- Provide comparable results for validation
- Enhanced error handling and recovery mechanisms

## Conclusion

The Mysticeti network orchestration and benchmarking system provides a complete solution for:

- **Network Orchestration**: Deploy and manage Mysticeti networks locally and remotely
- **Performance Benchmarking**: Comprehensive testing with real transaction simulation
- **Interactive Configuration**: User-friendly scripts for easy setup and execution
- **Flexible Architecture**: Support for both local development and production testing
- **Comprehensive Monitoring**: Detailed metrics and logging for performance analysis

The system is well-structured, documented, and ready for production use, with clear paths for future enhancements and improvements. Whether you're developing locally with Docker or testing on cloud infrastructure, the orchestrator provides the tools needed for effective Mysticeti network management and performance evaluation.