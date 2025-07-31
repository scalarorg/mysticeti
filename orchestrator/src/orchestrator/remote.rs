use color_eyre::eyre::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::{
    env,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::time::sleep;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Debug, Clone)]
pub struct RemoteNode {
    host: String,
    port: u16,
    ssh_user: String,
    ssh_key_path: PathBuf,
    authority_index: u32,
    rpc_port: u16,
    abci_port: u16,
}

impl RemoteNode {
    fn from_env(index: u32) -> Result<Self> {
        let host = env::var(&format!("MYSTICETI_NODE{}_HOST", index)).wrap_err(format!(
            "MYSTICETI_NODE{}_HOST environment variable not set",
            index
        ))?;

        let port = env::var(&format!("MYSTICETI_NODE{}_SSH_PORT", index))
            .unwrap_or_else(|_| "22".to_string())
            .parse::<u16>()
            .wrap_err(format!("Invalid SSH port for node {}", index))?;

        let ssh_user = env::var(&format!("MYSTICETI_NODE{}_SSH_USER", index))
            .unwrap_or_else(|_| "ubuntu".to_string());

        let ssh_key_path = PathBuf::from(
            env::var(&format!("MYSTICETI_NODE{}_SSH_KEY", index))
                .unwrap_or_else(|_| "~/.ssh/id_rsa".to_string()),
        );

        let rpc_port = 26657;
        let abci_port = 26670 + index as u16;

        Ok(Self {
            host,
            port,
            ssh_user,
            ssh_key_path,
            authority_index: index,
            rpc_port,
            abci_port,
        })
    }

    fn ssh_command(&self, command: &str) -> String {
        format!(
            "ssh -i {} -p {} {}@{} -o StrictHostKeyChecking=no -o ConnectTimeout={} '{}'",
            self.ssh_key_path.display(),
            self.port,
            self.ssh_user,
            self.host,
            env::var("SSH_TIMEOUT").unwrap_or_else(|_| "30".to_string()),
            command
        )
    }
}

pub struct RemoteNetworkOrchestrator {
    pub nodes: Vec<RemoteNode>,
    pub client: Client,
}

impl RemoteNetworkOrchestrator {
    pub fn new() -> Result<Self> {
        let mut nodes = Vec::new();

        // Load 4 nodes from environment
        for i in 0..4 {
            match RemoteNode::from_env(i) {
                Ok(node) => {
                    info!("Loaded node {}: {}:{}", i, node.host, node.port);
                    nodes.push(node);
                }
                Err(e) => {
                    return Err(color_eyre::eyre::eyre!("Failed to load node {}: {}", i, e));
                }
            }
        }

        if nodes.len() != 4 {
            return Err(color_eyre::eyre::eyre!(
                "Expected 4 nodes, got {}",
                nodes.len()
            ));
        }

        Ok(Self {
            nodes,
            client: Client::new(),
        })
    }

    async fn setup_docker_on_node(&self, node: &RemoteNode) -> Result<()> {
        info!(
            "Setting up Docker on node {} ({})",
            node.authority_index, node.host
        );

        // Check if Docker is installed
        let docker_check = node.ssh_command("docker --version");
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&docker_check)
            .output()
            .wrap_err("Failed to check Docker installation")?;

        if !output.status.success() {
            info!("Installing Docker on node {}", node.authority_index);

            let install_commands = vec![
                "sudo apt-get update",
                "sudo apt-get install -y apt-transport-https ca-certificates curl gnupg lsb-release",
                "curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg",
                "echo \"deb [arch=amd64 signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable\" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null",
                "sudo apt-get update",
                "sudo apt-get install -y docker-ce docker-ce-cli containerd.io",
                "sudo usermod -aG docker $USER",
            ];

            for cmd in install_commands {
                let ssh_cmd = node.ssh_command(cmd);
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&ssh_cmd)
                    .status()
                    .wrap_err(format!("Failed to execute: {}", cmd))?;

                if !status.success() {
                    warn!("Command '{}' failed on node {}", cmd, node.authority_index);
                }
            }
        } else {
            info!("Docker already installed on node {}", node.authority_index);
        }

        Ok(())
    }

    async fn start_mysticeti_container(&self, node: &RemoteNode) -> Result<()> {
        info!(
            "Starting Mysticeti container on node {} ({})",
            node.authority_index, node.host
        );

        // Create working directory
        let mkdir_cmd = node.ssh_command("mkdir -p ~/mysticeti-data");
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&mkdir_cmd)
            .status()
            .wrap_err("Failed to create working directory")?;

        if !status.success() {
            warn!(
                "Failed to create working directory on node {}",
                node.authority_index
            );
        }

        // Pull the Mysticeti image
        let pull_cmd = node.ssh_command("docker pull scalarorg/mysticeti:latest");
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&pull_cmd)
            .status()
            .wrap_err("Failed to pull Mysticeti image")?;

        if !status.success() {
            warn!(
                "Failed to pull Mysticeti image on node {}",
                node.authority_index
            );
        }

        // Start the container
        let container_cmd = format!(
            "docker run -d --name mysticeti-node{} \
             -p {}:26657 -p {}:{} \
             -v ~/mysticeti-data:/app/data \
             -e RUST_LOG=info \
             scalarorg/mysticeti:latest \
             --authority-index {} \
             --rpc-port 26657 \
             --abci-port {} \
             --working-directory /app/data",
            node.authority_index,
            node.rpc_port,
            node.abci_port,
            node.abci_port,
            node.authority_index,
            node.abci_port
        );

        let ssh_cmd = node.ssh_command(&container_cmd);
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&ssh_cmd)
            .status()
            .wrap_err("Failed to start Mysticeti container")?;

        if !status.success() {
            return Err(color_eyre::eyre::eyre!(
                "Failed to start Mysticeti container on node {}",
                node.authority_index
            ));
        }

        info!(
            "Mysticeti container started on node {}",
            node.authority_index
        );
        Ok(())
    }

    async fn stop_mysticeti_container(&self, node: &RemoteNode) -> Result<()> {
        info!(
            "Stopping Mysticeti container on node {} ({})",
            node.authority_index, node.host
        );

        let stop_cmd = node.ssh_command(&format!(
            "docker stop mysticeti-node{} && docker rm mysticeti-node{}",
            node.authority_index, node.authority_index
        ));

        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&stop_cmd)
            .status()
            .wrap_err("Failed to stop Mysticeti container")?;

        if !status.success() {
            warn!("Failed to stop container on node {}", node.authority_index);
        } else {
            info!(
                "Mysticeti container stopped on node {}",
                node.authority_index
            );
        }

        Ok(())
    }

    pub async fn wait_for_network_ready(&self, wait_time: u64) -> Result<()> {
        info!("Waiting {} seconds for network to be ready...", wait_time);
        sleep(Duration::from_secs(wait_time)).await;

        // Check if nodes are responding
        for node in &self.nodes {
            let url = format!("http://{}:{}/health", node.host, node.rpc_port);
            match self.client.get(&url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        info!("Node {} is ready at {}", node.authority_index, url);
                    } else {
                        warn!(
                            "Node {} responded with status: {}",
                            node.authority_index,
                            response.status()
                        );
                    }
                }
                Err(e) => {
                    warn!("Node {} not ready yet: {}", node.authority_index, e);
                }
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

        let delay = Duration::from_millis((1000 / transaction_rate) as u64);
        let mut successful_txs = 0;
        let mut failed_txs = 0;
        let start_time = Instant::now();

        // Generate random transaction data
        let tx_data = vec![0u8; transaction_size];

        for i in 0..num_transactions {
            // Round-robin between nodes
            let node = &self.nodes[i % self.nodes.len()];
            let url = format!("http://{}:{}/broadcast_tx_async", node.host, node.rpc_port);

            let payload = json!({
                "transaction": base64::encode(&tx_data)
            });

            match self.client.post(&url).json(&payload).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        successful_txs += 1;
                        if i % 100 == 0 {
                            info!(
                                "Submitted transaction {} to node {} ({})",
                                i, node.authority_index, node.host
                            );
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
            sleep(delay).await;
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

    pub async fn setup_all_nodes(&self) -> Result<()> {
        info!("Setting up all remote nodes...");

        for node in &self.nodes {
            self.setup_docker_on_node(node).await?;
        }

        info!("All nodes setup completed");
        Ok(())
    }

    pub async fn start_all_containers(&self) -> Result<()> {
        info!("Starting Mysticeti containers on all nodes...");

        for node in &self.nodes {
            self.start_mysticeti_container(node).await?;
        }

        info!("All containers started");
        Ok(())
    }

    pub async fn stop_all_containers(&self) -> Result<()> {
        info!("Stopping Mysticeti containers on all nodes...");

        for node in &self.nodes {
            self.stop_mysticeti_container(node).await?;
        }

        info!("All containers stopped");
        Ok(())
    }
}
