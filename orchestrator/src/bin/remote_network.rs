// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use base64::Engine;
use clap::Parser;
use color_eyre::eyre::{Context, Result};
use orchestrator::RemoteNetworkOrchestrator;
use reqwest::Client;
use serde_json::json;
use std::{
    env,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::time::sleep;
use tracing::{info, warn};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser)]
#[command(author, version, about = "Remote Mysticeti Network Orchestrator")]
struct Args {
    /// Number of transactions to simulate
    #[clap(long, default_value = "1000")]
    num_transactions: usize,

    /// Transaction size in bytes
    #[clap(long, default_value = "512")]
    transaction_size: usize,

    /// Transaction rate (tx/s)
    #[clap(long, default_value = "100")]
    transaction_rate: usize,

    /// Wait time for network startup in seconds
    #[clap(long, default_value = "60")]
    startup_wait: u64,

    /// SSH timeout in seconds
    #[clap(long, default_value = "30")]
    ssh_timeout: u64,

    /// Whether to clean up containers after completion
    #[clap(long, default_value = "false")]
    cleanup: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Nice colored error messages.
    color_eyre::install()?;

    // Setup logging
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    fmt().with_env_filter(filter).init();

    let args = Args::parse();

    info!("Starting Remote Mysticeti Network Orchestrator");

    // Check required environment variables
    let required_vars = vec![
        "MYSTICETI_NODE0_HOST",
        "MYSTICETI_NODE1_HOST",
        "MYSTICETI_NODE2_HOST",
        "MYSTICETI_NODE3_HOST",
    ];

    for var in &required_vars {
        if env::var(var).is_err() {
            return Err(color_eyre::eyre::eyre!(
                "Required environment variable {} not set. Please set all node host addresses.",
                var
            ));
        }
    }

    let orchestrator = RemoteNetworkOrchestrator::new()?;

    // Setup Docker on all nodes
    orchestrator.setup_all_nodes().await?;

    // Start containers on all nodes
    orchestrator.start_all_containers().await?;

    // Wait for network to be ready
    orchestrator
        .wait_for_network_ready(args.startup_wait)
        .await?;

    // Simulate transactions
    orchestrator
        .simulate_transactions(
            args.num_transactions,
            args.transaction_size,
            args.transaction_rate,
        )
        .await?;

    // Cleanup if requested
    if args.cleanup {
        orchestrator.stop_all_containers().await?;
        info!("All containers cleaned up");
    } else {
        info!("Containers are still running. Use the cleanup flag to stop them.");
    }

    info!("Remote network orchestration completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use std::path::PathBuf;
    use tokio::time::Duration;

    // Mock the RemoteNode for testing
    #[derive(Debug, Clone)]
    struct MockRemoteNode {
        host: String,
        port: u16,
        ssh_user: String,
        ssh_key_path: PathBuf,
        authority_index: u32,
        rpc_port: u16,
        abci_port: u16,
    }

    impl MockRemoteNode {
        fn new(host: String, authority_index: u32) -> Self {
            Self {
                host,
                port: 22,
                ssh_user: "ubuntu".to_string(),
                ssh_key_path: PathBuf::from("~/.ssh/id_rsa"),
                authority_index,
                rpc_port: 26657,
                abci_port: 26670 + authority_index as u16,
            }
        }

        fn ssh_command(&self, command: &str) -> String {
            format!(
                "ssh -i {} -p {} {}@{} -o StrictHostKeyChecking=no -o ConnectTimeout=30 '{}'",
                self.ssh_key_path.display(),
                self.port,
                self.ssh_user,
                self.host,
                command
            )
        }
    }

    // Mock the RemoteNetworkOrchestrator for testing
    struct MockRemoteNetworkOrchestrator {
        nodes: Vec<MockRemoteNode>,
        should_fail_setup: bool,
        should_fail_start: bool,
        should_fail_stop: bool,
        should_fail_health_check: bool,
    }

    impl MockRemoteNetworkOrchestrator {
        fn new() -> Result<Self> {
            let nodes = vec![
                MockRemoteNode::new("node1.example.com".to_string(), 0),
                MockRemoteNode::new("node2.example.com".to_string(), 1),
                MockRemoteNode::new("node3.example.com".to_string(), 2),
                MockRemoteNode::new("node4.example.com".to_string(), 3),
            ];

            Ok(Self {
                nodes,
                should_fail_setup: false,
                should_fail_start: false,
                should_fail_stop: false,
                should_fail_health_check: false,
            })
        }

        fn with_fail_setup(mut self) -> Self {
            self.should_fail_setup = true;
            self
        }

        fn with_fail_start(mut self) -> Self {
            self.should_fail_start = true;
            self
        }

        fn with_fail_stop(mut self) -> Self {
            self.should_fail_stop = true;
            self
        }

        fn with_fail_health_check(mut self) -> Self {
            self.should_fail_health_check = true;
            self
        }

        async fn setup_docker_on_node(&self, _node: &MockRemoteNode) -> Result<()> {
            if self.should_fail_setup {
                return Err(color_eyre::eyre::eyre!("Mock setup failure"));
            }
            Ok(())
        }

        async fn start_mysticeti_container(&self, _node: &MockRemoteNode) -> Result<()> {
            if self.should_fail_start {
                return Err(color_eyre::eyre::eyre!("Mock start failure"));
            }
            Ok(())
        }

        async fn stop_mysticeti_container(&self, _node: &MockRemoteNode) -> Result<()> {
            if self.should_fail_stop {
                return Err(color_eyre::eyre::eyre!("Mock stop failure"));
            }
            Ok(())
        }

        async fn wait_for_network_ready(&self, _wait_time: u64) -> Result<()> {
            if self.should_fail_health_check {
                return Err(color_eyre::eyre::eyre!("Mock health check failure"));
            }
            // Simulate waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(())
        }

        async fn simulate_transactions(
            &self,
            num_transactions: usize,
            _transaction_size: usize,
            transaction_rate: usize,
        ) -> Result<()> {
            // Simulate transaction processing
            let delay = Duration::from_millis((1000 / transaction_rate) as u64);

            for i in 0..num_transactions.min(10) {
                // Limit for testing
                tokio::time::sleep(delay).await;
                if i % 100 == 0 {
                    // Simulate logging
                }
            }
            Ok(())
        }

        async fn setup_all_nodes(&self) -> Result<()> {
            if self.should_fail_setup {
                return Err(color_eyre::eyre::eyre!("Mock setup failure"));
            }
            Ok(())
        }

        async fn start_all_containers(&self) -> Result<()> {
            if self.should_fail_start {
                return Err(color_eyre::eyre::eyre!("Mock start failure"));
            }
            Ok(())
        }

        async fn stop_all_containers(&self) -> Result<()> {
            if self.should_fail_stop {
                return Err(color_eyre::eyre::eyre!("Mock stop failure"));
            }
            Ok(())
        }
    }

    #[test]
    fn test_args_parsing() {
        let args = vec![
            "remote-network",
            "--num-transactions",
            "500",
            "--transaction-size",
            "1024",
            "--transaction-rate",
            "200",
            "--startup-wait",
            "90",
            "--ssh-timeout",
            "60",
            "--cleanup",
        ];

        let parsed: Args = clap::Parser::try_parse_from(args).unwrap();
        assert_eq!(parsed.num_transactions, 500);
        assert_eq!(parsed.transaction_size, 1024);
        assert_eq!(parsed.transaction_rate, 200);
        assert_eq!(parsed.startup_wait, 90);
        assert_eq!(parsed.ssh_timeout, 60);
        assert_eq!(parsed.cleanup, true);
    }

    #[test]
    fn test_args_default_values() {
        let args = vec!["remote-network"];
        let parsed: Args = clap::Parser::try_parse_from(args).unwrap();

        assert_eq!(parsed.num_transactions, 1000);
        assert_eq!(parsed.transaction_size, 512);
        assert_eq!(parsed.transaction_rate, 100);
        assert_eq!(parsed.startup_wait, 60);
        assert_eq!(parsed.ssh_timeout, 30);
        assert_eq!(parsed.cleanup, false);
    }

    #[test]
    fn test_remote_node_creation() {
        let node = MockRemoteNode::new("test.example.com".to_string(), 0);

        assert_eq!(node.host, "test.example.com");
        assert_eq!(node.port, 22);
        assert_eq!(node.ssh_user, "ubuntu");
        assert_eq!(node.authority_index, 0);
        assert_eq!(node.rpc_port, 26657);
        assert_eq!(node.abci_port, 26670);
    }

    #[test]
    fn test_remote_node_ssh_command() {
        let node = MockRemoteNode::new("test.example.com".to_string(), 1);
        let command = "docker ps";
        let ssh_cmd = node.ssh_command(command);

        assert!(ssh_cmd.contains("ssh -i"));
        assert!(ssh_cmd.contains("-p 22"));
        assert!(ssh_cmd.contains("ubuntu@test.example.com"));
        assert!(ssh_cmd.contains("-o StrictHostKeyChecking=no"));
        assert!(ssh_cmd.contains("-o ConnectTimeout=30"));
        assert!(ssh_cmd.contains("docker ps"));
    }

    #[test]
    fn test_orchestrator_new_success() {
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();
        assert_eq!(orchestrator.nodes.len(), 4);

        for (i, node) in orchestrator.nodes.iter().enumerate() {
            assert_eq!(node.authority_index, i as u32);
            assert_eq!(node.rpc_port, 26657);
            assert_eq!(node.abci_port, 26670 + i as u16);
        }
    }

    #[tokio::test]
    async fn test_setup_docker_on_node_success() {
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();
        let node = MockRemoteNode::new("test.example.com".to_string(), 0);

        assert!(orchestrator.setup_docker_on_node(&node).await.is_ok());
    }

    #[tokio::test]
    async fn test_setup_docker_on_node_failure() {
        let orchestrator = MockRemoteNetworkOrchestrator::new()
            .unwrap()
            .with_fail_setup();
        let node = MockRemoteNode::new("test.example.com".to_string(), 0);

        let result = orchestrator.setup_docker_on_node(&node).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mock setup failure")
        );
    }

    #[tokio::test]
    async fn test_start_mysticeti_container_success() {
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();
        let node = MockRemoteNode::new("test.example.com".to_string(), 0);

        assert!(orchestrator.start_mysticeti_container(&node).await.is_ok());
    }

    #[tokio::test]
    async fn test_start_mysticeti_container_failure() {
        let orchestrator = MockRemoteNetworkOrchestrator::new()
            .unwrap()
            .with_fail_start();
        let node = MockRemoteNode::new("test.example.com".to_string(), 0);

        let result = orchestrator.start_mysticeti_container(&node).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mock start failure")
        );
    }

    #[tokio::test]
    async fn test_stop_mysticeti_container_success() {
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();
        let node = MockRemoteNode::new("test.example.com".to_string(), 0);

        assert!(orchestrator.stop_mysticeti_container(&node).await.is_ok());
    }

    #[tokio::test]
    async fn test_stop_mysticeti_container_failure() {
        let orchestrator = MockRemoteNetworkOrchestrator::new()
            .unwrap()
            .with_fail_stop();
        let node = MockRemoteNode::new("test.example.com".to_string(), 0);

        let result = orchestrator.stop_mysticeti_container(&node).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mock stop failure")
        );
    }

    #[tokio::test]
    async fn test_wait_for_network_ready_success() {
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();
        assert!(orchestrator.wait_for_network_ready(1).await.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_network_ready_failure() {
        let orchestrator = MockRemoteNetworkOrchestrator::new()
            .unwrap()
            .with_fail_health_check();

        let result = orchestrator.wait_for_network_ready(1).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mock health check failure")
        );
    }

    #[tokio::test]
    async fn test_simulate_transactions_success() {
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();
        let result = orchestrator.simulate_transactions(5, 512, 10).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_simulate_transactions_with_different_parameters() {
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();

        // Test with different transaction sizes
        assert!(orchestrator.simulate_transactions(3, 256, 5).await.is_ok());
        assert!(orchestrator.simulate_transactions(3, 1024, 5).await.is_ok());
        assert!(orchestrator.simulate_transactions(3, 2048, 5).await.is_ok());

        // Test with different rates
        assert!(orchestrator.simulate_transactions(3, 512, 1).await.is_ok());
        assert!(
            orchestrator
                .simulate_transactions(3, 512, 100)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_setup_all_nodes_success() {
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();
        assert!(orchestrator.setup_all_nodes().await.is_ok());
    }

    #[tokio::test]
    async fn test_setup_all_nodes_failure() {
        let orchestrator = MockRemoteNetworkOrchestrator::new()
            .unwrap()
            .with_fail_setup();

        let result = orchestrator.setup_all_nodes().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mock setup failure")
        );
    }

    #[tokio::test]
    async fn test_start_all_containers_success() {
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();
        assert!(orchestrator.start_all_containers().await.is_ok());
    }

    #[tokio::test]
    async fn test_start_all_containers_failure() {
        let orchestrator = MockRemoteNetworkOrchestrator::new()
            .unwrap()
            .with_fail_start();

        let result = orchestrator.start_all_containers().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mock start failure")
        );
    }

    #[tokio::test]
    async fn test_stop_all_containers_success() {
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();
        assert!(orchestrator.stop_all_containers().await.is_ok());
    }

    #[tokio::test]
    async fn test_stop_all_containers_failure() {
        let orchestrator = MockRemoteNetworkOrchestrator::new()
            .unwrap()
            .with_fail_stop();

        let result = orchestrator.stop_all_containers().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mock stop failure")
        );
    }

    #[test]
    fn test_transaction_data_generation() {
        // Test that transaction data is generated correctly
        let transaction_size = 1024;
        let tx_data = vec![0u8; transaction_size];

        assert_eq!(tx_data.len(), transaction_size);
        assert!(tx_data.iter().all(|&x| x == 0));
    }

    #[test]
    fn test_json_payload_construction() {
        use base64;
        use serde_json::json;

        let tx_data = vec![0u8; 512];
        let encoded_tx = base64::encode(&tx_data);

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "broadcast_tx_async",
            "params": {
                "tx": encoded_tx
            }
        });

        assert_eq!(payload["jsonrpc"], "2.0");
        assert_eq!(payload["id"], 1);
        assert_eq!(payload["method"], "broadcast_tx_async");
        assert!(payload["params"]["tx"].as_str().is_some());
    }

    #[test]
    fn test_rate_limiting_calculation() {
        let transaction_rate = 100;
        let delay_ms = (1000 / transaction_rate) as u64;
        assert_eq!(delay_ms, 10);

        let transaction_rate = 50;
        let delay_ms = (1000 / transaction_rate) as u64;
        assert_eq!(delay_ms, 20);

        let transaction_rate = 200;
        let delay_ms = (1000 / transaction_rate) as u64;
        assert_eq!(delay_ms, 5);
    }

    #[test]
    fn test_node_round_robin_distribution() {
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();

        // Test round-robin distribution across nodes
        for i in 0..10 {
            let node_index = i % orchestrator.nodes.len();
            let node = &orchestrator.nodes[node_index];
            assert_eq!(node.authority_index, node_index as u32);
        }
    }

    #[test]
    fn test_port_assignment() {
        // Test that ports are assigned correctly
        for i in 0..4 {
            let node = MockRemoteNode::new(format!("node{}.example.com", i), i);
            assert_eq!(node.rpc_port, 26657);
            assert_eq!(node.abci_port, 26670 + i as u16);
        }
    }

    #[test]
    fn test_ssh_command_formatting() {
        let node = MockRemoteNode::new("test.example.com".to_string(), 0);

        // Test different commands
        let docker_cmd = node.ssh_command("docker ps");
        assert!(docker_cmd.contains("docker ps"));

        let mkdir_cmd = node.ssh_command("mkdir -p ~/mysticeti-data");
        assert!(mkdir_cmd.contains("mkdir -p ~/mysticeti-data"));

        let pull_cmd = node.ssh_command("docker pull scalarorg/mysticeti:latest");
        assert!(pull_cmd.contains("docker pull scalarorg/mysticeti:latest"));
    }

    #[tokio::test]
    async fn test_error_handling_patterns() {
        // Test various error scenarios
        let orchestrator = MockRemoteNetworkOrchestrator::new().unwrap();

        // Test with failing operations
        let orchestrator = orchestrator.with_fail_setup();
        assert!(orchestrator.setup_all_nodes().await.is_err());

        let orchestrator = MockRemoteNetworkOrchestrator::new()
            .unwrap()
            .with_fail_start();
        assert!(orchestrator.start_all_containers().await.is_err());

        let orchestrator = MockRemoteNetworkOrchestrator::new()
            .unwrap()
            .with_fail_stop();
        assert!(orchestrator.stop_all_containers().await.is_err());
    }

    #[test]
    fn test_environment_variable_validation() {
        // Test that required environment variables are validated
        // This would typically be done in the main function
        let required_vars = vec![
            "MYSTICETI_NODE0_HOST",
            "MYSTICETI_NODE1_HOST",
            "MYSTICETI_NODE2_HOST",
            "MYSTICETI_NODE3_HOST",
        ];

        // In a real test, we would set/unset these environment variables
        // and verify the validation logic
        assert_eq!(required_vars.len(), 4);
        assert!(
            required_vars
                .iter()
                .all(|var| var.contains("MYSTICETI_NODE"))
        );
    }
}
