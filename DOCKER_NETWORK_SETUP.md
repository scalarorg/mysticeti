# Docker Network Configuration for Mysticeti

This document explains the Docker network configuration changes made to fix peer connectivity issues in the Mysticeti consensus network.

## Problem

The original configuration had network connectivity issues where nodes couldn't communicate with each other because:

1. **Address Mismatch**: The committee configuration used `127.0.0.1` addresses, but Docker containers need to use the Docker network IP addresses
2. **Container Isolation**: Each container has its own network namespace, so `127.0.0.1` refers to the container itself, not other containers
3. **Peer Discovery**: Nodes couldn't discover each other due to incorrect network addresses

## Solution

### 1. Docker Compose Configuration

Updated `docker-compose.yml` with:

- **Static IP Addresses**: Each container gets a fixed IP in the `172.20.0.0/16` subnet
- **Environment Variables**: All network configuration uses environment variables for flexibility
- **Peer Address Configuration**: Each node knows the addresses of all other peers
- **Network Gateway**: Added gateway configuration for proper routing

```yaml
networks:
  mysticeti-network:
    driver: bridge
    ipam:
      config:
        - subnet: ${NETWORK_SUBNET:-172.20.0.0/16}
          gateway: ${NETWORK_GATEWAY:-172.20.0.1}
```

### 2. Container IP Assignment

- **mysticeti-node0**: `${NODE0_IP:-172.20.0.10}`
- **mysticeti-node1**: `${NODE1_IP:-172.20.0.11}`
- **mysticeti-node2**: `${NODE2_IP:-172.20.0.12}`
- **mysticeti-node3**: `${NODE3_IP:-172.20.0.13}`

### 3. Environment Variables

The configuration now uses environment variables for all network settings:

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

### 4. Committee Configuration

Added `docker_committee_and_keys()` function in `config/src/test_committee.rs` that:

- Uses Docker network IP addresses instead of localhost
- Configures proper peer addresses for each node
- Maintains the same cryptographic keys for consistency

### 5. Command Line Arguments

Updated the validator binary to support:

```bash
--peer-addresses "172.20.0.11:26657,172.20.0.12:26657,172.20.0.13:26657"
```

When this argument is provided, the validator uses Docker network configuration.

## Usage

### Environment Configuration

1. **Copy the example environment file:**

   ```bash
   cp env.example .env
   ```

2. **Customize the configuration (optional):**

   ```bash
   # Edit .env file to customize network settings
   nano .env
   ```

3. **Start the network:**

   ```bash
   docker-compose up -d
   ```

### Starting the Network

```bash
# Build and start all containers
docker-compose up -d

# Check network status
./docker-network-debug.sh
```

### Customizing Network Configuration

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

### Debugging Network Issues

Use the provided debug script:

```bash
./docker-network-debug.sh
```

This script will:

1. Check container status
2. Verify network configuration
3. Test inter-container connectivity
4. Check Mysticeti logs for connection errors

### Manual Testing

```bash
# Check if containers can reach each other
docker exec mysticeti-node0 ping -c 1 172.20.0.11

# Check Mysticeti logs
docker logs mysticeti-node0 | grep -E "(Error|WARN|disconnected)"
```

## Expected Behavior

After the changes:

1. **Initial Connection**: Nodes may still show connection warnings during startup (this is normal)
2. **Peer Discovery**: Nodes should discover each other within 30-60 seconds
3. **Consensus**: The network should reach consensus once all nodes are connected
4. **Transaction Processing**: Transactions should be processed normally

## Troubleshooting

### Common Issues

1. **Containers not starting**: Check Docker logs and ensure ports are available
2. **Network connectivity**: Use the debug script to verify container connectivity
3. **Consensus not reaching**: Check if all nodes are running and can communicate

### Debug Commands

```bash
# Check container status
docker ps --filter "name=mysticeti-node"

# Check network configuration
docker network inspect mysticeti_mysticeti-network

# Check container IPs
docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' mysticeti-node0

# Test connectivity
docker exec mysticeti-node0 ping -c 1 172.20.0.11

# Check logs
docker logs mysticeti-node0 --tail 50
```

## Configuration Files

### Updated Files

1. **docker-compose.yml**: Added environment variables for all network settings
2. **orchestrator/examples/docker-compose.yml**: Updated with same variable approach
3. **config/src/test_committee.rs**: Added `docker_committee_and_keys()` function
4. **execute/src/bin/validator.rs**: Added `--peer-addresses` argument
5. **orchestrator/src/bin/local_network.rs**: Improved error handling

### New Files

1. **env.example**: Environment variables template
2. **docker-network-debug.sh**: Network debugging script
3. **DOCKER_NETWORK_SETUP.md**: This documentation

## Network Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  mysticeti-     │    │  mysticeti-     │    │  mysticeti-     │    │  mysticeti-     │
│  node0          │    │  node1          │    │  node2          │    │  node3          │
│  ${NODE0_IP}    │    │  ${NODE1_IP}    │    │  ${NODE2_IP}    │    │  ${NODE3_IP}    │
│  Port: 26657    │    │  Port: 26657    │    │  Port: 26657    │    │  Port: 26657    │
└─────────────────┘    └─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │                       │
         └───────────────────────┼───────────────────────┼───────────────────────┘
                                 │                       │
                    Docker Bridge Network (${NETWORK_SUBNET})
```

## Benefits of Environment Variables

1. **Flexibility**: Easy to customize network configuration without editing docker-compose files
2. **Maintainability**: Single source of truth for network settings
3. **Deployment**: Different configurations for different environments
4. **Testing**: Easy to test with different network topologies
5. **Documentation**: Clear separation between configuration and code

## Performance Considerations

- **Network Latency**: Docker bridge networks add minimal latency
- **Bandwidth**: Network bandwidth is limited by host system
- **Scalability**: This configuration supports up to 254 nodes in the subnet

## Security Notes

- **Network Isolation**: Containers are isolated in the Docker network
- **Port Exposure**: Only necessary ports are exposed to the host
- **Key Management**: Cryptographic keys are generated per container
