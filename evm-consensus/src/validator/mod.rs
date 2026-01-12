mod config;
use anyhow::Result;
pub use config::{ValidatorConfig, ValidatorConfigs};
use consensus_config::{AuthorityKeyPair, NetworkKeyPair, ProtocolKeyPair};
use fastcrypto::{bls12381, ed25519, traits::KeyPair as _};
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_yaml;
use std::fs;
use std::path::Path;

/// Generates validator configurations and saves them to a YAML file
///
/// This function generates validator configurations based on the provided parameters
/// and saves them to `config_path`. The generated file can then be used by
/// `generate_committees` to create a committee configuration.
pub fn generate_validator(
    config_path: &Path,
    network_key_path: &Path,
    hostname: String,
    ip_address: String,
    port: u16,
    stake: u64,
) -> Result<()> {
    println!("Generated validator configs at: {}", config_path.display());
    println!("Configuration:");
    println!("  Hostname: {}", hostname);
    println!("  IP Address: {}", &ip_address);
    println!("  Port: {}", port);
    println!("  Stake per authority: {}", stake);
    // Generate validator config
    let mut rng = StdRng::from_entropy();

    // Generate keypairs using inner types so we can access private keys
    // For AuthorityKeyPair (BLS12381)
    let bls_keypair = bls12381::min_sig::BLS12381KeyPair::generate(&mut rng);
    // For BLS12381, get the private key bytes from the privkey field
    // We need to clone the keypair since private() consumes it
    let bls_private_key = bls_keypair.copy().private();
    // The private key has a privkey field that we can serialize
    let authority_private_key_bytes = bls_private_key.privkey.to_bytes();
    let authority_private_key_hex = hex::encode(&authority_private_key_bytes);
    let _authority_keypair = AuthorityKeyPair::new(bls_keypair);

    // For ProtocolKeyPair (Ed25519)
    let ed25519_protocol_keypair = ed25519::Ed25519KeyPair::generate(&mut rng);
    let protocol_private_key_bytes = ed25519_protocol_keypair.copy().private().0.to_bytes();
    let protocol_private_key_hex = hex::encode(protocol_private_key_bytes);
    let _protocol_keypair = ProtocolKeyPair::new(ed25519_protocol_keypair);

    // For NetworkKeyPair (Ed25519)
    let ed25519_network_keypair = ed25519::Ed25519KeyPair::generate(&mut rng);
    let network_private_key_bytes = ed25519_network_keypair.copy().private().0.to_bytes();
    let network_private_key_hex = hex::encode(network_private_key_bytes);
    let network_keypair = NetworkKeyPair::new(ed25519_network_keypair);
    let network_public_key = network_keypair.public();
    let network_public_key_hex = hex::encode(network_public_key.to_bytes());

    let validator_config = ValidatorConfig {
        hostname,
        ip_address,
        port,
        stake,
        authority_private_key: Some(authority_private_key_hex),
        protocol_private_key: Some(protocol_private_key_hex),
        network_private_key: Some(network_private_key_hex),
    };

    // Write validator configs to config_path
    let validator_yaml = serde_yaml::to_string(&validator_config)?;
    fs::write(config_path, validator_yaml)?;
    fs::write(network_key_path, network_public_key_hex)?;

    Ok(())
}

pub fn load_validator(config_path: &Path) -> Result<ValidatorConfig> {
    let validator_yaml = fs::read_to_string(config_path)?;
    let validator_config: ValidatorConfig = serde_yaml::from_str(&validator_yaml)?;
    Ok(validator_config)
}

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
        let _authority_keypair = AuthorityKeyPair::new(bls_keypair);

        // For ProtocolKeyPair (Ed25519)
        let ed25519_protocol_keypair = ed25519::Ed25519KeyPair::generate(&mut rng);
        let protocol_private_key_bytes = ed25519_protocol_keypair.copy().private().0.to_bytes();
        let protocol_private_key_hex = hex::encode(protocol_private_key_bytes);
        let _protocol_keypair = ProtocolKeyPair::new(ed25519_protocol_keypair);

        // For NetworkKeyPair (Ed25519)
        let ed25519_network_keypair = ed25519::Ed25519KeyPair::generate(&mut rng);
        let network_private_key_bytes = ed25519_network_keypair.copy().private().0.to_bytes();
        let network_private_key_hex = hex::encode(network_private_key_bytes);
        let _network_keypair = NetworkKeyPair::new(ed25519_network_keypair);

        // Store the keypairs for potential future use, but we only need the hex strings for the config
        // let _ = (authority_keypair, protocol_keypair, network_keypair);

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

mod tests {
    use super::*;

    #[test]
    fn test_generate_validator() {
        let config_path = Path::new("test_validator.yml");
        let network_key_path = Path::new("test_network_key.txt");
        let hostname = "validator-0".to_string();
        let ip_address = "127.0.0.1".to_string();
        let port = 26657;
        let stake = 1000;
        let result = generate_validator(
            config_path,
            network_key_path,
            hostname.clone(),
            ip_address.clone(),
            port.clone(),
            stake.clone(),
        );
        assert!(result.is_ok());
        assert!(config_path.exists());
        assert!(network_key_path.exists());
        // ---- Read & print generated files ----
        let config_content =
            fs::read_to_string(config_path).expect("Failed to read validator config file");
        let network_key_content =
            fs::read_to_string(network_key_path).expect("Failed to read network key file");

        println!("===== Validator Config (YAML) =====");
        println!("{}", config_content);

        println!("===== Network Key =====");
        println!("{}", network_key_content);
        let validator_config = load_validator(config_path).unwrap();
        assert_eq!(validator_config.hostname, hostname);
        assert_eq!(validator_config.ip_address, ip_address);
        assert_eq!(validator_config.port, port);
        assert_eq!(validator_config.stake, stake);
    }
}
