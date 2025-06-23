// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

pub async fn test_transaction_sending() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting transaction test client...");

    // Test data - a simple transaction
    let test_transaction = b"Hello from test client!";
    let encoded_transaction =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, test_transaction);

    // RPC endpoints for the 4 validator nodes
    let endpoints = vec![
        "http://127.0.0.1:26657",
        "http://127.0.0.1:26658",
        "http://127.0.0.1:26659",
        "http://127.0.0.1:26660",
    ];

    // Send test transaction to each node
    for (i, endpoint) in endpoints.iter().enumerate() {
        let url = format!("{}/broadcast_tx_async", endpoint);

        info!("Sending transaction to node {} at {}", i, url);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&serde_json::json!({
                "transaction": encoded_transaction
            }))
            .send()
            .await?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            info!("Node {} response: {:?}", i, result);
        } else {
            info!("Node {} returned error status: {}", i, response.status());
        }

        // Wait a bit between requests
        sleep(Duration::from_millis(100)).await;
    }

    info!("Transaction test completed");
    Ok(())
}

pub async fn check_network_health() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Checking network health...");

    let endpoints = vec![
        "http://127.0.0.1:26657",
        "http://127.0.0.1:26658",
        "http://127.0.0.1:26659",
        "http://127.0.0.1:26660",
    ];

    for (i, endpoint) in endpoints.iter().enumerate() {
        let url = format!("{}/health", endpoint);

        let client = reqwest::Client::new();
        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Node {} is healthy", i);
                } else {
                    info!("Node {} returned status: {}", i, response.status());
                }
            }
            Err(e) => {
                info!("Node {} health check failed: {}", i, e);
            }
        }
    }

    Ok(())
}
