//! No export uses the heap, without a recording: the pairing checks and the MSMs keep their
//! parsed inputs in scratch buffers.
mod common;

use airbender_crypto as crypto;
use airbender_crypto_c::*;
use common::point;
use crypto::ark_ec::AffineRepr;
use crypto::{bls12_381 as bls, bn254 as bn};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;
fn allocated() {
    let _ = TRACKING.try_with(|active| {
        if active.get() {
            let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        }
    });
}
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocated();
        System.alloc(layout)
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        allocated();
        System.alloc_zeroed(layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        allocated();
        System.realloc(ptr, layout, size)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn without_allocations(name: &str, action: impl FnOnce() -> i32) {
    struct StopTracking;
    impl Drop for StopTracking {
        fn drop(&mut self) {
            TRACKING.set(false);
        }
    }
    ALLOCATIONS.set(0);
    TRACKING.set(true);
    let guard = StopTracking;
    let status = action();
    drop(guard);
    assert_eq!(status, EOK, "{name}");
    assert_eq!(ALLOCATIONS.get(), 0, "{name} allocated heap memory");
}

#[test]
fn exports_without_an_allocator_use_no_heap() {
    use crypto::k256::ecdsa::SigningKey;
    let bn_p = point(&bn::G1Affine::generator());
    let bls_p = point(&bls::G1Affine::generator());
    let bls_q = point(&bls::G2Affine::generator());
    let key = SigningKey::from_bytes(&[7u8; 32].into()).unwrap();
    let msg = Bytes { data: [0x11; 32] };
    let (sig, id) = key.sign_prehash_recoverable(&msg.data).unwrap();
    let sig = Bytes {
        data: sig.to_bytes().into(),
    };
    let pk = Bytes {
        data: key.verifying_key().to_encoded_point(false).as_bytes()[1..]
            .try_into()
            .unwrap(),
    };
    // KZG of the zero polynomial: infinity as the commitment and the proof, z = y = 0
    let mut infinity = Bytes { data: [0; 48] };
    infinity.data[0] = 0xc0;
    let zero = Bytes { data: [0; 32] };
    let (mut out32, mut out64, mut out96, mut out192) = (
        Bytes { data: [0; 32] },
        Bytes { data: [0; 64] },
        Bytes { data: [0; 96] },
        Bytes { data: [0; 192] },
    );
    let mut flag = false;
    let scalar = Bytes { data: [0x5a; 32] };
    // EIP-197 puts the imaginary part of each BN254 G2 coordinate first
    let mut bn_q: Bytes<128> = point(&bn::G2Affine::generator());
    for coordinate in bn_q.data.as_chunks_mut::<64>().0 {
        let (c0, c1) = coordinate.split_at_mut(32);
        c0.swap_with_slice(c1);
    }
    let bn_pair = Pair {
        first: bn_p,
        second: bn_q,
    };
    let bls_pair = Pair {
        first: bls_p,
        second: bls_q,
    };
    let g1_msm = [Pair {
        first: bls_p,
        second: scalar,
    }; 4];
    let g2_msm = [Pair {
        first: bls_q,
        second: scalar,
    }; 4];
    // the name of each export, for the failure message
    macro_rules! no_heap {
        ($export:ident($($arg:expr),*)) => {
            without_allocations(stringify!($export), || unsafe { $export($($arg),*) })
        };
    }
    // the exports without a status
    macro_rules! no_heap_void {
        ($export:ident($($arg:expr),*)) => {
            without_allocations(stringify!($export), || unsafe {
                let _ = $export($($arg),*);
                EOK
            })
        };
    }
    let mut state = KeccakState { opaque: [0; 512] };
    no_heap_void!(airbender_keccak256_init(&mut state));
    no_heap_void!(airbender_keccak256_update(
        &mut state,
        msg.data.as_ptr(),
        32
    ));
    no_heap_void!(airbender_keccak256_finalize_reset(&mut state, &mut out32));
    no_heap_void!(airbender_keccak256_finalize(&mut state, &mut out32));
    no_heap_void!(airbender_sha3_256_init(&mut state));
    no_heap_void!(airbender_sha3_256_update(&mut state, msg.data.as_ptr(), 32));
    no_heap_void!(airbender_sha3_256_finalize_reset(&mut state, &mut out32));
    no_heap_void!(airbender_sha3_256_finalize(&mut state, &mut out32));
    no_heap!(airbender_blake2s256(msg.data.as_ptr(), 32, &mut out32));
    no_heap!(zkvm_bn254_g1_add(&bn_p, &bn_p, &mut out64));
    no_heap!(zkvm_bn254_g1_mul(
        &bn_p,
        &Bytes { data: [0x5a; 32] },
        &mut out64
    ));
    no_heap!(zkvm_bls12_g1_add(&bls_p, &bls_p, &mut out96));
    no_heap!(zkvm_bls12_g2_add(&bls_q, &bls_q, &mut out192));
    no_heap!(zkvm_bls12_map_fp_to_g1(
        &Bytes { data: [7; 48] },
        &mut out96
    ));
    no_heap!(zkvm_bls12_map_fp2_to_g2(
        &Bytes { data: [7; 96] },
        &mut out192
    ));
    no_heap!(zkvm_secp256k1_ecrecover(
        &msg,
        &sig,
        id.to_byte(),
        &mut out64
    ));
    no_heap!(zkvm_secp256k1_verify(&msg, &sig, &pk, &mut flag));
    no_heap!(zkvm_kzg_point_eval(
        &infinity, &zero, &zero, &infinity, &mut flag
    ));
    no_heap!(zkvm_bn254_pairing(&bn_pair, 1, &mut flag));
    no_heap!(zkvm_bls12_pairing(&bls_pair, 1, &mut flag));
    // one pair (the affine chain with hints, the double-and-add without) and four (the buckets)
    no_heap!(zkvm_bls12_g1_msm(g1_msm.as_ptr(), 1, &mut out96));
    no_heap!(zkvm_bls12_g1_msm(g1_msm.as_ptr(), 4, &mut out96));
    no_heap!(zkvm_bls12_g2_msm(g2_msm.as_ptr(), 1, &mut out192));
    no_heap!(zkvm_bls12_g2_msm(g2_msm.as_ptr(), 4, &mut out192));
}
