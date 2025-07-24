// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use std::path::PathBuf;
use tracing::{error, info};

use consensus_config::{Committee, NetworkKeyPair, ProtocolKeyPair};
use mysten_metrics::RegistryService;

use crate::validator::enhanced_node::EnhancedValidatorNode;

#[derive(Parser)]
#[command(name = "enhanced_validator")]
#[command(about = "Enhanced validator node with CometBFT + Mysticeti integration")]
struct Args {
    /// Authority index for this validator
    #[arg(long, default_value = "0")]
    authority_index: u32,

    /// Working directory for node data
    #[arg(long, default_value = "./data")]
    working_directory: String,

    /// CometBFT RPC port
    #[arg(long, default_value = "26657")]
    cometbft_rpc_port: u16,

    /// Mysticeti gRPC port
    #[arg(long, default_value = "50051")]
    mysticeti_grpc_port: u16,

    /// Number of validators in the committee
    #[arg(long, default_value = "4")]
    num_validators: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    info!("Starting enhanced validator with args: {:?}", args);

    // Create working directory
    let working_directory = PathBuf::from(args.working_directory);
    std::fs::create_dir_all(&working_directory)?;

    // Create registry service
    let registry_service = RegistryService::new(prometheus::Registry::new());

    // Create committee and keypairs
    let (committee, keypairs) = create_test_committee(args.num_validators);

    // Create and start the enhanced validator node
    let mut node = EnhancedValidatorNode::new(
        args.authority_index,
        working_directory,
        args.cometbft_rpc_port,
        args.mysticeti_grpc_port,
    );

    // Start the node
    node.start(committee, keypairs, registry_service).await?;

    info!("Enhanced validator node started successfully");
    info!(
        "CometBFT RPC available on port: {}",
        node.get_cometbft_rpc_port()
    );
    info!(
        "Mysticeti gRPC available on port: {}",
        node.get_mysticeti_grpc_port()
    );
    info!("ABCI server available on port: {}", node.get_abci_port());

    // Keep the node running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down enhanced validator node...");
    node.stop().await;

    Ok(())
}

fn create_test_committee(
    num_validators: u32,
) -> (Committee, Vec<(NetworkKeyPair, ProtocolKeyPair)>) {
    let mut authorities = Vec::new();
    let mut keypairs = Vec::new();

    for i in 0..num_validators {
        let network_keypair = NetworkKeyPair::new(fastcrypto::ed25519::Ed25519KeyPair::generate());
        let protocol_keypair =
            ProtocolKeyPair::new(fastcrypto::ed25519::Ed25519KeyPair::generate());

        authorities.push(consensus_config::Authority {
            stake: 1,
            address: mysten_network::Multiaddr::empty(),
            hostname: format!("test_host_{}", i),
            authority_key: fastcrypto::bls12381::min_sig::BLS12381KeyPair::generate().public(),
            network_key: network_keypair.public(),
            protocol_key: protocol_keypair.public(),
        });

        keypairs.push((network_keypair, protocol_keypair));
    }

    let committee = Committee::new(0, authorities);
    (committee, keypairs)
}
