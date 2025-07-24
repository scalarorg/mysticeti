// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

use consensus_config::{AuthorityIndex, NetworkKeyPair, Parameters, ProtocolKeyPair};
use consensus_core::{
    Clock, CommitConsumer, ConsensusAuthority, TransactionClient, TransactionIndex,
    TransactionVerifier, ValidationError,
};
use mysten_metrics::RegistryService;
use sui_protocol_config::{ConsensusNetwork, ProtocolConfig};

use crate::abci::enhanced_app::EnhancedMysticetiAbciApp;
use crate::grpc_server::MysticetiGrpcServer;

// Simple transaction verifier that accepts all transactions
struct SimpleTransactionVerifier;

impl TransactionVerifier for SimpleTransactionVerifier {
    fn verify_batch(&self, _batch: &[&[u8]]) -> Result<(), ValidationError> {
        Ok(())
    }

    fn verify_and_vote_batch(
        &self,
        _batch: &[&[u8]],
    ) -> Result<Vec<TransactionIndex>, ValidationError> {
        Ok(vec![])
    }
}

pub struct EnhancedValidatorNode {
    authority_index: AuthorityIndex,
    working_directory: PathBuf,
    cometbft_rpc_port: u16,
    mysticeti_grpc_port: u16,
    abci_port: u16,
    consensus_authority: Option<ConsensusAuthority>,
    transaction_client: Option<Arc<TransactionClient>>,
    consensus_output_sender: mpsc::Sender<consensus_core::CommittedSubDag>,
}

impl EnhancedValidatorNode {
    pub fn new(
        authority_index: u32,
        working_directory: PathBuf,
        cometbft_rpc_port: u16,
        mysticeti_grpc_port: u16,
    ) -> Self {
        let (consensus_output_sender, _consensus_output_receiver) = mpsc::channel(1000);
        let abci_port = 26670 + authority_index as u16;

        Self {
            authority_index: AuthorityIndex::new_for_test(authority_index),
            working_directory,
            cometbft_rpc_port,
            mysticeti_grpc_port,
            abci_port,
            consensus_authority: None,
            transaction_client: None,
            consensus_output_sender,
        }
    }

    pub async fn start(
        &mut self,
        committee: consensus_config::Committee,
        keypairs: Vec<(NetworkKeyPair, ProtocolKeyPair)>,
        registry_service: RegistryService,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Starting enhanced validator node {} on CometBFT RPC port {}, Mysticeti gRPC port {}, ABCI port {}",
            self.authority_index, self.cometbft_rpc_port, self.mysticeti_grpc_port, self.abci_port
        );

        // Create node directory
        let node_dir = self
            .working_directory
            .join(format!("node-{}", self.authority_index));
        std::fs::create_dir_all(&node_dir)?;
        let db_path = node_dir.join("consensus.db");

        // Get keypairs for this node
        let (network_keypair, protocol_keypair) = &keypairs[self.authority_index.value() as usize];

        // Create parameters
        let mut parameters = Parameters::default();
        parameters.db_path = db_path;

        // Create commit consumer
        let (commit_consumer, commit_receiver, block_receiver) = CommitConsumer::new(0);

        // Start the consensus authority
        let consensus_authority = ConsensusAuthority::start(
            ConsensusNetwork::Anemo,
            self.authority_index,
            committee,
            parameters,
            ProtocolConfig::get_for_max_version_UNSAFE(),
            protocol_keypair.clone(),
            network_keypair.clone(),
            Arc::new(Clock::new_for_test(0)),
            Arc::new(SimpleTransactionVerifier),
            commit_consumer,
            registry_service.registry(),
            0, // boot counter
        )
        .await;

        self.consensus_authority = Some(consensus_authority.clone());
        self.transaction_client = Some(Arc::new(consensus_authority.transaction_client()));

        // Start the Mysticeti gRPC server
        self.start_mysticeti_grpc_server().await?;

        // Start the ABCI server
        self.start_abci_server().await?;

        // Start transaction processing
        self.start_transaction_processing(commit_receiver, block_receiver)
            .await;

        info!(
            "Enhanced validator node {} started successfully",
            self.authority_index
        );
        Ok(())
    }

    async fn start_mysticeti_grpc_server(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let grpc_addr = format!("127.0.0.1:{}", self.mysticeti_grpc_port);

        if let (Some(consensus_authority), Some(transaction_client)) = (
            self.consensus_authority.as_ref(),
            self.transaction_client.as_ref(),
        ) {
            let grpc_server =
                MysticetiGrpcServer::new(transaction_client.clone(), consensus_authority.clone());

            // Start the gRPC server in a separate task
            let grpc_addr_clone = grpc_addr.clone();
            tokio::spawn(async move {
                if let Err(e) = grpc_server.start_server(grpc_addr_clone).await {
                    error!("Mysticeti gRPC server failed: {}", e);
                }
            });

            info!("Mysticeti gRPC server started on {}", grpc_addr);
        }

        Ok(())
    }

    async fn start_abci_server(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let abci_addr = format!("127.0.0.1:{}", self.abci_port);

        if let Some(transaction_client) = self.transaction_client.as_ref() {
            let app = EnhancedMysticetiAbciApp::new(
                transaction_client.clone(),
                self.consensus_output_sender.clone(),
            );

            // Start ABCI server in a separate thread
            let abci_addr_clone = abci_addr.clone();
            std::thread::spawn(move || {
                let server = tendermint_abci::ServerBuilder::default()
                    .bind(abci_addr_clone, app)
                    .expect("Failed to bind ABCI server");
                server.listen().expect("ABCI server failed");
            });

            info!("ABCI server started on {}", abci_addr);
        }

        Ok(())
    }

    async fn start_transaction_processing(
        &self,
        mut commit_receiver: mysten_metrics::monitored_mpsc::UnboundedReceiver<
            consensus_core::CommittedSubDag,
        >,
        mut block_receiver: mysten_metrics::monitored_mpsc::UnboundedReceiver<
            consensus_core::CertifiedBlocksOutput,
        >,
    ) {
        let consensus_output_sender = self.consensus_output_sender.clone();

        // Process committed sub-dags from Mysticeti consensus
        tokio::spawn(async move {
            while let Some(committed_subdag) = commit_receiver.recv().await {
                info!(
                    "Received committed sub-dag from Mysticeti: {} blocks",
                    committed_subdag.blocks.len()
                );

                // Forward consensus output to ABCI app
                if let Err(e) = consensus_output_sender.send(committed_subdag).await {
                    error!("Failed to forward consensus output to ABCI: {}", e);
                }
            }
        });

        // Process certified blocks from Mysticeti consensus
        tokio::spawn(async move {
            while let Some(certified_blocks) = block_receiver.recv().await {
                info!(
                    "Received certified blocks from Mysticeti: {} blocks",
                    certified_blocks.blocks.len()
                );
                // TODO: Process certified blocks if needed
            }
        });

        info!(
            "Transaction processing started for enhanced node {}",
            self.authority_index
        );
    }

    pub async fn stop(&mut self) {
        info!("Stopping enhanced validator node {}", self.authority_index);
        if let Some(authority) = self.consensus_authority.take() {
            authority.stop().await;
        }
    }

    // Getter methods for external access
    pub fn get_cometbft_rpc_port(&self) -> u16 {
        self.cometbft_rpc_port
    }

    pub fn get_mysticeti_grpc_port(&self) -> u16 {
        self.mysticeti_grpc_port
    }

    pub fn get_abci_port(&self) -> u16 {
        self.abci_port
    }
}
