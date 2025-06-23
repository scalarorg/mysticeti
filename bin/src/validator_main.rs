// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, command};
use eyre::Result;
use std::path::PathBuf;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt};

mod abci_app;
mod validator_network;
mod validator_node;

use validator_network::ValidatorNetwork;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The working directory where the validator nodes will store their data.
    #[clap(long, value_name = "DIR", default_value = "validator-network")]
    working_directory: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Nice colored error messages.
    color_eyre::install()?;

    // Setup logging
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    fmt().with_env_filter(filter).init();

    // Parse command line arguments
    let args = Args::parse();

    // Create and start the validator network
    let mut network = ValidatorNetwork::new(args.working_directory);

    // Start the network
    network
        .start()
        .await
        .map_err(|e| eyre::eyre!("Failed to start validator network: {}", e))?;

    // Print RPC endpoints
    println!("\n=== Validator Network Started ===");
    println!("RPC Endpoints:");
    for (i, endpoint) in network.get_rpc_endpoints().iter().enumerate() {
        println!("  Node {}: {}/broadcast_tx_async", i, endpoint);
    }
    println!("\nHealth check endpoints:");
    for (i, endpoint) in network.get_rpc_endpoints().iter().enumerate() {
        println!("  Node {}: {}/health", i, endpoint);
    }
    println!("\nPress Ctrl+C to stop the network");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await.unwrap();

    // Stop the network
    network.stop().await;

    println!("Validator network stopped");
    Ok(())
}
