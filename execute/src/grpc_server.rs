// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status, transport::Server};
use tracing::{error, info};

use consensus_core::{ConsensusAuthority, TransactionClient};

// Define the protobuf service (you'll need to generate this from .proto files)
pub mod mysticeti_grpc {
    // Temporarily comment out until build script generates the proto files
    // tonic::include_proto!("mysticeti.grpc");

    // Placeholder types until proto generation is complete
    use tonic::{Request, Response, Status};

    #[derive(Debug, Clone)]
    pub struct TransactionRequest {
        pub transaction: Vec<u8>,
    }

    #[derive(Debug, Clone)]
    pub struct TransactionResponse {
        pub success: bool,
        pub message: String,
        pub block_ref: Option<BlockRef>,
    }

    #[derive(Debug, Clone)]
    pub struct BlockRef {
        pub round: u64,
        pub authority: u32,
        pub sequence: u64,
    }

    #[derive(Debug, Clone)]
    pub struct ConsensusStatus {
        pub is_running: bool,
        pub current_round: u64,
        pub total_transactions: u64,
    }

    pub mod mysticeti_service_server {
        use super::*;

        #[async_trait::async_trait]
        pub trait MysticetiService: Send + Sync + 'static {
            async fn submit_transaction(
                &self,
                request: Request<TransactionRequest>,
            ) -> Result<Response<TransactionResponse>, Status>;

            async fn get_consensus_status(
                &self,
                request: Request<()>,
            ) -> Result<Response<ConsensusStatus>, Status>;
        }

        pub struct MysticetiServiceServer<T: MysticetiService>(pub T);
    }
}

use mysticeti_grpc::{
    BlockRef, ConsensusStatus, TransactionRequest, TransactionResponse,
    mysticeti_service_server::{MysticetiService, MysticetiServiceServer},
};

pub struct MysticetiGrpcServer {
    transaction_client: Arc<TransactionClient>,
    consensus_authority: Arc<ConsensusAuthority>,
}

impl MysticetiGrpcServer {
    pub fn new(
        transaction_client: Arc<TransactionClient>,
        consensus_authority: Arc<ConsensusAuthority>,
    ) -> Self {
        Self {
            transaction_client,
            consensus_authority,
        }
    }

    pub async fn start_server(
        self,
        addr: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = addr.parse()?;
        let svc = MysticetiServiceServer::new(self);

        info!("Starting Mysticeti gRPC server on {}", addr);

        Server::builder().add_service(svc).serve(addr).await?;

        Ok(())
    }
}

#[tonic::async_trait]
impl MysticetiService for MysticetiGrpcServer {
    async fn submit_transaction(
        &self,
        request: Request<TransactionRequest>,
    ) -> Result<Response<TransactionResponse>, Status> {
        let tx_data = request.into_inner().transaction;

        info!("Received transaction via gRPC: {} bytes", tx_data.len());

        // Submit transaction to Mysticeti consensus
        match self.transaction_client.submit(vec![tx_data]).await {
            Ok((block_ref, status_receiver)) => {
                info!(
                    "Transaction submitted successfully to Mysticeti consensus, included in block: {:?}",
                    block_ref
                );

                // Spawn a task to handle the status update
                let authority = self.consensus_authority.clone();
                tokio::spawn(async move {
                    if let Ok(status) = status_receiver.await {
                        info!("Transaction status: {:?}", status);
                    }
                });

                Ok(Response::new(TransactionResponse {
                    success: true,
                    block_ref: Some(BlockRef {
                        round: block_ref.round,
                        authority: block_ref.authority,
                        sequence: block_ref.sequence,
                    }),
                    message: "Transaction accepted and forwarded to consensus".to_string(),
                }))
            }
            Err(e) => {
                error!("Failed to submit transaction to Mysticeti consensus: {}", e);
                Ok(Response::new(TransactionResponse {
                    success: false,
                    block_ref: None,
                    message: format!("Failed to process transaction: {}", e),
                }))
            }
        }
    }

    async fn get_consensus_status(
        &self,
        _request: Request<()>,
    ) -> Result<Response<ConsensusStatus>, Status> {
        // Return current consensus status
        Ok(Response::new(ConsensusStatus {
            is_running: true,
            current_round: 0, // You'll need to get this from the consensus authority
            total_transactions: 0, // You'll need to track this
        }))
    }
}
