# Validator Network

This directory contains a new binary that starts 4 validator nodes, each with:

- A Tendermint-style RPC server for receiving transactions
- A Mysticeti consensus process for Byzantine Fault Tolerant consensus

## Architecture

The validator network consists of:

1. **4 Validator Nodes**: Each running on different ports (26657-26660)
2. **RPC Server**: HTTP server that accepts transactions via `/broadcast_tx_async` endpoint
3. **Consensus Engine**: Mysticeti consensus process that handles BFT consensus
4. **Transaction Flow**: RPC → Consensus → Agreement

## Building

```bash
cd mysticeti/bin
cargo build --release
```

## Running the Validator Network

### Start the Network

```bash
# Start all 4 validator nodes
cargo run --release --bin validator-network

# Or with custom working directory
cargo run --release --bin validator-network -- --working-directory ./my-network
```

This will start:

- Node 0: RPC on <http://127.0.0.1:26657>
- Node 1: RPC on <http://127.0.0.1:26658>  
- Node 2: RPC on <http://127.0.0.1:26659>
- Node 3: RPC on <http://127.0.0.1:26660>

### Test the Network

In a separate terminal, you can test the network:

```bash
# Check health of all nodes
cargo run --release --bin test-client check-health

# Send test transactions to all nodes
cargo run --release --bin test-client send-transactions
```

## API Endpoints

Each validator node exposes the following endpoints:

### Broadcast Transaction

```
POST /broadcast_tx_async
Content-Type: application/json

{
  "transaction": "base64_encoded_transaction_data"
}
```

Response:

```json
{
  "success": true,
  "message": "Transaction accepted"
}
```

### Health Check

```
GET /health
```

Response:

```
OK
```

## Transaction Flow

1. **Client sends transaction** to any validator node's RPC endpoint
2. **RPC server receives** the transaction and forwards it to the consensus engine
3. **Mysticeti consensus** processes the transaction through BFT consensus
4. **Agreement reached** when 2f+1 validators agree (where f is the number of faulty nodes)
5. **Transaction committed** to the blockchain

## Configuration

The validator network uses default Mysticeti parameters. You can modify:

- Working directory for data storage
- RPC ports (hardcoded in `validator_network.rs`)
- Consensus parameters (in `validator_node.rs`)

## Development

### Project Structure

```
src/
├── main.rs              # Original mysticeti binary
├── validator_main.rs    # New validator network binary
├── validator_node.rs    # Individual validator node implementation
├── validator_network.rs # Network management
├── test_main.rs         # Test client binary
└── test_client.rs       # Test client implementation
```

### Adding Features

To extend the validator network:

1. **Add new RPC endpoints** in `validator_node.rs::start_rpc_server()`
2. **Modify transaction processing** in `validator_node.rs::start_transaction_processing()`
3. **Add new consensus features** by extending the Mysticeti integration
4. **Add monitoring/metrics** using the existing Prometheus integration

### Integration with Tendermint-rs

Currently, this implementation provides a Tendermint-compatible RPC interface. To integrate more deeply with tendermint-rs:

1. Add `tendermint-rpc` as a dependency
2. Implement proper Tendermint RPC methods
3. Add ABCI application interface
4. Support Tendermint's event system

## Troubleshooting

### Common Issues

1. **Port already in use**: Change RPC ports in `validator_network.rs`
2. **Permission denied**: Ensure write permissions for working directory
3. **Consensus not starting**: Check logs for network connectivity issues

### Logs

The validator network uses structured logging. Key log levels:

- `INFO`: Normal operation
- `WARN`: Non-critical issues
- `ERROR`: Critical errors

### Monitoring

Each node exposes Prometheus metrics. You can scrape them for monitoring and alerting.

## Future Enhancements

1. **Full Tendermint RPC**: Implement complete Tendermint RPC API
2. **ABCI Integration**: Add ABCI application interface
3. **P2P Networking**: Use Tendermint's P2P networking
4. **State Management**: Add persistent state management
5. **Configuration**: Add configuration file support
6. **Docker Support**: Add containerization
