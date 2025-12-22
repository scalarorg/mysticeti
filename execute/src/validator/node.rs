// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use consensus_config::{AuthorityIndex, Committee, NetworkKeyPair, Parameters, ProtocolKeyPair};
use consensus_core::{
    Clock, CommitConsumerArgs, ConsensusAuthority, NetworkType, TransactionVerifier,
    ValidationError,
};
use consensus_types::block::{BlockRef, TransactionIndex};
use mysten_metrics::RegistryService;
use std::path::PathBuf;
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use sui_protocol_config::{ConsensusNetwork, ProtocolConfig};
use tokio::sync::mpsc;
use tracing::{debug, error, info};
const BATCH_TIMEOUT_MS: u64 = 100; // Send batch after 1 second even if not full
// Simple transaction verifier that accepts all transactions
struct SimpleTransactionVerifier;

type RawTransaction = Vec<u8>;
type RawTransactions = Vec<RawTransaction>;

impl TransactionVerifier for SimpleTransactionVerifier {
    fn verify_batch(&self, _batch: &[&[u8]]) -> Result<(), ValidationError> {
        Ok(())
    }

    fn verify_and_vote_batch(
        &self,
        _block_ref: &BlockRef,
        _batch: &[&[u8]],
    ) -> Result<Vec<TransactionIndex>, ValidationError> {
        Ok(vec![])
    }
}

pub struct ValidatorNode {
    authority_index: AuthorityIndex,
    working_directory: PathBuf,
    consensus_authority: Option<ConsensusAuthority>,
    protocol_config: ProtocolConfig,
}

impl ValidatorNode {
    pub fn new(authority_index: u32, working_directory: PathBuf) -> Self {
        let protocol_config = ProtocolConfig::get_for_max_version_UNSAFE();
        Self {
            authority_index: AuthorityIndex::new_for_test(authority_index),
            working_directory,
            consensus_authority: None,
            protocol_config,
        }
    }

    pub async fn start(
        &mut self,
        committee: Committee,
        parameters: Parameters,
        keypairs: Vec<(NetworkKeyPair, ProtocolKeyPair)>,
        registry_service: RegistryService,
        rx_transactions: mpsc::UnboundedReceiver<RawTransactions>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting validator node {}", self.authority_index);

        // Create node directory
        let node_dir = self
            .working_directory
            .join(format!("node-{}", self.authority_index));
        std::fs::create_dir_all(&node_dir)?;
        let db_path = node_dir.join("consensus.db");

        // Get keypairs for this node
        let (network_keypair, protocol_keypair) = &keypairs[self.authority_index.value()];

        // Create parameters
        let parameters = Parameters {
            db_path,
            ..parameters
        };
        // Log the loaded parameters for debugging
        info!("Loaded consensus parameters: {:?}", parameters);
        // Create commit consumer
        let (commit_consumer, commit_receiver, block_receiver) = CommitConsumerArgs::new(0, 0);

        // Start the consensus authority
        let consensus_authority = ConsensusAuthority::start(
            NetworkType::Tonic,
            0, // epoch_start_timestamp_ms
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
        self.start_transaction_processing(rx_transactions).await;

        // Start ABCI server with consensus output sender
        // self.start_abci_server().await?;

        // Start RPC server
        // self.start_rpc_server(self.protocol_config.rpc_port).await?;

        info!(
            "Validator node {} started successfully",
            self.authority_index
        );
        Ok(())
    }

    async fn start_transaction_processing(
        &self,
        mut rx_transactions: mpsc::UnboundedReceiver<RawTransactions>,
    ) {
        // Process received payload from execution client
        let transaction_client = self
            .consensus_authority
            .as_ref()
            .unwrap()
            .transaction_client();
        let max_transactions_in_block_count =
            self.protocol_config.max_num_transactions_in_block() as usize;
        tokio::spawn(async move {
            // Transaction buffer for batching
            let mut buffer = Vec::new();

            // Create a periodic timer for batch timeout
            let mut batch_timer = tokio::time::interval(Duration::from_millis(BATCH_TIMEOUT_MS));
            let mut total_received_txs = 0_u64;
            let mut total_send_txs = 0_u64;
            loop {
                tokio::select! {
                    // Handle new transaction events
                    Some(raw_tx) = rx_transactions.recv() => {
                        total_received_txs += raw_tx.len() as u64;
                        // because of this push, buffer has at least 1 transaction
                        for tx in raw_tx {
                            buffer.push(tx);
                        }
                        // Send batch if threshold is reached
                        if buffer.len() >= max_transactions_in_block_count {
                            // Split buffer to send only max_transactions_in_block_count transactions
                            let batch: Vec<Vec<u8>> = buffer.drain(0..max_transactions_in_block_count).collect();
                            let batch_size = batch.len();
                            total_send_txs += batch_size as u64;
                            if let Ok((block_ref, _transaction_indices, _status_receiver)) = transaction_client.submit(batch).await {
                                debug!("[Threshold] Sending batch of {} transactions to mysticeti. Total sent/received transactions: {}/{}", batch_size, total_send_txs, total_received_txs);
                            } else {
                                error!("[Threshold] Failed to submit batch of {} transactions", batch_size);
                            }
                            batch_timer.reset();
                        }
                    }
                    // Handle batch timeout
                    _ = batch_timer.tick() => {
                        if !buffer.is_empty() {
                            let batch = std::mem::take(&mut buffer);
                            let batch_size = batch.len();
                            total_send_txs += batch_size as u64;
                            if let Ok((block_ref, _transaction_indices, _status_receiver)) = transaction_client.submit(batch).await {
                                info!("[Timer] Sending batch of {} transactions to mysticeti. Total sent/received transactions: {}/{}", batch_size, total_send_txs, total_received_txs);
                            } else {
                                error!("[Timer] Failed to submit batch of {} transactions", batch_size);
                            }
                        }
                        batch_timer.reset();
                    }
                }
                //End loop
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

impl ValidatorNode {
    async fn start_rpc_server(
        &self,
        rpc_port: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting RPC server on port {}", rpc_port);

        // Create a channel to forward transactions from RPC to ABCI
        let (rpc_tx_sender, mut rpc_tx_receiver) = tokio::sync::mpsc::channel::<Vec<u8>>(1000);
        let transaction_client = self
            .consensus_authority
            .as_ref()
            .unwrap()
            .transaction_client();

        // Start transaction forwarding from RPC to consensus
        tokio::spawn(async move {
            while let Some(tx_data) = rpc_tx_receiver.recv().await {
                info!(
                    "Forwarding transaction from RPC to consensus: {} bytes",
                    tx_data.len()
                );
                // Forward to Mysticeti consensus
                // Submit transaction to Mysticeti consensus authority using the transaction client
                match transaction_client.submit(vec![tx_data]).await {
                    Ok((block_ref, _transaction_indices, _status_receiver)) => {
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

        let addr: SocketAddr = format!("0.0.0.0:{}", rpc_port).parse()?;

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
            struct AbciQueryRequest {}

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
}
