use anyhow::Result;
use consensus_config::{Authority, AuthorityKeyPair, Committee, NetworkKeyPair, ProtocolKeyPair};
use fastcrypto::{
    bls12381, ed25519,
    traits::{KeyPair as _, ToFromBytes as _},
};
use rand::{SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitteeConfig {
    pub epoch: u64,
    pub authorities: Vec<AuthorityConfig>,
    pub docker_network: NetworkConfig,
    pub quorum_threshold: usize,
    pub validity_threshold: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorityConfig {
    pub index: usize,
    pub stake: u64,
    pub hostname: String,
    pub address: String,
    pub authority_key: String,
    pub protocol_key: String,
    pub network_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub base_ip: String,
    pub start_ip: u8,
    pub end_ip: u8,
    pub port: u16,
}

/// Input configuration for validators
/// This struct represents the input file format for validators
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidatorConfigs {
    pub validators: Vec<ValidatorConfig>,
    pub epoch: Option<u64>,
}

/// Individual validator input configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidatorConfig {
    pub hostname: String,
    pub ip_address: String,
    pub port: u16,
    pub stake: u64,
    /// Hex-encoded authority private key (BLS12381). If not provided, will be generated randomly.
    #[serde(default)]
    pub authority_private_key: Option<String>,
    /// Hex-encoded protocol private key (Ed25519). If not provided, will be generated randomly.
    #[serde(default)]
    pub protocol_private_key: Option<String>,
    /// Hex-encoded network private key (Ed25519). If not provided, will be generated randomly.
    #[serde(default)]
    pub network_private_key: Option<String>,
}

/// Helper function to create an AuthorityKeyPair from hex-encoded private key bytes
fn authority_keypair_from_private_key(private_key_hex: &str) -> Result<AuthorityKeyPair> {
    // Decode hex string to bytes
    let private_key_bytes = hex::decode(private_key_hex)
        .map_err(|e| anyhow::anyhow!("Failed to decode authority private key from hex: {}", e))?;

    // Use BLS12381KeyPair::from_bytes which takes private key bytes and creates the keypair
    let bls_keypair =
        bls12381::min_sig::BLS12381KeyPair::from_bytes(&private_key_bytes).map_err(|e| {
            anyhow::anyhow!("Failed to create BLS keypair from private key bytes: {}", e)
        })?;

    // Use AuthorityKeyPair::new which accepts bls12381::min_sig::BLS12381KeyPair
    Ok(AuthorityKeyPair::new(bls_keypair))
}

/// Helper function to create a ProtocolKeyPair from hex-encoded private key bytes
fn protocol_keypair_from_private_key(private_key_hex: &str) -> Result<ProtocolKeyPair> {
    // Decode hex string to bytes
    let private_key_bytes = hex::decode(private_key_hex)
        .map_err(|e| anyhow::anyhow!("Failed to decode protocol private key from hex: {}", e))?;

    // For Ed25519, the private key is 32 bytes
    if private_key_bytes.len() != 32 {
        return Err(anyhow::anyhow!(
            "Protocol private key must be 32 bytes (64 hex chars), got {} bytes",
            private_key_bytes.len()
        ));
    }

    // Convert to fixed-size array
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&private_key_bytes);

    // Use Ed25519KeyPair::from_bytes which takes private key bytes and creates the keypair
    let ed25519_keypair = ed25519::Ed25519KeyPair::from_bytes(&key_bytes).map_err(|e| {
        anyhow::anyhow!(
            "Failed to create Ed25519 keypair from private key bytes: {}",
            e
        )
    })?;

    // Use ProtocolKeyPair::new which accepts ed25519::Ed25519KeyPair
    Ok(ProtocolKeyPair::new(ed25519_keypair))
}

/// Helper function to create a NetworkKeyPair from hex-encoded private key bytes
fn network_keypair_from_private_key(private_key_hex: &str) -> Result<NetworkKeyPair> {
    // Decode hex string to bytes
    let private_key_bytes = hex::decode(private_key_hex)
        .map_err(|e| anyhow::anyhow!("Failed to decode network private key from hex: {}", e))?;

    // For Ed25519, the private key is 32 bytes
    if private_key_bytes.len() != 32 {
        return Err(anyhow::anyhow!(
            "Network private key must be 32 bytes (64 hex chars), got {} bytes",
            private_key_bytes.len()
        ));
    }

    // Convert to fixed-size array
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&private_key_bytes);

    // Use Ed25519KeyPair::from_bytes which takes private key bytes and creates the keypair
    let ed25519_keypair = ed25519::Ed25519KeyPair::from_bytes(&key_bytes).map_err(|e| {
        anyhow::anyhow!(
            "Failed to create Ed25519 keypair from private key bytes: {}",
            e
        )
    })?;

    // Use NetworkKeyPair::new which accepts ed25519::Ed25519KeyPair
    Ok(NetworkKeyPair::new(ed25519_keypair))
}

// No helper functions needed - we'll generate keypairs using inner types directly

/// Generates validator configurations and saves them to a YAML file
///
/// This function generates validator configurations based on the provided parameters
/// and saves them to `config_path`. The generated file can then be used by
/// `generate_committees` to create a committee configuration.
pub fn generate_validators(
    config_path: &Path,
    authorities: usize,
    epoch: u64,
    stake: u64,
    docker_ips: &[String],
    network_ports: &[u16],
    hostname_prefix: &str,
) -> Result<()> {
    // Ensure we don't exceed the available Docker IPs
    if authorities > docker_ips.len() {
        return Err(anyhow::anyhow!(
            "Number of authorities ({}) exceeds available Docker IPs ({})",
            authorities,
            docker_ips.len()
        ));
    }

    if authorities > network_ports.len() {
        return Err(anyhow::anyhow!(
            "Number of authorities ({}) exceeds available network ports ({})",
            authorities,
            network_ports.len()
        ));
    }

    // Generate validator configs
    let mut validator_configs = ValidatorConfigs {
        validators: vec![],
        epoch: Some(epoch),
    };

    let mut rng = StdRng::from_seed([0; 32]);

    for i in 0..authorities {
        // Generate keypairs using inner types so we can access private keys
        // For AuthorityKeyPair (BLS12381)
        let bls_keypair = bls12381::min_sig::BLS12381KeyPair::generate(&mut rng);
        // For BLS12381, get the private key bytes from the privkey field
        // We need to clone the keypair since private() consumes it
        let bls_private_key = bls_keypair.copy().private();
        // The private key has a privkey field that we can serialize
        let authority_private_key_bytes = bls_private_key.privkey.to_bytes();
        let authority_private_key_hex = hex::encode(&authority_private_key_bytes);
        let authority_keypair = AuthorityKeyPair::new(bls_keypair);

        // For ProtocolKeyPair (Ed25519)
        let ed25519_protocol_keypair = ed25519::Ed25519KeyPair::generate(&mut rng);
        let protocol_private_key_bytes = ed25519_protocol_keypair.copy().private().0.to_bytes();
        let protocol_private_key_hex = hex::encode(protocol_private_key_bytes);
        let protocol_keypair = ProtocolKeyPair::new(ed25519_protocol_keypair);

        // For NetworkKeyPair (Ed25519)
        let ed25519_network_keypair = ed25519::Ed25519KeyPair::generate(&mut rng);
        let network_private_key_bytes = ed25519_network_keypair.copy().private().0.to_bytes();
        let network_private_key_hex = hex::encode(network_private_key_bytes);
        let network_keypair = NetworkKeyPair::new(ed25519_network_keypair);

        // Store the keypairs for potential future use, but we only need the hex strings for the config
        let _ = (authority_keypair, protocol_keypair, network_keypair);

        validator_configs.validators.push(ValidatorConfig {
            hostname: format!("{}{}", hostname_prefix, i),
            ip_address: docker_ips[i].clone(),
            port: network_ports[i],
            stake,
            authority_private_key: Some(authority_private_key_hex),
            protocol_private_key: Some(protocol_private_key_hex),
            network_private_key: Some(network_private_key_hex),
        });
    }

    // Write validator configs to config_path
    let validator_yaml = serde_yaml::to_string(&validator_configs)?;
    fs::write(config_path, validator_yaml)?;

    println!("Generated validator configs at: {}", config_path.display());
    println!("Configuration:");
    println!("  Epoch: {}", epoch);
    println!("  Authorities: {}", authorities);
    println!("  Stake per authority: {}", stake);

    Ok(())
}

/// Generates a committee configuration from validator configs
///
/// This function reads validator configurations from `config_path` and generates
/// a committee configuration in `committee_path`.
///
/// The validator config file should be in YAML format with the following structure:
///
/// ```yaml
/// validators:
///   - hostname: "validator-0"
///     ip_address: "127.0.0.1"
///     port: 26657
///     stake: 1000
///     authority_private_key: "hex-encoded-key"  # optional
///     protocol_private_key: "hex-encoded-key"   # optional
///     network_private_key: "hex-encoded-key"   # optional
///   - hostname: "validator-1"
///     ip_address: "127.0.0.1"
///     port: 26658
///     stake: 1000
/// epoch: 0  # optional
/// ```
///
/// If private keys are not provided, they will be generated randomly.
/// Private keys should be hex-encoded:
/// - Authority private key: 32 bytes (64 hex characters) for BLS12381
/// - Protocol private key: 32 bytes (64 hex characters) for Ed25519
/// - Network private key: 32 bytes (64 hex characters) for Ed25519
pub fn generate_committees(
    config_path: &Path,
    committee_path: &Path,
    epoch: Option<u64>,
) -> Result<()> {
    // Read and parse validator configs from config_path
    let content = fs::read_to_string(config_path)?;
    let validator_configs: ValidatorConfigs = serde_yaml::from_str(&content).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse validator config from {}: {}",
            config_path.display(),
            e
        )
    })?;

    if validator_configs.validators.is_empty() {
        return Err(anyhow::anyhow!(
            "Validator config must contain at least one validator"
        ));
    }

    // Generate committee config from validator configs
    generate_committee_from_validator_configs(&validator_configs, committee_path, epoch)?;

    println!(
        "Committee configuration generated from {} at: {}",
        config_path.display(),
        committee_path.display()
    );

    Ok(())
}

/// Internal helper function to generate committee config from validator configs
fn generate_committee_from_validator_configs(
    validator_configs: &ValidatorConfigs,
    committee_path: &Path,
    epoch_override: Option<u64>,
) -> Result<()> {
    if validator_configs.validators.is_empty() {
        return Err(anyhow::anyhow!(
            "Input validators config must contain at least one validator"
        ));
    }

    // Use epoch from override if provided, otherwise from validator_configs, otherwise default to 0
    let epoch = epoch_override.or(validator_configs.epoch).unwrap_or(0);

    let mut authorities_config = vec![];
    let mut rng = StdRng::from_seed([0; 32]);

    for (i, validator) in validator_configs.validators.iter().enumerate() {
        // Use provided private keys or generate new ones
        let authority_keypair = if let Some(ref priv_key) = validator.authority_private_key {
            authority_keypair_from_private_key(priv_key)?
        } else {
            AuthorityKeyPair::generate(&mut rng)
        };

        let protocol_keypair = if let Some(ref priv_key) = validator.protocol_private_key {
            protocol_keypair_from_private_key(priv_key)?
        } else {
            ProtocolKeyPair::generate(&mut rng)
        };

        let network_keypair = if let Some(ref priv_key) = validator.network_private_key {
            network_keypair_from_private_key(priv_key)?
        } else {
            NetworkKeyPair::generate(&mut rng)
        };

        let address = format!("/ip4/{}/udp/{}", validator.ip_address, validator.port);

        authorities_config.push(AuthorityConfig {
            index: i,
            stake: validator.stake,
            hostname: validator.hostname.clone(),
            address,
            authority_key: format!("{:?}", authority_keypair.public()),
            protocol_key: format!("{:?}", protocol_keypair.public()),
            network_key: format!("{:?}", network_keypair.public()),
        });
    }

    let num_authorities = authorities_config.len();
    let committee_config = CommitteeConfig {
        epoch,
        authorities: authorities_config,
        docker_network: NetworkConfig {
            base_ip: "172.20.0".to_string(),
            start_ip: 10,
            end_ip: 10 + num_authorities as u8 - 1,
            port: 26657,
        },
        quorum_threshold: (num_authorities * 2) / 3 + 1, // 2/3 + 1 for Byzantine fault tolerance
        validity_threshold: num_authorities / 2 + 1,     // 1/2 + 1 for validity
    };

    let yaml_content = serde_yaml::to_string(&committee_config)?;
    fs::write(committee_path, yaml_content)?;

    println!("Configuration:");
    println!("  Epoch: {}", committee_config.epoch);
    println!("  Authorities: {}", committee_config.authorities.len());
    println!("  Quorum threshold: {}", committee_config.quorum_threshold);
    println!(
        "  Validity threshold: {}",
        committee_config.validity_threshold
    );

    Ok(())
}

/// Loads a committee configuration from a YAML file
pub fn load_committees(
    config_path: &Path,
) -> Result<(Committee, Vec<(NetworkKeyPair, ProtocolKeyPair)>)> {
    let config_content = fs::read_to_string(config_path)?;
    let committee_config: CommitteeConfig = serde_yaml::from_str(&config_content)?;

    // Convert AuthorityConfig to Authority and generate keypairs
    let mut authorities = vec![];
    let mut key_pairs = vec![];
    let mut rng = StdRng::from_seed([0; 32]);

    for authority_config in committee_config.authorities {
        // Generate new keypairs (in a real scenario, you might want to load existing ones)
        let authority_keypair = AuthorityKeyPair::generate(&mut rng);
        let protocol_keypair = ProtocolKeyPair::generate(&mut rng);
        let network_keypair = NetworkKeyPair::generate(&mut rng);

        // Parse the address string
        let address = authority_config.address.parse()?;

        authorities.push(Authority {
            stake: authority_config.stake.into(),
            address,
            hostname: authority_config.hostname,
            authority_key: authority_keypair.public(),
            protocol_key: protocol_keypair.public(),
            network_key: network_keypair.public(),
        });
        key_pairs.push((network_keypair, protocol_keypair));
    }

    let committee = Committee::new(committee_config.epoch, authorities);

    println!(
        "Loaded committee configuration from: {}",
        config_path.display()
    );
    println!(
        "Committee size: {}, Epoch: {}",
        committee.size(),
        committee.epoch()
    );

    Ok((committee, key_pairs))
}

pub fn extract_peer_addresses(committee: &Committee) -> Vec<String> {
    committee
        .authorities()
        .map(|(_, authority)| {
            let address_str = authority.address.to_string();
            // Parse address in format "/ip4/172.20.0.11/udp/26657"
            if let Some(ip_port) = parse_ip_port_from_address(&address_str) {
                ip_port
            } else {
                // Fallback to original address if parsing fails
                address_str
            }
        })
        .collect()
}

/// Parse IP and port from address string in format "/ip4/172.20.0.11/udp/26657"
fn parse_ip_port_from_address(address: &str) -> Option<String> {
    // Split by "/" and extract IP and port
    let parts: Vec<&str> = address.split('/').collect();

    if parts.len() >= 5 && parts[1] == "ip4" && parts[3] == "udp" {
        let ip = parts[2];
        let port = parts[4];

        // Validate IP and port format
        if is_valid_ip(ip) && is_valid_port(port) {
            return Some(format!("{}:{}", ip, port));
        }
    }

    None
}

/// Check if string is a valid IP address
fn is_valid_ip(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }

    for part in parts {
        if part.parse::<u8>().is_err() {
            return false;
        }
    }
    true
}

/// Check if string is a valid port number
fn is_valid_port(port: &str) -> bool {
    port.parse::<u16>().is_ok()
}

/// Genesis configuration structure matching the expected JSON format
#[derive(Debug, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub validator_addresses: Vec<String>,
    pub consensus_public_keys: Vec<String>,
    pub voting_powers: Vec<String>,
    pub validator_network_addresses: Vec<String>,
    pub fullnode_network_addresses: Vec<String>,
    pub aptos_addresses: Vec<String>,
}

/// Generates a genesis configuration JSON file from validator configs
///
/// This function reads validator configurations from `config_path` and generates
/// a genesis configuration in JSON format at `genesis_path`.
///
/// The genesis config includes:
/// - validatorAddresses: Ethereum addresses (0x-prefixed)
/// - consensusPublicKeys: BLS12381 public keys in hex (96 hex chars)
/// - votingPowers: Stake values as strings
/// - validatorNetworkAddresses: Multiaddr format with noise-ik protocol
/// - fullnodeNetworkAddresses: Same as validatorNetworkAddresses
/// - aptosAddresses: Aptos addresses in hex (64 hex chars)
pub fn generate_genesis_config(config_path: &Path, genesis_path: &Path) -> Result<()> {
    // Read and parse validator configs from config_path
    let content = fs::read_to_string(config_path)?;
    let validator_configs: ValidatorConfigs = serde_yaml::from_str(&content).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse validator config from {}: {}",
            config_path.display(),
            e
        )
    })?;

    if validator_configs.validators.is_empty() {
        return Err(anyhow::anyhow!(
            "Validator config must contain at least one validator"
        ));
    }

    let mut validator_addresses = vec![];
    let mut consensus_public_keys = vec![];
    let mut voting_powers = vec![];
    let mut validator_network_addresses = vec![];
    let mut fullnode_network_addresses = vec![];
    let mut aptos_addresses = vec![];

    let mut rng = StdRng::from_seed([0; 32]);

    for validator in &validator_configs.validators {
        // Use provided private keys or generate new ones
        let authority_keypair = if let Some(ref priv_key) = validator.authority_private_key {
            authority_keypair_from_private_key(priv_key)?
        } else {
            AuthorityKeyPair::generate(&mut rng)
        };

        let network_keypair = if let Some(ref priv_key) = validator.network_private_key {
            network_keypair_from_private_key(priv_key)?
        } else {
            NetworkKeyPair::generate(&mut rng)
        };

        // Generate validator address from authority key
        let authority_pub_key = authority_keypair.public();
        let authority_pub_key_bytes = authority_pub_key.to_bytes();
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        authority_pub_key_bytes.hash(&mut hasher);
        let hash_value = hasher.finish();
        let mut eth_address_bytes = [0u8; 20];
        eth_address_bytes[..8].copy_from_slice(&hash_value.to_le_bytes());
        let validator_address = format!("0x{}", hex::encode(eth_address_bytes));

        // Generate Aptos address from authority key
        let mut aptos_address_bytes = [0u8; 32];
        aptos_address_bytes[..20].copy_from_slice(&eth_address_bytes);
        // Fill remaining bytes with hash
        aptos_address_bytes[20..28].copy_from_slice(&hash_value.to_le_bytes()[..8]);
        aptos_address_bytes[28..32].copy_from_slice(&hash_value.to_le_bytes()[..4]);
        let aptos_address = hex::encode(aptos_address_bytes);

        // Get consensus public key (BLS12381, 48 bytes = 96 hex chars)
        let consensus_pub_key = authority_keypair.public();
        let consensus_pub_key_bytes = consensus_pub_key.to_bytes();
        let consensus_public_key = hex::encode(consensus_pub_key_bytes);

        // Get network public key for multiaddr (Ed25519, 32 bytes = 64 hex chars)
        let network_pub_key_bytes = network_keypair.public().to_bytes();
        let network_public_key_hex = hex::encode(network_pub_key_bytes);

        // Build network address in format: /ip4/{ip}/tcp/{port}/noise-ik/{network_public_key}/handshake/0
        let validator_network_address = format!(
            "/ip4/{}/tcp/{}/noise-ik/{}/handshake/0",
            validator.ip_address, validator.port, network_public_key_hex
        );

        validator_addresses.push(validator_address);
        consensus_public_keys.push(consensus_public_key);
        voting_powers.push(validator.stake.to_string());
        validator_network_addresses.push(validator_network_address.clone());
        fullnode_network_addresses.push(validator_network_address);
        aptos_addresses.push(aptos_address);
    }

    let genesis_config = GenesisConfig {
        validator_addresses,
        consensus_public_keys,
        voting_powers,
        validator_network_addresses,
        fullnode_network_addresses,
        aptos_addresses,
    };

    // Write genesis config as JSON
    let json_content = serde_json::to_string_pretty(&genesis_config)?;
    fs::write(genesis_path, json_content)?;

    println!(
        "Genesis configuration generated from {} at: {}",
        config_path.display(),
        genesis_path.display()
    );
    println!("Configuration:");
    println!("  Validators: {}", genesis_config.validator_addresses.len());
    println!(
        "  Total voting power: {}",
        genesis_config
            .voting_powers
            .iter()
            .map(|v| v.parse::<u64>().unwrap_or(0))
            .sum::<u64>()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use consensus_config::{
        Authority, AuthorityKeyPair, Committee, NetworkKeyPair, ProtocolKeyPair, Stake,
    };

    // Helper function to create a test committee
    fn create_test_committee() -> Committee {
        let mut rng = StdRng::from_seed([0; 32]);
        let mut authorities = vec![];

        for i in 0..3 {
            let authority_keypair = AuthorityKeyPair::generate(&mut rng);
            let protocol_keypair = ProtocolKeyPair::generate(&mut rng);
            let network_keypair = NetworkKeyPair::generate(&mut rng);

            let address = format!("/ip4/172.20.0.{}/udp/{}", 10 + i, 26657 + i);
            let address = address.parse().unwrap();

            authorities.push(Authority {
                stake: Stake::from(1000u64),
                address,
                hostname: format!("test-node{}", i),
                authority_key: authority_keypair.public(),
                protocol_key: protocol_keypair.public(),
                network_key: network_keypair.public(),
            });
        }

        Committee::new(1, authorities)
    }

    #[test]
    fn test_extract_peer_addresses() {
        let committee = create_test_committee();
        let peer_addresses = extract_peer_addresses(&committee);

        assert_eq!(peer_addresses.len(), 3);
        assert_eq!(peer_addresses[0], "172.20.0.10:26657");
        assert_eq!(peer_addresses[1], "172.20.0.11:26658");
        assert_eq!(peer_addresses[2], "172.20.0.12:26659");
    }

    #[test]
    fn test_parse_ip_port_from_address_valid() {
        let test_cases = vec![
            ("/ip4/172.20.0.11/udp/26657", "172.20.0.11:26657"),
            ("/ip4/192.168.1.100/udp/8080", "192.168.1.100:8080"),
            ("/ip4/10.0.0.1/udp/9000", "10.0.0.1:9000"),
        ];

        for (input, expected) in test_cases {
            let result = parse_ip_port_from_address(input);
            assert_eq!(result, Some(expected.to_string()));
        }
    }

    #[test]
    fn test_parse_ip_port_from_address_invalid() {
        let test_cases = vec![
            "/ip4/172.20.0.11/tcp/26657", // Wrong protocol
            "/ip6/172.20.0.11/udp/26657", // Wrong IP version
            "/ip4/172.20.0.11/udp/",      // Missing port
            "/ip4/172.20.0.11/",          // Missing protocol and port
            "172.20.0.11:26657",          // Wrong format
            "",                           // Empty string
        ];

        for input in test_cases {
            let result = parse_ip_port_from_address(input);
            assert_eq!(result, None);
        }
    }

    #[test]
    fn test_parse_ip_port_from_address_malformed() {
        let test_cases = vec![
            "/ip4/172.20.0.11/udp/26657/extra", // Extra parts - should still parse correctly
            "/ip4/172.20.0.11/udp",             // Missing port
            "/ip4/udp/26657",                   // Missing IP
        ];

        for input in test_cases {
            let result = parse_ip_port_from_address(input);
            if input == "/ip4/172.20.0.11/udp/26657/extra" {
                // This should still parse correctly as we only need the first 5 parts
                assert_eq!(result, Some("172.20.0.11:26657".to_string()));
            } else {
                assert_eq!(result, None);
            }
        }
    }

    #[test]
    fn test_is_valid_ip() {
        let valid_ips = vec![
            "127.0.0.1",
            "192.168.1.1",
            "10.0.0.1",
            "172.20.0.11",
            "0.0.0.0",
            "255.255.255.255",
        ];

        for ip in valid_ips {
            assert!(is_valid_ip(ip), "IP {} should be valid", ip);
        }

        let invalid_ips = vec![
            "256.1.2.3", // Octet > 255
            "1.2.3.256", // Octet > 255
            "1.2.3",     // Too few octets
            "1.2.3.4.5", // Too many octets
            "1.2.3.a",   // Non-numeric
            "1.2.3.",    // Trailing dot
            ".1.2.3",    // Leading dot
            "",          // Empty string
        ];

        for ip in invalid_ips {
            assert!(!is_valid_ip(ip), "IP {} should be invalid", ip);
        }
    }

    #[test]
    fn test_is_valid_port() {
        let valid_ports = vec![
            "0",     // Min port
            "80",    // HTTP
            "443",   // HTTPS
            "26657", // Tendermint
            "65535", // Max port
        ];

        for port in valid_ports {
            assert!(is_valid_port(port), "Port {} should be valid", port);
        }

        let invalid_ports = vec![
            "65536", // Too large
            "99999", // Way too large
            "abc",   // Non-numeric
            "",      // Empty string
            "-1",    // Negative
        ];

        for port in invalid_ports {
            assert!(!is_valid_port(port), "Port {} should be invalid", port);
        }
    }

    #[test]
    fn test_extract_peer_addresses_with_invalid_addresses() {
        let mut rng = StdRng::from_seed([0; 32]);
        let mut authorities = vec![];

        // Create authorities with mixed valid and invalid addresses
        let addresses = vec![
            "/ip4/172.20.0.11/udp/26657",  // Valid
            "/ip4/127.0.0.1/udp/8080",     // Valid fallback
            "/ip4/192.168.1.100/udp/8080", // Valid
        ];

        for (i, address) in addresses.into_iter().enumerate() {
            let authority_keypair = AuthorityKeyPair::generate(&mut rng);
            let protocol_keypair = ProtocolKeyPair::generate(&mut rng);
            let network_keypair = NetworkKeyPair::generate(&mut rng);

            // Parse the address string
            let address = address.parse().unwrap();

            authorities.push(Authority {
                stake: Stake::from(1000u64),
                address,
                hostname: format!("test-node{}", i),
                authority_key: authority_keypair.public(),
                protocol_key: protocol_keypair.public(),
                network_key: network_keypair.public(),
            });
        }

        let committee = Committee::new(1, authorities);
        let peer_addresses = extract_peer_addresses(&committee);

        // Should have 3 addresses, all valid
        assert_eq!(peer_addresses.len(), 3);
        assert_eq!(peer_addresses[0], "172.20.0.11:26657");
        assert_eq!(peer_addresses[1], "127.0.0.1:8080");
        assert_eq!(peer_addresses[2], "192.168.1.100:8080");
    }

    #[test]
    fn test_edge_cases() {
        // Test with minimum valid values
        assert!(is_valid_ip("0.0.0.0"));
        assert!(is_valid_port("0"));

        // Test with maximum valid values
        assert!(is_valid_ip("255.255.255.255"));
        assert!(is_valid_port("65535"));

        // Test boundary cases
        assert!(!is_valid_ip("256.0.0.0"));
        assert!(!is_valid_port("65536"));
    }

    #[test]
    fn test_committee_config_serialization() {
        let config = CommitteeConfig {
            epoch: 1,
            authorities: vec![AuthorityConfig {
                index: 0,
                stake: 1000,
                hostname: "test-node".to_string(),
                address: "/ip4/172.20.0.11/udp/26657".to_string(),
                authority_key: "key1".to_string(),
                protocol_key: "key2".to_string(),
                network_key: "key3".to_string(),
            }],
            docker_network: NetworkConfig {
                base_ip: "172.20.0".to_string(),
                start_ip: 10,
                end_ip: 11,
                port: 26657,
            },
            quorum_threshold: 1,
            validity_threshold: 1,
        };

        // Test that the config can be serialized and deserialized
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: CommitteeConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(config.epoch, deserialized.epoch);
        assert_eq!(config.authorities.len(), deserialized.authorities.len());
        assert_eq!(
            config.authorities[0].address,
            deserialized.authorities[0].address
        );
    }

    #[test]
    fn test_generate_validators_and_committees() {
        use tempfile::tempdir;

        // Test with exact parameters:
        // First generate validators, then generate committee from validators
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("validators.yml");
        let committee_path = temp_dir.path().join("committees.yml");

        let authorities = 4;
        let epoch = 0;
        let stake = 1000;
        let ip_addresses = vec![
            "127.0.0.1".to_string(),
            "127.0.0.1".to_string(),
            "127.0.0.1".to_string(),
            "127.0.0.1".to_string(),
        ];
        let network_ports = vec![26657, 26658, 26659, 26660];
        let hostname_prefix = "fastevm-consensus";

        // First generate validators
        let result = generate_validators(
            &config_path,
            authorities,
            epoch,
            stake,
            &ip_addresses,
            &network_ports,
            hostname_prefix,
        );
        assert!(result.is_ok(), "generate_validators should succeed");
        assert!(config_path.exists(), "Validators file should be created");

        // Then generate committee from validators
        let result = generate_committees(&config_path, &committee_path, Some(epoch));
        assert!(result.is_ok(), "generate_committees should succeed");
        assert!(committee_path.exists(), "Committee file should be created");

        // Read and verify the committee file content
        let file_content = fs::read_to_string(&committee_path).unwrap();
        let config: CommitteeConfig = serde_yaml::from_str(&file_content).unwrap();

        // Verify basic configuration
        assert_eq!(config.epoch, epoch);
        assert_eq!(config.authorities.len(), authorities);
        assert_eq!(config.quorum_threshold, (authorities * 2) / 3 + 1); // Should be 3 for 4 authorities
        assert_eq!(config.validity_threshold, authorities / 2 + 1); // Should be 3 for 4 authorities

        // Verify each authority
        for (i, authority) in config.authorities.iter().enumerate() {
            assert_eq!(authority.index, i);
            assert_eq!(authority.stake, stake);
            assert_eq!(authority.hostname, format!("{}{}", hostname_prefix, i));
            assert_eq!(
                authority.address,
                format!("/ip4/{}/udp/{}", ip_addresses[i], network_ports[i])
            );
            // Verify keys are present (they should be non-empty strings)
            assert!(!authority.authority_key.is_empty());
            assert!(!authority.protocol_key.is_empty());
            assert!(!authority.network_key.is_empty());
        }

        // Verify docker network configuration
        assert_eq!(config.docker_network.base_ip, "172.20.0");
        assert_eq!(config.docker_network.start_ip, 10);
        assert_eq!(config.docker_network.end_ip, 10 + authorities as u8 - 1); // Should be 13 for 4 authorities
        assert_eq!(config.docker_network.port, 26657);

        // Verify the file can be loaded back using load_committees
        let (loaded_committee, _keypairs) = load_committees(&committee_path).unwrap();
        assert_eq!(loaded_committee.epoch(), epoch);
        assert_eq!(loaded_committee.size(), authorities);
    }

    #[test]
    fn test_generate_validators_error_cases() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("validators.yml");

        // Test: authorities exceed available IPs
        let result = generate_validators(
            &config_path,
            5, // 5 authorities
            0,
            1000,
            &vec!["127.0.0.1".to_string(); 4], // Only 4 IPs
            &vec![26657, 26658, 26659, 26660],
            "test",
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exceeds available Docker IPs")
        );

        // Test: authorities exceed available ports
        let result = generate_validators(
            &config_path,
            5, // 5 authorities
            0,
            1000,
            &vec!["127.0.0.1".to_string(); 5],
            &vec![26657, 26658, 26659], // Only 3 ports
            "test",
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exceeds available network ports")
        );
    }

    #[test]
    fn test_generate_validators_output_structure() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("validators.yml");

        let authorities = 4;
        let epoch = 0;
        let stake = 1000;
        let ip_addresses = vec![
            "172.20.0.10".to_string(),
            "172.20.0.11".to_string(),
            "172.20.0.12".to_string(),
            "172.20.0.13".to_string(),
        ];
        let network_ports = vec![26657, 26657, 26657, 26657];
        let hostname_prefix = "fastevm-consensus";

        // Generate validators
        let result = generate_validators(
            &config_path,
            authorities,
            epoch,
            stake,
            &ip_addresses,
            &network_ports,
            hostname_prefix,
        );

        // Verify generation succeeded
        assert!(result.is_ok(), "generate_validators should succeed");
        assert!(config_path.exists(), "Validators file should be created");

        // Read and verify the file content
        let file_content = fs::read_to_string(&config_path).unwrap();
        let validator_configs: ValidatorConfigs = serde_yaml::from_str(&file_content).unwrap();

        // Verify basic configuration
        assert_eq!(validator_configs.epoch, Some(epoch));
        assert_eq!(validator_configs.validators.len(), authorities);

        // Verify each validator has non-null required fields
        for (i, validator) in validator_configs.validators.iter().enumerate() {
            // Verify hostname is not empty
            assert!(
                !validator.hostname.is_empty(),
                "Validator {} hostname should not be empty",
                i
            );
            assert_eq!(
                validator.hostname,
                format!("{}{}", hostname_prefix, i),
                "Validator {} hostname should match expected format",
                i
            );

            // Verify ip_address is not empty
            assert!(
                !validator.ip_address.is_empty(),
                "Validator {} ip_address should not be empty",
                i
            );
            assert_eq!(
                validator.ip_address, ip_addresses[i],
                "Validator {} ip_address should match input",
                i
            );

            // Verify port is set
            assert_eq!(
                validator.port, network_ports[i],
                "Validator {} port should match input",
                i
            );

            // Verify stake is set
            assert_eq!(
                validator.stake, stake,
                "Validator {} stake should match input",
                i
            );

            // Verify private keys are generated and not null
            assert!(
                validator.authority_private_key.is_some(),
                "Validator {} authority_private_key should be generated",
                i
            );
            assert!(
                !validator.authority_private_key.as_ref().unwrap().is_empty(),
                "Validator {} authority_private_key should not be empty",
                i
            );
            // Verify it's valid hex (64 chars for 32 bytes)
            let authority_key_hex = validator.authority_private_key.as_ref().unwrap();
            assert_eq!(
                authority_key_hex.len(),
                64,
                "Validator {} authority_private_key should be 64 hex characters (32 bytes)",
                i
            );
            assert!(
                hex::decode(authority_key_hex).is_ok(),
                "Validator {} authority_private_key should be valid hex",
                i
            );

            assert!(
                validator.protocol_private_key.is_some(),
                "Validator {} protocol_private_key should be generated",
                i
            );
            assert!(
                !validator.protocol_private_key.as_ref().unwrap().is_empty(),
                "Validator {} protocol_private_key should not be empty",
                i
            );
            // Verify it's valid hex (64 chars for 32 bytes)
            let protocol_key_hex = validator.protocol_private_key.as_ref().unwrap();
            assert_eq!(
                protocol_key_hex.len(),
                64,
                "Validator {} protocol_private_key should be 64 hex characters (32 bytes)",
                i
            );
            assert!(
                hex::decode(protocol_key_hex).is_ok(),
                "Validator {} protocol_private_key should be valid hex",
                i
            );

            assert!(
                validator.network_private_key.is_some(),
                "Validator {} network_private_key should be generated",
                i
            );
            assert!(
                !validator.network_private_key.as_ref().unwrap().is_empty(),
                "Validator {} network_private_key should not be empty",
                i
            );
            // Verify it's valid hex (64 chars for 32 bytes)
            let network_key_hex = validator.network_private_key.as_ref().unwrap();
            assert_eq!(
                network_key_hex.len(),
                64,
                "Validator {} network_private_key should be 64 hex characters (32 bytes)",
                i
            );
            assert!(
                hex::decode(network_key_hex).is_ok(),
                "Validator {} network_private_key should be valid hex",
                i
            );
        }

        // Verify the first validator matches the expected format from the user's example
        let first_validator = &validator_configs.validators[0];
        assert_eq!(first_validator.hostname, "fastevm-consensus0");
        assert_eq!(first_validator.ip_address, "172.20.0.10");
        assert_eq!(first_validator.port, 26657);
        assert_eq!(first_validator.stake, 1000);
        // Verify private keys are generated (not null)
        assert!(
            first_validator.authority_private_key.is_some(),
            "authority_private_key should be generated"
        );
        assert!(
            first_validator.protocol_private_key.is_some(),
            "protocol_private_key should be generated"
        );
        assert!(
            first_validator.network_private_key.is_some(),
            "network_private_key should be generated"
        );
        // Verify they are valid hex strings (64 chars for 32 bytes)
        assert_eq!(
            first_validator
                .authority_private_key
                .as_ref()
                .unwrap()
                .len(),
            64
        );
        assert_eq!(
            first_validator.protocol_private_key.as_ref().unwrap().len(),
            64
        );
        assert_eq!(
            first_validator.network_private_key.as_ref().unwrap().len(),
            64
        );
    }

    #[test]
    fn test_generate_committees_missing_validators_file() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.yml");
        let committee_path = temp_dir.path().join("committees.yml");

        // Test: should fail when validators file doesn't exist
        let result = generate_committees(&config_path, &committee_path, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_committees_from_validator_configs() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("validators_input.yml");
        let committee_path = temp_dir.path().join("committees.yml");

        // Create a test input file
        let input_config = ValidatorConfigs {
            validators: vec![
                ValidatorConfig {
                    hostname: "validator-0".to_string(),
                    ip_address: "127.0.0.1".to_string(),
                    port: 26657,
                    stake: 1000,
                    authority_private_key: None,
                    protocol_private_key: None,
                    network_private_key: None,
                },
                ValidatorConfig {
                    hostname: "validator-1".to_string(),
                    ip_address: "127.0.0.1".to_string(),
                    port: 26658,
                    stake: 2000,
                    authority_private_key: None,
                    protocol_private_key: None,
                    network_private_key: None,
                },
                ValidatorConfig {
                    hostname: "validator-2".to_string(),
                    ip_address: "192.168.1.1".to_string(),
                    port: 26659,
                    stake: 1500,
                    authority_private_key: None,
                    protocol_private_key: None,
                    network_private_key: None,
                },
            ],
            epoch: Some(5),
        };

        let input_yaml = serde_yaml::to_string(&input_config).unwrap();
        fs::write(&config_path, input_yaml).unwrap();

        // Generate committee from input
        let result = generate_committees(&config_path, &committee_path, None);
        assert!(
            result.is_ok(),
            "generate_committees should succeed when reading from config"
        );

        // Verify output file was created
        assert!(committee_path.exists(), "Committee file should be created");

        // Read and verify the output file
        let output_content = fs::read_to_string(&committee_path).unwrap();
        let committee_config: CommitteeConfig = serde_yaml::from_str(&output_content).unwrap();

        // Verify basic configuration
        assert_eq!(committee_config.epoch, 5); // Should use epoch from input file
        assert_eq!(committee_config.authorities.len(), 3);
        assert_eq!(committee_config.quorum_threshold, (3 * 2) / 3 + 1); // Should be 3 for 3 authorities
        assert_eq!(committee_config.validity_threshold, 3 / 2 + 1); // Should be 2 for 3 authorities

        // Verify each authority matches input
        assert_eq!(committee_config.authorities[0].hostname, "validator-0");
        assert_eq!(committee_config.authorities[0].stake, 1000);
        assert_eq!(
            committee_config.authorities[0].address,
            "/ip4/127.0.0.1/udp/26657"
        );

        assert_eq!(committee_config.authorities[1].hostname, "validator-1");
        assert_eq!(committee_config.authorities[1].stake, 2000);
        assert_eq!(
            committee_config.authorities[1].address,
            "/ip4/127.0.0.1/udp/26658"
        );

        assert_eq!(committee_config.authorities[2].hostname, "validator-2");
        assert_eq!(committee_config.authorities[2].stake, 1500);
        assert_eq!(
            committee_config.authorities[2].address,
            "/ip4/192.168.1.1/udp/26659"
        );

        // Verify keys are generated
        for authority in &committee_config.authorities {
            assert!(!authority.authority_key.is_empty());
            assert!(!authority.protocol_key.is_empty());
            assert!(!authority.network_key.is_empty());
        }
    }

    #[test]
    fn test_generate_committees_from_validator_configs_with_parameter_epoch() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("validators_input.yml");
        let committee_path = temp_dir.path().join("committees.yml");

        // Create a test input file without epoch
        let input_config = ValidatorConfigs {
            validators: vec![ValidatorConfig {
                hostname: "validator-0".to_string(),
                ip_address: "127.0.0.1".to_string(),
                port: 26657,
                stake: 1000,
                authority_private_key: None,
                protocol_private_key: None,
                network_private_key: None,
            }],
            epoch: None,
        };

        let input_yaml = serde_yaml::to_string(&input_config).unwrap();
        fs::write(&config_path, input_yaml).unwrap();

        // Generate committee with epoch parameter
        let result = generate_committees(&config_path, &committee_path, Some(10));
        assert!(result.is_ok());

        let output_content = fs::read_to_string(&committee_path).unwrap();
        let committee_config: CommitteeConfig = serde_yaml::from_str(&output_content).unwrap();

        // Should use epoch from parameter
        assert_eq!(committee_config.epoch, 10);
    }

    #[test]
    fn test_generate_committees_from_validator_configs_empty_validators() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("validators_input.yml");
        let committee_path = temp_dir.path().join("committees.yml");

        // Create a test input file with empty validators
        let input_config = ValidatorConfigs {
            validators: vec![],
            epoch: Some(0),
        };

        let input_yaml = serde_yaml::to_string(&input_config).unwrap();
        fs::write(&config_path, input_yaml).unwrap();

        // Should fail with empty validators
        let result = generate_committees(&config_path, &committee_path, None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must contain at least one validator")
        );
    }

    #[test]
    fn test_generate_genesis_config() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("validators.yml");
        let genesis_path = temp_dir.path().join("genesis.json");

        // First generate validators
        let authorities = 4;
        let epoch = 0;
        let stake = 20000;
        let ip_addresses = vec![
            "127.0.0.1".to_string(),
            "127.0.0.1".to_string(),
            "127.0.0.1".to_string(),
            "127.0.0.1".to_string(),
        ];
        let network_ports = vec![2024, 2025, 2026, 6180];
        let hostname_prefix = "fastevm-consensus";

        let result = generate_validators(
            &config_path,
            authorities,
            epoch,
            stake,
            &ip_addresses,
            &network_ports,
            hostname_prefix,
        );
        assert!(result.is_ok(), "generate_validators should succeed");
        assert!(config_path.exists(), "Validators file should be created");

        // Then generate genesis config from validators
        let result = generate_genesis_config(&config_path, &genesis_path);
        assert!(
            result.is_ok(),
            "generate_genesis_config should succeed: {:?}",
            result
        );
        assert!(genesis_path.exists(), "Genesis file should be created");

        // Read and verify the genesis file content
        let file_content = fs::read_to_string(&genesis_path).unwrap();
        let genesis_config: GenesisConfig = serde_json::from_str(&file_content).unwrap();

        // Verify basic structure
        assert_eq!(genesis_config.validator_addresses.len(), authorities);
        assert_eq!(genesis_config.consensus_public_keys.len(), authorities);
        assert_eq!(genesis_config.voting_powers.len(), authorities);
        assert_eq!(
            genesis_config.validator_network_addresses.len(),
            authorities
        );
        assert_eq!(genesis_config.fullnode_network_addresses.len(), authorities);
        assert_eq!(genesis_config.aptos_addresses.len(), authorities);

        // Verify each validator's data
        for i in 0..authorities {
            let validator_addr = &genesis_config.validator_addresses[i];
            let consensus_key = &genesis_config.consensus_public_keys[i];
            let voting_power = &genesis_config.voting_powers[i];
            let network_addr = &genesis_config.validator_network_addresses[i];
            let fullnode_addr = &genesis_config.fullnode_network_addresses[i];
            let aptos_addr = &genesis_config.aptos_addresses[i];
            // Verify validator address format (0x + 40 hex chars = 42 chars total)
            assert!(
                validator_addr.starts_with("0x"),
                "Validator address {} should start with 0x",
                i
            );
            assert_eq!(
                validator_addr.len(),
                42,
                "Validator address {} should be 42 chars (0x + 40 hex)",
                i
            );
            assert!(
                hex::decode(&validator_addr[2..]).is_ok(),
                "Validator address {} should be valid hex",
                i
            );

            // Verify consensus public key format (BLS12381 public key)
            // BLS12381 public keys can be 48 or 96 bytes depending on compression
            assert!(
                consensus_key.len() >= 96,
                "Consensus public key {} should be at least 96 hex chars",
                i
            );
            assert!(
                hex::decode(consensus_key).is_ok(),
                "Consensus public key {} should be valid hex",
                i
            );
            let decoded_key = hex::decode(consensus_key).unwrap();
            assert!(
                decoded_key.len() >= 48,
                "Consensus public key {} should decode to at least 48 bytes, got {}",
                i,
                decoded_key.len()
            );

            // Verify voting power
            assert_eq!(
                voting_power, "20000",
                "Voting power {} should match stake",
                i
            );

            // Verify network address format: /ip4/{ip}/tcp/{port}/noise-ik/{network_public_key}/handshake/0
            assert!(
                network_addr.starts_with("/ip4/"),
                "Network address {} should start with /ip4/",
                i
            );
            assert!(
                network_addr.contains("/tcp/"),
                "Network address {} should contain /tcp/",
                i
            );
            assert!(
                network_addr.contains("/noise-ik/"),
                "Network address {} should contain /noise-ik/",
                i
            );
            assert!(
                network_addr.ends_with("/handshake/0"),
                "Network address {} should end with /handshake/0",
                i
            );
            assert_eq!(
                network_addr, fullnode_addr,
                "Fullnode address {} should match validator network address",
                i
            );

            // Verify IP and port in network address
            let ip_port_part = network_addr
                .strip_prefix("/ip4/")
                .unwrap()
                .split("/tcp/")
                .collect::<Vec<_>>();
            assert_eq!(
                ip_port_part.len(),
                2,
                "Network address {} should have IP and port",
                i
            );
            assert_eq!(
                ip_port_part[0], ip_addresses[i],
                "Network address {} should have correct IP",
                i
            );
            assert_eq!(
                ip_port_part[1].split("/noise-ik/").next().unwrap(),
                network_ports[i].to_string(),
                "Network address {} should have correct port",
                i
            );

            // Verify Aptos address format (64 hex chars = 32 bytes)
            assert_eq!(
                aptos_addr.len(),
                64,
                "Aptos address {} should be 64 hex chars (32 bytes)",
                i
            );
            assert!(
                hex::decode(aptos_addr).is_ok(),
                "Aptos address {} should be valid hex",
                i
            );
            assert_eq!(
                hex::decode(aptos_addr).unwrap().len(),
                32,
                "Aptos address {} should decode to 32 bytes",
                i
            );
        }

        // Verify total voting power
        let total_voting_power: u64 = genesis_config
            .voting_powers
            .iter()
            .map(|v| v.parse::<u64>().unwrap())
            .sum();
        assert_eq!(
            total_voting_power,
            stake * authorities as u64,
            "Total voting power should match sum of all stakes"
        );
    }

    #[test]
    fn test_generate_genesis_config_from_existing_validators() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("validators_input.yml");
        let genesis_path = temp_dir.path().join("genesis.json");

        // Create a test input file with validators
        let input_config = ValidatorConfigs {
            validators: vec![
                ValidatorConfig {
                    hostname: "validator-0".to_string(),
                    ip_address: "127.0.0.1".to_string(),
                    port: 2024,
                    stake: 20000,
                    authority_private_key: None,
                    protocol_private_key: None,
                    network_private_key: None,
                },
                ValidatorConfig {
                    hostname: "validator-1".to_string(),
                    ip_address: "127.0.0.1".to_string(),
                    port: 2025,
                    stake: 20000,
                    authority_private_key: None,
                    protocol_private_key: None,
                    network_private_key: None,
                },
            ],
            epoch: Some(0),
        };

        let input_yaml = serde_yaml::to_string(&input_config).unwrap();
        fs::write(&config_path, input_yaml).unwrap();

        // Generate genesis config
        let result = generate_genesis_config(&config_path, &genesis_path);
        assert!(
            result.is_ok(),
            "generate_genesis_config should succeed: {:?}",
            result
        );
        assert!(genesis_path.exists(), "Genesis file should be created");

        // Read and verify the genesis file content
        let file_content = fs::read_to_string(&genesis_path).unwrap();
        let genesis_config: GenesisConfig = serde_json::from_str(&file_content).unwrap();

        // Verify we have 2 validators
        assert_eq!(genesis_config.validator_addresses.len(), 2);
        assert_eq!(genesis_config.consensus_public_keys.len(), 2);
        assert_eq!(genesis_config.voting_powers.len(), 2);

        // Verify all arrays have the same length
        assert_eq!(genesis_config.validator_network_addresses.len(), 2);
        assert_eq!(genesis_config.fullnode_network_addresses.len(), 2);
        assert_eq!(genesis_config.aptos_addresses.len(), 2);

        // Verify addresses are unique
        let mut seen_addresses = std::collections::HashSet::new();
        for addr in &genesis_config.validator_addresses {
            assert!(
                seen_addresses.insert(addr),
                "Validator addresses should be unique"
            );
        }
    }

    #[test]
    fn test_generate_genesis_config_empty_validators() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("validators_input.yml");
        let genesis_path = temp_dir.path().join("genesis.json");

        // Create a test input file with empty validators
        let input_config = ValidatorConfigs {
            validators: vec![],
            epoch: Some(0),
        };

        let input_yaml = serde_yaml::to_string(&input_config).unwrap();
        fs::write(&config_path, input_yaml).unwrap();

        // Should fail with empty validators
        let result = generate_genesis_config(&config_path, &genesis_path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must contain at least one validator")
        );
    }

    #[test]
    fn test_generate_genesis_config_with_provided_keys() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("validators_input.yml");
        let genesis_path = temp_dir.path().join("genesis.json");

        // Generate deterministic keys for testing using inner types so we can access private keys
        let mut rng = StdRng::from_seed([0; 32]);

        let bls_keypair = bls12381::min_sig::BLS12381KeyPair::generate(&mut rng);
        let bls_private_key = bls_keypair.copy().private();
        let authority_private_key_bytes = bls_private_key.privkey.to_bytes();
        let authority_private_key_hex = hex::encode(&authority_private_key_bytes);
        let authority_keypair = AuthorityKeyPair::new(bls_keypair);

        let ed25519_network_keypair = ed25519::Ed25519KeyPair::generate(&mut rng);
        let network_private_key_bytes = ed25519_network_keypair.copy().private().0.to_bytes();
        let network_private_key_hex = hex::encode(network_private_key_bytes);
        let network_keypair = NetworkKeyPair::new(ed25519_network_keypair);

        // Create a test input file with provided keys
        let input_config = ValidatorConfigs {
            validators: vec![ValidatorConfig {
                hostname: "validator-0".to_string(),
                ip_address: "127.0.0.1".to_string(),
                port: 2024,
                stake: 20000,
                authority_private_key: Some(authority_private_key_hex),
                protocol_private_key: None,
                network_private_key: Some(network_private_key_hex),
            }],
            epoch: Some(0),
        };

        let input_yaml = serde_yaml::to_string(&input_config).unwrap();
        fs::write(&config_path, input_yaml).unwrap();

        // Generate genesis config
        let result = generate_genesis_config(&config_path, &genesis_path);
        assert!(
            result.is_ok(),
            "generate_genesis_config should succeed: {:?}",
            result
        );

        // Read and verify the genesis file content
        let file_content = fs::read_to_string(&genesis_path).unwrap();
        let genesis_config: GenesisConfig = serde_json::from_str(&file_content).unwrap();

        // Verify the consensus public key matches the provided authority key
        let expected_consensus_key = hex::encode(authority_keypair.public().to_bytes());
        assert_eq!(
            genesis_config.consensus_public_keys[0], expected_consensus_key,
            "Consensus public key should match provided authority key"
        );

        // Verify network address contains the correct network public key
        let expected_network_key = hex::encode(network_keypair.public().to_bytes());
        assert!(
            genesis_config.validator_network_addresses[0].contains(&expected_network_key),
            "Network address should contain the provided network public key"
        );
    }
}
