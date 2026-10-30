//! The guest layout of a bls12-381 pairing request: the guest's answer from its memory matches the
//! one a native recording gives. Outside `cfg(test)` the native points are the arkworks ones,
//! whose layout is not the guest's.
#![cfg(all(feature = "alloc", target_pointer_width = "64"))]

use airbender_crypto::{
    ark_ec::AffineRepr,
    ark_ff::{Field, PrimeField},
    bls12_381::{Fq, Fr, G1Affine, G2Affine},
    hints::{
        answer_query, guest_layout::*, Bls12381Pair, FieldHintOp, FieldOpsHint64, HintError,
        NativeQuerierMemory, QuerierMemory, FIELD_OPS_ADVISE_QUERY_ID,
    },
};

/// Guest memory: words from address 0
struct Guest(Vec<u32>);

impl QuerierMemory for Guest {
    fn word_size(&self) -> usize {
        4
    }
    fn read_u32(&self, address: usize) -> Result<u32, HintError> {
        Ok(self.0[address / 4])
    }
}

/// The guest's Montgomery limbs of `value` (`R = 2^512`)
fn limbs(value: Fq) -> Vec<u32> {
    let value = (value * Fq::from(2u64).pow([512])).into_bigint();
    let mut words: Vec<_> = value
        .as_ref()
        .iter()
        .flat_map(|v| [*v as u32, (*v >> 32) as u32])
        .collect();
    words.resize(16, 0);
    words
}

fn guest_pair(pair: &Bls12381Pair) -> Vec<u32> {
    let mut words = vec![0; BLS12_381_PAIR_SIZE / 4];
    words[BLS12_381_G1_AFFINE.infinity / 4] = pair.g1.infinity as u32;
    words[(BLS12_381_PAIR_G2 + BLS12_381_G2_AFFINE.infinity) / 4] = pair.g2.infinity as u32;
    for (offset, value) in [
        (0, pair.g1.x),
        (64, pair.g1.y),
        (160, pair.g2.x.c0),
        (224, pair.g2.x.c1),
        (288, pair.g2.y.c0),
        (352, pair.g2.y.c1),
    ] {
        words[offset / 4..][..16].copy_from_slice(&limbs(value));
    }
    words
}

#[test]
fn arbitrary_pairing_guest_layout_matches_native_recording() {
    let p = G1Affine::generator();
    let q: G2Affine = (G2Affine::generator() * Fr::from(5u64)).into();
    // a non-identity product, an identity product of two pairs, and a point at infinity
    for pairs in [
        vec![Bls12381Pair { g1: p, g2: q }],
        vec![
            Bls12381Pair { g1: p, g2: q },
            Bls12381Pair { g1: -p, g2: q },
        ],
        vec![Bls12381Pair {
            g1: G1Affine::identity(),
            g2: q,
        }],
    ] {
        let hint = FieldOpsHint64 {
            op: FieldHintOp::Bls12381PairingResidueWitness as u32,
            src_ptr: pairs.as_ptr() as u64,
            src_len_u32_words: (std::mem::size_of_val(pairs.as_slice()) / 4) as u32,
        };
        let mut native = Vec::new();
        let mut recorded = Vec::new();
        // SAFETY: the request and its operand are alive during the query
        let memory = unsafe { NativeQuerierMemory::new() };
        // SAFETY: as above, and `Bls12381Pair`s are the operand type of the op
        let answered = unsafe {
            answer_query(
                FIELD_OPS_ADVISE_QUERY_ID,
                &hint as *const _ as usize,
                &memory,
                Some(&mut native),
                Some(&mut recorded),
            )
        }
        .expect("must answer the field hint query");
        assert!(answered);
        let operand: Vec<_> = pairs.iter().flat_map(guest_pair).collect();
        // the request (a `FieldOpsHint`) at 4, the operand at 64
        let mut memory = Guest(vec![0; 16 + operand.len()]);
        memory.0[1] = hint.op;
        memory.0[2] = 64;
        memory.0[3] = operand.len() as u32;
        memory.0[16..].copy_from_slice(&operand);
        let mut simulated = Vec::new();
        // SAFETY: a querier on the guest is read through `memory` only
        let answered = unsafe {
            answer_query(
                FIELD_OPS_ADVISE_QUERY_ID,
                4,
                &memory,
                None,
                Some(&mut simulated),
            )
        }
        .expect("must answer the field hint query");
        assert!(answered);
        assert_eq!(recorded, simulated);
        assert_eq!(native[0], recorded[0]);
    }
}
