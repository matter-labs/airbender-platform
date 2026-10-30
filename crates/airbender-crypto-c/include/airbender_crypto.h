/* Airbender's crypto accelerators: all 19 functions of the pinned upstream
 * zkvm_accelerators.h, and the Airbender functions below. C11 / C++11. See README.md.
 * BLAKE2f is software only, without hints or delegation.
 */
#ifndef AIRBENDER_CRYPTO_H
#define AIRBENDER_CRYPTO_H
#include "zkvm_accelerators.h"
#define AIRBENDER_HAS_MODEXP 1
#define AIRBENDER_HAS_BLAKE2F 1
#define AIRBENDER_BLAKE2F_ACCELERATED 0

/* ModExp: big-endian. Writes exactly mod_len output bytes, left-padded with zeros (zeros for
 * a zero modulus). No input size limit.
 */

#ifdef __cplusplus
extern "C" {
#endif

/* RV32 c-runtime builds only: initialize the allocator once, before anything allocates.
 * [start,end) must be writable, exclusively owned, aligned to sizeof(uintptr_t), and stay
 * live. A second call traps. Rust guests started by airbender-rt do not call it.
 */
void airbender_crypto_init_heap(uint8_t* start, uint8_t* end);

/* Native builds only: record the hint tape that an RV32 csr-oracle build reads. Between
 * begin and end, calls on this thread take the guest's hinted path and append the guest's
 * answers to the tape as raw words, unframed (a ModExp reduction answer starts with its two
 * length words). Build the native library without features, make the guest's calls with the
 * same inputs in the same order, and place the words in the guest's input where it makes them.
 * end returns the number of words, 0 without a recording or once they were copied. It copies
 * them to words if they fit in capacity, and otherwise keeps them for the next end; words may
 * be NULL with capacity 0 (a size query). begin restarts an active recording and drops words
 * that were not copied. Calls outside a recording take the path without hints. Recordings are
 * per thread (see README.md for airbender-crypto/proving).
 */
void airbender_crypto_record_begin(void);
size_t airbender_crypto_record_end(uint32_t* words, size_t capacity);

/* Additional byte structs, aligned to 8 like the upstream ones. */
typedef struct { ALIGN8 uint8_t data[144]; } airbender_bytes_144;
typedef struct { ALIGN8 uint8_t data[288]; } airbender_bytes_288;
typedef struct { ALIGN8 uint8_t data[384]; } airbender_bytes_384;
typedef struct { ALIGN8 uint8_t data[576]; } airbender_bytes_576;

/* All pointers must be non-null, aligned and valid, even for empty inputs (except
 * airbender_crypto_record_end(NULL, 0)); violations fail execution. Inputs may overlap
 * outputs. Invalid inputs return ZKVM_EFAIL and leave the output unchanged. A wrong or
 * missing hint fails execution or is ignored; it never changes a result.
 *
 * Field elements: canonical big-endian base-field components, c0 first recursively for
 * extension fields (also BN254 Fq2, unlike the standard's imaginary-first pairing points).
 * A zero inverse or divisor, a non-canonical operand and a nonsquare return ZKVM_EFAIL. sqrt
 * returns the even root (lowest bit clear, so sqrt(1) = p - 1). Hints: an Fq quotient takes a
 * division hint, an Fq2 inverse or quotient one inverse hint of the norm, Fq12 an Fq12 inverse.
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

/* Jacobian X || Y || Z in the field encoding above; Z = 0 is infinity, whatever X and Y.
 * Returns affine x || y (all zero for infinity), and ZKVM_EFAIL off the curve. No subgroup
 * check. Hints: an inverse of Z, unless Z = 1.
 */
zkvm_status airbender_bn254_g1_to_affine(const zkvm_bytes_96* point, zkvm_bytes_64* result);
zkvm_status airbender_bls12_g1_to_affine(const airbender_bytes_144* point, zkvm_bytes_96* result);
zkvm_status airbender_bls12_g2_to_affine(const airbender_bytes_288* point, zkvm_bytes_192* result);

/* On-curve affine points in the standard BLS encoding, inside the subgroup or not.
 * is_in_subgroup returns ZKVM_EOK with false outside it. clear_cofactor multiplies by the
 * effective cofactor, unreduced. Hints: division hints, none for infinity. */
zkvm_status airbender_bls12_g1_is_in_subgroup(const zkvm_bytes_96* point, bool* result);
zkvm_status airbender_bls12_g1_clear_cofactor(const zkvm_bytes_96* point, zkvm_bytes_96* result);
zkvm_status airbender_bls12_g2_is_in_subgroup(const zkvm_bytes_192* point, bool* result);
zkvm_status airbender_bls12_g2_clear_cofactor(const zkvm_bytes_192* point, zkvm_bytes_192* result);

/* BLAKE2s-256 (not BLAKE2f). */
zkvm_status airbender_blake2s256(const uint8_t* data, size_t len, zkvm_bytes_32* result);

#ifdef __cplusplus
}
#endif
#endif
