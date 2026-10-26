#include "airbender_crypto.h"

static void unhex(uint8_t* out, const char* hex, size_t len) {
    for (size_t i = 0; i < len; ++i) {
        unsigned a = (unsigned)hex[2*i], b = (unsigned)hex[2*i+1];
        a = a <= '9' ? a - '0' : a - 'a' + 10;
        b = b <= '9' ? b - '0' : b - 'a' + 10;
        out[i] = (uint8_t)((a << 4) | b);
    }
}

static int same(const void* x, const void* y, size_t len) {
    const uint8_t* a = x; const uint8_t* b = y;
    for (size_t i = 0; i < len; ++i) if (a[i] != b[i]) return 0;
    return 1;
}

/* FNV-1a over every result of the fixture. The RV32 runs must end with the digest of the native
 * run, and a tampered hint must make the guest fail rather than change any result. */
static uint32_t digest_state;
static void mix(const void* data, size_t len) {
    const uint8_t* bytes = data;
    for (size_t i = 0; i < len; ++i) { digest_state ^= bytes[i]; digest_state *= 16777619u; }
}
#define MIX(value) mix(&(value), sizeof(value))
#define CHECK(call, code) do { if ((call) != ZKVM_EOK) return (code); } while (0)

static const char BLS_G1[] = "17f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb08b3f481e3aaa0f1a09e30ed741d8ae4fcf5e095d5d00af600db18cb2c04b3edd03cc744a2888ae40caa232946c5e7e1";
static const char BLS_G2[] = "024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb813e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e0ce5d527727d6e118cc9cdc6da2e351aadfd9baa8cbdd3a76d429a695160d12c923ac9cc3baca289e193548608b828010606c4a02ea734cc32acd2b02bc28b99cb3e287e85a763af267492ab572e99ab3f370d275cec1da1aaa9075ff05f79be";

/* Field bindings, including the extension fields, and zero operands (which take no hint). */
static uint32_t fields(void) {
    zkvm_bytes_32 a = {0}, b = {0}, zero = {0}, result = {0};
    a.data[31] = 6; b.data[31] = 3;
    if (airbender_bn254_fq_divide(&a, &b, &result) != ZKVM_EOK || result.data[31] != 2)
        return 3;
    MIX(result);
    CHECK(airbender_bn254_fq_inverse(&b, &result), 20); MIX(result);
    if (airbender_secp256k1_fe_divide(&a, &b, &result) != ZKVM_EOK || result.data[31] != 2)
        return 4;
    MIX(result);
    CHECK(airbender_secp256k1_fe_inverse(&b, &result), 21); MIX(result);
    CHECK(airbender_secp256k1_scalar_inverse(&b, &result), 22); MIX(result);
    /* the even root: 2 for 4, and p - 9 for 81, whose natural root x^((p+1)/4) is 9 */
    zkvm_bytes_32 four = {0}, eighty_one = {0}, minus_nine = {0};
    four.data[31] = 4; eighty_one.data[31] = 81;
    unhex(minus_nine.data, "fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc26", 32);
    if (airbender_secp256k1_fe_sqrt(&four, &result) != ZKVM_EOK || result.data[31] != 2) return 23;
    for (unsigned i = 0; i < 31; ++i) if (result.data[i]) return 23;
    if (airbender_secp256k1_fe_sqrt(&eighty_one, &result) != ZKVM_EOK || !same(&result, &minus_nine, 32)) return 23;
    MIX(result);
    /* 3 is not a square modulo the secp256k1 prime: the hint proves it */
    if (airbender_secp256k1_fe_sqrt(&b, &result) != ZKVM_EFAIL) return 24;
    if (airbender_secp256k1_fe_inverse(&zero, &result) != ZKVM_EFAIL) return 24;
    if (airbender_bn254_fq_inverse(&zero, &result) != ZKVM_EFAIL) return 24;

    zkvm_bytes_64 fq2 = {0}, fq2_inverse = {0}, u = {0}, quotient = {0};
    fq2.data[31] = 1;
    if (airbender_bn254_fq2_inverse(&fq2, &fq2_inverse) != ZKVM_EOK) return 64;
    if (!same(&fq2, &fq2_inverse, sizeof fq2)) return 64;
    u.data[31] = 3; u.data[63] = 5;
    CHECK(airbender_bn254_fq2_inverse(&u, &fq2_inverse), 25); MIX(fq2_inverse);
    CHECK(airbender_bn254_fq2_divide(&fq2, &u, &quotient), 26);
    if (!same(&quotient, &fq2_inverse, sizeof quotient)) return 26;

    airbender_bytes_384 f = {0}, f_inverse = {0}, f_one = {0};
    for (unsigned i = 0; i < 12; ++i) f.data[32 * i + 31] = (uint8_t)(i + 1);
    CHECK(airbender_bn254_fq12_inverse(&f, &f_inverse), 27); MIX(f_inverse);
    CHECK(airbender_bn254_fq12_divide(&f, &f, &f_one), 28);
    if (f_one.data[31] != 1) return 28;
    for (unsigned i = 0; i < 384; ++i) if (i != 31 && f_one.data[i]) return 28;

    zkvm_bytes_48 c = {0}, d = {0}, e = {0};
    c.data[47] = 6; d.data[47] = 3;
    CHECK(airbender_bls12_fq_inverse(&d, &e), 29); MIX(e);
    if (airbender_bls12_fq_divide(&c, &d, &e) != ZKVM_EOK || e.data[47] != 2) return 30;
    zkvm_bytes_96 v = {0}, v_inverse = {0}, w = {0}, one96 = {0};
    v.data[47] = 3; v.data[95] = 5; one96.data[47] = 1;
    CHECK(airbender_bls12_fq2_inverse(&v, &v_inverse), 31); MIX(v_inverse);
    CHECK(airbender_bls12_fq2_divide(&one96, &v, &w), 32);
    if (!same(&w, &v_inverse, sizeof w)) return 32;
    airbender_bytes_576 g = {0}, g_inverse = {0}, g_one = {0};
    for (unsigned i = 0; i < 12; ++i) g.data[48 * i + 47] = (uint8_t)(i + 1);
    CHECK(airbender_bls12_fq12_inverse(&g, &g_inverse), 33); MIX(g_inverse);
    CHECK(airbender_bls12_fq12_divide(&g, &g, &g_one), 34);
    if (g_one.data[47] != 1) return 34;
    return 0;
}

/* BLS12-381 maps, additions, MSMs (both algorithms, with an infinity point and a zero scalar),
 * subgroup checks, cofactor clearing, Jacobian normalization and the pairing check. */
static uint32_t bls12_381(void) {
    zkvm_bytes_48 fp = {0}; zkvm_bytes_96 point = {0}, product = {0}, doubled = {0}, zero1 = {0};
    CHECK(zkvm_bls12_map_fp_to_g1(&fp, &point), 5); MIX(point);
    bool flag = false;
    if (airbender_bls12_g1_is_in_subgroup(&point, &flag) != ZKVM_EOK || !flag) return 6;
    zkvm_bls12_381_g1_msm_pair pair = {0}; pair.point = point; pair.scalar.data[31] = 2;
    CHECK(zkvm_bls12_g1_msm(&pair, 1, &product), 7);
    CHECK(zkvm_bls12_g1_add(&point, &point, &doubled), 8);
    if (!same(&product, &doubled, sizeof product)) return 9;
    MIX(product);
    CHECK(zkvm_bls12_g1_add(&point, &zero1, &product), 35);
    if (!same(&product, &point, sizeof product)) return 35;
    zkvm_bls12_381_g1_msm_pair terms[3] = {0};
    terms[0].point = point; terms[0].scalar.data[31] = 2;
    terms[1].scalar.data[31] = 5; /* the point at infinity */
    terms[2].point = point;       /* a zero scalar */
    CHECK(zkvm_bls12_g1_msm(terms, 3, &product), 36);
    if (!same(&product, &doubled, sizeof product)) return 36;

    /* (4, y): a point of the curve outside the subgroup, until its cofactor is cleared */
    zkvm_bytes_96 outside = {0}, cleared = {0};
    unhex(outside.data, "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040a989badd40d6212b33cffc3f3763e9bc760f988c9926b26da9dd85e928483446346b8ed00e1de5d5ea93e354abe706c", 96);
    if (airbender_bls12_g1_is_in_subgroup(&outside, &flag) != ZKVM_EOK || flag) return 37;
    CHECK(airbender_bls12_g1_clear_cofactor(&outside, &cleared), 38); MIX(cleared);
    if (airbender_bls12_g1_is_in_subgroup(&cleared, &flag) != ZKVM_EOK || !flag) return 39;

    zkvm_bytes_96 g1 = {0}; zkvm_bytes_192 g2 = {0};
    unhex(g1.data, BLS_G1, 96); unhex(g2.data, BLS_G2, 192);
    airbender_bytes_144 g1_jacobian = {0}; airbender_bytes_288 g2_jacobian = {0};
    unhex(g1_jacobian.data, "0777e18ef2575f630263423cb299a5d2baf292a4b6f1eb37723977301a715dfad9a52a4a3aaaf272051a7061b43ba73b00f12876fd7fde0f4cb643a2e174929425bd0820fd437f37766435c4f84652cfe25d0448e872a60fcbfab55a76e274b8000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003", 144);
    unhex(g2_jacobian.data, "149fb84a75075f1a56482e609bed92dffe08517ccc42131856d9668852025d2f690c1c5be8339b6e78830d0eca2fab7816db1ae58cff32c6a86d825f359bbd83c9fe9237ad07f673f9887fd4f8540dba15ac79aa883f4818b71665304527876c0a2f924427bbe60408e12fb79807d2e03fb0953c7a435ef0488b96ef0e398fdddd77489b49f1248e5896ea22eb708d6c06b0516792a329eb95944a4f09beac2c14c2803a638912fca326884c69c47036f2c6632ea4ef200ea3d9c71e5a13d708000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", 288);
    zkvm_bytes_96 affine1 = {0}; zkvm_bytes_192 affine2 = {0};
    if (airbender_bls12_g1_to_affine(&g1_jacobian, &affine1) != ZKVM_EOK || !same(&affine1, &g1, 96)) return 41;
    if (airbender_bls12_g2_to_affine(&g2_jacobian, &affine2) != ZKVM_EOK || !same(&affine2, &g2, 192)) return 42;

    zkvm_bytes_96 fp2 = {0}; zkvm_bytes_192 q = {0}, q_doubled = {0}, q_product = {0}, q_cleared = {0};
    CHECK(zkvm_bls12_map_fp2_to_g2(&fp2, &q), 15); MIX(q);
    zkvm_bls12_381_g2_msm_pair term = {0}; term.point = q; term.scalar.data[31] = 2;
    if (zkvm_bls12_g2_msm(&term, 1, &q_product) != ZKVM_EOK || zkvm_bls12_g2_add(&q, &q, &q_doubled) != ZKVM_EOK) return 16;
    if (!same(&q_doubled, &q_product, sizeof q_product)) return 17;
    MIX(q_product);
    if (airbender_bls12_g2_is_in_subgroup(&q, &flag) != ZKVM_EOK || !flag) return 44;
    CHECK(airbender_bls12_g2_clear_cofactor(&q, &q_cleared), 45); MIX(q_cleared);

    /* e(G1, G2) e(-G1, G2) = 1, and e(G1, G2) alone is not */
    zkvm_bls12_381_pairing_pair pairs[2] = {0};
    pairs[0].g1 = g1; pairs[0].g2 = g2; pairs[1].g2 = g2;
    unhex(pairs[1].g1.data, "17f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb114d1d6855d545a8aa7d76c8cf2e21f267816aef1db507c96655b9d5caac42364e6f38ba0ecb751bad54dcd6b939c2ca", 96);
    bool verified = false;
    if (zkvm_bls12_pairing(pairs, 2, &verified) != ZKVM_EOK || !verified) return 53;
    if (zkvm_bls12_pairing(pairs, 1, &verified) != ZKVM_EOK || verified) return 54;
    return 0;
}

/* BN254 additions and multiplications (zero scalar, the group order, infinity), normalization,
 * and pairing checks (true, false, empty, with infinity). */
static uint32_t bn254(void) {
    zkvm_bn254_g1_point g = {0}, minus_g = {0}, sum = {0}, product = {0}, zero = {0};
    g.data[31] = 1; g.data[63] = 2;
    minus_g.data[31] = 1;
    unhex(minus_g.data + 32, "30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd45", 32);
    zkvm_bn254_scalar two = {0}, order = {0}, scalar = {0};
    two.data[31] = 2;
    unhex(order.data, "30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001", 32);
    CHECK(zkvm_bn254_g1_add(&g, &g, &sum), 46); MIX(sum);
    if (zkvm_bn254_g1_mul(&g, &two, &product) != ZKVM_EOK || !same(&sum, &product, 64)) return 47;
    if (zkvm_bn254_g1_mul(&g, &scalar, &product) != ZKVM_EOK || !same(&product, &zero, 64)) return 48;
    if (zkvm_bn254_g1_mul(&g, &order, &product) != ZKVM_EOK || !same(&product, &zero, 64)) return 49;
    if (zkvm_bn254_g1_mul(&zero, &two, &product) != ZKVM_EOK || !same(&product, &zero, 64)) return 49;
    if (zkvm_bn254_g1_add(&g, &minus_g, &sum) != ZKVM_EOK || !same(&sum, &zero, 64)) return 50;
    for (unsigned i = 0; i < 32; ++i) scalar.data[i] = 0xff;
    CHECK(zkvm_bn254_g1_mul(&g, &scalar, &product), 50); MIX(product);

    zkvm_bytes_96 jacobian = {0}; zkvm_bytes_64 affine = {0};
    jacobian.data[31] = 9; jacobian.data[63] = 54; jacobian.data[95] = 3; /* G with Z = 3 */
    if (airbender_bn254_g1_to_affine(&jacobian, &affine) != ZKVM_EOK || !same(&affine, &g, 64)) return 40;
    jacobian.data[95] = 0; /* Z = 0: infinity, whatever X and Y */
    if (airbender_bn254_g1_to_affine(&jacobian, &affine) != ZKVM_EOK || !same(&affine, &zero, 64)) return 43;

    zkvm_bn254_pairing_pair bn[2] = {0};
    bn[0].g1 = g;
    unhex(bn[0].g2.data, "198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c21800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa", 128);
    bn[1] = bn[0]; bn[1].g1 = minus_g;
    bool verified = false;
    if (zkvm_bn254_pairing(bn, 2, &verified) != ZKVM_EOK || !verified) return 18;
    if (zkvm_bn254_pairing(bn, 1, &verified) != ZKVM_EOK || verified) return 19;
    if (zkvm_bn254_pairing(bn, 0, &verified) != ZKVM_EOK || !verified) return 51;
    verified = false;
    bn[1].g1 = zero; /* e(O, Q) = 1 */
    if (zkvm_bn254_pairing(bn + 1, 1, &verified) != ZKVM_EOK || !verified) return 52;
    return 0;
}

/* Signatures and KZG. */
static uint32_t signatures(void) {
    zkvm_bytes_32 message = { 56, 209, 138, 203, 103, 210, 92, 139, 185, 148, 39, 100, 182, 47, 24, 225, 112, 84, 246, 106, 129, 123, 212, 41, 84, 35, 173, 249, 237, 152, 135, 62 };
    const uint8_t signature_s[32] = { 120, 157, 29, 212, 35, 210, 95, 7, 114, 210, 116, 141, 96, 247, 228, 184, 27, 177, 77, 8, 110, 186, 142, 142, 142, 251, 109, 207, 248, 164, 174, 2 };
    zkvm_bytes_64 signature = {0}, public_key = {0};
    for (unsigned i = 0; i < 32; ++i) {
        signature.data[i] = message.data[i]; signature.data[i + 32] = signature_s[i];
    }
    if (zkvm_secp256k1_ecrecover(&message, &signature, 0, &public_key) != ZKVM_EOK ||
        public_key.data[0] != 0x86 || public_key.data[1] != 0x12 ||
        public_key.data[2] != 0x54 || public_key.data[3] != 0xa4) return 10;
    MIX(public_key);
    bool verified = false;
    if (zkvm_secp256k1_verify(&message, &signature, &public_key, &verified) != ZKVM_EOK || !verified) return 12;
    zkvm_bytes_48 commitment = {0}; commitment.data[0] = 0xc0;
    zkvm_bytes_32 zero = {0};
    verified = false;
    if (zkvm_kzg_point_eval(&commitment, &zero, &zero, &commitment, &verified) != ZKVM_EOK || !verified)
        return 11;
    /* Nonconstant official c-kzg vector */
    zkvm_bytes_48 c = {0}, pi = {0}; zkvm_bytes_32 z = {0}, y = {0};
    unhex(c.data, "a421e229565952cfff4ef3517100a97da1d4fe57956fa50a442f92af03b1bf37adacc8ad4ed209b31287ea5bb94d9d06", sizeof(c.data));
    unhex(pi.data, "a1fcd37a924af9ec04143b44853c26f6b0738f6e15a3e0755057e7d5460406c7e148adb0e2d608982140d0ae42fe0b3b", sizeof(pi.data));
    unhex(z.data, "5eb7004fe57383e6c88b99d839937fddf3f99279353aaf8d5c9a75f91ce33c62", sizeof(z.data));
    unhex(y.data, "5ee1e9a4a06a02ca6ea14b0ca73415a8ba0fba888f18dde56df499b480d4b9e0", sizeof(y.data));
    if (zkvm_kzg_point_eval(&c, &z, &y, &pi, &verified) != ZKVM_EOK || !verified) return 13;
    /* ECDSA key = nonce = digest = 1; r = x(G), s = r + 1. */
    zkvm_bytes_32 one = {0}; one.data[31] = 1;
    zkvm_bytes_64 pkey = {0}, psig = {0};
    unhex(pkey.data, "6b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c2964fe342e2fe1a7f9b8ee7eb4a7c0f9e162bce33576b315ececbb6406837bf51f5", 64);
    unhex(psig.data, "6b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c2966b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c297", 64);
    if (zkvm_secp256r1_verify(&one, &psig, &pkey, &verified) != ZKVM_EOK || !verified) return 14;
    return 0;
}

/* ModExp: repeated calls exercise airbender-rt's allocation and deallocation; moduli of 0, 1,
 * 4, 32 and 33 bytes take the single-digit and the multi-digit hinted paths, and a zero modulus
 * gives zeroes. */
static uint32_t modexp(void) {
    const uint8_t base[] = {2}, exponent[] = {10}, modulus[] = {0, 0, 3, 232};
    for (unsigned i = 0; i < 256; ++i) {
        uint8_t output[6] = {0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5};
        if (zkvm_modexp(base, 1, exponent, 1, modulus, 4, output + 1) != ZKVM_EOK)
            return 1;
        if (output[0] != 0xa5 || output[1] || output[2] || output[3] ||
            output[4] != 24 || output[5] != 0xa5)
            return 2;
    }
    uint8_t output[34] = {0};
    const uint8_t zero_modulus[4] = {0}, one_modulus[1] = {1};
    output[0] = 0xa5;
    if (zkvm_modexp(base, 1, exponent, 1, modulus, 0, output) != ZKVM_EOK || output[0] != 0xa5) return 55;
    if (zkvm_modexp(base, 1, exponent, 1, zero_modulus, 4, output) != ZKVM_EOK || !same(output, zero_modulus, 4)) return 56;
    if (zkvm_modexp(base, 1, exponent, 1, one_modulus, 1, output) != ZKVM_EOK || output[0] != 0) return 57;
    uint8_t modulus32[32] = {0}, modulus33[33] = {0}, base17[17] = {0};
    const uint8_t three[] = {3}, five[] = {5};
    modulus32[0] = 0x80; modulus32[31] = 1;
    if (zkvm_modexp(three, 1, five, 1, modulus32, 32, output) != ZKVM_EOK || output[31] != 243) return 58;
    MIX(output);
    /* (2^128)^3 = 2^256 - 2^128 + 1 modulo 2^256 + 1 */
    modulus33[0] = 1; modulus33[32] = 1; base17[0] = 1;
    CHECK(zkvm_modexp(base17, 17, three, 1, modulus33, 33, output), 59);
    if (output[0] != 0 || output[1] != 0xff || output[16] != 0xff || output[17] != 0 || output[32] != 1) return 59;
    MIX(output);
    return 0;
}

/* RV32 c-runtime integration fixture. The runner supplies an exclusive heap. Built natively, it
 * records the hints that the RV32 csr-oracle build reads (see main). The digest of every result
 * goes to *digest. */
uint32_t airbender_crypto_runtime_smoke(uint8_t* heap_start, uint8_t* heap_end, uint32_t scenario, uint32_t* digest) {
#ifdef __riscv
    airbender_crypto_init_heap(heap_start, heap_end);
    if (scenario == 1) airbender_crypto_init_heap(heap_start, heap_end);
#else
    (void)heap_start; (void)heap_end; (void)scenario;
#endif
    digest_state = 2166136261u;
    uint32_t status = fields();
    if (status == 0) status = bls12_381();
    if (status == 0) status = bn254();
    if (status == 0) status = signatures();
    if (status == 0) status = modexp();
    *digest = digest_state;
    return status;
}

#ifdef __riscv
/* Consumer-owned libc symbols must coexist with the static accelerator archive. */
void* memcpy(void* out, const void* in, size_t len) {
    uint8_t* d = out; const uint8_t* s = in;
    for (size_t i = 0; i < len; ++i) d[i] = s[i];
    return out;
}
void* memset(void* out, int byte, size_t len) {
    uint8_t* d = out;
    for (size_t i = 0; i < len; ++i) d[i] = (uint8_t)byte;
    return out;
}
void* memmove(void* out, const void* in, size_t len) {
    uint8_t* d = out; const uint8_t* s = in;
    if ((uintptr_t)d < (uintptr_t)s) return memcpy(out, in, len);
    while (len) { --len; d[len] = s[len]; }
    return out;
}
#else
#include <stdio.h>
#include <stdlib.h>
/* Native hint recording, for an RV32 csr-oracle build of the fixture: run it without and with a
 * recording (the results must not differ), then print the digest of its results, then the
 * recorded words, one hex word per line. Build it without features: native builds record the
 * hints of the guest's delegated arithmetic without delegation. */
int main(void) {
    uint32_t plain = 0, hinted = 0;
    if (airbender_crypto_runtime_smoke(NULL, NULL, 0, &plain) != 0) return 1;
    airbender_crypto_record_begin();
    uint32_t status = airbender_crypto_runtime_smoke(NULL, NULL, 0, &hinted);
    uint32_t none;
    size_t len = airbender_crypto_record_end(&none, 0);
    if (status != 0) return 2;
    if (hinted != plain || len == 0) return 3;
    uint32_t* tape = malloc(len * sizeof *tape);
    if (tape == NULL || airbender_crypto_record_end(tape, len) != len) return 4;
    printf("digest %08x\n", (unsigned)plain);
    for (size_t i = 0; i < len; ++i) printf("%08x\n", (unsigned)tape[i]);
    free(tape);
    return 0;
}
#endif
