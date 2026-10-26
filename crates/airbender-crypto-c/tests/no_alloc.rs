//! The exports that take no allocator use no heap, without a recording (a recording is a host
//! buffer). The pairing checks, the MSMs and ModExp allocate their working vectors, as the
//! zksync-os endpoints that they call do (see the README).
use airbender_crypto as crypto;
use airbender_crypto_c::*;
use crypto::ark_ec::{AffineRepr, CurveGroup};
use crypto::ark_ff::{BigInteger, PrimeField};
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

/// The canonical big-endian components of `values` into `N` bytes
fn encode<const N: usize, F: PrimeField>(values: &[F]) -> Bytes<N> {
    let mut output = Bytes { data: [0; N] };
    let width = N / values.len();
    for (dst, value) in output.data.chunks_exact_mut(width).zip(values) {
        let bytes = value.into_bigint().to_bytes_be();
        dst.copy_from_slice(&bytes[bytes.len() - width..]);
    }
    output
}

#[test]
fn exports_without_an_allocator_use_no_heap() {
    let bn_g = bn::G1Affine::generator();
    let bn_p = encode::<64, _>(&[bn_g.x, bn_g.y]);
    let bls_g = bls::G1Affine::generator();
    let bls_p = encode::<96, _>(&[bls_g.x, bls_g.y]);
    let bls_q = bls::G2Affine::generator();
    let bls_q = encode::<192, _>(&[bls_q.x.c0, bls_q.x.c1, bls_q.y.c0, bls_q.y.c1]);
    let mut out64 = Bytes { data: [0; 64] };
    let mut out96 = Bytes { data: [0; 96] };
    let mut out192 = Bytes { data: [0; 192] };
    let mut out32 = Bytes { data: [0; 32] };
    let mut flag = false;
    let scalar = Bytes { data: [0x5a; 32] };
    let two = encode::<32, _>(&[bn::Fq::from(2u64)]);
    unsafe {
        without_allocations("zkvm_bn254_g1_add", || {
            zkvm_bn254_g1_add(&bn_p, &bn_p, &mut out64)
        });
        without_allocations("zkvm_bn254_g1_mul", || {
            zkvm_bn254_g1_mul(&bn_p, &scalar, &mut out64)
        });
        without_allocations("zkvm_bls12_g1_add", || {
            zkvm_bls12_g1_add(&bls_p, &bls_p, &mut out96)
        });
        without_allocations("zkvm_bls12_g2_add", || {
            zkvm_bls12_g2_add(&bls_q, &bls_q, &mut out192)
        });
        without_allocations("zkvm_bls12_map_fp_to_g1", || {
            zkvm_bls12_map_fp_to_g1(&Bytes { data: [7; 48] }, &mut out96)
        });
        without_allocations("zkvm_bls12_map_fp2_to_g2", || {
            zkvm_bls12_map_fp2_to_g2(&Bytes { data: [7; 96] }, &mut out192)
        });
        without_allocations("airbender_bls12_g1_is_in_subgroup", || {
            airbender_bls12_g1_is_in_subgroup(&bls_p, &mut flag)
        });
        without_allocations("airbender_bls12_g2_clear_cofactor", || {
            airbender_bls12_g2_clear_cofactor(&bls_q, &mut out192)
        });
        let jacobian = {
            let p = (bn_g.into_group() + bn_g).into_affine();
            let z = bn::Fq::from(3u64);
            let z2 = z * z;
            encode::<96, _>(&[p.x * z2, p.y * z2 * z, z])
        };
        without_allocations("airbender_bn254_g1_to_affine", || {
            airbender_bn254_g1_to_affine(&jacobian, &mut out64)
        });
        without_allocations("airbender_bn254_fq_divide", || {
            airbender_bn254_fq_divide(&two, &two, &mut out32)
        });
        without_allocations("airbender_secp256k1_fe_inverse", || {
            airbender_secp256k1_fe_inverse(&two, &mut out32)
        });
        let message = Bytes {
            data: [
                56, 209, 138, 203, 103, 210, 92, 139, 185, 148, 39, 100, 182, 47, 24, 225, 112, 84,
                246, 106, 129, 123, 212, 41, 84, 35, 173, 249, 237, 152, 135, 62,
            ],
        };
        let mut signature = Bytes { data: [0; 64] };
        signature.data[..32].copy_from_slice(&message.data);
        signature.data[32..].copy_from_slice(&[
            120, 157, 29, 212, 35, 210, 95, 7, 114, 210, 116, 141, 96, 247, 228, 184, 27, 177, 77,
            8, 110, 186, 142, 142, 142, 251, 109, 207, 248, 164, 174, 2,
        ]);
        without_allocations("zkvm_secp256k1_ecrecover", || {
            zkvm_secp256k1_ecrecover(&message, &signature, 0, &mut out64)
        });
        without_allocations("zkvm_secp256k1_verify", || {
            zkvm_secp256k1_verify(&message, &signature, &out64, &mut flag)
        });
        assert!(flag);
        let infinity = {
            let mut bytes = Bytes { data: [0; 48] };
            bytes.data[0] = 0xc0;
            bytes
        };
        without_allocations("zkvm_kzg_point_eval", || {
            zkvm_kzg_point_eval(
                &infinity,
                &Bytes { data: [0; 32] },
                &Bytes { data: [0; 32] },
                &infinity,
                &mut flag,
            )
        });
        assert!(flag);
    }
}
