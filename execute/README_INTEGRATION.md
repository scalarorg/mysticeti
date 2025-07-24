# Mysticeti-CometBFT Integration

This document explains how to integrate Mysticeti consensus with CometBFT while keeping the JSON-RPC and gRPC servers in CometBFT.

## Architecture Overview

The integration creates a hybrid architecture where:

- **CometBFT** handles the RPC layer (JSON-RPC and gRPC servers)
- **Mysticeti** handles the consensus layer
- **ABCI** serves as the bridge between them

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Client Apps   │    │   CometBFT      │    │   Mysticeti     │
│                 │    │                 │    │   Consensus     │
│ JSON-RPC/gRPC   │◄──►│ RPC Servers     │◄──►│                 │
│ Requests        │    │ + Mempool       │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │                       │
                                ▼                       ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │   ABCI App      │    │   Transaction   │
                       │   (Bridge)      │◄──►│   Client        │
                       └─────────────────┘    └─────────────────┘
```

## Key Components

### 1. Enhanced ABCI App (`enhanced_app.rs`)

- Handles transaction validation and finalization
- Forwards transactions to Mysticeti consensus
- Manages transaction status tracking
- Implements all required ABCI methods

### 2. Mysticeti gRPC Server (`grpc_server.rs`)

- Provides gRPC interface for Mysticeti consensus
- Handles transaction submission
- Returns consensus status
- Acts as a frontend for Mysticeti

### 3. Enhanced Validator Node (`enhanced_node.rs`)

- Orchestrates the entire integration
- Manages both CometBFT and Mysticeti components
- Handles communication between components
- Provides unified interface

## Building and Running

### Prerequisites

1. Install Rust and Cargo
2. Install CometBFT
3. Install protobuf compiler

### Build the Project

```bash
cd mysticeti/execute
cargo build --release
```

### Run the Enhanced Validator

```bash
# Basic run with default settings
cargo run --bin enhanced_validator

# With custom parameters
cargo run --bin enhanced_validator -- \
    --authority-index 0 \
    --working-directory ./data \
    --cometbft-rpc-port 26657 \
    --mysticeti-grpc-port 50051 \
    --num-validators 4
```

## Configuration

### CometBFT Configuration

1. Create CometBFT configuration directory:

```bash
mkdir -p ~/.cometbft/config
```

2. Copy the configuration template:

```bash
cp config/cometbft_config.toml ~/.cometbft/config/config.toml
```

3. Update the ABCI address in the configuration:

```toml
# In ~/.cometbft/config/config.toml
proxy_app = "tcp://127.0.0.1:26670"
```

### Port Configuration

The integration uses the following ports:

- **CometBFT RPC**: 26657 (default)
- **Mysticeti gRPC**: 50051 (default)
- **ABCI**: 26670 + authority_index

## Usage Examples

### 1. Starting a Single Node

```bash
cargo run --bin enhanced_validator -- \
    --authority-index 0 \
    --working-directory ./node0 \
    --cometbft-rpc-port 26657 \
    --mysticeti-grpc-port 50051
```

### 2. Starting Multiple Nodes

```bash
# Node 0
cargo run --bin enhanced_validator -- \
    --authority-index 0 \
    --working-directory ./node0 \
    --cometbft-rpc-port 26657 \
    --mysticeti-grpc-port 50051

# Node 1
cargo run --bin enhanced_validator -- \
    --authority-index 1 \
    --working-directory ./node1 \
    --cometbft-rpc-port 26658 \
    --mysticeti-grpc-port 50052
```

### 3. Testing with CometBFT RPC

Once the node is running, you can test the integration:

```bash
# Check node status
curl http://localhost:26657/status

# Broadcast a transaction
curl -X POST http://localhost:26657/broadcast_tx_sync \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"broadcast_tx_sync","params":{"tx":"dGVzdA=="}}'

# Query transaction
curl -X POST http://localhost:26657/abci_query \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"abci_query","params":{"path":"","data":"","height":0,"prove":false}}'
```

### 4. Testing with Mysticeti gRPC

You can also interact directly with the Mysticeti gRPC server:

```bash
# Using grpcurl (install with: go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest)
grpcurl -plaintext -d '{"transaction":"dGVzdA=="}' \
    localhost:50051 mysticeti.grpc.MysticetiService/SubmitTransaction

# Get consensus status
grpcurl -plaintext localhost:50051 mysticeti.grpc.MysticetiService/GetConsensusStatus
```

## Transaction Flow

1. **Client submits transaction** via CometBFT RPC (`/broadcast_tx_sync`)
2. **CometBFT mempool** receives and validates the transaction
3. **ABCI CheckTx** is called for initial validation
4. **ABCI FinalizeBlock** is called when the block is ready
5. **Enhanced ABCI App** forwards transactions to Mysticeti
6. **Mysticeti consensus** processes and commits the transaction
7. **Status updates** are sent back through the ABCI app

## Monitoring and Debugging

### Logs

The integration provides comprehensive logging:

```bash
# Enable debug logging
RUST_LOG=debug cargo run --bin enhanced_validator

# Enable trace logging for detailed debugging
RUST_LOG=trace cargo run --bin enhanced_validator
```

### Metrics

The integration exposes metrics for monitoring:

- Transaction processing rates
- Consensus performance
- Network connectivity
- Error rates

### Health Checks

```bash
# Check CometBFT health
curl http://localhost:26657/health

# Check Mysticeti gRPC health
grpcurl -plaintext localhost:50051 mysticeti.grpc.MysticetiService/GetConsensusStatus
```

## Production Deployment

### 1. Security Considerations

- Use proper key management
- Enable TLS for gRPC connections
- Implement proper authentication
- Use firewall rules to restrict access

### 2. Performance Tuning

- Adjust mempool size based on transaction volume
- Tune consensus timeouts
- Monitor and adjust network parameters
- Use appropriate hardware resources

### 3. High Availability

- Deploy multiple validator nodes
- Use load balancers for RPC endpoints
- Implement proper backup and recovery procedures
- Monitor node health and auto-restart if needed

## Troubleshooting

### Common Issues

1. **Port conflicts**: Ensure ports are not already in use
2. **Permission errors**: Check file permissions for data directories
3. **Network issues**: Verify firewall and network configuration
4. **Consensus failures**: Check validator configuration and connectivity

### Debug Commands

```bash
# Check if ports are listening
netstat -tlnp | grep -E ':(26657|50051|26670)'

# Check process status
ps aux | grep enhanced_validator

# Check logs
tail -f ./node0/logs/validator.log
```

## Development

### Adding New Features

1. **ABCI Methods**: Extend `EnhancedMysticetiAbciApp`
2. **gRPC Services**: Add new methods to `MysticetiGrpcServer`
3. **Configuration**: Update configuration templates
4. **Testing**: Add integration tests

### Testing

```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test integration_tests

# Run with specific test
cargo test test_transaction_flow
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the Apache License 2.0.
