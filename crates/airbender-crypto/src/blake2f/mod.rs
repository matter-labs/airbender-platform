//! BLAKE2b compression F (EIP-152 / RFC 7693 section 3.2).
//! Done entirely in software.

const IV: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];
const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

#[inline(always)]
fn g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}

/// Compress one block, updating `h` in place. State, message and counters are
/// little-endian bytes. `rounds` is a numeric count, not a serialized EVM field.
pub fn compress(rounds: u32, h: &mut [u8; 64], m: &[u8; 128], t: &[u8; 16], final_block: bool) {
    let mut state = [0u64; 8];
    let mut message = [0u64; 16];
    for (x, bytes) in state.iter_mut().zip(h.as_chunks::<8>().0.iter()) {
        *x = u64::from_le_bytes(*bytes);
    }
    for (x, bytes) in message.iter_mut().zip(m.as_chunks::<8>().0.iter()) {
        *x = u64::from_le_bytes(*bytes);
    }
    let mut v = [0u64; 16];
    v[..8].copy_from_slice(&state);
    v[8..].copy_from_slice(&IV);
    let (counter, _) = t.as_chunks::<8>();
    v[12] ^= u64::from_le_bytes(counter[0]);
    v[13] ^= u64::from_le_bytes(counter[1]);
    if final_block {
        v[14] = !v[14];
    }
    for round in 0..rounds {
        let s = &SIGMA[(round % 10) as usize];
        g(&mut v, 0, 4, 8, 12, message[s[0]], message[s[1]]);
        g(&mut v, 1, 5, 9, 13, message[s[2]], message[s[3]]);
        g(&mut v, 2, 6, 10, 14, message[s[4]], message[s[5]]);
        g(&mut v, 3, 7, 11, 15, message[s[6]], message[s[7]]);
        g(&mut v, 0, 5, 10, 15, message[s[8]], message[s[9]]);
        g(&mut v, 1, 6, 11, 12, message[s[10]], message[s[11]]);
        g(&mut v, 2, 7, 8, 13, message[s[12]], message[s[13]]);
        g(&mut v, 3, 4, 9, 14, message[s[14]], message[s[15]]);
    }
    for (i, bytes) in h.as_chunks_mut::<8>().0.iter_mut().enumerate() {
        bytes.copy_from_slice(&(state[i] ^ v[i] ^ v[i + 8]).to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::compress;

    fn decode_hex(value: &str) -> Vec<u8> {
        assert_eq!(value.len() % 2, 0);
        value
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    /// Keyed BLAKE2b-512 built on `compress` (every official vector has a key)
    fn hash_with_compress(message: &[u8], key: &[u8]) -> [u8; 64] {
        assert!(!key.is_empty() && key.len() <= 64);
        let mut state = super::IV;
        state[0] ^= 0x0101_0000 | ((key.len() as u64) << 8) | 64;
        let mut h = [0u8; 64];
        for (word, bytes) in state.iter().zip(h.as_chunks_mut::<8>().0) {
            bytes.copy_from_slice(&word.to_le_bytes());
        }
        // the key block, then the message blocks; each is compressed once the next is known
        let mut block = [0u8; 128];
        block[..key.len()].copy_from_slice(key);
        let mut counter = 128u128;
        for chunk in message.chunks(128) {
            compress(12, &mut h, &block, &counter.to_le_bytes(), false);
            block = [0; 128];
            block[..chunk.len()].copy_from_slice(chunk);
            counter += chunk.len() as u128;
        }
        compress(12, &mut h, &block, &counter.to_le_bytes(), true);
        h
    }

    /// The records of a vector file: groups of three `prefix: value` lines
    fn records<'a>(text: &'a str, prefixes: [&str; 3]) -> Vec<[&'a str; 3]> {
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        lines
            .chunks(3)
            .map(|record| {
                core::array::from_fn(|i| record[i].strip_prefix(prefixes[i]).unwrap().trim())
            })
            .collect()
    }

    /// Messages of every length from 0 to 255, with a 64-byte key and 12 rounds
    #[test]
    fn blake2f_matches_official_blake2b_vectors() {
        let text = include_str!("testdata/blake2b-kat.txt");
        let records = records(text, ["in:", "key:", "hash:"]);
        assert_eq!(records.len(), 256);
        for (i, [input, key, hash]) in records.into_iter().enumerate() {
            let input = decode_hex(input);
            assert_eq!(input.len(), i);
            let digest = hash_with_compress(&input, &decode_hex(key));
            assert_eq!(digest[..], decode_hex(hash), "vector {i}");
        }
    }

    /// Other round counts and a nonzero high counter word; inputs in the 213-byte EIP-152
    /// encoding: big-endian rounds, `h`, `m`, `t` and the final-block flag
    #[test]
    fn blake2f_matches_eip152_vectors() {
        let text = include_str!("testdata/eip152-vectors.txt");
        let records = records(text, ["name:", "in:", "out:"]);
        assert_eq!(records.len(), 24);
        for [name, input, output] in records {
            let input = decode_hex(input);
            assert_eq!(input.len(), 213, "{name}");
            assert!(input[212] <= 1, "{name}");
            let rounds = u32::from_be_bytes(input[..4].try_into().unwrap());
            let mut h: [u8; 64] = input[4..68].try_into().unwrap();
            let m = input[68..196].try_into().unwrap();
            compress(
                rounds,
                &mut h,
                m,
                input[196..212].try_into().unwrap(),
                input[212] == 1,
            );
            assert_eq!(h[..], decode_hex(output), "{name}");
        }
    }
}
