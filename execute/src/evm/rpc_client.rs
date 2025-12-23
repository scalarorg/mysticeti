use anyhow::Result;

use axum::http;
use consensus_core::{CertifiedBlocksOutput, CommittedSubDag};
use jsonrpsee_core::client::{ClientT, SubscriptionClientT};
use jsonrpsee_http_client::HttpClientBuilder;
use mysten_metrics::monitored_mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use reth_rpc_layer::{AuthClientLayer, JwtSecret, secret_to_bearer_header};
use rpc_shared_api::{MysticetiConsensusApiClient, RawTransactionApiClient};
use tracing::{debug, error, info};

use crate::evm::create_evm_committed_subdag;

type RawTransactions = Vec<Vec<u8>>;

pub struct RawTransactionClient {
    jwt_secret: String,
    execution_http_url: String,
    execution_ws_url: String,
    tx_transactions: UnboundedSender<RawTransactions>,
}

impl RawTransactionClient {
    pub fn new(
        jwt_secret: String,
        execution_http_url: String,
        execution_ws_url: String,
        tx_transactions: UnboundedSender<RawTransactions>,
    ) -> Self {
        Self {
            jwt_secret,
            execution_http_url,
            execution_ws_url,
            tx_transactions,
        }
    }
    pub fn jwt_secret(&self) -> JwtSecret {
        match JwtSecret::from_hex(&self.jwt_secret) {
            Ok(jwt_secret) => jwt_secret,
            Err(err) => {
                error!(
                    "Invalid JWT secret format: {:?}.JWT secret should be a 32-byte hex string starting with 0x",
                    err
                );
                panic!("JWT secret parsing failed: {:?}", err);
            }
        }
    }
    pub fn http_url(&self) -> String {
        self.execution_http_url.clone()
    }
    pub fn ws_url(&self) -> String {
        self.execution_ws_url.clone()
    }
    pub fn http_client(
        &self,
    ) -> impl jsonrpsee_core::client::ClientT + SubscriptionClientT + Clone + Send + Sync + Unpin + 'static
    {
        // Create a middleware that adds a new JWT token to every request.
        let secret_layer = AuthClientLayer::new(self.jwt_secret());
        let middleware = tower::ServiceBuilder::default().layer(secret_layer);
        HttpClientBuilder::default()
            .set_http_middleware(middleware)
            .build(self.http_url())
            .expect("Failed to create http client")
    }

    pub async fn ws_client(&self) -> impl SubscriptionClientT + Send + Sync + Unpin + 'static {
        let mut auth_header = secret_to_bearer_header(&self.jwt_secret());
        // The header value should not be visible in logs for security.
        auth_header.set_sensitive(true);
        let url = self.ws_url();
        debug!(
            "Creating ws client with url: {} and auth header: {:?}",
            url,
            auth_header.to_str().unwrap()
        );
        let mut headers = http::HeaderMap::new();
        headers.insert(http::header::AUTHORIZATION, auth_header);

        HttpClientBuilder::default()
            .set_headers(headers)
            .build(url)
            .expect("Failed to create ws client")
    }

    pub async fn start(
        &mut self,
        mut commit_receiver: UnboundedReceiver<CommittedSubDag>,
    ) -> Result<()> {
        info!("Starting Engine API client...");

        // Try to connect to the execution client
        let ws_client = self.ws_client().await;
        let mut tx_subscriber = RawTransactionApiClient::subscribe_raw_transactions(&ws_client)
            .await
            .expect("failed to subscribe");

        info!("Engine API client started successfully");
        let txs_sender = self.tx_transactions.clone();
        let transaction_handler = tokio::spawn(async move {
            let mut total_received_txs = 0;
            loop {
                if let Some(may_txs) = tx_subscriber.next().await {
                    match may_txs {
                        Ok(txs) => {
                            total_received_txs += txs.len();
                            info!(
                                "Received transactions: {:?}. Total received transactions: {:?}",
                                txs.len(),
                                total_received_txs
                            );
                            if let Err(e) = txs_sender.send(txs) {
                                error!("Error sending transaction to consensus: {:?}", e);
                            }
                        }
                        Err(e) => {
                            error!("Error receiving transaction: {:?}", e);
                        }
                    }
                }
            }
        });
        let http_client = self.http_client();
        let subdag_handler = tokio::spawn(async move {
            let mut total_committed_txs = 0;
            let mut total_sent_txs = 0;
            // let mut buffer = Vec::new();
            // let mut last_sent = std::time::Instant::now();
            loop {
                if let Some(subdag) = commit_receiver.recv().await {
                    //TODO: findout why timestamp_ms is 0
                    let timestamp_ms = subdag.timestamp_ms;
                    let commit_index = subdag.commit_ref.index;
                    let leader_round = subdag.leader.round;
                    let evm_subdag = create_evm_committed_subdag(subdag);
                    let current_len = evm_subdag.len();
                    total_committed_txs += current_len;
                    if let Err(e) = MysticetiConsensusApiClient::submit_committed_subdag(
                        &http_client,
                        evm_subdag,
                    )
                    .await
                    {
                        error!("submit_committed_subdags failed: {:?}", e);
                    }
                    // Log every 100 commits or when the first non-empty subdag is committed
                    if total_committed_txs > 0
                        && (commit_index % 100 == 0 || total_committed_txs == current_len)
                    {
                        info!(
                            "Received committed subdag with timestamp: {:?}, Commit Index {:?}, Leader round {:?}, Tx count {:?}, Total txs {:?}",
                            timestamp_ms,
                            commit_index,
                            leader_round,
                            current_len,
                            total_committed_txs
                        );
                    }
                }
            }
        });
        let _ = tokio::join!(transaction_handler, subdag_handler);

        info!("Engine API client stopped");
        Ok(())
    }
}
