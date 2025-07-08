// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;
use tracing::info;

use consensus_config::local_committee_and_keys;
use mysten_metrics::RegistryService;
use prometheus::Registry;

use crate::validator::node::ValidatorNode;

pub struct ValidatorNetwork {
    working_directory: PathBuf,
    nodes: Vec<ValidatorNode>,
    registry_service: RegistryService,
}

impl ValidatorNetwork {
    pub fn new(working_directory: PathBuf) -> Self {
        let registry_service = RegistryService::new(Registry::new());

        Self {
            working_directory,
            nodes: Vec::new(),
            registry_service,
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Starting validator network with 4 nodes in directory: {}",
            self.working_directory.display()
        );

        // Create working directory
        std::fs::create_dir_all(&self.working_directory)?;

        // Generate committee and keypairs for 4 nodes
        let committee_size = 4;
        let (committee, keypairs) = local_committee_and_keys(0, vec![1; committee_size]);

        // Define RPC ports for each node
        let rpc_ports = vec![26657, 26658, 26659, 26660];

        // Start all 4 validator nodes
        for i in 0..committee_size {
            let authority_index = i as u32;
            let rpc_port = rpc_ports[i];

            let mut node =
                ValidatorNode::new(authority_index, self.working_directory.clone(), rpc_port);

            // Create a unique registry for each node to avoid conflicts
            let node_registry_service = RegistryService::new(Registry::new());

            // Start the node
            node.start(committee.clone(), keypairs.clone(), node_registry_service)
                .await?;

            self.nodes.push(node);

            info!(
                "Started validator node {} on RPC port {}",
                authority_index, rpc_port
            );
        }

        info!("Validator network started successfully!");
        info!("RPC endpoints:");
        for (i, port) in rpc_ports.iter().enumerate() {
            info!("  Node {}: http://127.0.0.1:{}/broadcast_tx_async", i, port);
        }

        Ok(())
    }

    pub async fn stop(&mut self) {
        info!("Stopping validator network...");

        for (i, node) in self.nodes.iter_mut().enumerate() {
            info!("Stopping node {}", i);
            node.stop().await;
        }

        info!("Validator network stopped");
    }

    pub fn get_rpc_endpoints(&self) -> Vec<String> {
        let rpc_ports = vec![26657, 26658, 26659, 26660];
        rpc_ports
            .iter()
            .map(|port| format!("http://127.0.0.1:{}", port))
            .collect()
    }
}
