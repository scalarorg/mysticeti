// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use consensus_core;
use std::sync::Arc;
use tendermint_abci::Application;
use tendermint_proto::v0_38::abci::{
    RequestCheckTx, RequestFinalizeBlock, RequestInfo, RequestInitChain, RequestQuery,
    ResponseCheckTx, ResponseFinalizeBlock, ResponseInfo, ResponseInitChain, ResponseQuery,
};
use tokio::sync::mpsc;
use tracing::info;

#[derive(Clone)]
pub struct MysticetiAbciApp {
    transaction_sender: Arc<mpsc::Sender<Vec<u8>>>,
    consensus_output_sender: Arc<mpsc::Sender<consensus_core::CommittedSubDag>>,
}

impl MysticetiAbciApp {
    pub fn new(
        transaction_sender: mpsc::Sender<Vec<u8>>,
        consensus_output_sender: mpsc::Sender<consensus_core::CommittedSubDag>,
    ) -> Self {
        Self {
            transaction_sender: Arc::new(transaction_sender),
            consensus_output_sender: Arc::new(consensus_output_sender),
        }
    }
}

impl Application for MysticetiAbciApp {
    fn info(&self, _request: RequestInfo) -> ResponseInfo {
        ResponseInfo {
            data: "Mysticeti ABCI App".to_string(),
            version: "0.1.0".to_string(),
            app_version: 1,
            last_block_height: 0,
            last_block_app_hash: vec![].into(),
        }
    }

    fn init_chain(&self, _request: RequestInitChain) -> ResponseInitChain {
        ResponseInitChain::default()
    }

    fn check_tx(&self, request: RequestCheckTx) -> ResponseCheckTx {
        info!("ABCI check_tx called: {} bytes", request.tx.len());
        // Forward transaction to Mysticeti for validation
        let sender = self.transaction_sender.clone();
        let tx = request.tx.to_vec();
        tokio::spawn(async move {
            if let Err(e) = sender.send(tx).await {
                info!("Failed to forward transaction to Mysticeti: {}", e);
            }
        });

        ResponseCheckTx {
            code: 0,
            ..Default::default()
        }
    }

    fn finalize_block(&self, request: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        info!(
            "ABCI finalize_block called with {} transactions",
            request.txs.len()
        );

        // Forward all transactions to Mysticeti consensus
        let sender = self.transaction_sender.clone();
        for (i, tx) in request.txs.iter().enumerate() {
            info!("Processing transaction {}: {} bytes", i, tx.len());
            let tx_clone = tx.to_vec();
            let sender_clone = sender.clone();
            tokio::spawn(async move {
                if let Err(e) = sender_clone.send(tx_clone).await {
                    info!("Failed to forward transaction {} to Mysticeti: {}", i, e);
                }
            });
        }

        ResponseFinalizeBlock {
            events: vec![],
            tx_results: vec![],
            validator_updates: vec![],
            consensus_param_updates: None,
            app_hash: vec![].into(),
        }
    }

    fn query(&self, _request: RequestQuery) -> ResponseQuery {
        ResponseQuery {
            code: 0,
            value: b"Mysticeti query stub".to_vec().into(),
            ..Default::default()
        }
    }
}
