# Mysticeti Benchmark Scripts

This directory contains comprehensive benchmark scripts for running Mysticeti performance tests with support for both local and remote networks.

## Overview

The comprehensive benchmark runner provides:

- **Dual Network Support**: Run benchmarks on both local and remote networks
- **Console Output**: Real-time benchmark results displayed in the terminal
- **File Output**: Detailed results saved to JSON and human-readable text files
- **Comparison Analysis**: Automatic comparison between local and remote network performance
- **Flexible Configuration**: Customizable parameters for different network types

## Local vs Remote Network Comparison

### Architecture Differences

| Aspect | Remote Network | Local Network |
|--------|---------------|---------------|
| **Infrastructure** | AWS/Vultr instances | Docker containers |
| **Connection** | SSH to remote hosts | Local Docker commands |
| **Network** | Real network latency | Local network (minimal latency) |
| **Setup** | Cloud provider setup | Docker Compose |
| **Scaling** | Limited by cloud instances | Limited by local resources |

### Implementation Differences

#### Remote Network Approach

- Uses `Orchestrator` with SSH connections
- Requires cloud provider credentials
- Manages remote instances via SSH
- Collects metrics from remote Prometheus endpoints

#### Local Network Approach

- Uses `LocalNetworkOrchestrator` with Docker commands
- Uses local Docker Compose setup
- Manages containers via Docker CLI
- Simulates transactions to local endpoints

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

## Local Network System

### Docker Setup

The local network uses a `docker-compose.yml` file that defines 4 Mysticeti validator nodes:

```yaml
services:
  mysticeti-node0:
    build: .
    ports:
      - "26657:26657"  # RPC port
      - "26670:26670"  # ABCI port
    environment:
      - NODE_INDEX=0
      - PEER_ADDRESSES=172.20.0.11:26657,172.20.0.12:26657,172.20.0.13:26657
    networks:
      mysticeti-network:
        ipv4_address: 172.20.0.10

  # ... similar for nodes 1-3
```

### Container Management

The `LocalNetworkOrchestrator` provides several methods for managing containers:

#### Container Status

```rust
// Check if container is running
orchestrator.is_container_running("mysticeti-node0")?;

// Get network status
orchestrator.get_network_status()?;
```

#### Container Logs

```rust
// Get container logs for debugging
let logs = orchestrator.get_container_logs("mysticeti-node0")?;
```

#### Network Operations

```rust
// Start network
orchestrator.start_network()?;

// Stop network
orchestrator.stop_network()?;

// Thorough cleanup (removes volumes and containers)
orchestrator.stop_network_thorough()?;
```

### Transaction Simulation

The local benchmark simulates transactions by:

1. **Round-robin distribution**: Sends transactions to different nodes in rotation
2. **Rate limiting**: Controls transaction rate using delays
3. **HTTP requests**: Uses JSON-RPC endpoints exposed by containers
4. **Base64 encoding**: Encodes transaction data for transmission

```rust
// Example transaction payload
let payload = json!({
    "jsonrpc": "2.0",
    "id": i,
    "method": "broadcast_tx_async",
    "params": {
        "tx": base64::encode(&tx_data)
    }
});
```

### How Local Network Works

#### 1. Network Startup

```rust
// Start Docker containers
orchestrator.start_network()?;

// Wait for network to be ready
orchestrator.wait_for_network_ready(startup_wait).await?;
```

#### 2. Transaction Simulation

```rust
// Simulate transactions to local endpoints
orchestrator.simulate_transactions(
    total_transactions,
    transaction_size,
    load,
).await?;
```

#### 3. Metrics Collection

```rust
// Collect metrics from containers
orchestrator.collect_metrics().await?;

// Create measurement collection
let measurements = MeasurementsCollection::new(&settings, parameters);
```

#### 4. Results Processing

```rust
// Create benchmark result
let result = BenchmarkResult::new(NetworkType::Local, parameters, measurements);

// Save results
result.save_to_file(&output_dir)?;
```

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

# Local network with thorough cleanup (removes volumes and containers completely)
cargo run --bin benchmark -- \
  --network-type local \
  --local-loads 100,500,1000 \
  --duration 300 \
  --transaction-size 1024 \
  --cleanup-thorough
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
- `--cleanup-thorough`: Whether to perform thorough cleanup (remove volumes and containers completely) (default: `false`)

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

### Local vs Remote Performance

| Metric | Local Network | Remote Network |
|--------|---------------|----------------|
| **Latency** | ~1-5ms | ~50-200ms |
| **Throughput** | Higher (no network overhead) | Lower (network overhead) |
| **Resource Usage** | Limited by local machine | Limited by cloud instances |
| **Cost** | Free (local resources) | Cloud provider costs |

### Optimization Tips

1. **Resource allocation**: Ensure sufficient CPU/memory for containers
2. **Network configuration**: Use bridge networking for container communication
3. **Storage**: Use volume mounts for persistent data
4. **Monitoring**: Enable container metrics collection

## Troubleshooting

### Common Issues

1. **Docker not running**: Ensure Docker is running for local benchmarks
2. **Missing environment variables**: Set required host variables for remote benchmarks
3. **SSH connection issues**: Verify SSH keys and connectivity for remote servers
4. **Build failures**: Check that all dependencies are installed

### Local Network Specific Issues

1. **Docker not running**

   ```
   Error: Docker is not running. Please start Docker and try again.
   ```

2. **docker-compose.yml not found**

   ```
   Error: docker-compose.yml not found at ../docker-compose.yml
   ```

3. **Containers not starting**

   ```bash
   # Check container status
   docker ps
   
   # Check container logs
   docker logs mysticeti-node0
   ```

4. **Network connectivity issues**

   ```bash
   # Test node endpoints
   curl http://localhost:26657/health
   ```

### Debugging

Use the enhanced container management methods:

```rust
// Check network status
orchestrator.get_network_status()?;

// Get container logs
let logs = orchestrator.get_container_logs("mysticeti-node0")?;
println!("Node 0 logs: {}", logs);
```

### Debug Mode

Enable verbose logging by setting the `RUST_LOG` environment variable:

```bash
RUST_LOG=debug cargo run --bin benchmark -- --network-type local
```

## Future Enhancements

### Planned Improvements

1. **Real metrics collection**: Collect actual Prometheus metrics from containers
2. **Container monitoring**: Real-time container health monitoring
3. **Load generation**: More sophisticated load generation patterns
4. **Fault injection**: Simulate node failures and recovery
5. **Network simulation**: Add network latency and packet loss simulation

### Integration with Remote System

The local system can be enhanced to:

- Use the same metrics collection as remote systems
- Support the same benchmark parameters
- Provide comparable results for validation

## Conclusion

The local network benchmark system provides a convenient way to test and benchmark Mysticeti without requiring cloud infrastructure. While it has different characteristics than remote networks, it serves as an excellent development and testing tool.

For production benchmarking, use the remote network system for more realistic results that include network latency and cloud infrastructure characteristics.
