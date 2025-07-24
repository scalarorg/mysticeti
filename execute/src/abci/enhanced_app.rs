// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use tendermint_abci::Application;
use tendermint_proto::v0_38::abci::{
    RequestCheckTx, RequestFinalizeBlock, RequestInfo, RequestInitChain, RequestPrepareProposal,
    RequestProcessProposal, RequestQuery, ResponseCheckTx, ResponseFinalizeBlock, ResponseInfo,
    ResponseInitChain, ResponsePrepareProposal, ResponseProcessProposal, ResponseQuery,
};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use consensus_core::{CommittedSubDag, TransactionClient};

#[derive(Clone)]
pub struct EnhancedMysticetiAbciApp {
    transaction_client: Arc<TransactionClient>,
    consensus_output_sender: Arc<mpsc::Sender<CommittedSubDag>>,
    // Track transaction status for better error handling
    pending_transactions: Arc<tokio::sync::RwLock<std::collections::HashMap<String, bool>>>,
}

impl EnhancedMysticetiAbciApp {
    pub fn new(
        transaction_client: Arc<TransactionClient>,
        consensus_output_sender: mpsc::Sender<CommittedSubDag>,
    ) -> Self {
        Self {
            transaction_client,
            consensus_output_sender: Arc::new(consensus_output_sender),
            pending_transactions: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    async fn submit_transaction_to_mysticeti(&self, tx_data: Vec<u8>) -> Result<(), String> {
        let tx_hash = format!("{:?}", tx_data);

        // Submit transaction to Mysticeti consensus
        match self.transaction_client.submit(vec![tx_data]).await {
            Ok((block_ref, status_receiver)) => {
                info!(
                    "Transaction submitted to Mysticeti consensus, block: {:?}",
                    block_ref
                );

                // Track the transaction
                {
                    let mut pending = self.pending_transactions.write().await;
                    pending.insert(tx_hash, true);
                }

                // Handle status updates
                let pending_clone = self.pending_transactions.clone();
                tokio::spawn(async move {
                    if let Ok(status) = status_receiver.await {
                        info!("Transaction status update: {:?}", status);
                        // Remove from pending when we get status
                        let mut pending = pending_clone.write().await;
                        pending.remove(&tx_hash);
                    }
                });

                Ok(())
            }
            Err(e) => {
                error!("Failed to submit transaction to Mysticeti: {}", e);
                Err(format!("Consensus error: {}", e))
            }
        }
    }
}

impl Application for EnhancedMysticetiAbciApp {
    fn info(&self, _request: RequestInfo) -> ResponseInfo {
        ResponseInfo {
            data: "Enhanced Mysticeti ABCI App".to_string(),
            version: "0.2.0".to_string(),
            app_version: 2,
            last_block_height: 0,
            last_block_app_hash: vec![].into(),
        }
    }

    fn init_chain(&self, _request: RequestInitChain) -> ResponseInitChain {
        info!("Initializing Mysticeti ABCI chain");
        ResponseInitChain::default()
    }

    fn check_tx(&self, request: RequestCheckTx) -> ResponseCheckTx {
        let tx_data = request.tx.to_vec();
        info!("ABCI check_tx called: {} bytes", tx_data.len());

        // For now, we'll accept all transactions in CheckTx
        // In a production system, you might want to do basic validation here
        // and defer full validation to FinalizeBlock

        ResponseCheckTx {
            code: 0, // OK
            gas_wanted: 1,
            gas_used: 0,
            ..Default::default()
        }
    }

    fn prepare_proposal(&self, request: RequestPrepareProposal) -> ResponsePrepareProposal {
        info!(
            "ABCI prepare_proposal called with {} transactions",
            request.txs.len()
        );

        // In this integration, we don't modify the proposal
        // Mysticeti handles the actual block creation
        ResponsePrepareProposal { txs: request.txs }
    }

    fn process_proposal(&self, request: RequestProcessProposal) -> ResponseProcessProposal {
        info!(
            "ABCI process_proposal called with {} transactions",
            request.txs.len()
        );

        // Accept all proposals for now
        // In production, you might want to validate the proposal
        ResponseProcessProposal {
            status: tendermint_proto::v0_38::abci::response_process_proposal::ProposalStatus::Accept
                as i32,
        }
    }

    fn finalize_block(&self, request: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        info!(
            "ABCI finalize_block called with {} transactions",
            request.txs.len()
        );

        let mut tx_results = Vec::new();
        let mut events = Vec::new();

        // Process each transaction
        for (i, tx) in request.txs.iter().enumerate() {
            let tx_data = tx.to_vec();

            // Submit to Mysticeti consensus
            match tokio::runtime::Handle::current()
                .block_on(self.submit_transaction_to_mysticeti(tx_data.clone()))
            {
                Ok(_) => {
                    info!("Transaction {} processed successfully", i);
                    tx_results.push(tendermint_proto::v0_38::abci::ExecTxResult {
                        code: 0, // OK
                        data: vec![].into(),
                        log: "Transaction accepted by Mysticeti consensus".to_string(),
                        gas_wanted: 1,
                        gas_used: 0,
                        events: vec![],
                        ..Default::default()
                    });
                }
                Err(e) => {
                    warn!("Transaction {} failed: {}", i, e);
                    tx_results.push(tendermint_proto::v0_38::abci::ExecTxResult {
                        code: 1, // Error
                        data: vec![].into(),
                        log: format!("Transaction failed: {}", e),
                        gas_wanted: 1,
                        gas_used: 0,
                        events: vec![],
                        ..Default::default()
                    });
                }
            }
        }

        ResponseFinalizeBlock {
            events,
            tx_results,
            validator_updates: vec![],
            consensus_param_updates: None,
            app_hash: vec![].into(),
        }
    }

    fn query(&self, request: RequestQuery) -> ResponseQuery {
        info!(
            "ABCI query called: path={:?}, data={:?}",
            request.path, request.data
        );

        ResponseQuery {
            code: 0,
            value: b"Mysticeti query response".to_vec().into(),
            log: "Query handled by Mysticeti ABCI app".to_string(),
            ..Default::default()
        }
    }
}
