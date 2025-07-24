// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

use consensus_config::{AuthorityIndex, NetworkKeyPair, Parameters, ProtocolKeyPair};
use consensus_core::{
    Clock, CommitConsumer, ConsensusAuthority, TransactionIndex, TransactionVerifier,
    ValidationError,
};
use mysten_metrics::RegistryService;
use sui_protocol_config::{ConsensusNetwork, ProtocolConfig};

use crate::abci::app::MysticetiAbciApp;

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

pub struct ValidatorNode {
    authority_index: AuthorityIndex,
    working_directory: PathBuf,
    rpc_port: u16,
    abci_port: u16,
    consensus_authority: Option<ConsensusAuthority>,
    transaction_sender: mpsc::Sender<Vec<u8>>,
    consensus_output_sender: mpsc::Sender<consensus_core::CommittedSubDag>,
}

impl ValidatorNode {
    pub fn new(authority_index: u32, working_directory: PathBuf, rpc_port: u16) -> Self {
        let (transaction_sender, _transaction_receiver) = mpsc::channel(1000);
        let (consensus_output_sender, _consensus_output_receiver) = mpsc::channel(1000);
        let abci_port = 26670 + authority_index as u16;
        Self {
            authority_index: AuthorityIndex::new_for_test(authority_index),
            working_directory,
            rpc_port,
            abci_port,
            consensus_authority: None,
            transaction_sender,
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
            "Starting validator node {} on RPC port {} and ABCI port {}",
            self.authority_index, self.rpc_port, self.abci_port
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
            registry_service.default_registry().clone(),
            0, // boot_counter
        )
        .await;

        self.consensus_authority = Some(consensus_authority);

        // Start transaction processing and consensus output handling
        self.start_transaction_processing(commit_receiver, block_receiver)
            .await;

        // Start ABCI server with consensus output sender
        self.start_abci_server().await?;

        // Start CommetBft RPC server
        self.start_commetbft_rpc_server().await?;

        // Start EVM RPC server
        self.start_evm_rpc_server().await?;

        info!(
            "Validator node {} started successfully",
            self.authority_index
        );
        Ok(())
    }

    async fn start_abci_server(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let abci_addr = format!("127.0.0.1:{}", self.abci_port);
        let _transaction_sender = self.transaction_sender.clone();
        let consensus_output_sender = self.consensus_output_sender.clone();

        // Create a channel for transactions from ABCI to Mysticeti consensus
        let (abci_tx_sender, mut abci_tx_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(1000);
        let consensus_authority = self.consensus_authority.as_ref().unwrap().clone();
        let transaction_client = consensus_authority.transaction_client();
        let transaction_client = transaction_client.clone();

        // Process transactions from ABCI and submit to Mysticeti consensus
        tokio::spawn(async move {
            while let Some(tx_data) = abci_tx_receiver.recv().await {
                info!(
                    "Submitting transaction from ABCI to Mysticeti consensus: {} bytes",
                    tx_data.len()
                );

                // Submit transaction to Mysticeti consensus authority using the transaction client
                match transaction_client.submit(vec![tx_data]).await {
                    Ok((block_ref, _status_receiver)) => {
                        info!(
                            "Transaction submitted successfully to Mysticeti consensus, included in block: {:?}",
                            block_ref
                        );
                    }
                    Err(e) => {
                        error!("Failed to submit transaction to Mysticeti consensus: {}", e);
                    }
                }
            }
        });

        let app = MysticetiAbciApp::new(abci_tx_sender, consensus_output_sender);
        std::thread::spawn(move || {
            let server = tendermint_abci::ServerBuilder::default()
                .bind(abci_addr, app)
                .expect("Failed to bind ABCI server");
            server.listen().expect("ABCI server failed");
        });
        Ok(())
    }

    async fn start_commetbft_rpc_server(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting RPC server on port {}", self.rpc_port);

        // Create a channel to forward transactions from RPC to ABCI
        let (rpc_tx_sender, mut rpc_tx_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(1000);
        let transaction_sender = self.transaction_sender.clone();

        // Start transaction forwarding from RPC to ABCI
        tokio::spawn(async move {
            while let Some(tx_data) = rpc_tx_receiver.recv().await {
                info!(
                    "Forwarding transaction from RPC to ABCI: {} bytes",
                    tx_data.len()
                );
                // Forward to Mysticeti consensus
                if let Err(e) = transaction_sender.send(tx_data).await {
                    error!("Failed to forward transaction to ABCI: {}", e);
                }
            }
        });

        let addr: SocketAddr = format!("127.0.0.1:{}", self.rpc_port).parse()?;

        tokio::spawn(async move {
            use axum::{
                Json, Router,
                http::StatusCode,
                routing::{get, post},
            };
            use serde::{Deserialize, Serialize};

            #[derive(Deserialize)]
            struct TransactionRequest {
                transaction: String, // Base64 encoded transaction
            }

            #[derive(Serialize)]
            struct TransactionResponse {
                success: bool,
                message: String,
            }

            #[derive(Serialize)]
            struct StatusResponse {
                node_info: &'static str,
                abci_app_version: &'static str,
            }

            #[derive(Deserialize)]
            struct AbciQueryRequest {
                path: Option<String>,
                data: Option<String>,
            }

            #[derive(Serialize)]
            struct AbciQueryResponse {
                code: u32,
                value: String,
            }

            let app = Router::new()
                .route(
                    "/broadcast_tx_async",
                    post(|Json(payload): Json<TransactionRequest>| async move {
                        match base64::Engine::decode(
                            &base64::engine::general_purpose::STANDARD,
                            &payload.transaction,
                        ) {
                            Ok(tx_data) => {
                                if let Err(e) = rpc_tx_sender.send(tx_data).await {
                                    error!("Failed to forward transaction to ABCI: {}", e);
                                    return (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(TransactionResponse {
                                            success: false,
                                            message: "Failed to process transaction".to_string(),
                                        }),
                                    );
                                }
                                (
                                    StatusCode::OK,
                                    Json(TransactionResponse {
                                        success: true,
                                        message: "Transaction accepted and forwarded to ABCI"
                                            .to_string(),
                                    }),
                                )
                            }
                            Err(e) => {
                                error!("Failed to decode transaction: {}", e);
                                (
                                    StatusCode::BAD_REQUEST,
                                    Json(TransactionResponse {
                                        success: false,
                                        message: "Invalid transaction format".to_string(),
                                    }),
                                )
                            }
                        }
                    }),
                )
                .route(
                    "/status",
                    get(|| async move {
                        (
                            StatusCode::OK,
                            Json(StatusResponse {
                                node_info: "Mysticeti Validator Node",
                                abci_app_version: "0.1.0",
                            }),
                        )
                    }),
                )
                .route(
                    "/abci_query",
                    post(|Json(_payload): Json<AbciQueryRequest>| async move {
                        // For now, just return a stub
                        (
                            StatusCode::OK,
                            Json(AbciQueryResponse {
                                code: 0,
                                value: "Mysticeti query stub".to_string(),
                            }),
                        )
                    }),
                )
                .route("/health", get(|| async { "OK" }));

            info!("RPC server listening on {}", addr);
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });

        Ok(())
    }

    async fn start_evm_rpc_server(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting EVM RPC server on port {}", self.rpc_port);
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
            "Transaction processing started for node {}",
            self.authority_index
        );
    }

    pub async fn stop(&mut self) {
        info!("Stopping validator node {}", self.authority_index);
        if let Some(authority) = self.consensus_authority.take() {
            authority.stop().await;
        }
    }
}
