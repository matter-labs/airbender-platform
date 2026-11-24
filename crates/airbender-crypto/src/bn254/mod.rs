pub mod curves;
pub mod fields;
#[cfg(all(
    any(feature = "oracle_implementations", test),
    target_pointer_width = "64"
))]
pub mod oracle_implementations;

pub use self::curves::{g1, g2, G1Affine, G1Projective, G2Affine, G2Projective};
pub use self::fields::{Fq, Fq12, Fq2, Fq6, Fr};
