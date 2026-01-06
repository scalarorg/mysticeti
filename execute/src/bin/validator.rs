// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, command};
use eyre::Result;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The working directory where the validator node will store its data.
    #[clap(long, value_name = "DIR", default_value = "validator-node")]
    working_directory: PathBuf,

    /// The authority index for this validator node (0-3 for 4-node network).
    #[clap(long, value_name = "INDEX", default_value = "0")]
    authority_index: u32,

    /// The RPC port for this validator node.
    #[clap(long, value_name = "PORT", default_value = "26657")]
    rpc_port: u16,

    /// The ABCI port for this validator node.
    #[clap(long, value_name = "PORT")]
    abci_port: Option<u16>,

    /// Comma-separated list of peer addresses (e.g., "172.20.0.11:26657,172.20.0.12:26657")
    #[clap(long, value_name = "ADDRESSES")]
    peer_addresses: Option<String>,

    /// Enable debug logging.
    #[clap(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Nice colored error messages.
    color_eyre::install()?;

    // Parse command line arguments
    let args = Args::parse();

    // Setup logging
    let log_level = if args.debug {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    let filter = EnvFilter::builder()
        .with_default_directive(log_level.into())
        .from_env_lossy();
    fmt().with_env_filter(filter).init();

    // Create working directory
    std::fs::create_dir_all(&args.working_directory)?;

    // Determine ABCI port
    let abci_port = args
        .abci_port
        .unwrap_or(26670 + args.authority_index as u16);

    info!(
        "Starting single Mysticeti validator node {} on RPC port {} and ABCI port {}",
        args.authority_index, args.rpc_port, abci_port
    );

    Ok(())
}
