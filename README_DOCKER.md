# Mysticeti Docker Cluster

This directory contains Docker configuration for running Mysticeti validator nodes in containers.

## 📁 Structure

```
mysticeti/
├── Dockerfile              # Docker image for Mysticeti validator
├── docker-compose.yml      # 4-node cluster configuration
├── Makefile               # Management commands
├── data/                  # Persistent data storage (created automatically)
│   ├── node0/
│   ├── node1/
│   ├── node2/
│   └── node3/
└── README_DOCKER.md       # This file
```

## 🚀 Quick Start

### 1. Build the Docker Image

```bash
make build
```

### 2. Start the 4-Node Cluster

```bash
make start
```

### 3. Check Status

```bash
make status
```

### 4. View Logs

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

## 🌐 Network Endpoints

### RPC Endpoints (HTTP)

| Node | Port | Endpoint |
|------|------|----------|
| Node 0 | 26657 | <http://localhost:26657> |
| Node 1 | 26658 | <http://localhost:26658> |
| Node 2 | 26659 | <http://localhost:26659> |
| Node 3 | 26660 | <http://localhost:26660> |

### ABCI Ports

| Node | Port |
|------|------|
| Node 0 | 26670 |
| Node 1 | 26671 |
| Node 2 | 26672 |
| Node 3 | 26673 |

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

- **mysticeti-node0**: Authority index 0, RPC port 26657, ABCI port 26670
- **mysticeti-node1**: Authority index 1, RPC port 26658, ABCI port 26671
- **mysticeti-node2**: Authority index 2, RPC port 26659, ABCI port 26672
- **mysticeti-node3**: Authority index 3, RPC port 26660, ABCI port 26673

## 🔍 Troubleshooting

### Check Container Status

```bash
docker compose ps
```

### View Container Logs

```bash
docker compose logs mysticeti-node0
```

### Access Container Shell

```bash
docker compose exec mysticeti-node0 /bin/bash
```

### Restart Specific Node

```bash
docker compose restart mysticeti-node0
```

### Clean Start

```bash
make clean
make build
make start
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

## 🔐 Security Notes

- Each node runs as a non-root user (`mysticeti`)
- Data is persisted in `./data/` directory
- Network is isolated using Docker bridge network
- Ports are exposed only to localhost by default

## 🛠️ Development

### Build Single Validator Binary Locally

```bash
cd bin
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

## 📚 Additional Resources

- [Mysticeti Documentation](https://github.com/MystenLabs/mysticeti)
- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [Tendermint ABCI](https://docs.tendermint.com/v0.34/spec/abci/)
