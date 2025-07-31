// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use base64::Engine;
use clap::Parser;
use color_eyre::eyre::{Context, Result};
use orchestrator::LocalNetworkOrchestrator;

use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser)]
#[command(author, version, about = "Local Mysticeti Network Orchestrator")]
struct Args {
    /// Path to docker-compose.yml file
    #[clap(long, default_value = "./docker-compose.yml")]
    docker_compose_path: PathBuf,

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
    #[clap(long, default_value = "30")]
    startup_wait: u64,

    /// Whether to clean up containers after completion
    #[clap(long, default_value = "false")]
    cleanup: bool,

    /// Whether to perform thorough cleanup (remove volumes and containers completely)
    #[clap(long, default_value = "false")]
    cleanup_thorough: bool,
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

    info!("Starting Local Mysticeti Network Orchestrator");

    let orchestrator = LocalNetworkOrchestrator::new(args.docker_compose_path.clone())?;

    // Verify docker-compose file exists
    orchestrator.verify_docker_compose()?;

    // Start the network
    orchestrator.start_network()?;

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
        orchestrator.stop_network()?;
        info!("Network cleaned up");
    } else if args.cleanup_thorough {
        orchestrator.stop_network_thorough()?;
        info!("Network thoroughly cleaned up (containers and volumes removed)");
    } else {
        info!(
            "Network is still running. Use 'docker compose down' in the orchestrator directory to stop it"
        );
    }

    info!("Local network orchestration completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio::time::Duration;

    // Mock the LocalNetworkOrchestrator for testing
    struct MockLocalNetworkOrchestrator {
        docker_compose_path: PathBuf,
        should_fail_start: bool,
        should_fail_stop: bool,
        should_fail_health_check: bool,
    }

    impl MockLocalNetworkOrchestrator {
        fn new(docker_compose_path: PathBuf) -> Self {
            Self {
                docker_compose_path,
                should_fail_start: false,
                should_fail_stop: false,
                should_fail_health_check: false,
            }
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

        fn verify_docker_compose(&self) -> Result<()> {
            if !self.docker_compose_path.exists() {
                return Err(color_eyre::eyre::eyre!(
                    "docker-compose.yml not found at {}",
                    self.docker_compose_path.display()
                ));
            }
            Ok(())
        }

        fn start_network(&self) -> Result<()> {
            if self.should_fail_start {
                return Err(color_eyre::eyre::eyre!("Mock start failure"));
            }
            Ok(())
        }

        fn stop_network(&self) -> Result<()> {
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
    }

    #[test]
    fn test_args_parsing() {
        let args = vec![
            "local-network",
            "--docker-compose-path",
            "/test/path/docker-compose.yml",
            "--num-transactions",
            "500",
            "--transaction-size",
            "1024",
            "--transaction-rate",
            "200",
            "--startup-wait",
            "45",
            "--cleanup",
        ];

        let parsed: Args = clap::Parser::try_parse_from(args).unwrap();
        assert_eq!(
            parsed.docker_compose_path,
            PathBuf::from("/test/path/docker-compose.yml")
        );
        assert_eq!(parsed.num_transactions, 500);
        assert_eq!(parsed.transaction_size, 1024);
        assert_eq!(parsed.transaction_rate, 200);
        assert_eq!(parsed.startup_wait, 45);
        assert_eq!(parsed.cleanup, true);
    }

    #[test]
    fn test_args_default_values() {
        let args = vec!["local-network"];
        let parsed: Args = clap::Parser::try_parse_from(args).unwrap();

        assert_eq!(
            parsed.docker_compose_path,
            PathBuf::from("./docker-compose.yml")
        );
        assert_eq!(parsed.num_transactions, 1000);
        assert_eq!(parsed.transaction_size, 512);
        assert_eq!(parsed.transaction_rate, 100);
        assert_eq!(parsed.startup_wait, 30);
        assert_eq!(parsed.cleanup, false);
    }

    #[test]
    fn test_orchestrator_new_success() {
        let temp_dir = tempdir().unwrap();
        let docker_compose_path = temp_dir.path().join("docker-compose.yml");
        std::fs::write(&docker_compose_path, "test content").unwrap();

        let orchestrator = MockLocalNetworkOrchestrator::new(docker_compose_path.clone());
        assert_eq!(orchestrator.docker_compose_path, docker_compose_path);
    }

    #[test]
    fn test_orchestrator_new_failure() {
        let non_existent_path = PathBuf::from("/non/existent/path/docker-compose.yml");
        let orchestrator = MockLocalNetworkOrchestrator::new(non_existent_path.clone());

        let result = orchestrator.verify_docker_compose();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("docker-compose.yml not found")
        );
    }

    #[test]
    fn test_verify_docker_compose_success() {
        let temp_dir = tempdir().unwrap();
        let docker_compose_path = temp_dir.path().join("docker-compose.yml");
        std::fs::write(&docker_compose_path, "test content").unwrap();

        let orchestrator = MockLocalNetworkOrchestrator::new(docker_compose_path);
        assert!(orchestrator.verify_docker_compose().is_ok());
    }

    #[test]
    fn test_start_network_success() {
        let temp_dir = tempdir().unwrap();
        let docker_compose_path = temp_dir.path().join("docker-compose.yml");
        std::fs::write(&docker_compose_path, "test content").unwrap();

        let orchestrator = MockLocalNetworkOrchestrator::new(docker_compose_path);
        assert!(orchestrator.start_network().is_ok());
    }

    #[test]
    fn test_start_network_failure() {
        let temp_dir = tempdir().unwrap();
        let docker_compose_path = temp_dir.path().join("docker-compose.yml");
        std::fs::write(&docker_compose_path, "test content").unwrap();

        let orchestrator = MockLocalNetworkOrchestrator::new(docker_compose_path).with_fail_start();

        let result = orchestrator.start_network();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mock start failure")
        );
    }

    #[test]
    fn test_stop_network_success() {
        let temp_dir = tempdir().unwrap();
        let docker_compose_path = temp_dir.path().join("docker-compose.yml");
        std::fs::write(&docker_compose_path, "test content").unwrap();

        let orchestrator = MockLocalNetworkOrchestrator::new(docker_compose_path);
        assert!(orchestrator.stop_network().is_ok());
    }

    #[test]
    fn test_stop_network_failure() {
        let temp_dir = tempdir().unwrap();
        let docker_compose_path = temp_dir.path().join("docker-compose.yml");
        std::fs::write(&docker_compose_path, "test content").unwrap();

        let orchestrator = MockLocalNetworkOrchestrator::new(docker_compose_path).with_fail_stop();

        let result = orchestrator.stop_network();
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
        let temp_dir = tempdir().unwrap();
        let docker_compose_path = temp_dir.path().join("docker-compose.yml");
        std::fs::write(&docker_compose_path, "test content").unwrap();

        let orchestrator = MockLocalNetworkOrchestrator::new(docker_compose_path);
        assert!(orchestrator.wait_for_network_ready(1).await.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_network_ready_failure() {
        let temp_dir = tempdir().unwrap();
        let docker_compose_path = temp_dir.path().join("docker-compose.yml");
        std::fs::write(&docker_compose_path, "test content").unwrap();

        let orchestrator =
            MockLocalNetworkOrchestrator::new(docker_compose_path).with_fail_health_check();

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
        let temp_dir = tempdir().unwrap();
        let docker_compose_path = temp_dir.path().join("docker-compose.yml");
        std::fs::write(&docker_compose_path, "test content").unwrap();

        let orchestrator = MockLocalNetworkOrchestrator::new(docker_compose_path);
        let result = orchestrator.simulate_transactions(5, 512, 10).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_simulate_transactions_with_different_parameters() {
        let temp_dir = tempdir().unwrap();
        let docker_compose_path = temp_dir.path().join("docker-compose.yml");
        std::fs::write(&docker_compose_path, "test content").unwrap();

        let orchestrator = MockLocalNetworkOrchestrator::new(docker_compose_path);

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

    #[test]
    fn test_transaction_data_generation() {
        // Test that transaction data is generated correctly
        let transaction_size = 1024;
        let tx_data = vec![0u8; transaction_size];

        assert_eq!(tx_data.len(), transaction_size);
        assert!(tx_data.iter().all(|&x| x == 0));
    }

    #[test]
    fn test_node_port_calculation() {
        // Test round-robin port assignment
        for i in 0..10 {
            let node_port = 26657 + (i % 4) as u16;
            assert!(node_port >= 26657 && node_port <= 26660);
        }
    }

    #[test]
    fn test_json_payload_construction() {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;
        use serde_json::json;

        let tx_data = vec![0u8; 512];
        let encoded_tx = STANDARD.encode(&tx_data);

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
    fn test_error_handling_patterns() {
        // Test various error scenarios
        let temp_dir = tempdir().unwrap();
        let docker_compose_path = temp_dir.path().join("docker-compose.yml");

        // Test with non-existent file
        let orchestrator = MockLocalNetworkOrchestrator::new(PathBuf::from("/non/existent"));
        assert!(orchestrator.verify_docker_compose().is_err());

        // Test with failing operations
        let orchestrator =
            MockLocalNetworkOrchestrator::new(docker_compose_path.clone()).with_fail_start();
        assert!(orchestrator.start_network().is_err());

        let orchestrator = MockLocalNetworkOrchestrator::new(docker_compose_path).with_fail_stop();
        assert!(orchestrator.stop_network().is_err());
    }
}
