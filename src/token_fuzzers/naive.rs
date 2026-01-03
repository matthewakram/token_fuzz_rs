use crate::hashing_funcs::compute_signature;
use crate::{hashing_funcs::generate_seeds, internal_token_fuzzer::InternalTokenFuzzer};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rayon::slice::ParallelSliceMut;

#[derive(Debug)]
pub struct NaiveTokenFuzzer {
    strings: Vec<String>,
    tokencache: Vec<u64>,
    num_hashes: usize,
    hash_seeds: Vec<u64>,
}

impl NaiveTokenFuzzer {
    pub fn new(strings: Vec<String>, num_hashes: usize) -> Self {
        let hash_seeds = generate_seeds(num_hashes, 0x1234_5678_9abc_def0u64);
        let tokencache = build_cache(&strings, num_hashes, &hash_seeds);

        NaiveTokenFuzzer {
            strings,
            tokencache,
            num_hashes,
            hash_seeds,
        }
    }
}

impl InternalTokenFuzzer for NaiveTokenFuzzer {
    fn match_closest(&self, s: &String) -> Result<String, String> {
        if self.strings.is_empty() {
            return Err("TokenFuzzer contains no strings to match against".to_string());
        }

        let mut query_sig = vec![u64::MAX; self.num_hashes];
        compute_signature(&s, &self.hash_seeds, &mut query_sig);

        let mut best_idx = 0usize;
        let mut best_score = -1.0_f64;

        for (i, _) in self.strings.iter().enumerate() {
            let offset = i * self.num_hashes;
            let mut equal = 0usize;

            for j in 0..self.num_hashes {
                if self.tokencache[offset + j] == query_sig[j] {
                    equal += 1;
                }
            }

            let score = equal as f64 / self.num_hashes as f64;
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        Ok(self.strings[best_idx].clone())
    }
}

/// Build the token cache (flattened signatures) for all strings.
fn build_cache(strings: &[String], num_hashes: usize, seeds: &[u64]) -> Vec<u64> {
    let mut cache = vec![u64::MAX; strings.len() * num_hashes];

    cache
        .par_chunks_mut(num_hashes)
        .zip(strings.par_iter())
        .for_each(|(chunck, s)| compute_signature(s, seeds, chunck));

    cache
}
