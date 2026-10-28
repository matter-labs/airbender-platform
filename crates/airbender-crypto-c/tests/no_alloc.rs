//! The exports that take no allocator use no heap, without a recording. The pairing checks, MSMs
//! and ModExp allocate their working vectors.
mod common;

use airbender_crypto as crypto;
use airbender_crypto_c::*;
use common::{encode, point};
use crypto::ark_ec::{AffineRepr, CurveGroup};
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
    let two = encode(&[bn::Fq::from(2u64)]);
    let jacobian = {
        let p = (bn::G1Affine::generator() * bn::Fr::from(2u64)).into_affine();
        let z = bn::Fq::from(3u64);
        encode(&[p.x * z * z, p.y * z * z * z, z])
    };
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
    // the name of each export, for the failure message
    macro_rules! no_heap {
        ($export:ident($($arg:expr),*)) => {
            without_allocations(stringify!($export), || unsafe { $export($($arg),*) })
        };
    }
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
    no_heap!(airbender_bls12_g1_is_in_subgroup(&bls_p, &mut flag));
    no_heap!(airbender_bls12_g2_clear_cofactor(&bls_q, &mut out192));
    no_heap!(airbender_bn254_g1_to_affine(&jacobian, &mut out64));
    no_heap!(airbender_bn254_fq_divide(&two, &two, &mut out32));
    no_heap!(airbender_secp256k1_fe_inverse(&two, &mut out32));
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
}
