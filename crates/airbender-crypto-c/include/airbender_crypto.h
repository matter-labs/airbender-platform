/* Airbender's bindings to existing crypto and hint-capable operations.
 * C11 / C++11. See README.md for hint/runtime setup and encoding details.
 * The upstream header below is pinned unchanged. Of its 19 declarations, this
 * library supplies all 19 declarations.
 * BLAKE2f is a software fallback, without hint/delegation acceleration.
 */
#ifndef AIRBENDER_CRYPTO_H
#define AIRBENDER_CRYPTO_H
#include "zkvm_accelerators.h"
#define AIRBENDER_HAS_MODEXP 1
#define AIRBENDER_HAS_BLAKE2F 1
#define AIRBENDER_BLAKE2F_ACCELERATED 0

/* ModExp inputs/output are big-endian. Exactly mod_len output bytes are written,
 * left-padded with zeros. A zero/empty modulus produces zero/empty output.
 * No EVM-specific size limit is imposed: a Fusaka consumer can validate input
 * lengths <= 1024 bytes and provide a reusable 1024-byte output allocation.
 */

#ifdef __cplusplus
extern "C" {
#endif

/* RV32 c-runtime builds only: initialize airbender-rt's allocator once, after
 * guest memory setup and before operations that allocate. [start,end) must be
 * writable, exclusively owned, aligned to sizeof(uintptr_t), and remain live.
 * Rust guests initialized through airbender-rt::start do not call this function.
 */
void airbender_crypto_init_heap(uint8_t* start, uint8_t* end);

/* Native builds only: record the hint tape that an RV32 csr-oracle build reads.
 * Between begin and end, the calls on this thread run the guest's hinted path and
 * append the answer of each hint, as the guest reads it, to the tape: raw words,
 * with no framing added between answers (the answer of a ModExp reduction starts
 * with its two length words). Build the native library without features
 * (delegation is RV32 only): it records the hints of the guest's delegated
 * arithmetic without delegating. The guest reads the tape from its ordinary input
 * stream at the point of use, so make the same calls with the same inputs, in the
 * same order, and place the words in the guest's input where it makes them (after
 * the inputs it reads first). RV32 builds without csr-oracle read no hints.
 * end returns the number of words; it copies them to words when they fit in
 * capacity, and otherwise copies nothing and keeps them for the next end. Without
 * a recording, or once its words were copied, end returns 0. With capacity 0,
 * words may be NULL: end(NULL, 0) only returns the number of words, and keeps
 * them for the next end (as any capacity too small does). begin while a
 * recording is active starts it over, and drops the words of an ended recording
 * that were not copied. Calls outside a recording take the path without hints.
 * Recordings on several threads are independent only if nothing in the same
 * native build enables airbender-crypto/proving (see README.md).
 */
void airbender_crypto_record_begin(void);
size_t airbender_crypto_record_end(uint32_t* words, size_t capacity);

/* Additional byte structs have the same eight-byte alignment as upstream. */
typedef struct { ALIGN8 uint8_t data[144]; } airbender_bytes_144;
typedef struct { ALIGN8 uint8_t data[288]; } airbender_bytes_288;
typedef struct { ALIGN8 uint8_t data[384]; } airbender_bytes_384;
typedef struct { ALIGN8 uint8_t data[576]; } airbender_bytes_576;

/* All pointers must be valid, non-null, and aligned for the declared type,
 * even for empty inputs (the one exception is the size query
 * airbender_crypto_record_end(NULL, 0) above). Inputs may overlap outputs.
 * Errors leave output intact. Invalid crypto inputs return ZKVM_EFAIL. Pointer
 * contract violations fail execution. A wrong or missing hint either fails
 * execution or is ignored, and never changes a result (see README.md). There
 * are no caller-selected hint producers or C callbacks.
 *
 * Field operands/results: canonical big-endian prime-field components. Extension
 * fields use c0-first ordering recursively, including BN254 Fq2 (which differs
 * from the imaginary-first BN254 pairing-point encoding in the standard).
 * Zero inverse/divisor returns ZKVM_EFAIL. Sqrt returns the even root (canonical
 * value with its lowest bit clear, so sqrt(1) = p - 1 for the field prime p), and
 * EFAIL for nonsquares. With hints, inverses, quotients and square roots (and the
 * proof that a nonsquare has none) are checked hints: an Fq quotient is a division
 * hint, an Fq2 inverse or quotient one base-field inverse hint (of the norm), and
 * an Fq12 inverse or quotient an Fq12 inverse hint.
 */
zkvm_status airbender_secp256k1_fe_inverse(const zkvm_bytes_32* a, zkvm_bytes_32* result);
zkvm_status airbender_secp256k1_fe_divide(const zkvm_bytes_32* a, const zkvm_bytes_32* b, zkvm_bytes_32* result);
zkvm_status airbender_bn254_fq_inverse(const zkvm_bytes_32* a, zkvm_bytes_32* result);
zkvm_status airbender_bn254_fq_divide(const zkvm_bytes_32* a, const zkvm_bytes_32* b, zkvm_bytes_32* result);
zkvm_status airbender_bn254_fq2_inverse(const zkvm_bytes_64* a, zkvm_bytes_64* result);
zkvm_status airbender_bn254_fq2_divide(const zkvm_bytes_64* a, const zkvm_bytes_64* b, zkvm_bytes_64* result);
zkvm_status airbender_bn254_fq12_inverse(const airbender_bytes_384* a, airbender_bytes_384* result);
zkvm_status airbender_bn254_fq12_divide(const airbender_bytes_384* a, const airbender_bytes_384* b, airbender_bytes_384* result);
zkvm_status airbender_bls12_fq_inverse(const zkvm_bytes_48* a, zkvm_bytes_48* result);
zkvm_status airbender_bls12_fq_divide(const zkvm_bytes_48* a, const zkvm_bytes_48* b, zkvm_bytes_48* result);
zkvm_status airbender_bls12_fq2_inverse(const zkvm_bytes_96* a, zkvm_bytes_96* result);
zkvm_status airbender_bls12_fq2_divide(const zkvm_bytes_96* a, const zkvm_bytes_96* b, zkvm_bytes_96* result);
zkvm_status airbender_bls12_fq12_inverse(const airbender_bytes_576* a, airbender_bytes_576* result);
zkvm_status airbender_bls12_fq12_divide(const airbender_bytes_576* a, const airbender_bytes_576* b, airbender_bytes_576* result);
zkvm_status airbender_secp256k1_scalar_inverse(const zkvm_bytes_32* a, zkvm_bytes_32* result);
zkvm_status airbender_secp256k1_fe_sqrt(const zkvm_bytes_32* a, zkvm_bytes_32* result);

/* Jacobian coordinates X || Y || Z, with the field encoding above. Z=0 is
 * infinity, including arbitrary canonical X/Y. Otherwise normalizes, with a
 * checked inverse hint of Z unless Z=1, and checks the curve equation. These
 * operations accept on-curve points outside the prime-order subgroup; no subgroup
 * test is implied. Output is the standard affine x || y encoding, all zero for
 * infinity.
 */
zkvm_status airbender_bn254_g1_to_affine(const zkvm_bytes_96* point, zkvm_bytes_64* result);
zkvm_status airbender_bls12_g1_to_affine(const airbender_bytes_144* point, zkvm_bytes_96* result);
zkvm_status airbender_bls12_g2_to_affine(const airbender_bytes_288* point, zkvm_bytes_192* result);

/* On-curve affine inputs in the standard BLS encoding. Membership returns EOK
 * with false for an on-curve non-subgroup point. Clearing multiplies by the
 * effective cofactor without reducing it, and accepts non-subgroup points. With
 * hints both are affine, with checked division hints; infinity takes none. */
zkvm_status airbender_bls12_g1_is_in_subgroup(const zkvm_bytes_96* point, bool* result);
zkvm_status airbender_bls12_g1_clear_cofactor(const zkvm_bytes_96* point, zkvm_bytes_96* result);
zkvm_status airbender_bls12_g2_is_in_subgroup(const zkvm_bytes_192* point, bool* result);
zkvm_status airbender_bls12_g2_clear_cofactor(const zkvm_bytes_192* point, zkvm_bytes_192* result);

/* Existing BLAKE2s-256 hash; this is a different operation from BLAKE2f. */
zkvm_status airbender_blake2s256(const uint8_t* data, size_t len, zkvm_bytes_32* result);

#ifdef __cplusplus
}
#endif
#endif
