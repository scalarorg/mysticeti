# Mysticeti Docker Cluster

This document provides comprehensive guidance for running Mysticeti validator nodes in Docker containers, including cluster management, network configuration, and troubleshooting.

## 📁 Project Structure

```
mysticeti/
├── Dockerfile              # Docker image for Mysticeti validator
├── docker-compose.yml      # 4-node cluster configuration
├── Makefile               # Management commands
├── env.example            # Environment variables template
├── docker-network-debug.sh # Network debugging script
├── data/                  # Persistent data storage (created automatically)
│   ├── node0/
│   ├── node1/
│   ├── node2/
│   └── node3/
└── README.md             # This file
```

## 🚀 Quick Start

### 1. Environment Setup

Copy and configure the environment file:

```bash
cp env.example .env
# Edit .env file to customize network settings (optional)
nano .env
```

### 2. Build the Docker Image

```bash
make build
```

### 3. Start the 4-Node Cluster

```bash
make start
```

### 4. Check Status

```bash
make status
```

### 5. View Logs

```bash
make logs
```

## 📋 Available Commands

### Cluster Management

```bash
make build      # Build Docker image
make start      # Start 4-node cluster
make stop       # Stop cluster
make restart    # Restart cluster
make clean      # Stop and remove everything
```

### Monitoring

```bash
make status     # Show cluster status and endpoints
make logs       # Show logs from all nodes
make logs-node0 # Show logs from node 0
make logs-node1 # Show logs from node 1
make logs-node2 # Show logs from node 2
make logs-node3 # Show logs from node 3
```

### Testing

```bash
make single     # Start single node (node0) for testing
make health     # Check health of all nodes
make test-tx    # Test transaction submission
```

## 🌐 Network Configuration

### Docker Network Architecture

The Mysticeti cluster uses a custom Docker bridge network with static IP addresses to ensure reliable peer connectivity:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  mysticeti-     │    │  mysticeti-     │    │  mysticeti-     │    │  mysticeti-     │
│  node0          │    │  node1          │    │  node2          │    │  node3          │
│  172.20.0.10    │    │  172.20.0.11    │    │  172.20.0.12    │    │  172.20.0.13    │
│  Port: 26657    │    │  Port: 26657    │    │  Port: 26657    │    │  Port: 26657    │
└─────────────────┘    └─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │                       │
         └───────────────────────┼───────────────────────┼───────────────────────┘
                                 │                       │
                    Docker Bridge Network (172.20.0.0/16)
```

### Network Endpoints

#### RPC Endpoints (HTTP)

| Node | Internal IP | Host Port | Endpoint |
|------|-------------|-----------|----------|
| Node 0 | 172.20.0.10 | 26657 | <http://localhost:26657> |
| Node 1 | 172.20.0.11 | 26658 | <http://localhost:26658> |
| Node 2 | 172.20.0.12 | 26659 | <http://localhost:26659> |
| Node 3 | 172.20.0.13 | 26660 | <http://localhost:26660> |

### Environment Variables

The configuration uses environment variables for flexibility and easy customization:

#### Network Configuration

- `NETWORK_SUBNET`: Docker network subnet (default: `172.20.0.0/16`)
- `NETWORK_GATEWAY`: Network gateway (default: `172.20.0.1`)

#### Node IP Addresses

- `NODE0_IP`: IP address for node 0 (default: `172.20.0.10`)
- `NODE1_IP`: IP address for node 1 (default: `172.20.0.11`)
- `NODE2_IP`: IP address for node 2 (default: `172.20.0.12`)
- `NODE3_IP`: IP address for node 3 (default: `172.20.0.13`)

#### Peer Addresses

- `PEER_ADDRESSES_NODE0`: Peer addresses for node 0
- `PEER_ADDRESSES_NODE1`: Peer addresses for node 1
- `PEER_ADDRESSES_NODE2`: Peer addresses for node 2
- `PEER_ADDRESSES_NODE3`: Peer addresses for node 3

### Custom Network Configuration

You can customize the network by setting environment variables:

```bash
# Custom network subnet
export NETWORK_SUBNET=192.168.1.0/24

# Custom node IPs
export NODE0_IP=192.168.1.10
export NODE1_IP=192.168.1.11
export NODE2_IP=192.168.1.12
export NODE3_IP=192.168.1.13

# Custom peer addresses
export PEER_ADDRESSES_NODE0=192.168.1.11:26657,192.168.1.12:26657,192.168.1.13:26657

# Start with custom configuration
docker-compose up -d
```

## 🔧 API Endpoints

### Health Check

```bash
curl http://localhost:26657/health
```

### Transaction Submission

```bash
curl -X POST http://localhost:26657/broadcast_tx_async \
  -H "Content-Type: application/json" \
  -d '{"transaction": "dGVzdCB0cmFuc2FjdGlvbg=="}'
```

### Status

```bash
curl http://localhost:26657/status
```

## 🐳 Docker Compose Services

The cluster consists of 4 validator nodes:

- **mysticeti-node0**: Authority index 0, RPC port 26657
- **mysticeti-node1**: Authority index 1, RPC port 26658
- **mysticeti-node2**: Authority index 2, RPC port 26659
- **mysticeti-node3**: Authority index 3, RPC port 26660

## 🔍 Troubleshooting

### Network Debugging

Use the provided debug script to diagnose network issues:

```bash
./docker-network-debug.sh
```

This script will:

1. Check container status
2. Verify network configuration
3. Test inter-container connectivity
4. Check Mysticeti logs for connection errors

### Common Issues and Solutions

#### 1. Containers Not Starting

**Symptoms**: Containers fail to start or exit immediately

**Solutions**:

```bash
# Check container status
docker compose ps

# Check Docker logs
docker compose logs mysticeti-node0

# Ensure ports are available
netstat -tulpn | grep -E ":(26657|26658|26659|26660|26670|26671|26672|26673)"
```

#### 2. Network Connectivity Issues

**Symptoms**: Nodes can't communicate with each other

**Solutions**:

```bash
# Check if containers can reach each other
docker exec mysticeti-node0 ping -c 1 172.20.0.11

# Verify network configuration
docker network inspect mysticeti_mysticeti-network

# Check container IPs
docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' mysticeti-node0
```

#### 3. Consensus Not Reaching

**Symptoms**: Network doesn't reach consensus, transactions not processed

**Solutions**:

```bash
# Check if all nodes are running
docker ps --filter "name=mysticeti-node"

# Check Mysticeti logs for connection errors
docker logs mysticeti-node0 | grep -E "(Error|WARN|disconnected)"

# Restart specific node
docker compose restart mysticeti-node0
```

### Manual Testing Commands

```bash
# Check container status
docker compose ps

# View container logs
docker compose logs mysticeti-node0

# Access container shell
docker compose exec mysticeti-node0 /bin/bash

# Restart specific node
docker compose restart mysticeti-node0

# Clean start
make clean
make build
make start
```

### Debug Commands

```bash
# Check container status
docker ps --filter "name=mysticeti-node"

# Check network configuration
docker network inspect mysticeti_mysticeti-network

# Check container IPs
docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' mysticeti-node0

# Test connectivity between containers
docker exec mysticeti-node0 ping -c 1 172.20.0.11

# Check logs for errors
docker logs mysticeti-node0 --tail 50
```

## 📊 Monitoring

### Check Node Health

```bash
make health
```

### Monitor Logs in Real-time

```bash
make logs
```

### Check Resource Usage

```bash
docker stats
```

### Expected Behavior

After starting the cluster:

1. **Initial Connection**: Nodes may show connection warnings during startup (this is normal)
2. **Peer Discovery**: Nodes should discover each other within 30-60 seconds
3. **Consensus**: The network should reach consensus once all nodes are connected
4. **Transaction Processing**: Transactions should be processed normally

## 🔐 Security Notes

- Each node runs as a non-root user (`mysticeti`)
- Data is persisted in `./data/` directory
- Network is isolated using Docker bridge network
- Ports are exposed only to localhost by default
- Cryptographic keys are generated per container
- Network isolation prevents unauthorized access

## 🛠️ Development

### Build Single Validator Binary Locally

```bash
cargo build --release --bin single-validator
```

### Run Single Validator Locally

```bash
./target/release/single-validator --authority-index 0 --rpc-port 26657
```

### Custom Configuration

You can modify the `docker-compose.yml` file to:

- Change port mappings
- Add environment variables
- Modify volume mounts
- Adjust resource limits

## 📝 Logs

Logs are available at different levels:

- **Container logs**: `docker compose logs`
- **Application logs**: Inside containers at `/app/data/`
- **System logs**: `docker system logs`

## 🔄 Updates

To update the cluster:

1. Stop the cluster: `make stop`
2. Rebuild the image: `make build`
3. Start the cluster: `make start`

## ⚡ Performance Considerations

- **Network Latency**: Docker bridge networks add minimal latency
- **Bandwidth**: Network bandwidth is limited by host system
- **Scalability**: This configuration supports up to 254 nodes in the subnet
- **Resource Usage**: Monitor CPU and memory usage with `docker stats`

## 🔧 Technical Details

### Network Configuration Changes

The Docker configuration addresses previous connectivity issues by:

1. **Static IP Addresses**: Each container gets a fixed IP in the `172.20.0.0/16` subnet
2. **Environment Variables**: All network configuration uses environment variables for flexibility
3. **Peer Address Configuration**: Each node knows the addresses of all other peers
4. **Committee Configuration**: Added `docker_committee_and_keys()` function for Docker network addresses

### Configuration Files Updated

1. **docker-compose.yml**: Added environment variables for all network settings
2. **orchestrator/examples/docker-compose.yml**: Updated with same variable approach
3. **config/src/test_committee.rs**: Added `docker_committee_and_keys()` function
4. **execute/src/bin/validator.rs**: Added `--peer-addresses` argument
5. **orchestrator/src/bin/local_network.rs**: Improved error handling

### New Files Added

1. **env.example**: Environment variables template
2. **docker-network-debug.sh**: Network debugging script
3. **README.md**: This comprehensive documentation

### Benefits of Environment Variables

1. **Flexibility**: Easy to customize network configuration without editing docker-compose files
2. **Maintainability**: Single source of truth for network settings
3. **Deployment**: Different configurations for different environments
4. **Testing**: Easy to test with different network topologies
5. **Documentation**: Clear separation between configuration and code

## 📚 Additional Resources

- [Mysticeti Documentation](https://github.com/MystenLabs/mysticeti)
- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [Tendermint ABCI](https://docs.tendermint.com/v0.34/spec/abci/)
- [Docker Networking](https://docs.docker.com/network/)

## 🆘 Getting Help

If you encounter issues:

1. Use the debug script: `./docker-network-debug.sh`
2. Check the troubleshooting section above
3. Review container logs: `docker compose logs`
4. Verify network connectivity between containers
5. Ensure all environment variables are properly set

For additional support, refer to the Mysticeti documentation or submit an issue to the project repository.
