use consensus_config::Authority;
use consensus_core::{BlockAPI, CommittedSubDag};
use fastcrypto::hash::{Blake2b256, HashFunction};
use rpc_shared_api::{
    BlockDigest, BlockRef, CommitRef, CommittedSubDag as EvmCommittedSubDag, SignedBlock,
    Transaction, VerifiedBlock,
};

use crate::evm::committee::generate_validator_sui_address;

pub fn create_evm_committed_subdag(
    subdag: CommittedSubDag,
    leader_authority: Option<&Authority>,
) -> EvmCommittedSubDag {
    let CommittedSubDag {
        leader,
        blocks,
        rejected_transactions_by_block: _rejected_transactions_by_block,
        timestamp_ms,
        commit_ref,
        reputation_scores_desc,
        decided_with_local_blocks: _decided_with_local_blocks,
        recovered_rejected_transactions: _recovered_rejected_transactions,
    } = subdag;

    let leader_address = leader_authority
        .and_then(|authority| {
            let sui_address = generate_validator_sui_address(&authority.network_key);
            hex::decode(&sui_address)
                .map(|sui_address_bytes| format!("0x{}", hex::encode(sui_address_bytes.as_slice())))
                .ok()
        })
        .unwrap_or_default();

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
                // Use Blake2b256 hash of the transaction data as a digest for cryptographic strength
                let mut hasher = Blake2b256::new();
                hasher.update(&digest_bytes);
                let digest_array: [u8; 32] = hasher.finalize().into();
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
            leader_address,
            digest: leader.digest.0,
            round: leader.round as u64,
        },
        blocks,
        timestamp_ms,
        commit_ref: CommitRef {
            round: commit_ref.index as usize,
            digest: commit_ref.digest.into_inner(),
        },
        reputation_scores_desc: reputation_scores_desc
            .into_iter()
            .map(|(authority_index, score)| (authority_index.value() as u32, score))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use consensus_config::AuthorityIndex;
    use consensus_core::{
        CommitDigest, CommitRef as ConsensusCommitRef, TestBlock,
        Transaction as ConsensusTransaction, VerifiedBlock as ConsensusVerifiedBlock,
    };

    fn create_test_block(
        round: u32,
        author: u32,
        transactions: Vec<ConsensusTransaction>,
    ) -> ConsensusVerifiedBlock {
        ConsensusVerifiedBlock::new_for_test(
            TestBlock::new(round, author)
                .set_transactions(transactions)
                .build(),
        )
    }

    fn create_test_committed_subdag(
        leader_round: u32,
        leader_author: u32,
        blocks: Vec<ConsensusVerifiedBlock>,
        timestamp_ms: u64,
        commit_index: u32,
    ) -> CommittedSubDag {
        let leader_ref = if blocks.is_empty() {
            let default_block = create_test_block(leader_round, leader_author, vec![]);
            default_block.reference()
        } else {
            blocks.first().unwrap().reference()
        };
        let commit_ref = ConsensusCommitRef {
            index: commit_index,
            digest: CommitDigest::MIN,
        };

        CommittedSubDag::new(leader_ref, blocks, timestamp_ms, commit_ref)
    }

    #[test]
    fn test_create_evm_committed_subdag_empty_blocks() {
        let subdag = create_test_committed_subdag(1, 0, vec![], 1000, 1);

        let evm_subdag = create_evm_committed_subdag(subdag, None);

        assert_eq!(evm_subdag.blocks.len(), 0);
        assert_eq!(evm_subdag.timestamp_ms, 1000);
        assert_eq!(evm_subdag.commit_ref.round, 1);
    }

    #[test]
    fn test_create_evm_committed_subdag_single_block_no_transactions() {
        let block = create_test_block(1, 0, vec![]);
        let leader_ref = block.reference();
        let subdag = create_test_committed_subdag(1, 0, vec![block], 2000, 2);

        let evm_subdag = create_evm_committed_subdag(subdag, None);

        assert_eq!(evm_subdag.blocks.len(), 1);
        assert_eq!(evm_subdag.blocks[0].digest, BlockDigest::MIN);
        assert_eq!(evm_subdag.leader.round, leader_ref.round as u64);
        assert_eq!(evm_subdag.leader.digest, leader_ref.digest.0);
        assert_eq!(evm_subdag.timestamp_ms, 2000);
    }

    #[test]
    fn test_create_evm_committed_subdag_single_block_with_transactions() {
        let tx1 = ConsensusTransaction::new(vec![1, 2, 3, 4]);
        let tx2 = ConsensusTransaction::new(vec![5, 6, 7, 8]);
        let block = create_test_block(2, 1, vec![tx1.clone(), tx2.clone()]);
        let leader_ref = block.reference();
        let subdag = create_test_committed_subdag(2, 1, vec![block], 3000, 3);

        let evm_subdag = create_evm_committed_subdag(subdag, None);

        assert_eq!(evm_subdag.blocks.len(), 1);
        assert_eq!(evm_subdag.blocks[0].block.transactions().len(), 2);
        assert_eq!(
            evm_subdag.blocks[0].block.transactions()[0].data(),
            tx1.data()
        );
        assert_eq!(
            evm_subdag.blocks[0].block.transactions()[1].data(),
            tx2.data()
        );
        assert_ne!(evm_subdag.blocks[0].digest, BlockDigest::MIN);
        assert_eq!(evm_subdag.leader.round, leader_ref.round as u64);
        assert_eq!(evm_subdag.leader.digest, leader_ref.digest.0);
    }

    #[test]
    fn test_create_evm_committed_subdag_multiple_blocks() {
        let block1 = create_test_block(1, 0, vec![ConsensusTransaction::new(vec![1, 2, 3])]);
        let block2 = create_test_block(1, 1, vec![ConsensusTransaction::new(vec![4, 5, 6])]);
        let block3 = create_test_block(2, 0, vec![ConsensusTransaction::new(vec![7, 8, 9])]);
        let leader_ref = block1.reference();
        let subdag = create_test_committed_subdag(1, 0, vec![block1, block2, block3], 4000, 4);

        let evm_subdag = create_evm_committed_subdag(subdag, None);

        assert_eq!(evm_subdag.blocks.len(), 3);
        assert_eq!(evm_subdag.blocks[0].block.transactions().len(), 1);
        assert_eq!(evm_subdag.blocks[1].block.transactions().len(), 1);
        assert_eq!(evm_subdag.blocks[2].block.transactions().len(), 1);
        assert_eq!(evm_subdag.leader.round, leader_ref.round as u64);
    }

    #[test]
    fn test_create_evm_committed_subdag_with_reputation_scores() {
        let block = create_test_block(3, 2, vec![]);
        let leader_ref = block.reference();
        let mut subdag = create_test_committed_subdag(3, 2, vec![block], 5000, 5);
        // Set reputation scores directly on the struct
        subdag.reputation_scores_desc = vec![
            (AuthorityIndex::new_for_test(0), 100),
            (AuthorityIndex::new_for_test(1), 200),
            (AuthorityIndex::new_for_test(2), 150),
        ];

        let evm_subdag = create_evm_committed_subdag(subdag, None);

        assert_eq!(evm_subdag.reputation_scores_desc.len(), 3);
        assert_eq!(evm_subdag.reputation_scores_desc[0], (0, 100));
        assert_eq!(evm_subdag.reputation_scores_desc[1], (1, 200));
        assert_eq!(evm_subdag.reputation_scores_desc[2], (2, 150));
        assert_eq!(evm_subdag.leader.round, leader_ref.round as u64);
    }

    #[test]
    fn test_create_evm_committed_subdag_digest_calculation() {
        // Test that blocks with same transactions produce same digest
        let tx = ConsensusTransaction::new(vec![10, 20, 30]);
        let block1 = create_test_block(4, 0, vec![tx.clone()]);
        let block2 = create_test_block(4, 1, vec![tx.clone()]);
        let leader_ref = block1.reference();
        let subdag = create_test_committed_subdag(4, 0, vec![block1, block2], 6000, 6);

        let evm_subdag = create_evm_committed_subdag(subdag, None);

        assert_eq!(evm_subdag.blocks.len(), 2);
        // Both blocks have the same transaction data, so they should have the same digest
        assert_eq!(evm_subdag.blocks[0].digest, evm_subdag.blocks[1].digest);
        assert_eq!(evm_subdag.leader.round, leader_ref.round as u64);
        assert_eq!(evm_subdag.leader.digest, leader_ref.digest.0);
    }

    #[test]
    fn test_create_evm_committed_subdag_different_transactions_different_digests() {
        let block1 = create_test_block(5, 0, vec![ConsensusTransaction::new(vec![1, 2, 3])]);
        let block2 = create_test_block(5, 1, vec![ConsensusTransaction::new(vec![4, 5, 6])]);
        let _leader_ref = block1.reference();
        let subdag = create_test_committed_subdag(5, 0, vec![block1, block2], 7000, 7);

        let evm_subdag = create_evm_committed_subdag(subdag, None);

        assert_eq!(evm_subdag.blocks.len(), 2);
        // Blocks with different transactions should have different digests
        assert_ne!(evm_subdag.blocks[0].digest, evm_subdag.blocks[1].digest);
    }

    #[test]
    fn test_create_evm_committed_subdag_commit_ref_conversion() {
        let block = create_test_block(6, 0, vec![]);
        let leader_ref = block.reference();
        let commit_ref = ConsensusCommitRef::new(100, CommitDigest::MIN);
        let subdag = CommittedSubDag::new(leader_ref, vec![block], 8000, commit_ref);

        let evm_subdag = create_evm_committed_subdag(subdag, None);

        assert_eq!(evm_subdag.commit_ref.round, 100);
        assert_eq!(evm_subdag.commit_ref.digest, CommitDigest::MIN.into_inner());
        assert_eq!(evm_subdag.leader.round, leader_ref.round as u64);
    }

    #[test]
    fn test_create_evm_committed_subdag_large_transaction_data() {
        let large_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let block = create_test_block(7, 0, vec![ConsensusTransaction::new(large_data.clone())]);
        let _leader_ref = block.reference();
        let subdag = create_test_committed_subdag(7, 0, vec![block], 9000, 8);

        let evm_subdag = create_evm_committed_subdag(subdag, None);

        assert_eq!(evm_subdag.blocks.len(), 1);
        assert_eq!(evm_subdag.blocks[0].block.transactions().len(), 1);
        assert_eq!(
            evm_subdag.blocks[0].block.transactions()[0].data(),
            large_data.as_slice()
        );
        assert_ne!(evm_subdag.blocks[0].digest, BlockDigest::MIN);
    }
}
