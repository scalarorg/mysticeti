// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{fs, path::PathBuf, sync::Arc};

use clap::{command, Parser};
use eyre::{Context, Result};
use futures::future;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{fmt, EnvFilter};

use consensus_config::{local_committee_and_keys, AuthorityIndex, Parameters};
use consensus_core::{
    Clock, CommitConsumer, ConsensusAuthority, TransactionIndex, TransactionVerifier,
    ValidationError,
};
use mysten_metrics::RegistryService;
use prometheus::Registry;
use sui_protocol_config::{ConsensusNetwork, ProtocolConfig};

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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    operation: Operation,
}

#[derive(Parser)]
enum Operation {
    /// Start 4 consensus authority nodes for testing.
    StartFourNodes {
        /// The working directory where the nodes will store their data.
        #[clap(long, value_name = "DIR", default_value = "four-nodes-test")]
        working_directory: PathBuf,
    },
    /// Start a single consensus authority node for testing.
    StartSingleNode {
        /// The authority index of this node (0-3).
        #[clap(long, value_name = "INT", default_value = "0")]
        authority_index: u32,
        /// The working directory where the node will store its data.
        #[clap(long, value_name = "DIR", default_value = "single-node-test")]
        working_directory: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Nice colored error messages.
    color_eyre::install()?;
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    fmt().with_env_filter(filter).init();

    // Parse the command line arguments.
    match Args::parse().operation {
        Operation::StartFourNodes { working_directory } => {
            start_four_nodes(working_directory).await?
        }
        Operation::StartSingleNode {
            authority_index,
            working_directory,
        } => start_single_node(authority_index, working_directory).await?,
    }

    Ok(())
}

/// Start 4 consensus authority nodes for testing.
async fn start_four_nodes(working_directory: PathBuf) -> Result<()> {
    tracing::info!(
        "Starting 4 consensus authority nodes in directory: {}",
        working_directory.display()
    );

    // Create working directory
    fs::create_dir_all(&working_directory).wrap_err(format!(
        "Failed to create directory '{}'",
        working_directory.display()
    ))?;

    // Generate committee and keypairs for 4 nodes
    let committee_size = 4;
    let (committee, keypairs) = local_committee_and_keys(0, vec![1; committee_size]);

    // Create parameters with default values
    let parameters = Parameters::default();

    // Create registry service for metrics
    let registry_service = RegistryService::new(Registry::new());

    // Start all 4 nodes
    let mut handles = Vec::new();
    for i in 0..committee_size {
        let authority = AuthorityIndex::new_for_test(i as u32);
        let node_dir = working_directory.join(format!("node-{}", i));
        let db_path = node_dir.join("consensus.db");

        // Create directory for this node
        fs::create_dir_all(&node_dir)?;

        // Get keypairs for this node
        let (network_keypair, protocol_keypair) = &keypairs[i];

        // Create parameters with correct db path
        let mut node_parameters = parameters.clone();
        node_parameters.db_path = db_path;

        // Create commit consumer
        let (commit_consumer, _commit_receiver, _block_receiver) = CommitConsumer::new(0);

        // Start the authority node
        let authority_node = ConsensusAuthority::start(
            ConsensusNetwork::Anemo,
            authority,
            committee.clone(),
            node_parameters,
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

        handles.push(tokio::spawn(async move {
            tracing::info!("Node {} started successfully", authority);
            // Keep the node running
            tokio::signal::ctrl_c().await.unwrap();
            tracing::info!("Shutting down node {}", authority);
            authority_node.stop().await;
        }));
    }

    tracing::info!("All 4 consensus authority nodes started successfully!");
    tracing::info!("Press Ctrl+C to stop all nodes");

    // Wait for all nodes to complete
    future::join_all(handles).await;

    Ok(())
}

/// Start a single consensus authority node for testing.
async fn start_single_node(authority_index: u32, working_directory: PathBuf) -> Result<()> {
    tracing::info!(
        "Starting single consensus authority node {} in directory: {}",
        authority_index,
        working_directory.display()
    );

    // Create working directory
    fs::create_dir_all(&working_directory).wrap_err(format!(
        "Failed to create directory '{}'",
        working_directory.display()
    ))?;

    // Generate committee and keypairs for 4 nodes (we need a full committee)
    let committee_size = 4;
    let (committee, keypairs) = local_committee_and_keys(0, vec![1; committee_size]);

    // Create parameters with default values
    let parameters = Parameters::default();

    // Create directory for this node
    let node_dir = working_directory.join(format!("node-{}", authority_index));
    let db_path = node_dir.join("consensus.db");
    fs::create_dir_all(&node_dir)?;

    // Get keypairs for this node
    let (network_keypair, protocol_keypair) = &keypairs[authority_index as usize];

    // Create parameters with correct db path
    let mut node_parameters = parameters.clone();
    node_parameters.db_path = db_path;

    // Create commit consumer
    let (commit_consumer, _commit_receiver, _block_receiver) = CommitConsumer::new(0);

    // Create registry service for metrics
    let registry_service = RegistryService::new(Registry::new());

    // Start the authority node
    let authority_node = ConsensusAuthority::start(
        ConsensusNetwork::Anemo,
        AuthorityIndex::new_for_test(authority_index),
        committee,
        node_parameters,
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

    tracing::info!("Node {} started successfully", authority_index);
    tracing::info!("Press Ctrl+C to stop the node");

    // Keep the node running
    tokio::signal::ctrl_c().await.unwrap();
    tracing::info!("Shutting down node {}", authority_index);
    authority_node.stop().await;

    Ok(())
}
