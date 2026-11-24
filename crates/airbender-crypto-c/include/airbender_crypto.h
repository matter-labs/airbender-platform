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
 * a zero modulus). A length over 1024 bytes (EIP-7823) returns ZKVM_EFAIL before the operands
 * are read.
 */

/* Over-aligned types, defined like the upstream ALIGN8. */
#ifdef __cplusplus
  #define AIRBENDER_ALIGN(n) alignas(n)
#else
  #define AIRBENDER_ALIGN(n) _Alignas(n)
#endif

#ifdef __cplusplus
extern "C" {
#endif

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

/* All pointers must be non-null, aligned and valid, even for empty inputs (except
 * airbender_crypto_record_end(NULL, 0)); violations fail execution. Inputs may overlap
 * outputs. A wrong or missing hint fails execution or is ignored; it never changes a result.
 */

/* ----------------------------------------------------------------------------------------
 * Byte order
 * ---------------------------------------------------------------------------------------- */

/* Reverses the bytes of a word: one Zbb rev8 on Airbender (the only Zbb instruction it runs,
 * so it is enabled for this instruction alone), the compiler's byte swap elsewhere. Converts
 * a word of a big-endian integer (zkvm_bytes_32) into a word of airbender_u256 and back.
 */
static inline uint32_t airbender_bswap32(uint32_t value) {
#if defined(__riscv) && __riscv_xlen == 32
    uint32_t result;
    __asm__(".option push\n\t.option arch, +zbb\n\trev8 %0, %1\n\t.option pop"
            : "=r"(result) : "r"(value));
    return result;
#elif defined(__GNUC__) || defined(__clang__)
    return __builtin_bswap32(value);
#else
    return (value >> 24) | ((value >> 8) & 0xff00u) | ((value << 8) & 0xff0000u) | (value << 24);
#endif
}

/* ----------------------------------------------------------------------------------------
 * 256-bit integers: the operand layout of Airbender's bigint delegation
 * ---------------------------------------------------------------------------------------- */

/* A 256-bit unsigned integer, little-endian (data[0] is the lowest byte), 32-byte aligned as
 * the delegation requires. The operations below are inline. On RV32 each is one delegation
 * call: the operand addresses in x10 and x11, the operation mask in x12, `csrrw x0, 0x7ca, x0`,
 * and the flag back in x12. Elsewhere, or on RV32 with AIRBENDER_NO_DELEGATION defined, they
 * are portable C (little-endian hosts) with the same results.
 *
 * Both operands of a call must be 32-byte aligned (as the type is, unless a pointer was cast
 * into it), in RAM on RV32, and must not overlap: in particular a and b can never be the same
 * address. The delegation does not check its operands and the functions never do (no null
 * check either); a violation is undefined behaviour.
 */
typedef struct { AIRBENDER_ALIGN(32) uint8_t data[32]; } airbender_u256;

/* The operation bits of the delegation's mask (bit 6 is the carry or borrow in). */
#define AIRBENDER_U256_ADD_ 0
#define AIRBENDER_U256_SUB_ 1
#define AIRBENDER_U256_SUB_AND_NEGATE_ 2
#define AIRBENDER_U256_MUL_LOW_ 3
#define AIRBENDER_U256_MUL_HIGH_ 4
#define AIRBENDER_U256_EQ_ 5
#define AIRBENDER_U256_COPY_ 7
#define AIRBENDER_U256_CARRY_BIT_ 6

#if defined(__riscv) && __riscv_xlen == 32 && !defined(AIRBENDER_NO_DELEGATION)
/* One delegation call: a = op(a, b), or a comparison that reads only. Returns the flag. */
static inline uint32_t airbender_u256_delegate_(airbender_u256* a, const airbender_u256* b,
                                                uint32_t mask) {
    register airbender_u256* x10 __asm__("x10") = a;
    register const airbender_u256* x11 __asm__("x11") = b;
    register uint32_t x12 __asm__("x12") = mask;
    __asm__ volatile("csrrw x0, 0x7ca, x0" : "+r"(x12), "+m"(*a) : "r"(x10), "r"(x11), "m"(*b));
    return x12;
}
#else
/* Portable, for little-endian hosts: an operand is 32-byte aligned, so its bytes are read as
 * eight 32-bit limbs in place (a may_alias type, so the cast is sound under strict aliasing).
 */
#if defined(__GNUC__) || defined(__clang__)
typedef uint32_t airbender_u256_limb_ __attribute__((may_alias));
#else
typedef uint32_t airbender_u256_limb_;
#endif
#define AIRBENDER_U256_LIMBS_(value) ((airbender_u256_limb_*)(value)->data)
/* The full product, 16 limbs. */
static inline void airbender_u256_product_(const uint32_t x[8], const uint32_t y[8],
                                           uint32_t product[16]) {
    for (unsigned i = 0; i < 16; ++i) product[i] = 0;
    for (unsigned i = 0; i < 8; ++i) {
        uint64_t carry = 0;
        for (unsigned j = 0; j < 8; ++j) {
            uint64_t t = (uint64_t)x[i] * y[j] + product[i+j] + carry;
            product[i+j] = (uint32_t)t;
            carry = t >> 32;
        }
        product[i+8] = (uint32_t)carry;
    }
}
static inline uint32_t airbender_u256_delegate_(airbender_u256* a, const airbender_u256* b,
                                                uint32_t mask) {
    airbender_u256_limb_* x = AIRBENDER_U256_LIMBS_(a);
    const airbender_u256_limb_* y = AIRBENDER_U256_LIMBS_(b);
    uint32_t product[16];
    uint64_t carry = (mask >> AIRBENDER_U256_CARRY_BIT_) & 1u;
    switch (mask & ~(1u << AIRBENDER_U256_CARRY_BIT_)) {
    case 1u << AIRBENDER_U256_ADD_:
        for (unsigned i = 0; i < 8; ++i) { carry += (uint64_t)x[i] + y[i]; x[i] = (uint32_t)carry; carry >>= 32; }
        return (uint32_t)carry;
    case 1u << AIRBENDER_U256_SUB_:
        for (unsigned i = 0; i < 8; ++i) { carry = (uint64_t)x[i] - y[i] - carry; x[i] = (uint32_t)carry; carry = (carry >> 32) & 1u; }
        return (uint32_t)carry;
    case 1u << AIRBENDER_U256_SUB_AND_NEGATE_:
        for (unsigned i = 0; i < 8; ++i) { carry = (uint64_t)y[i] - x[i] - carry; x[i] = (uint32_t)carry; carry = (carry >> 32) & 1u; }
        return (uint32_t)carry;
    case 1u << AIRBENDER_U256_MUL_LOW_:
        airbender_u256_product_(x, y, product);
        carry = 0;
        for (unsigned i = 0; i < 8; ++i) { x[i] = product[i]; carry |= product[i+8]; }
        return carry != 0;
    case 1u << AIRBENDER_U256_MUL_HIGH_:
        airbender_u256_product_(x, y, product);
        for (unsigned i = 0; i < 8; ++i) x[i] = product[i+8];
        return 0;
    case 1u << AIRBENDER_U256_EQ_:
        carry = 1;
        for (unsigned i = 0; i < 8; ++i) carry &= x[i] == y[i];
        return (uint32_t)carry;
    default: /* copy */
        for (unsigned i = 0; i < 8; ++i) x[i] = y[i];
        return 0;
    }
}
#endif

/* a += b + carry; returns the carry out. */
static inline bool airbender_u256_add(airbender_u256* a, const airbender_u256* b, bool carry) {
    return airbender_u256_delegate_(a, b, (1u << AIRBENDER_U256_ADD_) | ((uint32_t)carry << AIRBENDER_U256_CARRY_BIT_)) != 0;
}
/* a -= b + borrow; returns the borrow out. */
static inline bool airbender_u256_sub(airbender_u256* a, const airbender_u256* b, bool borrow) {
    return airbender_u256_delegate_(a, b, (1u << AIRBENDER_U256_SUB_) | ((uint32_t)borrow << AIRBENDER_U256_CARRY_BIT_)) != 0;
}
/* a = b - a - borrow; returns the borrow out. */
static inline bool airbender_u256_sub_and_negate(airbender_u256* a, const airbender_u256* b, bool borrow) {
    return airbender_u256_delegate_(a, b, (1u << AIRBENDER_U256_SUB_AND_NEGATE_) | ((uint32_t)borrow << AIRBENDER_U256_CARRY_BIT_)) != 0;
}
/* a = the low 256 bits of a * b; returns true when the high 256 bits are zero, that is, when
 * the product fits (the delegation flags a non-zero high half). */
static inline bool airbender_u256_mul_low(airbender_u256* a, const airbender_u256* b) {
    return airbender_u256_delegate_(a, b, 1u << AIRBENDER_U256_MUL_LOW_) == 0;
}
/* a = the high 256 bits of a * b. */
static inline void airbender_u256_mul_high(airbender_u256* a, const airbender_u256* b) {
    airbender_u256_delegate_(a, b, 1u << AIRBENDER_U256_MUL_HIGH_);
}
/* a == b; reads only (the delegation does not write its first operand for a comparison). */
static inline bool airbender_u256_eq(const airbender_u256* a, const airbender_u256* b) {
    return airbender_u256_delegate_((airbender_u256*)a, b, 1u << AIRBENDER_U256_EQ_) != 0;
}
/* dst = src. */
static inline void airbender_u256_copy(airbender_u256* dst, const airbender_u256* src) {
    airbender_u256_delegate_(dst, src, 1u << AIRBENDER_U256_COPY_);
}

/* ----------------------------------------------------------------------------------------
 * Keccak-256 and SHA3-256, streaming
 * ---------------------------------------------------------------------------------------- */

/* The opaque state of a hasher (airbender-crypto's Keccak256Core): the Keccak-f[1600] state
 * in the layout of Airbender's keccak delegation, 256-byte aligned, plus the fill. Keccak-256
 * and SHA3-256 differ only in their padding, so they share the type; use the functions of one
 * family on a state. finalize leaves the state unspecified: init before reusing it.
 * finalize_reset leaves it initialized.
 */
typedef struct { AIRBENDER_ALIGN(256) uint8_t opaque[512]; } airbender_keccak_state;

void airbender_keccak256_init(airbender_keccak_state* state);
void airbender_keccak256_update(airbender_keccak_state* state, const uint8_t* data, size_t len);
void airbender_keccak256_finalize(airbender_keccak_state* state, zkvm_bytes_32* digest);
void airbender_keccak256_finalize_reset(airbender_keccak_state* state, zkvm_bytes_32* digest);

void airbender_sha3_256_init(airbender_keccak_state* state);
void airbender_sha3_256_update(airbender_keccak_state* state, const uint8_t* data, size_t len);
void airbender_sha3_256_finalize(airbender_keccak_state* state, zkvm_bytes_32* digest);
void airbender_sha3_256_finalize_reset(airbender_keccak_state* state, zkvm_bytes_32* digest);

/* BLAKE2s-256 (not BLAKE2f). */
zkvm_status airbender_blake2s256(const uint8_t* data, size_t len, zkvm_bytes_32* result);

#ifdef __cplusplus
}
#endif
#endif
