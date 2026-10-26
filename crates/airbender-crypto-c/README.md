# Airbender C crypto accelerators

Thin bindings to existing Airbender crypto and checked hinted operations.
Implements all 19 functions from [zkvm_accelerators.h at
d5f18133720b7cfc1547a91f78f8e152ec80b2bb](https://github.com/eth-act/zkevm-standards/blob/d5f18133720b7cfc1547a91f78f8e152ec80b2bb/standards/c-interface-accelerators/zkvm_accelerators.h).
The upstream header is vendored unchanged in `include/` (Git blob
`4e9818d90084676a334cbf1cbdfb64d1eae1dbea`). The revision is pinned here for
reproducibility.

## Implementation mapping

All paths below are relative to `crates/airbender-crypto/src`, unless qualified. The
precompiles call the `precompiles` endpoints, the crypto half of zksync-os's precompiles
(zksync-os `e855bca2`, ModExp `185a0568`); the C side keeps only the encodings and the
input-size policies. Every export takes the same path with or without hints except where
the hint column says otherwise, and gives the same result either way. The hint column
describes the path with an oracle (see [Hints](#hints)).

| C function | Existing implementation / adapter | With hints |
| --- | --- | --- |
| `zkvm_keccak256` | `sha3::Keccak256`, `MiniDigest` | None; guest Keccak delegation is independent |
| `zkvm_sha256` | `sha256::Sha256` | None |
| `zkvm_ripemd160` | `ripemd160::Ripemd160`; 12 leading zero bytes | None |
| `zkvm_secp256k1_verify` | `secp256k1::verify_prehash_with_hooks`; accepts both s forms | `hints::Secp256k1HooksWithOracle`: scalar inverse, affine multiplication with division hints (no zksync-os counterpart) |
| `zkvm_secp256k1_ecrecover` | IDs 0/1: `precompiles::secp256k1_ecrecover` (zksync-os `ecrecover_inner`); IDs 2/3 (`x = r + n`): `secp256k1::recover_with_hooks`; raw x/y output | The same hooks, plus the square root of the decompression |
| `zkvm_secp256r1_verify` | `secp256r1::verify_prehash` | None |
| `zkvm_bn254_g1_add` | `precompiles::bn254_g1_add` | Affine addition, division hint |
| `zkvm_bn254_g1_mul` | `precompiles::bn254_g1_mul`; arbitrary 256-bit scalars | Affine multiplication, division hints |
| `zkvm_bn254_pairing` | `precompiles::bn254_pairing_check` over the EIP-197 bytes of the pairs | G2 slope hints (subgroup check and lines), G1 division hints, residue witness instead of the final exponentiation |
| `zkvm_bls12_g1_add` | `precompiles::bls12_381_g1_add` (EIP-2537, no subgroup requirement) | Affine addition, division hint |
| `zkvm_bls12_g2_add` | `precompiles::bls12_381_g2_add` | Affine addition, Fq2 inverse hint of the norm |
| `zkvm_bls12_g1_msm` | `precompiles::bls12_381_g1_msm`; subgroup check of every point | Affine subgroup checks; up to two pairs affine GLV, more Pippenger with one inverse hint |
| `zkvm_bls12_g2_msm` | `precompiles::bls12_381_g2_msm` | Affine subgroup checks; one Fq2 inverse hint |
| `zkvm_bls12_pairing` | `precompiles::bls12_381_pairing_check`; both subgroups checked | Affine subgroup checks, residue witness |
| `zkvm_bls12_map_fp_to_g1` | `precompiles::bls12_381_map_fp_to_g1` (SWU, then cofactor clearing) | Inverse and square-root hints, affine cofactor clearing |
| `zkvm_bls12_map_fp2_to_g2` | `precompiles::bls12_381_map_fp2_to_g2` | The same over Fq2 |
| `zkvm_kzg_point_eval` | `precompiles::kzg_verify_proof` (compressed points, canonical z/y) | Square roots of the decompressions, affine subgroup checks and arithmetic, residue witness |
| `zkvm_modexp` | `modexp::modexp` and `modexp::write_modexp_output`; caller-provided output | zksync-os's advice algorithm: wide-quotient hints for moduli up to 32 bytes, reduction hints above |
| `zkvm_blake2f` | `blake2f::compress`: software BLAKE2b compression, EIP-152 / RFC 7693 §3.2 | None; separate from existing BLAKE2s |

### Existing hinted operations callable directly from C

Include `airbender_crypto.h`. Calls take operands and outputs, never a hint producer
or callback. These additional symbols are Airbender-specific; they do not come
from the 19-function standard.

| C binding(s) | Existing Rust API | With hints |
| --- | --- | --- |
| `airbender_secp256k1_fe_inverse`, `airbender_secp256k1_scalar_inverse` | `Secp256k1Hooks::{fe_invert_and_assign, scalar_invert_and_assign}` | `Secp256k1HooksWithOracle`: base field / scalar inverse hints |
| `airbender_secp256k1_fe_divide` | `Secp256k1Hooks::fe_divide` | Division hint |
| `airbender_secp256k1_fe_sqrt` | `Secp256k1Hooks::fe_sqrt_and_assign`, then the even root of the two | Square-root hint (or proof of none) |
| `airbender_{bn254,bls12}_fq_inverse` | `hints::{bn254,bls12_381}_fq_inverse`; field inverse without hints | Inverse hint |
| `airbender_{bn254,bls12}_fq_divide` | `hints::{Bn254,Bls12381}FqDivider` | Division hint |
| `airbender_bls12_fq2_{inverse,divide}` | `hints::bls12_381_fq2_inverse` (then a multiplication) | One base-field inverse hint, of the norm |
| `airbender_bn254_fq2_{inverse,divide}` | the same norm construction over `hints::bn254_fq_inverse` (in this crate) | One base-field inverse hint, of the norm; no hint of its own |
| `airbender_{bn254,bls12}_fq12_{inverse,divide}` | `hints::{bn254,bls12_381}_fq12_inverse` (quotient: then a multiplication) | Fq12 inverse hint |
| `airbender_bn254_g1_to_affine`, `airbender_bls12_g1_to_affine`, `airbender_bls12_g2_to_affine` | `hinted_ops::to_affine_with_inverse`, `bls12_381::{g1,g2}::into_affine_with_inverse`; `into_affine` without hints | Inverse hint of Z; infinity and Z=1 take none |
| `airbender_bls12_{g1,g2}_is_in_subgroup` | `is_in_subgroup_with_divider`; projective `is_in_correct_subgroup_assuming_on_curve` without hints | Division hints; infinity takes none |
| `airbender_bls12_{g1,g2}_clear_cofactor` | `clear_cofactor_with_divider`; `clear_cofactor` without hints | Division hints, without assuming subgroup membership; infinity takes none |
| `airbender_blake2s256` | `blake2s::Blake2s256`, `MiniDigest` | None. Delegated with `delegation` (`airbender-crypto/proving`), software otherwise |

The standard curve entry points already compose these checked operations internally;
C consumers do not need to split recovery, maps, MSM, or pairing into field calls.
Prepared pairing representations, raw line slopes, and residue witnesses are not
separate public C APIs.

For the additional field APIs, integers are canonical big-endian, and extension
fields concatenate base-prime coefficients c0-first recursively. This is a stable
byte interface, not a view of Rust/Montgomery memory. BN254 Fq2 here is real-first,
unlike the standard's imaginary-first BN254 pairing coordinates. Jacobian inputs
are X || Y || Z and normalization checks the curve equation; it does not check
subgroup membership. Zero inverses/divisors and noncanonical operands return EFAIL
without changing output. Details and exact sizes are in the platform header.

### Trust boundaries

Every hint is checked by the operation that asks for it before it is used: an inverse
or a quotient with one multiplication, a square root with one squaring (and a claimed
nonsquare with the square of a root of its negation), a pairing's residue witness by
its equation. A wrong or missing answer either fails execution or is ignored; it never
returns `ZKVM_EFAIL` and never changes a result. A noncanonical answer fails execution
too, except a secp256k1 quotient (`airbender_secp256k1_fe_divide`, and the affine
arithmetic of the secp256k1 recovery and verification), which the RV32 guest takes as it
is and checks modulo p: a noncanonical representative of the right quotient is the same
element. A square-root check admits both roots, so no result depends on which one the
prover gives: `airbender_secp256k1_fe_sqrt` returns the even root (with or without hints;
1 is odd, so √1 = p − 1), point decompressions pick the root of the encoded parity or sign,
and the SWU maps fix the sign of `y` from their input.
Which hint words are ignored is fixed by the protocol: a bn254 pairing's G2 slope answer is
fixed-size, and the slopes that a chain does not use, or all of them when the prover marks the
chain as exceptional, are skipped (the projective computation runs in place of an exceptional
chain).
A pairing's claimed outcome cannot be chosen by the prover: a claimed identity comes with
a residue witness that must pass, and a claimed non-identity is settled by the exact
final exponentiation with a checked inverse.

## C contract and conventions

Include `airbender_crypto.h` with C11 or C++11 or newer. It includes the pinned
upstream header and advertises `AIRBENDER_HAS_MODEXP=1` and
`AIRBENDER_HAS_BLAKE2F=1`. `AIRBENDER_BLAKE2F_ACCELERATED=0` identifies the software
fallback. Use
the upstream struct types: all byte structs have alignment 8, including on RV32.
Do not compile with
`-fshort-enums` (the status return is a signed C int), or struct packing options.
`size_t` follows the target pointer width; `bool` follows the target C ABI.

All pointers must be non-null, valid, initialized for the declared input lengths,
and appropriately aligned; outputs must be writable for their declared sizes. The one
exception is the size query `airbender_crypto_record_end(NULL, 0)` (see [Hints](#hints)).
For zero-length arrays pass a valid aligned dummy pointer. The functions assert
on null/alignment/length-overflow violations; they cannot detect dangling pointers
or undersized allocations. Input/output overlap is allowed, including in-place
point addition and hashes whose output overlaps input. Do not concurrently access
an output without synchronization. Hints on a shared CSR stream also require
serialization by the caller.

Malformed cryptographic inputs return `ZKVM_EFAIL` (-1); **all output bytes stay
unchanged on failure**, including `bool` outputs. Well-formed verification inputs
that fail the equation return `ZKVM_EOK` with `false`. No Rust panic unwinds into
C. Resource exhaustion, internal invariant failures, and pointer contract violations
abort the process (native) or fail guest execution (RV32). A wrong or missing hint
either does the same or is ignored, and never changes a result (see
[Trust boundaries](#trust-boundaries)).

Integers and curve coordinates use big-endian bytes. Infinity is all-zero x/y.
BN254 Fp2 uses EIP-197's imaginary-first ordering; BLS Fp2 uses EIP-2537's
real-first ordering. BLS coordinates use compact 48-byte limbs, which the bindings
widen to the endpoints' 64-byte EIP-2537 encoding (and narrow back for outputs); the
padded representation does not cross the C ABI. Addition checks on-curve
membership; MSM/pairing also check subgroups. Scalars for BN/BLS multiplication
are arbitrary 256-bit integers, reduced modulo the group order, not rejected for
being above it. KZG z/y are canonical Fr elements and commitments/proofs use
48-byte compressed BLS points. Maps include cofactor clearing.

Recovery IDs are the primitive's 0..3, not Ethereum transaction v (27/28 or
EIP-155). Recovery returns exactly 64 bytes of x || y, without SEC1's 0x04 prefix.
ECDSA verification accepts both s forms; no transaction-level low-s rule is added.
Empty BN254 pairing is true. Empty BLS MSM/pairing fails, following EIP-2537.
These encoding, empty-input, recovery-ID and invalid-input policies resolve details
the header leaves implicit using its referenced EIPs and the underlying APIs.

BLAKE2f uses the standard's in-place state pointer, little-endian state/message/
counter bytes, and f=0 or f=1. Invalid f returns EFAIL and preserves the state.
`rounds` is a numeric C `uint32_t`: pass 12 for twelve rounds. The upstream comment
calls rounds big-endian, but the ABI passes a scalar; callers must decode EVM's
serialized rounds before calling. The software implementation is always available,
including in CSR/delegation builds, and has no heap or hint dependency.

## ModExp output

`zkvm_modexp` takes separate big-endian base, exponent and modulus byte spans.
The caller supplies at least `mod_len` writable output bytes; exactly that span is
written, left-padded with zeros. A zero or empty modulus produces zero or empty
output. A zero exponent produces `1 % modulus`, including `0^0`. Inputs may
partially overlap output: the result is computed into an owned buffer before any
output byte is written. The library supports arbitrary lengths; an EIP-7823/Fusaka
consumer can enforce lengths <= 1024 bytes for each input and reuse a 1024-byte
output buffer. Source revision and port details are recorded in
`../airbender-crypto/src/modexp/README.md`.

## Allocation

The pairing checks, the MSMs and ModExp allocate their working vectors from the global
allocator, as the zksync-os endpoints do (the bindings add the EIP-2537 MSM and pairing
inputs, widened from the compact encoding). With hints, the bn254 pairing check also
builds the 16 KB line table of each G2 point on the heap. Without hints, the other
exports allocate nothing: `tests/no_alloc.rs` checks it natively, KZG included, with no
recording active. That test does not cover their hinted paths, and no test checks
allocation on the RV32 hinted path (the RV32 fixture checks results and tape
consumption). The native recorder allocates its tape. Native builds use Rust std's
allocator; RV32 builds use the global allocator of the embedding guest, or airbender-rt's
with `c-runtime` (initialized with `airbender_crypto_init_heap`; size the heap for the
largest pairing and MSM inputs).

## Hints

This crate owns no hint protocol, producer or oracle of its own. The exports call
airbender-crypto's endpoints and hinted operations with an oracle when there is one, and
without one otherwise; nothing else selects a path. The protocol is zksync-os's: a query
is two CSR writes, the query ID and then the address of a request in guest memory
(`0x4005_0011` for the 15 field hint ops, `0x4005_0010` for ModExp reductions,
`0x4005_0031` for the ModExp wide quotient), and the answer is raw `u32` words in the
guest's representation (the Montgomery limbs of the delegated fields); the protocol adds no
framing. The field hints and the wide quotient have no length prefix; the answer to a ModExp
reduction (`0x4005_0010`) starts with two length words (the lengths of the quotient and of
the remainder, in `u32` words), which are not Montgomery limbs, followed by their words.
airbender-host's replay answers each CSR read with the next input word, and the
query writes have no effect on it (its UART state machine only reacts to a `u32::MAX`
word), so the guest reads the answers as positional input.

Features:

- `csr-oracle` (RV32 only, requires `delegation`): the exports take hints through the
  non-determinism CSR, from a tape that the host recorded. Native builds ignore it.
- `delegation` (RV32 only; a compile error natively): `airbender-crypto/proving`, the
  delegated big-integer arithmetic.
- `c-runtime` (RV32 only): airbender-rt's allocator and panic handler, see below.

Hosts record the tape natively, before the RV32 run, as zksync-os's native run does: a
C/C++ host builds the native library *without features* (it records the hints of the
guest's delegated arithmetic without delegating), then makes the guest's calls, with the
same inputs and in the same order, between `airbender_crypto_record_begin()` and
`airbender_crypto_record_end(words, capacity)`. Each hinted operation runs the guest's
hinted path and appends the guest's answers to the tape. The host places the words in the
guest's input where the guest makes the calls: after the ordinary inputs, for a guest that
reads all of them first. Rust hosts use `airbender_crypto::hints::record_hints` (with
`airbender-crypto/alloc`). RV32 builds without `csr-oracle` read no hints and send no
queries.

The recording API is per thread: this crate keeps a recorder (airbender-crypto's
`RecordingHintOracle`) for each thread that records. `record_begin` while a recording is
active starts it over, and drops the words of an ended recording that `record_end` could
not copy; `record_end` without a recording, or after its words were copied, returns 0;
`record_end(NULL, 0)` is a size query, which returns the number of words and keeps them
for the next call, as any capacity too small does; calls outside a recording take the path
without hints. No call sequence panics.

Recordings on several threads are independent only if nothing in the same native build
turns on `airbender-crypto/proving` (this crate's `delegation` feature is a compile error
natively, but another crate in the dependency graph can enable it, since Cargo unifies
features). With `proving`, the native fields are the emulated delegated ones, which share
one scratch space between all threads: record, and call anything that uses those fields,
from one thread at a time.

`airbender-crypto/secp256k1-shamir-msm` changes only the projective secp256k1
multiplication. The hinted recoveries and verifications take the affine one
(`Secp256k1HooksWithOracle::FE_DIVIDE_IS_CHEAP`), so the feature does not change the
tape, and combining it with `csr-oracle` is allowed.

## Building and linking

Use the repository's pinned nightly. The workspace currently expects a compatible
`../zksync-airbender` checkout containing `prover_pipeline` (tested at
`d68f312aaa86abbd53a3eed91757b98ea7254fa8`); an older checkout
cannot resolve the workspace even for a targeted build. No sibling checkout is
changed by the build scripts.

Native development/integration:

```sh
cargo test -p airbender-crypto-c
cargo rustc -p airbender-crypto-c --release --crate-type staticlib
sh crates/airbender-crypto-c/tests/c-abi.sh
```

The static library is `target/release/libairbender_crypto_c.a`. The package declares
only the `rlib` crate type, so Rust dependents never build the archive. Native builds
link Rust std and use its allocator; append `-- --print native-static-libs` to the
`cargo rustc` command to list required platform libraries. The C test
script links a C11 consumer and a separate C++11 consumer. They include the
platform header and call all 19 standard symbols and all 26 additional native
Airbender symbols. This crate does not claim to build zilkworm.

RV32 static library (uses `airbender-rt`; C supplies its entry point and memory layout):
`sh crates/airbender-crypto-c/build-rv32.sh` builds with delegated arithmetic and
no hints; `FEATURES='csr-oracle delegation'` adds hints from a recorded tape, and
`FEATURES=''` selects software arithmetic. Every variant enables `c-runtime`. The three
configurations are tested on the FullUnsigned machine. Software secp inversion uses
unsigned widening multiplication supported by that machine. The equivalent explicit
command is:

```sh
RUSTFLAGS="-C panic=abort -C target-feature=+m,-unaligned-scalar-mem,+relax -C passes=lower-atomic" \
cargo +nightly-2026-08-09 rustc -p airbender-crypto-c --release \
  --target riscv32im-risc0-zkvm-elf --crate-type staticlib \
  -Z build-std=core,alloc,panic_abort,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem \
  --features c-runtime,delegation
```

Apply RUSTFLAGS to the entire dependency graph: passing `lower-atomic` only to
the final crate leaves unresolved atomic libcalls in dependencies. Add `csr-oracle`
for hints; omit `delegation` (and `csr-oracle`) for software big-integer arithmetic.
Output: `target/riscv32im-risc0-zkvm-elf/release/libairbender_crypto_c.a`.
With `c-runtime`, the RV32 archive uses `airbender-rt`'s Talc allocator and
panic/OOM handlers, with `external-startup` and the runtime's default boot and
memory-layout features disabled. It exports no `_start` and no strong libc memory
routines; compiler-builtins supplies weak fallbacks which a C runtime can replace.
The archive has no unresolved external symbol requirements, including randomness
hooks. It requires no C `malloc` or `free`. The consumer's startup sets the stack,
initializes data/BSS, and moves constants into RAM before using delegation, then
initializes the allocator exactly once before operations that allocate:

```c
/* The linker/startup provides an exclusive, writable, word-aligned heap range. */
airbender_crypto_init_heap(heap_start, heap_end);
```

The range must remain live for the guest's execution and must not also be managed
by another allocator. The C bridge forwards directly to
`airbender_rt::allocator::init`; it does not initialize data sections or reset the
heap on each operation. A repeated initializer call traps before resetting the allocator.
Rust guests using `airbender_rt::start` already initialize
the allocator and leave `c-runtime` disabled. The C archive requires neither
`_start_rust` nor `riscv_common`'s linker-layout symbols. The integration fixture
provides an independent C-owned startup, linker script and strong memory routines.

`tests/runtime.c` is the RV32 integration fixture. After heap initialization it runs every
hinted export: the field bindings (including zero operands and a nonsquare), BLS12-381
maps, additions, MSMs (both algorithms, with an infinity point and a zero scalar),
subgroup checks and cofactor clearing of a point outside the subgroup, Jacobian
normalization (Z=3 and Z=0), true and false BLS12-381 and BN254 pairings (empty and with
infinity for BN254), BN254 multiplications by zero, the group order and 2^256-1,
recovery, both verifications, two KZG vectors, and ModExp with moduli of 0, 1, 4, 32 and
33 bytes (256 times for the 4-byte one, which exercises airbender-rt's allocation and
deallocation). It folds every result into a digest. Its gaps: it adds no two distinct points
(bn254 and BLS12-381 additions are doublings, opposite points and the point at infinity
only), and has no BLS12-381 pairing with a point at infinity; `airbender-crypto`'s tests
cover such inputs through the same endpoints, but natively. Run the reproducible
build/link/execution test with:

```sh
python3 crates/airbender-crypto-c/tests/rv32-runtime.py --all
```

It uses FullUnsigned and a 1 MiB heap. It first builds `tests/runtime.c` natively against
the native library without features, and runs it without and with a recording (the
digests must agree); that records the tape and the digest. Then, for each of the
configurations plain, `delegation` and `csr-oracle delegation`, it builds the RV32
archive, links the fixture with its own startup, and runs
`crates/airbender-crypto/tests/rv32_runtime.rs`, which checks that:

- the guest ends with status 0 and the native digest, in every configuration;
- a `csr-oracle` guest reads the whole tape and no more (a word past it stays unread), and
  the other configurations get an empty tape and send no query;
- repeated heap initialization, a truncated tape (at its last word and halfway) and an
  all-zero tape trap;
- a tampered tape word (XOR 1) makes the guest trap or leaves its status and digest
  unchanged. Every word is tampered with for a tape of up to 512 words; for a longer one,
  the first and last word of every answer, plus 256 words drawn with a fixed seed;
- the other root in the secp256k1 square-root answers (alone and all at once), a valid
  answer too, leaves the status and digest unchanged.

The runtime test requires Python 3, clang and the pinned Rust `llvm-tools-preview`
component. `--features 'csr-oracle delegation'` selects a single configuration.

The archive is not a complete C executable; the application supplies its entry point
and sufficient stack/heap. There is no EVM gas limit or arbitrary input-size cap in
this library.

## Test scope

C tests cover layouts and all exports (`abi.c`), buffer/overlap contracts
(`buffers.rs`), compact point encodings, status handling, subgroup checks, cofactor
clearing and normalization (`curves.rs`), signature and recovery encodings
(`signatures.rs`), and the field bindings (`fields.rs`). `c_api_vectors.rs` runs the
zksync-os vectors of the endpoints (all 62 ModExp vectors, the ecrecover, bn254 pairing
and KZG vectors, and c-kzg vectors) and their edge inputs through the C exports: infinity,
points off the curve or outside the subgroup, scalars of at least the group order, empty
MSM and pairing inputs, ModExp with empty operands or a zero modulus. The native tests
run each export without and with a recording, require the same result, and compare each
tape with the tape of the Rust hinted operation that the export stands for.
`recorder_misuse.rs` covers the recording API's misuse cases, and `no_alloc.rs` the
exports that allocate nothing. The hinted operations themselves, their checks against
malicious oracles, and their equivalence with the unhinted paths are tested in
`airbender-crypto`.

## BLAKE2f reference tests

The unit test beside `blake2f::compress` in `airbender-crypto/src/blake2f/mod.rs`
checks all 256 official BLAKE2b-512 known-answer vectors from the pinned BLAKE2
repository. Test-only hash framing exercises the compression function with keyed
messages of lengths 0 through 255 bytes, including message-block boundaries and
both final and non-final blocks. Expected hashes are checked in verbatim; tests
need no external tools or network. Provenance and the pinned revision are in
`../airbender-crypto/src/blake2f/testdata/README.md`.

```sh
cargo test -p airbender-crypto --lib blake2f_matches_official_blake2b_vectors
```
