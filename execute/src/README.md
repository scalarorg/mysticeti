# Mysticeti Bin Source Structure

This directory contains the source code for Mysticeti binaries, organized into a clean folder structure.

## 📁 Folder Structure

```
src/
├── lib.rs                    # Library root - exposes all modules
├── bin/                      # Main binary executables
│   ├── main.rs              # Main binary with multiple operations
│   ├── validator_main.rs    # Validator network binary
│   └── single_validator_main.rs # Single validator node binary
├── lib/                      # Support library modules
│   ├── abci_app.rs          # ABCI application implementation
│   ├── validator_node.rs    # Individual validator node logic
│   └── validator_network.rs # Network management for multiple nodes
└── tests/                    # Testing files
    ├── test_main.rs         # Test client main
    └── test_client.rs       # Test client implementation
```

## 🏗️ Architecture

### **bin/** - Main Executables

- **main.rs**: Multi-purpose binary with subcommands for different operations
- **validator_main.rs**: Starts a 4-node validator network
- **single_validator_main.rs**: Starts a single validator node

### **lib/** - Support Modules

- **abci_app.rs**: Tendermint ABCI application implementation
- **validator_node.rs**: Core validator node functionality
- **validator_network.rs**: Network orchestration for multiple nodes

### **tests/** - Testing Infrastructure

- **test_main.rs**: Test client entry point
- **test_client.rs**: Test client implementation for interacting with nodes

## 🔧 Usage

### Build all binaries

```bash
cargo build --release
```

### Run specific binaries

```bash
# Main binary with subcommands
cargo run --bin bin start-four-nodes
cargo run --bin bin start-single-node --authority-index 0

# Validator network
cargo run --bin validator-network

# Single validator
cargo run --bin single-validator --authority-index 0

# Test client
cargo run --bin test-client
```

## 📋 Module Dependencies

```
bin/ (executables)
├── lib.rs (library root)
    ├── lib/abci_app.rs
    ├── lib/validator_node.rs
    └── lib/validator_network.rs
```

All binaries use the shared library modules through `crate::` imports.
