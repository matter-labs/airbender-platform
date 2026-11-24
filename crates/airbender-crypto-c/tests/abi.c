#include "airbender_crypto.h"
#ifndef LAYOUT_ONLY
#include <assert.h>
#include <string.h>
#endif

#ifdef __cplusplus
#define ASSERT static_assert
#define ALIGNOF alignof
#else
#define ASSERT _Static_assert
#define ALIGNOF _Alignof
#endif
#define LAYOUT(T,N) ASSERT(sizeof(T)==N, "size: " #T); ASSERT(ALIGNOF(T)==8, "alignment: " #T)
LAYOUT(zkvm_bytes_16,16); LAYOUT(zkvm_bytes_32,32); LAYOUT(zkvm_bytes_48,48);
LAYOUT(zkvm_bytes_64,64); LAYOUT(zkvm_bytes_96,96); LAYOUT(zkvm_bytes_128,128);
ASSERT(sizeof(airbender_u256)==32, "size: airbender_u256"); ASSERT(ALIGNOF(airbender_u256)==32, "alignment: airbender_u256");
ASSERT(sizeof(airbender_keccak_state)==512, "size: airbender_keccak_state"); ASSERT(ALIGNOF(airbender_keccak_state)==256, "alignment: airbender_keccak_state");
LAYOUT(zkvm_bytes_192,192); LAYOUT(zkvm_bn254_pairing_pair,192);
LAYOUT(zkvm_bls12_381_g1_msm_pair,128); LAYOUT(zkvm_bls12_381_g2_msm_pair,224);
LAYOUT(zkvm_bls12_381_pairing_pair,288);
ASSERT(sizeof(zkvm_status)==4,"status enum"); ASSERT(sizeof(bool)==1,"bool");
ASSERT(offsetof(zkvm_bn254_pairing_pair,g2)==64,"BN pair offset");
ASSERT(offsetof(zkvm_bls12_381_pairing_pair,g2)==96,"BLS pair offset");
ASSERT(offsetof(zkvm_bls12_381_g1_msm_pair,scalar)==96,"G1 scalar offset");
ASSERT(offsetof(zkvm_bls12_381_g2_msm_pair,scalar)==192,"G2 scalar offset");

#ifndef LAYOUT_ONLY
int main(int argc,char** argv) {
    zkvm_bytes_32 hash={0}, msg={0};
    zkvm_bytes_64 sig={0}, pk={0};
    bool verified=true;
    if (argc>1 && strcmp(argv[1],"--null")==0) {
        return zkvm_sha256(NULL,0,&hash); /* Must fail execution, never unwind into C. */
    }
    const uint8_t abc[]={97,98,99};
    assert(zkvm_keccak256(abc,3,&hash)==ZKVM_EOK && hash.data[0]==0x4e && hash.data[31]==0x45);
    assert(zkvm_sha256(abc,3,&hash)==ZKVM_EOK && hash.data[0]==0xba && hash.data[31]==0xad);
    assert(zkvm_ripemd160(abc,3,&hash)==ZKVM_EOK && hash.data[11]==0 && hash.data[12]==0x8e && hash.data[31]==0xfc);
    const uint8_t base[]={2}, exponent[]={10}, modulus[]={0,0,3,232};
    uint8_t modexp_output[6]; memset(modexp_output,0xa5,sizeof(modexp_output));
    assert(AIRBENDER_HAS_MODEXP==1);
    assert(zkvm_modexp(base,1,exponent,1,modulus,4,modexp_output+1)==ZKVM_EOK);
    assert(modexp_output[0]==0xa5 && modexp_output[1]==0 && modexp_output[2]==0 && modexp_output[3]==0 && modexp_output[4]==24 && modexp_output[5]==0xa5);
    assert(zkvm_secp256k1_verify(&msg,&sig,&pk,&verified)==ZKVM_EFAIL && verified);
    assert(zkvm_secp256r1_verify(&msg,&sig,&pk,&verified)==ZKVM_EFAIL && verified);
    assert(zkvm_secp256k1_ecrecover(&msg,&sig,0,&pk)==ZKVM_EFAIL);
    zkvm_bn254_g1_point g={0},sum={0}; g.data[31]=1; g.data[63]=2;
    zkvm_bn254_scalar scalar={0}; scalar.data[31]=1;
    assert(zkvm_bn254_g1_add(&g,&sum,&sum)==ZKVM_EOK && memcmp(&g,&sum,sizeof(g))==0);
    assert(zkvm_bn254_g1_mul(&g,&scalar,&sum)==ZKVM_EOK && memcmp(&g,&sum,sizeof(g))==0);
    zkvm_bn254_pairing_pair bn_pair={0};
    assert(zkvm_bn254_pairing(&bn_pair,0,&verified)==ZKVM_EOK && verified);
    zkvm_kzg_commitment commitment={0}; commitment.data[0]=0xc0;
    zkvm_kzg_proof proof=commitment;
    assert(zkvm_kzg_point_eval(&commitment,&msg,&msg,&proof,&verified)==ZKVM_EOK && verified);
    zkvm_bls12_381_fp fp={0}; zkvm_bls12_381_fp2 fp2={0};
    zkvm_bls12_381_g1_point p1={0},q1={0},zero1={0};
    zkvm_bls12_381_g2_point p2={0},q2={0},zero2={0};
    assert(zkvm_bls12_map_fp_to_g1(&fp,&p1)==ZKVM_EOK);
    assert(zkvm_bls12_map_fp2_to_g2(&fp2,&p2)==ZKVM_EOK);
    assert(zkvm_bls12_g1_add(&p1,&zero1,&q1)==ZKVM_EOK && memcmp(&p1,&q1,sizeof(p1))==0);
    assert(zkvm_bls12_g2_add(&p2,&zero2,&q2)==ZKVM_EOK && memcmp(&p2,&q2,sizeof(p2))==0);
    zkvm_bls12_381_g1_msm_pair pair1={0}; pair1.point=p1; pair1.scalar=scalar;
    zkvm_bls12_381_g2_msm_pair pair2={0}; pair2.point=p2; pair2.scalar=scalar;
    assert(zkvm_bls12_g1_msm(&pair1,1,&q1)==ZKVM_EOK && memcmp(&p1,&q1,sizeof(p1))==0);
    assert(zkvm_bls12_g2_msm(&pair2,1,&q2)==ZKVM_EOK && memcmp(&p2,&q2,sizeof(p2))==0);
    zkvm_bls12_381_pairing_pair bls_pair={0};
    assert(zkvm_bls12_pairing(&bls_pair,1,&verified)==ZKVM_EOK && verified);
    assert(airbender_blake2s256(abc,3,&hash)==ZKVM_EOK && hash.data[0]==0x50 && hash.data[31]==0x82);
    /* Recording: a point multiplication takes hints; end(NULL, 0) is a size query. */
    static uint32_t tape[1<<16];
    airbender_crypto_record_begin();
    assert(zkvm_bn254_g1_mul(&g,&scalar,&sum)==ZKVM_EOK && memcmp(&g,&sum,sizeof(g))==0);
    size_t words=airbender_crypto_record_end(NULL,0);
    assert(words>0 && words<=sizeof(tape)/sizeof(tape[0]));
    assert(airbender_crypto_record_end(tape,words)==words);
    assert(airbender_crypto_record_end(tape,words)==0);
    /* Byte swap and the bigint operations: little-endian, flags as documented. */
    assert(airbender_bswap32(0x01020304u)==0x04030201u);
    airbender_u256 a={0}, b={0}; a.data[0]=3; b.data[0]=4;
    assert(airbender_u256_mul_low(&a,&b) && a.data[0]==12 && a.data[1]==0);
    assert(!airbender_u256_add(&a,&b,true) && a.data[0]==17);
    assert(!airbender_u256_eq(&a,&b));
    airbender_u256_copy(&b,&a);
    assert(airbender_u256_eq(&a,&b) && b.data[0]==17);
    assert(!airbender_u256_sub(&a,&b,false) && a.data[0]==0 && a.data[31]==0);
    assert(airbender_u256_sub(&a,&b,false) && a.data[0]==(uint8_t)(256-17) && a.data[31]==0xff);
    memset(&a,0,sizeof(a)); memset(&b,0,sizeof(b)); a.data[16]=1; b.data[16]=1; /* 2^128 */
    assert(!airbender_u256_mul_low(&a,&b) && a.data[0]==0 && a.data[31]==0);
    a.data[16]=1;
    airbender_u256_mul_high(&a,&b);
    assert(a.data[0]==1 && a.data[16]==0);
    memset(&a,0,sizeof(a)); memset(&b,0,sizeof(b)); a.data[0]=2; b.data[0]=5;
    assert(!airbender_u256_sub_and_negate(&a,&b,false) && a.data[0]==3);
    /* Streaming hashes: Keccak-256 of "abc" in two parts, SHA3-256 reset and reused. */
    airbender_keccak_state state;
    airbender_keccak256_init(&state);
    airbender_keccak256_update(&state,abc,1);
    airbender_keccak256_update(&state,abc+1,2);
    airbender_keccak256_finalize(&state,&hash);
    assert(hash.data[0]==0x4e && hash.data[31]==0x45);
    airbender_sha3_256_init(&state);
    airbender_sha3_256_update(&state,abc,3);
    airbender_sha3_256_finalize_reset(&state,&hash);
    assert(hash.data[0]==0x3a && hash.data[31]==0x32);
    airbender_sha3_256_update(&state,abc,3);
    airbender_sha3_256_finalize(&state,&hash);
    assert(hash.data[0]==0x3a && hash.data[31]==0x32);
    /* In-place BLAKE2f; zero rounds give a fixed IV-derived state. */
    zkvm_blake2f_state blake_state={0}, blake_saved={0};
    zkvm_blake2f_message blake_message={0}; zkvm_blake2f_offset blake_offset={0};
    assert(zkvm_blake2f(0,&blake_state,&blake_message,&blake_offset,0)==ZKVM_EOK);
    assert(blake_state.data[0]==0x08 && blake_state.data[63]==0x5b);
    blake_saved=blake_state;
    assert(zkvm_blake2f(12,&blake_state,&blake_message,&blake_offset,2)==ZKVM_EFAIL);
    assert(memcmp(&blake_saved,&blake_state,sizeof(blake_state))==0);
    return 0;
}
#endif
