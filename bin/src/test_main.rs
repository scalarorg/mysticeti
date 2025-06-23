// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, command};
use eyre::Result;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt};

mod test_client;

use test_client::{check_network_health, test_transaction_sending};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    operation: Operation,
}

#[derive(Parser)]
enum Operation {
    /// Send test transactions to all validator nodes
    SendTransactions,
    /// Check health of all validator nodes
    CheckHealth,
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

    match args.operation {
        Operation::SendTransactions => {
            println!("Sending test transactions to validator network...");
            test_transaction_sending()
                .await
                .map_err(|e| eyre::eyre!("{}", e))?;
        }
        Operation::CheckHealth => {
            println!("Checking validator network health...");
            check_network_health()
                .await
                .map_err(|e| eyre::eyre!("{}", e))?;
        }
    }

    Ok(())
}
