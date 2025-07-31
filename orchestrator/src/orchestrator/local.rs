use base64::Engine;
use color_eyre::eyre::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use tokio::time::sleep;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt};

pub struct LocalNetworkOrchestrator {
    docker_compose_path: PathBuf,
}

impl LocalNetworkOrchestrator {
    pub fn new(docker_compose_path: PathBuf) -> Result<Self> {
        // Verify the docker-compose.yml file exists
        if !docker_compose_path.exists() {
            return Err(color_eyre::eyre::eyre!(
                "docker-compose.yml not found at {}",
                docker_compose_path.display()
            ));
        }

        Ok(Self {
            docker_compose_path,
        })
    }

    pub fn verify_docker_compose(&self) -> Result<()> {
        info!(
            "Using existing docker-compose.yml at {}",
            self.docker_compose_path.display()
        );
        Ok(())
    }

    pub fn start_network(&self) -> Result<()> {
        info!("Starting Mysticeti network with docker compose...");

        // Get the orchestrator directory (parent of docker-compose.yml)
        let orchestrator_dir = self
            .docker_compose_path
            .parent()
            .ok_or_else(|| color_eyre::eyre::eyre!("Failed to get orchestrator directory"))?;

        let status = Command::new("docker")
            .current_dir(orchestrator_dir)
            .args(&["compose", "up", "-d"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .wrap_err("Failed to start docker compose")?;

        if !status.success() {
            return Err(color_eyre::eyre::eyre!(
                "Docker compose failed with status: {}",
                status
            ));
        }

        info!("Mysticeti network started successfully");
        Ok(())
    }

    pub fn stop_network(&self) -> Result<()> {
        info!("Stopping Mysticeti network...");

        // Get the orchestrator directory (parent of docker-compose.yml)
        let orchestrator_dir = self
            .docker_compose_path
            .parent()
            .ok_or_else(|| color_eyre::eyre::eyre!("Failed to get orchestrator directory"))?;

        let status = Command::new("docker")
            .current_dir(orchestrator_dir)
            .args(&["compose", "down"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .wrap_err("Failed to stop docker compose")?;

        if !status.success() {
            warn!("Docker compose down failed with status: {}", status);
        } else {
            info!("Mysticeti network stopped successfully");
        }
        Ok(())
    }

    pub fn stop_network_thorough(&self) -> Result<()> {
        info!(
            "Performing thorough cleanup of Mysticeti network (removing volumes and containers)..."
        );

        // Get the orchestrator directory (parent of docker-compose.yml)
        let orchestrator_dir = self
            .docker_compose_path
            .parent()
            .ok_or_else(|| color_eyre::eyre::eyre!("Failed to get orchestrator directory"))?;

        // Stop and remove containers with volumes
        let status = Command::new("docker")
            .current_dir(orchestrator_dir)
            .args(&["compose", "down", "-v"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .wrap_err("Failed to stop docker compose with volumes")?;

        if !status.success() {
            warn!("Docker compose down -v failed with status: {}", status);
        } else {
            info!("Mysticeti network stopped and volumes removed successfully");
        }

        // Remove any orphaned containers
        let status_orphans = Command::new("docker")
            .args(&["container", "prune", "-f"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .wrap_err("Failed to prune orphaned containers")?;

        if !status_orphans.success() {
            warn!(
                "Docker container prune failed with status: {}",
                status_orphans
            );
        } else {
            info!("Orphaned containers cleaned up");
        }

        // Remove any orphaned volumes
        let status_volumes = Command::new("docker")
            .args(&["volume", "prune", "-f"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .wrap_err("Failed to prune orphaned volumes")?;

        if !status_volumes.success() {
            warn!("Docker volume prune failed with status: {}", status_volumes);
        } else {
            info!("Orphaned volumes cleaned up");
        }

        Ok(())
    }

    pub async fn wait_for_network_ready(&self, wait_time: u64) -> Result<()> {
        info!("Waiting {} seconds for network to be ready...", wait_time);
        sleep(Duration::from_secs(wait_time)).await;

        // Check if nodes are responding
        let client = Client::new();
        let node_urls = vec![
            "http://localhost:26657",
            "http://localhost:26658",
            "http://localhost:26659",
            "http://localhost:26660",
        ];

        let mut all_nodes_ready = true;
        for (i, url) in node_urls.iter().enumerate() {
            match client.get(&format!("{}/health", url)).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        info!("Node {} is ready at {}", i, url);
                    } else {
                        warn!("Node {} responded with status: {}", i, response.status());
                        all_nodes_ready = false;
                    }
                }
                Err(e) => {
                    warn!("Node {} not ready yet: {}", i, e);
                    all_nodes_ready = false;
                }
            }
        }

        if !all_nodes_ready {
            warn!("Some nodes are not ready yet. Network may still be initializing.");
            info!("This is normal during startup. The network will continue to retry connections.");
        }

        Ok(())
    }

    /// Get container logs for debugging
    pub fn get_container_logs(&self, container_name: &str) -> Result<String> {
        let output = Command::new("docker")
            .args(&["logs", container_name])
            .output()
            .wrap_err(format!(
                "Failed to get logs for container {}",
                container_name
            ))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(color_eyre::eyre::eyre!(
                "Failed to get logs for container {}: {}",
                container_name,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    /// Check if a container is running
    pub fn is_container_running(&self, container_name: &str) -> Result<bool> {
        let output = Command::new("docker")
            .args(&[
                "ps",
                "--filter",
                &format!("name={}", container_name),
                "--format",
                "{{.Names}}",
            ])
            .output()
            .wrap_err(format!(
                "Failed to check if container {} is running",
                container_name
            ))?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(!output_str.is_empty() && output_str.contains(container_name))
        } else {
            Ok(false)
        }
    }

    /// Get container status for all nodes
    pub fn get_network_status(&self) -> Result<()> {
        info!("Checking network status...");

        let container_names = vec![
            "mysticeti-node0",
            "mysticeti-node1",
            "mysticeti-node2",
            "mysticeti-node3",
        ];

        for container_name in &container_names {
            match self.is_container_running(container_name) {
                Ok(true) => info!("Container {} is running", container_name),
                Ok(false) => warn!("Container {} is not running", container_name),
                Err(e) => warn!("Failed to check container {}: {}", container_name, e),
            }
        }

        Ok(())
    }

    pub async fn simulate_transactions(
        &self,
        num_transactions: usize,
        transaction_size: usize,
        transaction_rate: usize,
    ) -> Result<()> {
        info!("Starting transaction simulation...");
        info!(
            "Parameters: {} transactions, {} bytes each, {} tx/s",
            num_transactions, transaction_size, transaction_rate
        );

        let client = Client::new();
        //let delay = Duration::from_millis((1000 / transaction_rate) as u64);
        let mut successful_txs = 0;
        let mut failed_txs = 0;
        let start_time = Instant::now();

        // Generate random transaction data
        let tx_data = vec![0u8; transaction_size];

        for i in 0..num_transactions {
            // Round-robin between nodes
            let node_port = 26657 + (i % 4) as u16;
            let url = format!("http://localhost:{}/broadcast_tx_async", node_port);
            let payload = json!({
                "transaction": base64::engine::general_purpose::STANDARD.encode(&tx_data)
            });

            match client.post(&url).json(&payload).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        successful_txs += 1;
                        if i % 100 == 0 {
                            info!("Submitted transaction {} to port {}", i, node_port);
                        }
                    } else {
                        failed_txs += 1;
                        warn!(
                            "Transaction {} failed with status: {}",
                            i,
                            response.status()
                        );
                    }
                }
                Err(e) => {
                    failed_txs += 1;
                    warn!("Transaction {} failed: {}", i, e);
                }
            }

            // Rate limiting
            //sleep(delay).await;
        }

        let duration = start_time.elapsed();
        let actual_rate = successful_txs as f64 / duration.as_secs_f64();

        info!("Transaction simulation completed!");
        info!("Duration: {:.2}s", duration.as_secs_f64());
        info!("Successful transactions: {}", successful_txs);
        info!("Failed transactions: {}", failed_txs);
        info!("Actual rate: {:.2} tx/s", actual_rate);

        Ok(())
    }

    /// Collect metrics from containers (placeholder for future implementation)
    pub async fn collect_metrics(&self) -> Result<()> {
        info!("Collecting metrics from containers...");

        // TODO: Implement actual metrics collection from containers
        // This could involve:
        // 1. Executing commands inside containers to get metrics
        // 2. Reading log files from containers
        // 3. Using container monitoring APIs

        // For now, just check container status
        self.get_network_status()?;

        Ok(())
    }
}
