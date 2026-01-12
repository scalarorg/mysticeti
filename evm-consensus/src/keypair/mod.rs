use anyhow::Result;
use consensus_config::{AuthorityKeyPair, NetworkKeyPair, ProtocolKeyPair};
use fastcrypto::{bls12381, ed25519, traits::ToFromBytes as _};

/// Helper function to create an AuthorityKeyPair from hex-encoded private key bytes
pub fn authority_keypair_from_private_key(private_key_hex: &str) -> Result<AuthorityKeyPair> {
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
pub fn protocol_keypair_from_private_key(private_key_hex: &str) -> Result<ProtocolKeyPair> {
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
pub fn network_keypair_from_private_key(private_key_hex: &str) -> Result<NetworkKeyPair> {
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
