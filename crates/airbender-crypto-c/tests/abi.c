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
LAYOUT(airbender_bytes_144,144); LAYOUT(airbender_bytes_288,288);
LAYOUT(airbender_bytes_384,384); LAYOUT(airbender_bytes_576,576);
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
    /* Extension field one is in the first base-field component. */
#define FIELD_TEST(prefix, type, width) do { \
    type a={0}, out={0}; a.data[(width)-1]=1; \
    assert(prefix##_inverse(&a,&out)==ZKVM_EOK && memcmp(&a,&out,sizeof(a))==0); \
    assert(prefix##_divide(&a,&a,&out)==ZKVM_EOK && memcmp(&a,&out,sizeof(a))==0); \
    type zero={0}; \
    assert(prefix##_divide(&a,&zero,&out)==ZKVM_EFAIL && memcmp(&a,&out,sizeof(a))==0); \
} while(0)
    FIELD_TEST(airbender_secp256k1_fe,zkvm_bytes_32,32);
    FIELD_TEST(airbender_bn254_fq,zkvm_bytes_32,32);
    FIELD_TEST(airbender_bn254_fq2,zkvm_bytes_64,32);
    FIELD_TEST(airbender_bn254_fq12,airbender_bytes_384,32);
    FIELD_TEST(airbender_bls12_fq,zkvm_bytes_48,48);
    FIELD_TEST(airbender_bls12_fq2,zkvm_bytes_96,48);
    FIELD_TEST(airbender_bls12_fq12,airbender_bytes_576,48);
#undef FIELD_TEST
    zkvm_bytes_32 one={0}; one.data[31]=1;
    assert(airbender_secp256k1_scalar_inverse(&one,&hash)==ZKVM_EOK && memcmp(&one,&hash,sizeof(one))==0);
    /* The even root: p - 1 for 1, 2 for 4. */
    assert(airbender_secp256k1_fe_sqrt(&one,&hash)==ZKVM_EOK && hash.data[0]==0xff && hash.data[27]==0xfe && hash.data[31]==0x2e);
    zkvm_bytes_32 four={0}; four.data[31]=4;
    assert(airbender_secp256k1_fe_sqrt(&four,&hash)==ZKVM_EOK && hash.data[0]==0 && hash.data[31]==2);
    /* Recording: a scalar inverse is one 8-word answer; end(NULL, 0) is a size query. */
    uint32_t tape[8]={0};
    airbender_crypto_record_begin();
    assert(airbender_secp256k1_scalar_inverse(&one,&hash)==ZKVM_EOK);
    assert(airbender_crypto_record_end(NULL,0)==8);
    assert(airbender_crypto_record_end(tape,8)==8);
    assert(airbender_crypto_record_end(tape,8)==0);
    zkvm_bytes_96 bn_xyz={0}; memcpy(bn_xyz.data,g.data,64); bn_xyz.data[95]=1;
    assert(airbender_bn254_g1_to_affine(&bn_xyz,&sum)==ZKVM_EOK && memcmp(&sum,&g,sizeof(g))==0);
    airbender_bytes_144 bls_xyz1={0}; memcpy(bls_xyz1.data,p1.data,96); bls_xyz1.data[143]=1;
    airbender_bytes_288 bls_xyz2={0}; memcpy(bls_xyz2.data,p2.data,192); bls_xyz2.data[239]=1;
    assert(airbender_bls12_g1_to_affine(&bls_xyz1,&q1)==ZKVM_EOK && memcmp(&q1,&p1,sizeof(q1))==0);
    assert(airbender_bls12_g2_to_affine(&bls_xyz2,&q2)==ZKVM_EOK && memcmp(&q2,&p2,sizeof(q2))==0);
    assert(airbender_bls12_g1_is_in_subgroup(&p1,&verified)==ZKVM_EOK && verified);
    assert(airbender_bls12_g2_is_in_subgroup(&p2,&verified)==ZKVM_EOK && verified);
    assert(airbender_bls12_g1_clear_cofactor(&p1,&q1)==ZKVM_EOK);
    assert(airbender_bls12_g2_clear_cofactor(&p2,&q2)==ZKVM_EOK);
    assert(airbender_bls12_g1_is_in_subgroup(&q1,&verified)==ZKVM_EOK && verified);
    assert(airbender_bls12_g2_is_in_subgroup(&q2,&verified)==ZKVM_EOK && verified);
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
