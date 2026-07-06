//! L1-001 — Attestation aggregation into an entropy seed.
//!
//! N accepted attestations → one reproducible 32-byte seed, independent of
//! arrival order. The hashed inputs are the attesters' Ed25519 ENVELOPE
//! SIGNATURES — bytes the aggregator can neither forge nor predict (forging
//! one would require the attester's private key). Hashing challenge ids or
//! any other aggregator-chosen bytes would hand the seed to the aggregator.
//!
//! ⚠️ HONEST LIMIT (documented in the spec §3.2): the aggregator still
//! chooses WHICH attestations to include — subset-selection grinding is
//! OPEN at L1-001 and is exactly what L1-002 (VDF / passive beacon /
//! threshold signature) must close. Do not claim non-biasability here.

use sha2::{Digest, Sha256};

/// Aggregate attestation signatures into an order-independent seed.
///
/// Sorting canonicalizes the input set: the same set of signatures yields
/// the same seed regardless of arrival order.
pub fn aggregate_seed<'a, I>(signatures: I) -> [u8; 32]
where
    I: IntoIterator<Item = &'a [u8]>,
{
    let mut sigs: Vec<&[u8]> = signatures.into_iter().collect();
    sigs.sort_unstable();

    let mut hasher = Sha256::new();
    for sig in sigs {
        hasher.update(sig);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_is_order_independent() {
        let a = vec![1u8; 64];
        let b = vec![2u8; 64];
        let c = vec![3u8; 64];

        let s1 = aggregate_seed([a.as_slice(), b.as_slice(), c.as_slice()]);
        let s2 = aggregate_seed([c.as_slice(), a.as_slice(), b.as_slice()]);
        let s3 = aggregate_seed([b.as_slice(), c.as_slice(), a.as_slice()]);
        assert_eq!(s1, s2);
        assert_eq!(s2, s3);
    }

    #[test]
    fn seed_changes_with_set() {
        let a = vec![1u8; 64];
        let b = vec![2u8; 64];
        let s_ab = aggregate_seed([a.as_slice(), b.as_slice()]);
        let s_a = aggregate_seed([a.as_slice()]);
        assert_ne!(s_ab, s_a);
    }

    #[test]
    fn empty_set_is_stable() {
        let s1 = aggregate_seed(std::iter::empty());
        let s2 = aggregate_seed(std::iter::empty());
        assert_eq!(s1, s2);
    }
}
