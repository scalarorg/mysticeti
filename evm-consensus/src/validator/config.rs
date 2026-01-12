use crate::{
    authority_keypair_from_private_key, network_keypair_from_private_key,
    protocol_keypair_from_private_key,
};
use consensus_config::{AuthorityKeyPair, NetworkKeyPair, ProtocolKeyPair};
use rand::{SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
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

impl ValidatorConfig {
    pub fn authority_keypair(&self) -> anyhow::Result<AuthorityKeyPair> {
        match &self.authority_private_key {
            Some(hex) => authority_keypair_from_private_key(hex),
            None => {
                let mut rng = StdRng::from_entropy();
                let authority_keypair = AuthorityKeyPair::generate(&mut rng);
                Ok(authority_keypair)
            }
        }
    }

    pub fn protocol_keypair(&self) -> anyhow::Result<ProtocolKeyPair> {
        match &self.protocol_private_key {
            Some(hex) => protocol_keypair_from_private_key(hex),
            None => {
                let mut rng = StdRng::from_entropy();
                let protocol_keypair = ProtocolKeyPair::generate(&mut rng);
                Ok(protocol_keypair)
            }
        }
    }

    pub fn network_keypair(&self) -> anyhow::Result<NetworkKeyPair> {
        match &self.network_private_key {
            Some(hex) => network_keypair_from_private_key(hex),
            None => {
                let mut rng = StdRng::from_entropy();
                let network_keypair = NetworkKeyPair::generate(&mut rng);
                Ok(network_keypair)
            }
        }
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_validator_config() {
        let validator_config = ValidatorConfig {
            hostname: "validator-0".to_string(),
            ip_address: "127.0.0.1".to_string(),
            port: 26657,
            stake: 1000,
            authority_private_key: None,
            protocol_private_key: None,
            network_private_key: None,
        };

        let authority_keypair = validator_config.authority_keypair().unwrap();
        let protocol_keypair = validator_config.protocol_keypair().unwrap();
        let network_keypair = validator_config.network_keypair().unwrap();

        assert_eq!(authority_keypair.public().to_bytes().len(), 96);
        assert_eq!(protocol_keypair.public().to_bytes().len(), 32);
        assert_eq!(network_keypair.public().to_bytes().len(), 32);

        assert_eq!(
            authority_keypair.public().to_bytes(),
            protocol_keypair.public().to_bytes()
        );
        assert_eq!(
            authority_keypair.public().to_bytes(),
            network_keypair.public().to_bytes()
        );
        assert_eq!(
            protocol_keypair.public().to_bytes(),
            network_keypair.public().to_bytes()
        );
    }
}
