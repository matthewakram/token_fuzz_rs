/// Generate `num` deterministic 64-bit seeds using a simple SplitMix64 PRNG.
pub fn generate_seeds(num: usize, base_seed: u64) -> Vec<u64> {
    let mut seeds = Vec::with_capacity(num);
    let mut x = base_seed;
    for _ in 0..num {
        x = splitmix64(x);
        seeds.push(x);
    }
    seeds
}

/// SplitMix64 hash / PRNG step.
#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hash a token with a given seed using SplitMix64.
#[inline]
fn hash_token(token: u64, seed: u64) -> u64 {
    splitmix64(token ^ seed)
}

/// Compute the MinHash signature for a single string.
pub fn compute_signature(s: &str, seeds: &[u64], sig_buffer: &mut [u64]) {
    debug_assert_eq!(seeds.len(), sig_buffer.len());
    let bytes = s.as_bytes();

    for i in 0..bytes.len() {
        let max_len = 8.min(bytes.len() - i);
        let mut token: u64 = 0;

        // Build tokens incrementally for lengths 1..=max_len
        for l in 0..max_len {
            let b = unsafe { *bytes.get_unchecked(i + l) };
            // Pack bytes into a u64, little-endian in the low bytes
            token |= (b as u64) << (8 * l);

            for (h_idx, seed) in seeds.iter().enumerate() {
                let h = hash_token(token, *seed);
                if h < unsafe { *sig_buffer.get_unchecked(h_idx) } {
                    unsafe { *sig_buffer.get_unchecked_mut(h_idx) = h };
                }
            }
        }
    }
}
