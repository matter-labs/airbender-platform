//! The cfgs that the sources test in place of the `bigint_ops` and `proving` features:
//!
//! - `cfg(bigint_ops)`: the bigint delegation does the field arithmetic, and the fields, scalars
//!   and big integers have its representations. The feature, except on `riscv64`. On `riscv32`
//!   the delegation is the real one; on every other target it is emulated in software
//!   (`cfg(emulated_delegation)`), which is how a host runs and tests the code of the RV32 guest.
//! - `cfg(emulated_delegation)`: `cfg(bigint_ops)` off `riscv32`. The emulation keeps its scratch
//!   space per thread there (and links `std` for that): a host process computes on several
//!   threads, the guest on one.
//! - `cfg(proving)`: the delegated hashes of the proving target (the delegation itself on
//!   `riscv32`, its emulation elsewhere). The feature, except on `riscv64`. The feature also
//!   enables `bigint_ops` and `hinted_operations`.
//!
//! A `riscv64` guest has no delegation and does not emulate one: with either feature it builds
//! the software arithmetic of a 64-bit target, as without them. The other features that
//! `proving` enables (`hinted_operations`, `secp256k1-static-context`) are tested as features
//! and stay in force there; the delegated hashes are already for `riscv32` only.
//!
//! Sources never test `feature = "bigint_ops"` or `feature = "proving"` directly, and no
//! representation depends on `cfg(test)`: the unit tests run on the fields of the build, so
//! each configuration is tested by building it.
//!
//! The four configurations of a native build (`hinted_operations` is independent of the
//! delegation; the oracle side that answers the hints is `oracle_implementations`):
//!
//! | Features | Fields | For |
//! | --- | --- | --- |
//! | none | software, 64-bit | forward execution without an oracle |
//! | `hinted_operations` | software, 64-bit | the twin of the `riscv64` guest |
//! | `bigint_ops` | emulated delegation | the RV32 arithmetic without an oracle |
//! | `bigint_ops`, `hinted_operations` | emulated delegation | the twin of the `riscv32` guest |
//!
//! And the representation of the secp256k1 base field that follows from them:
//!
//! - `cfg(secp256k1_field_4x64)`: four saturated 64-bit limbs (`field_4x64`), on 64-bit targets
//!   without `cfg(bigint_ops)`: hosts and the `riscv64` guest, which are twins then.
//! - `cfg(fp_lazy_reduction)`: the products of the bn254 and bls12-381 extension towers reduce
//!   once per coefficient (`lazy_fp`), on the same targets: their base fields are arkworks' `Fp`
//!   there.
//! - `cfg(secp256k1_weak_field)`: an element is any representative below `2^256` and has no
//!   magnitude, so the formulas neither track magnitudes nor normalize: that field or the
//!   delegated one. Without it the field has lazy reduction with magnitudes (`field_10x26`, on
//!   32-bit targets without the delegation).

use std::env;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(bigint_ops)");
    println!("cargo::rustc-check-cfg=cfg(proving)");
    println!("cargo::rustc-check-cfg=cfg(secp256k1_field_4x64)");
    println!("cargo::rustc-check-cfg=cfg(secp256k1_weak_field)");
    println!("cargo::rustc-check-cfg=cfg(fp_lazy_reduction)");
    println!("cargo::rustc-check-cfg=cfg(emulated_delegation)");

    let feature = |name: &str| env::var_os(format!("CARGO_FEATURE_{name}")).is_some();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("cargo sets the target architecture");
    let has_delegation = arch != "riscv64";

    let bigint_ops = feature("BIGINT_OPS") && has_delegation;
    if bigint_ops {
        println!("cargo::rustc-cfg=bigint_ops");
        if arch != "riscv32" {
            println!("cargo::rustc-cfg=emulated_delegation");
        }
    }
    let pointer_width =
        env::var("CARGO_CFG_TARGET_POINTER_WIDTH").expect("cargo sets the pointer width");
    let field_4x64 = !bigint_ops && pointer_width == "64";
    if field_4x64 {
        println!("cargo::rustc-cfg=secp256k1_field_4x64");
        println!("cargo::rustc-cfg=fp_lazy_reduction");
    }
    if bigint_ops || field_4x64 {
        println!("cargo::rustc-cfg=secp256k1_weak_field");
    }
    if feature("PROVING") && has_delegation {
        println!("cargo::rustc-cfg=proving");
    }
}
