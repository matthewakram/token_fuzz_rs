use crate::hashing_funcs::{compute_signature, generate_seeds};
use crate::internal_token_fuzzer::InternalTokenFuzzer;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::{u64, usize};

#[derive(Debug, Clone, Copy)]
struct IndexEntry {
    hash_val: u64,
    string_id: usize,
}

#[derive(Debug)]
pub struct HashedTokenFuzzer {
    strings: Vec<String>,
    num_hashes: usize,
    hash_seeds: Vec<u64>,

    // reverse_index[h] is the list of (hash_val, string_id) for hash function h,
    // sorted by (hash_val, string_id)
    reverse_index: Vec<Vec<IndexEntry>>,

    // reverse_index_map: Vec<HashMap<u64, usize, IdentityBuildHasher>>,
    reverse_index_map: Vec<HashMap<u64, usize>>,

    min_token_length: usize,
    max_token_length: usize,
}

impl HashedTokenFuzzer {
    pub fn new(
        strings: Vec<String>,
        num_hashes: usize,
        min_token_length: usize,
        max_token_length: usize,
    ) -> Self {
        let hash_seeds = generate_seeds(num_hashes, 0x1234_5678_9abc_def0u64);

        // Compute all signatures: flattened [s0_h0, s0_h1, ..., s1_h0, ...]
        let mut signatures = vec![u64::MAX; strings.len() * num_hashes];

        signatures
            .par_chunks_mut(num_hashes)
            .zip(strings.par_iter())
            .for_each(|(chunk, s)| {
                compute_signature(s, &hash_seeds, chunk, min_token_length, max_token_length)
            });

        // Build reverse index: one vector per hash function
        let mut reverse_index: Vec<Vec<IndexEntry>> = (0..num_hashes)
            .map(|_| Vec::with_capacity(strings.len()))
            .collect();

        for (string_id, sig_chunk) in signatures.chunks(num_hashes).enumerate() {
            for (hash_idx, &hash_val) in sig_chunk.iter().enumerate() {
                reverse_index[hash_idx].push(IndexEntry {
                    hash_val,
                    string_id,
                });
            }
        }

        // Sort each reverse_index[h] by (hash_val, string_id)
        reverse_index.par_iter_mut().for_each(|entries| {
            entries.sort_unstable_by(|a, b| match a.hash_val.cmp(&b.hash_val) {
                Ordering::Equal => a.string_id.cmp(&b.string_id),
                other => other,
            });
        });

        let mut reverse_index_map: Vec<HashMap<u64, usize>> = Vec::new();

        for entries in &reverse_index {
            let mut hash_value_map: HashMap<u64, usize> = HashMap::with_hasher(Default::default());
            for i in 0..entries.len() {
                let entry = &entries[i];
                if !hash_value_map.contains_key(&entry.hash_val) {
                    hash_value_map.insert(entry.hash_val, i);
                }
            }
            reverse_index_map.push(hash_value_map);
        }

        HashedTokenFuzzer {
            strings,
            num_hashes,
            hash_seeds,
            reverse_index,
            reverse_index_map,
            min_token_length,
            max_token_length,
        }
    }
}

impl InternalTokenFuzzer for HashedTokenFuzzer {
    fn match_closest(&self, s: &String) -> Result<String, String> {
        if self.strings.is_empty() {
            return Err("TokenFuzzer contains no strings to match against".to_string());
        }

        // 1. Compute query signature for all hash functions
        let mut query_sig = vec![u64::MAX; self.num_hashes];
        compute_signature(
            s,
            &self.hash_seeds,
            &mut query_sig,
            self.min_token_length,
            self.max_token_length,
        );

        // 2. For each hash function, find first matching index in reverse_index[h]
        let mut first_match_indices = Vec::with_capacity(self.num_hashes);

        for (h, &hash_val) in query_sig.iter().enumerate() {
            let entries = &self.reverse_index[h];
            let cache_row = &self.reverse_index_map[h];

            let len = entries.len();
            assert!(len == self.strings.len());

            let found_idx = *cache_row.get(&hash_val).unwrap_or(&len);

            first_match_indices.push(found_idx);
        }
        let num_strings = self.strings.len();

        let mut current_score = 1;
        let mut best_score = 0;
        let mut best_index = 0;
        while current_score != 0 {
            current_score = 0;
            let mut min_string_idx = usize::MAX;
            for (h, &pointer) in first_match_indices.iter().enumerate() {
                if pointer >= num_strings {
                    continue;
                }

                let entry = self.reverse_index[h][pointer];
                assert!(entry.hash_val >= query_sig[h]);

                if entry.hash_val != query_sig[h] {
                    continue;
                }

                if min_string_idx < entry.string_id {
                    continue;
                }

                if min_string_idx > entry.string_id {
                    min_string_idx = entry.string_id;
                    current_score = 0;
                }

                current_score += 1;
            }

            if current_score > best_score {
                best_score = current_score;
                best_index = min_string_idx;
            }

            for h in 0..first_match_indices.len() {
                let pointer = first_match_indices[h];
                if pointer >= num_strings {
                    continue;
                }

                let entry = self.reverse_index[h][pointer];
                assert!(entry.hash_val >= query_sig[h]);

                if entry.string_id == min_string_idx {
                    first_match_indices[h] += 1;
                }
            }
        }

        return Ok(self.strings[best_index].clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_reverse_index_and_cache_basic() {
        let strings = vec![
            "apple".to_string(),
            "banana".to_string(),
            "cherry".to_string(),
            "date".to_string(),
            "elderberry".to_string(),
        ];
        let num_hashes = 3;

        let fuzzer = HashedTokenFuzzer::new(strings.clone(), num_hashes, 1, 10);

        // Basic structural sanity checks
        assert_eq!(fuzzer.strings, strings);
        assert_eq!(fuzzer.num_hashes, num_hashes);
        assert_eq!(fuzzer.reverse_index.len(), num_hashes);
        assert_eq!(fuzzer.reverse_index_map.len(), num_hashes);

        // For each hash function, we should have one entry per string
        for (h, entries) in fuzzer.reverse_index.iter().enumerate() {
            println!("==== reverse_index[{}] (len = {}) ====", h, entries.len());
            for (i, e) in entries.iter().enumerate() {
                println!(
                    "  entry[{}]: hash_val = {}, string_id = {}",
                    i, e.hash_val, e.string_id
                );
            }

            assert_eq!(
                entries.len(),
                strings.len(),
                "reverse_index[{}] should have one entry per string",
                h
            );

            // Ensure sorted by (hash_val, string_id)
            for pair in entries.windows(2) {
                let a = pair[0];
                let b = pair[1];
                assert!(
                    a.hash_val < b.hash_val
                        || (a.hash_val == b.hash_val && a.string_id <= b.string_id),
                    "reverse_index[{}] is not sorted properly at pair: {:?} -> {:?}",
                    h,
                    a,
                    b
                );
            }
        }

        // Print cache contents
        for (h, row) in fuzzer.reverse_index_map.iter().enumerate() {
            println!("cache row for hash {}:", h);
            for val in row {
                println!("  cache[{}][({})] = {}", h, val.0, val.1);
            }
        }
    }

    #[test]
    fn reverse_index_cache_sampling_matches_reverse_index() {
        let strings = vec![
            "alpha".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
            "delta".to_string(),
            "epsilon".to_string(),
            "zeta".to_string(),
        ];
        let num_hashes = 4;
        let fuzzer = HashedTokenFuzzer::new(strings, num_hashes, 1, 10);
        let mut zeta_sig = vec![u64::MAX; num_hashes];
        let fake_test_string = "gamma".to_string();
        compute_signature(&fake_test_string, &fuzzer.hash_seeds, &mut zeta_sig, 1, 10);
        println!("{:?}", zeta_sig);

        let res = fuzzer.match_closest(&fake_test_string);

        println!("{}", res.unwrap());
    }
}
