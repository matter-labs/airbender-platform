use ark_ec::{short_weierstrass::Affine, AffineRepr};
use ark_ff::PrimeField;
use ark_serialize::SerializationError;

#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
))]
use crate::ark_ff_delegation::BigInt;
#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
)))]
use ark_ff::BigInt;

use crate::bls12_381::{
    g1::Config as G1Config, g2::Config as G2Config, Fq, Fq2, G1Affine, G2Affine,
};

pub const G1_SERIALIZED_SIZE: usize = 48;
pub const G2_SERIALIZED_SIZE: usize = 96;

pub struct EncodingFlags {
    pub is_compressed: bool,
    pub is_infinity: bool,
    pub is_lexographically_largest: bool,
}

impl EncodingFlags {
    /// Fetches the flags from the byte-string
    pub fn get_flags(bytes: &[u8]) -> Result<Self, SerializationError> {
        let compression_flag_set = (bytes[0] >> 7) & 1;
        let infinity_flag_set = (bytes[0] >> 6) & 1;
        let sort_flag_set = (bytes[0] >> 5) & 1;

        let is_compressed = compression_flag_set == 1;
        let is_infinity = infinity_flag_set == 1;
        let is_lexographically_largest = sort_flag_set == 1;

        if is_lexographically_largest && (!is_compressed || is_infinity) {
            return Err(SerializationError::InvalidData);
        }

        Ok(Self {
            is_compressed,
            is_infinity,
            is_lexographically_largest,
        })
    }

    /// Encodes the flags into the byte-string
    pub fn encode_flags(&self, bytes: &mut [u8]) {
        if self.is_compressed {
            bytes[0] |= 1 << 7;
        }

        if self.is_infinity {
            bytes[0] |= 1 << 6;
        }

        if self.is_compressed && !self.is_infinity && self.is_lexographically_largest {
            bytes[0] |= 1 << 5;
        }
    }

    /// Removes the flags from the byte-string.
    ///
    /// This reverses the effects of `encode_flags`.
    pub fn remove_flags(bytes: &mut [u8]) {
        bytes[0] &= 0b0001_1111;
    }
}

#[cfg(not(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
)))]
pub(crate) fn deserialize_fq(bytes: [u8; 48]) -> Option<Fq> {
    let mut tmp = BigInt::new([0, 0, 0, 0, 0, 0]);

    // Note: The following unwraps are if the compiler cannot convert
    // the byte slice into [u8;8], we know this is infallible since we
    // are providing the indices at compile time and bytes has a fixed size
    tmp.0[5] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[0..8]).unwrap());
    tmp.0[4] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[8..16]).unwrap());
    tmp.0[3] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[16..24]).unwrap());
    tmp.0[2] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[24..32]).unwrap());
    tmp.0[1] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[32..40]).unwrap());
    tmp.0[0] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[40..48]).unwrap());

    Fq::from_bigint(tmp)
}

#[cfg(any(
    all(target_arch = "riscv32", feature = "bigint_ops"),
    test,
    feature = "proving"
))]
pub(crate) fn deserialize_fq(bytes: [u8; 48]) -> Option<Fq> {
    let mut tmp = BigInt::new([0, 0, 0, 0, 0, 0, 0, 0]);

    // Note: The following unwraps are if the compiler cannot convert
    // the byte slice into [u8;8], we know this is infallible since we
    // are providing the indices at compile time and bytes has a fixed size
    tmp.0[5] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[0..8]).unwrap());
    tmp.0[4] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[8..16]).unwrap());
    tmp.0[3] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[16..24]).unwrap());
    tmp.0[2] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[24..32]).unwrap());
    tmp.0[1] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[32..40]).unwrap());
    tmp.0[0] = u64::from_be_bytes(<[u8; 8]>::try_from(&bytes[40..48]).unwrap());

    Fq::from_bigint(tmp)
}

pub(crate) fn serialize_fq(field: Fq) -> [u8; 48] {
    let mut result = [0u8; 48];

    let rep = field.into_bigint();

    result[0..8].copy_from_slice(&rep.0[5].to_be_bytes());
    result[8..16].copy_from_slice(&rep.0[4].to_be_bytes());
    result[16..24].copy_from_slice(&rep.0[3].to_be_bytes());
    result[24..32].copy_from_slice(&rep.0[2].to_be_bytes());
    result[32..40].copy_from_slice(&rep.0[1].to_be_bytes());
    result[40..48].copy_from_slice(&rep.0[0].to_be_bytes());

    result
}

fn read_bytes_with_offset(bytes: &[u8], offset: usize, mask: bool) -> [u8; G1_SERIALIZED_SIZE] {
    let mut tmp = [0; G1_SERIALIZED_SIZE];
    // read `G1_SERIALIZED_SIZE` bytes
    tmp.copy_from_slice(&bytes[offset * G1_SERIALIZED_SIZE..G1_SERIALIZED_SIZE * (offset + 1)]);

    if mask {
        EncodingFlags::remove_flags(&mut tmp);
    }
    tmp
}

pub(crate) fn read_g1_compressed<R: ark_serialize::Read>(
    mut reader: R,
) -> Result<Affine<G1Config>, ark_serialize::SerializationError> {
    let mut bytes = [0u8; G1_SERIALIZED_SIZE];
    reader
        .read_exact(&mut bytes)
        .ok()
        .ok_or(SerializationError::InvalidData)?;

    // Obtain the three flags from the start of the byte sequence
    let flags = EncodingFlags::get_flags(&bytes[..])?;

    // We expect to be deserializing a compressed point
    if !flags.is_compressed {
        return Err(SerializationError::UnexpectedFlags);
    }

    // Attempt to obtain the x-coordinate
    let x_bytes = read_bytes_with_offset(&bytes, 0, true);

    if flags.is_infinity {
        // Check that the `x` co-ordinate was `0`
        if x_bytes != [0u8; 48] {
            return Err(SerializationError::InvalidData);
        }

        return Ok(G1Affine::zero());
    }

    let x = deserialize_fq(x_bytes).ok_or(SerializationError::InvalidData)?;
    let p = G1Affine::get_point_from_x_unchecked(x, flags.is_lexographically_largest)
        .ok_or(SerializationError::InvalidData)?;

    Ok(p)
}

pub(crate) fn read_g1_uncompressed<R: ark_serialize::Read>(
    mut reader: R,
) -> Result<Affine<G1Config>, ark_serialize::SerializationError> {
    let mut bytes = [0u8; 2 * G1_SERIALIZED_SIZE];
    reader
        .read_exact(&mut bytes)
        .map_err(|_| SerializationError::InvalidData)?;

    // Obtain the three flags from the start of the byte sequence
    let flags = EncodingFlags::get_flags(&bytes[..])?;

    // we expect to be deserializing an uncompressed point
    if flags.is_compressed {
        return Err(SerializationError::UnexpectedFlags);
    }

    let x_bytes = read_bytes_with_offset(&bytes, 0, true);
    let y_bytes = read_bytes_with_offset(&bytes, 1, false);

    if flags.is_infinity {
        if x_bytes != [0u8; 48] || y_bytes != [0u8; 48] {
            return Err(SerializationError::InvalidData);
        }
        return Ok(G1Affine::zero());
    }

    // Attempt to obtain the x-coordinate
    let x = deserialize_fq(x_bytes).ok_or(SerializationError::InvalidData)?;
    // Attempt to obtain the y-coordinate
    let y = deserialize_fq(y_bytes).ok_or(SerializationError::InvalidData)?;
    let p = G1Affine::new_unchecked(x, y);

    Ok(p)
}

pub(crate) fn read_g2_compressed<R: ark_serialize::Read>(
    mut reader: R,
) -> Result<Affine<G2Config>, ark_serialize::SerializationError> {
    let mut bytes = [0u8; G2_SERIALIZED_SIZE];
    reader
        .read_exact(&mut bytes)
        .map_err(|_| SerializationError::InvalidData)?;

    // Obtain the three flags from the start of the byte sequence
    let flags = EncodingFlags::get_flags(&bytes)?;

    // we expect to be deserializing a compressed point
    if !flags.is_compressed {
        return Err(SerializationError::UnexpectedFlags);
    }

    let xc1_bytes = read_bytes_with_offset(&bytes, 0, true);
    let xc0_bytes = read_bytes_with_offset(&bytes, 1, false);

    if flags.is_infinity {
        if xc1_bytes != [0u8; 48] || xc0_bytes != [0u8; 48] {
            return Err(SerializationError::InvalidData);
        }
        return Ok(G2Affine::zero());
    }

    // Attempt to obtain the x-coordinate
    let xc1 = deserialize_fq(xc1_bytes).ok_or(SerializationError::InvalidData)?;
    let xc0 = deserialize_fq(xc0_bytes).ok_or(SerializationError::InvalidData)?;
    let x = Fq2::new(xc0, xc1);

    let p = G2Affine::get_point_from_x_unchecked(x, flags.is_lexographically_largest)
        .ok_or(SerializationError::InvalidData)?;

    Ok(p)
}

pub(crate) fn read_g2_uncompressed<R: ark_serialize::Read>(
    mut reader: R,
) -> Result<Affine<G2Config>, ark_serialize::SerializationError> {
    let mut bytes = [0u8; 2 * G2_SERIALIZED_SIZE];
    reader
        .read_exact(&mut bytes)
        .map_err(|_| SerializationError::InvalidData)?;

    // Obtain the three flags from the start of the byte sequence
    let flags = EncodingFlags::get_flags(&bytes)?;

    // we expect to be deserializing an uncompressed point
    if flags.is_compressed {
        return Err(SerializationError::UnexpectedFlags);
    }

    let xc1_bytes = read_bytes_with_offset(&bytes, 0, true);
    let xc0_bytes = read_bytes_with_offset(&bytes, 1, false);

    let yc1_bytes = read_bytes_with_offset(&bytes, 2, false);
    let yc0_bytes = read_bytes_with_offset(&bytes, 3, false);

    if flags.is_infinity {
        if xc1_bytes != [0u8; 48]
            || xc0_bytes != [0u8; 48]
            || yc1_bytes != [0u8; 48]
            || yc0_bytes != [0u8; 48]
        {
            return Err(SerializationError::InvalidData);
        }
        return Ok(G2Affine::zero());
    }

    let xc1 = deserialize_fq(xc1_bytes).ok_or(SerializationError::InvalidData)?;
    let xc0 = deserialize_fq(xc0_bytes).ok_or(SerializationError::InvalidData)?;
    let yc1 = deserialize_fq(yc1_bytes).ok_or(SerializationError::InvalidData)?;
    let yc0 = deserialize_fq(yc0_bytes).ok_or(SerializationError::InvalidData)?;

    // Attempt to obtain the x-coordinate
    let x = Fq2::new(xc0, xc1);

    // Attempt to obtain the y-coordinate
    let y = Fq2::new(yc0, yc1);

    let p = G2Affine::new_unchecked(x, y);

    Ok(p)
}

/// Decodes a compressed `G1` point (the ZCash / arkworks encoding, as `deserialize_compressed`
/// with validation) with the square root of `y² = x³ + b` supplied by `sqrt`: it returns `None`
/// when its argument is not a square and either root otherwise (the sign flag of the encoding
/// then picks between the root and its negation). A prover can take the root from a hint that
/// it checks with one squaring. The point is checked to be in the prime-order subgroup.
pub fn g1_from_compressed_with_sqrt(
    bytes: &[u8],
    sqrt: impl FnOnce(&Fq) -> Option<Fq>,
) -> Result<G1Affine, SerializationError> {
    use ark_ec::short_weierstrass::SWCurveConfig;
    use ark_ff::Field;

    if bytes.len() != G1_SERIALIZED_SIZE {
        return Err(SerializationError::InvalidData);
    }
    let flags = EncodingFlags::get_flags(bytes)?;
    if !flags.is_compressed {
        return Err(SerializationError::UnexpectedFlags);
    }
    let x_bytes = read_bytes_with_offset(bytes, 0, true);
    if flags.is_infinity {
        if x_bytes != [0u8; G1_SERIALIZED_SIZE] {
            return Err(SerializationError::InvalidData);
        }
        return Ok(G1Affine::zero());
    }
    let x = deserialize_fq(x_bytes).ok_or(SerializationError::InvalidData)?;
    // y² = x³ + b (a = 0)
    let mut y_squared = x;
    y_squared.square_in_place();
    y_squared *= &x;
    y_squared += &G1Config::COEFF_B;
    let y = sqrt(&y_squared).ok_or(SerializationError::InvalidData)?;
    let neg_y = -y;
    // the same ordering as `Affine::get_ys_from_x_unchecked`: as integers
    let (smaller, larger) = if y < neg_y { (y, neg_y) } else { (neg_y, y) };
    let y = if flags.is_lexographically_largest {
        larger
    } else {
        smaller
    };
    let p = G1Affine::new_unchecked(x, y);
    if !p.is_in_correct_subgroup_assuming_on_curve() {
        return Err(SerializationError::InvalidData);
    }
    Ok(p)
}

#[cfg(test)]
mod hinted_decoding_tests {
    use super::*;
    use ark_ff::Field;
    use ark_serialize::CanonicalDeserialize;
    use hex_literal::hex;

    fn reference(bytes: &[u8]) -> Result<G1Affine, SerializationError> {
        G1Affine::deserialize_compressed(bytes)
    }

    #[test]
    fn matches_deserialize_compressed() {
        // generator, two KZG test vectors, and their sign-flipped and infinity encodings
        let mut inputs: Vec<Vec<u8>> = vec![
            hex!("97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb").to_vec(),
            hex!("8f59a8d2a1a625a17f3fea0fe5eb8c896db3764f3185481bc22f91b4aaffcca25f26936857bc3a7c2539ea8ec3a952b7").to_vec(),
            hex!("a62ad71d14c5719385c0686f1871430475bf3a00f0aa3f7b8dd99a9abc2160744faf0070725e00b60ad9a026a15b1a8c").to_vec(),
        ];
        for i in 0..3 {
            let mut flipped = inputs[i].clone();
            flipped[0] ^= 1 << 5;
            inputs.push(flipped);
        }
        let mut infinity = vec![0u8; 48];
        infinity[0] = 0xc0;
        inputs.push(infinity);
        // invalid: not on the curve, wrong flags, x >= p, infinity with a payload, short
        inputs.push({
            let mut v = vec![0u8; 48];
            v[0] = 0x80;
            v[47] = 2;
            v
        });
        inputs.push({
            let mut v = vec![0u8; 48];
            v[0] = 0x40;
            v
        });
        inputs.push(vec![0xffu8; 48]);
        inputs.push({
            let mut v = vec![0u8; 48];
            v[0] = 0xc0;
            v[47] = 1;
            v
        });
        inputs.push(vec![0u8; 47]);
        for input in inputs {
            let expected = reference(&input);
            let hinted = g1_from_compressed_with_sqrt(&input, |y2| y2.sqrt());
            assert_eq!(expected.is_ok(), hinted.is_ok(), "{input:02x?}");
            if let (Ok(a), Ok(b)) = (expected, hinted) {
                assert_eq!(a, b, "{input:02x?}");
            }
            // the other root gives the same point
            let hinted = g1_from_compressed_with_sqrt(&input, |y2| y2.sqrt().map(|y| -y));
            if let Ok(a) = reference(&input) {
                assert_eq!(a, hinted.unwrap());
            }
        }
    }
}
