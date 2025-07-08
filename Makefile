# Mysticeti Docker Cluster Management

.PHONY: help build start stop restart logs clean status

# Default target
help:
	@echo "Mysticeti Docker Cluster Management"
	@echo ""
	@echo "Available commands:"
	@echo "  make build     - Build the Docker image"
	@echo "  make start     - Start the 4-node cluster"
	@echo "  make stop      - Stop the cluster"
	@echo "  make restart   - Restart the cluster"
	@echo "  make logs      - Show logs from all nodes"
	@echo "  make status    - Show status of all containers"
	@echo "  make clean     - Stop cluster and remove containers/volumes"
	@echo "  make single    - Start a single validator node (node0)"

# Build the Docker image
build:
	@echo "Building Mysticeti Docker image..."
	docker compose build

# Start the 4-node cluster
start:
	@echo "Starting Mysticeti 4-node cluster..."
	mkdir -p data/node0 data/node1 data/node2 data/node3
	docker compose up -d

# Stop the cluster
stop:
	@echo "Stopping Mysticeti cluster..."
	docker compose down

# Restart the cluster
restart: stop start

# Show logs from all nodes
logs:
	docker compose logs -f

# Show logs from a specific node
logs-node0:
	docker compose logs -f mysticeti-node0

logs-node1:
	docker compose logs -f mysticeti-node1

logs-node2:
	docker compose logs -f mysticeti-node2

logs-node3:
	docker compose logs -f mysticeti-node3

# Show status of all containers
status:
	@echo "Mysticeti Cluster Status:"
	@echo "========================"
	docker compose ps
	@echo ""
	@echo "RPC Endpoints:"
	@echo "  Node 0: http://localhost:26657"
	@echo "  Node 1: http://localhost:26658"
	@echo "  Node 2: http://localhost:26659"
	@echo "  Node 3: http://localhost:26660"
	@echo ""
	@echo "ABCI Ports:"
	@echo "  Node 0: 26670"
	@echo "  Node 1: 26671"
	@echo "  Node 2: 26672"
	@echo "  Node 3: 26673"

# Start a single validator node (for testing)
single:
	@echo "Starting single Mysticeti validator node..."
	mkdir -p data/node0
	docker compose up -d mysticeti-node0

# Clean up everything
clean:
	@echo "Cleaning up Mysticeti cluster..."
	docker compose down -v
	rm -rf data/

# Health check
health:
	@echo "Checking cluster health..."
	@for port in 26657 26658 26659 26660; do \
		echo "Checking node on port $$port..."; \
		curl -s http://localhost:$$port/health || echo "Node on port $$port is not responding"; \
	done

# Test transaction submission
test-tx:
	@echo "Testing transaction submission to node 0..."
	curl -X POST http://localhost:26657/broadcast_tx_async \
		-H "Content-Type: application/json" \
		-d '{"transaction": "dGVzdCB0cmFuc2FjdGlvbg=="}' 