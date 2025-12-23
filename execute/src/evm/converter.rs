use consensus_core::{BlockAPI, CommittedSubDag};
use serde_json;

use rpc_shared_api::{
    BlockDigest, BlockRef, CommitRef, CommittedSubDag as EvmCommittedSubDag, SignedBlock,
    Transaction, VerifiedBlock,
};
pub fn create_evm_committed_subdag(subdag: CommittedSubDag) -> EvmCommittedSubDag {
    let CommittedSubDag {
        leader,
        blocks,
        rejected_transactions_by_block,
        timestamp_ms,
        commit_ref,
        reputation_scores_desc,
        decided_with_local_blocks,
        recovered_rejected_transactions,
    } = subdag;
    let blocks = blocks
        .into_iter()
        .map(|vb| {
            // Extract transactions from the block (available through Deref<Target = Block>)
            let consensus_txs = vb.transactions();

            // Convert consensus-core transactions to rpc-shared-api transactions
            let transactions: Vec<Transaction> = consensus_txs
                .iter()
                .map(|tx| Transaction::new(tx.data().to_vec()))
                .collect();

            // Create a SignedBlock with the transaction data
            // Since new_genesis is pub(crate) and fields are private, we use serde
            // to construct it from JSON, which works because SignedBlock implements Deserialize
            let reth_signed_block: SignedBlock = SignedBlock::new(transactions);

            // For the digest, we'll use a computed hash of the transactions for now
            // This can be enhanced when we have better access to the actual digest
            let digest_bytes = vb
                .transactions()
                .iter()
                .flat_map(|tx| tx.data().to_vec())
                .collect::<Vec<u8>>();
            let reth_digest = if !digest_bytes.is_empty() {
                // Use a simple hash of the transaction data as a digest
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                digest_bytes.hash(&mut hasher);
                let hash_value = hasher.finish();
                let mut digest_array = [0u8; 32];
                digest_array[..8].copy_from_slice(&hash_value.to_le_bytes());
                BlockDigest(digest_array)
            } else {
                BlockDigest::MIN
            };

            VerifiedBlock {
                block: reth_signed_block,
                digest: reth_digest,
            }
        })
        .collect();

    EvmCommittedSubDag {
        leader: BlockRef {
            digest: leader.digest.0,
            round: leader.round as u64,
        },
        blocks,
        timestamp_ms,
        commit_ref: CommitRef {
            index: commit_ref.index as usize,
            digest: commit_ref.digest.into_inner(),
        },
        reputation_scores_desc: reputation_scores_desc
            .into_iter()
            .map(|(authority_index, score)| (authority_index.value() as u32, score))
            .collect(),
    }
}
