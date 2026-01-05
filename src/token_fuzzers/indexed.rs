use crate::hashing_funcs::{compute_signature, generate_seeds};
use crate::internal_token_fuzzer::InternalTokenFuzzer;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::usize;

const NUM_CACHE_ENTRIES: usize = 64;

#[derive(Debug, Clone, Copy)]
struct IndexEntry {
    hash_val: u64,
    string_id: usize,
}

#[derive(Debug)]
pub struct IndexedTokenFuzzer {
    strings: Vec<String>,
    num_hashes: usize,
    hash_seeds: Vec<u64>,

    // reverse_index[h] is the list of (hash_val, string_id) for hash function h,
    // sorted by (hash_val, string_id)
    reverse_index: Vec<Vec<IndexEntry>>,

    // reverse_index_cache[h] is an array of NUM_CACHE_ENTRIES equidistant
    // hash_val samples from reverse_index[h], for fast narrowing.
    reverse_index_cache: Vec<[u64; NUM_CACHE_ENTRIES]>,
}

impl IndexedTokenFuzzer {
    pub fn new(strings: Vec<String>, num_hashes: usize) -> Self {
        let hash_seeds = generate_seeds(num_hashes, 0x1234_5678_9abc_def0u64);

        // Compute all signatures: flattened [s0_h0, s0_h1, ..., s1_h0, ...]
        let mut signatures = vec![u64::MAX; strings.len() * num_hashes];

        signatures
            .par_chunks_mut(num_hashes)
            .zip(strings.par_iter())
            .for_each(|(chunk, s)| compute_signature(s, &hash_seeds, chunk));

        // Build reverse index: one vector per hash function
        let mut reverse_index: Vec<Vec<IndexEntry>> = (0..num_hashes)
            .map(|_| Vec::with_capacity(strings.len()))
            .collect();

        for (string_id, sig_chunk) in signatures.chunks(num_hashes).enumerate() {
            for (hash_idx, &hash_val) in sig_chunk.iter().enumerate() {
                reverse_index[hash_idx].push(IndexEntry { hash_val, string_id });
            }
        }

        // Sort each reverse_index[h] by (hash_val, string_id)
        reverse_index.par_iter_mut().for_each(|entries| {
            entries.sort_unstable_by(|a, b| {
                match a.hash_val.cmp(&b.hash_val) {
                    Ordering::Equal => a.string_id.cmp(&b.string_id),
                    other => other,
                }
            });
        });

        // Build the reverse_index_cache: equidistant samples from each list
        let mut reverse_index_cache = Vec::with_capacity(num_hashes);
        let len = strings.len();

        for entries in &reverse_index {
            let mut row = [0u64; NUM_CACHE_ENTRIES];

            if len > 0 {
                // Take NUM_CACHE_ENTRIES equidistant indices into entries
                for i in 0..NUM_CACHE_ENTRIES {
                    // Integer division gives us roughly equidistant samples.
                    let idx = cache_idx_to_sector_start(i, len);
                    // Clamp to last index to avoid len == NUM_CACHE_ENTRIES edge issues
                    row[i] = entries[idx].hash_val;
                }
            } else {
                // If there are no entries (e.g. no strings), leave row as zeros.
                // Caller can special-case the empty-string set.
            }

            reverse_index_cache.push(row);
        }

        IndexedTokenFuzzer {
            strings,
            num_hashes,
            hash_seeds,
            reverse_index,
            reverse_index_cache,
        }
    }
}

impl InternalTokenFuzzer for IndexedTokenFuzzer {
    fn match_closest(&self, s: &String) -> Result<String, String> {
        if self.strings.is_empty() {
            return Err("TokenFuzzer contains no strings to match against".to_string());
        }

        // 1. Compute query signature for all hash functions
        let mut query_sig = vec![u64::MAX; self.num_hashes];
        compute_signature(s, &self.hash_seeds, &mut query_sig);

        // 2. For each hash function, find first matching index in reverse_index[h]
        let mut first_match_indices = Vec::with_capacity(self.num_hashes);

        for (h, &hash_val) in query_sig.iter().enumerate() {
            let entries = &self.reverse_index[h];
            let cache_row = &self.reverse_index_cache[h];

            let len = entries.len();
            assert!(len == self.strings.len());

            // --- 2a. Find the sector in the cache row ---
            //
            // Cache row is built from sorted entries, so it is non-decreasing.
            // We find the first cache entry >= hash_val.
            let mut sector = 0usize;

            for i in 0..NUM_CACHE_ENTRIES {
                if cache_row[i] >= hash_val {
                    break;
                }
                sector = i;
            }

            // --- 2b. Map sector to a range in `entries` ---
            //
            // In `new`, cache_row[i] corresponds roughly to entries[(i * len) / NUM_CACHE_ENTRIES].
            // We use the same mapping to define sector bounds.
            let sector_start = cache_idx_to_sector_start(sector, len);

            // --- 2c. Scan the sector for the first matching hash_val ---
            let mut found_idx = usize::MAX;

            for idx in sector_start..len {
                let entry = &entries[idx];

                if entry.hash_val >= hash_val {
                    found_idx = idx;
                    break;
                }
            }

            first_match_indices.push(found_idx);
        }

        println!("{:?}", first_match_indices);
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

            for h in 0..first_match_indices.len(){
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

#[inline]
fn cache_idx_to_sector_start(i: usize, len: usize) -> usize {
    (i * len) / NUM_CACHE_ENTRIES
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

        let fuzzer = IndexedTokenFuzzer::new(strings.clone(), num_hashes);

        // Basic structural sanity checks
        assert_eq!(fuzzer.strings, strings);
        assert_eq!(fuzzer.num_hashes, num_hashes);
        assert_eq!(fuzzer.reverse_index.len(), num_hashes);
        assert_eq!(fuzzer.reverse_index_cache.len(), num_hashes);

        // For each hash function, we should have one entry per string
        for (h, entries) in fuzzer.reverse_index.iter().enumerate() {
            println!("==== reverse_index[{}] (len = {}) ====", h, entries.len());
            for (i, e) in entries.iter().enumerate() {
                println!("  entry[{}]: hash_val = {}, string_id = {}", i, e.hash_val, e.string_id);
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
        println!("==== reverse_index_cache (NUM_CACHE_ENTRIES = {}) ====", NUM_CACHE_ENTRIES);
        for (h, row) in fuzzer.reverse_index_cache.iter().enumerate() {
            println!("cache row for hash {}:", h);
            for (i, val) in row.iter().enumerate() {
                println!("  cache[{}][{}] = {}", h, i, val);
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
        let fuzzer = IndexedTokenFuzzer::new(strings, num_hashes);
        let mut zeta_sig = vec![u64::MAX; num_hashes];
        let fake_test_string = "delts".to_string();
        compute_signature(&fake_test_string, &fuzzer.hash_seeds, &mut zeta_sig);
        println!("{:?}", zeta_sig);

        // For each hash function, verify that cache entries correspond to
        // equidistant samples from the sorted reverse_index list.
        for (h, entries) in fuzzer.reverse_index.iter().enumerate() {
            let cache_row = &fuzzer.reverse_index_cache[h];
            let len = entries.len();

            println!(
                "Checking sampling for hash {}: entries.len() = {}, cache.len() = {}",
                h,
                len,
                NUM_CACHE_ENTRIES
            );

            if len == 0 {
                // When there are no entries, we only check that the row is all zeros.
                for (i, &val) in cache_row.iter().enumerate() {
                    println!("  (empty) cache[{}][{}] = {}", h, i, val);
                    assert_eq!(
                        val, 0,
                        "Expected zero cache for empty reverse_index at h={}",
                        h
                    );
                }
                continue;
            }

            // For non-empty entries, the i-th cache value should be
            // entries[idx].hash_val, with idx = (i * len) / NUM_CACHE_ENTRIES, clamped.
            for (i, &cached_hash) in cache_row.iter().enumerate() {
                let mut idx = (i * len) / NUM_CACHE_ENTRIES;
                if idx >= len {
                    idx = len - 1;
                }
                let expected_hash = entries[idx].hash_val;

                println!(
                    "  h = {}, i = {} -> idx = {}, cached = {}, expected = {}",
                    h, i, idx, cached_hash, expected_hash
                );

                assert_eq!(
                    cached_hash, expected_hash,
                    "Cache value mismatch at h={}, i={}, idx={}",
                    h, i, idx
                );
            }

        }
        let res = fuzzer.match_closest(&fake_test_string);

        println!("{}", res.unwrap());
    }
}
