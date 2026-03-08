use crate::hashing_funcs::compute_signature;
use crate::{hashing_funcs::generate_seeds, internal_token_fuzzer::InternalTokenFuzzer};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rayon::slice::ParallelSliceMut;
use std::cmp::Ordering;
use std::collections::HashMap;

const GROUP_SIZE: usize = 4;

#[derive(Debug)]
pub struct GroupedTokenFuzzer {
    strings: Vec<String>,
    tokencache: Vec<u64>,
    num_hashes: usize, // rounded up to multiple of GROUP_SIZE
    hash_seeds: Vec<u64>,
    min_token_length: usize,
    max_token_length: usize,
    group_indices: Vec<Vec<usize>>,
    group_maps: Vec<HashMap<[u64; GROUP_SIZE], usize>>,
}

impl GroupedTokenFuzzer {
    pub fn new(
        strings: Vec<String>,
        num_hashes: usize,
        min_token_length: usize,
        max_token_length: usize,
    ) -> Self {
        let num_hashes = round_up_to_multiple_of_group(num_hashes);
        let hash_seeds = generate_seeds(num_hashes, 0x1234_5678_9abc_def0u64);

        let tokencache = build_cache(
            &strings,
            num_hashes,
            &hash_seeds,
            min_token_length,
            max_token_length,
        );

        let (group_indices, group_maps) =
            build_groups(&tokencache, num_hashes, strings.len());

        GroupedTokenFuzzer {
            strings,
            tokencache,
            num_hashes,
            hash_seeds,
            min_token_length,
            max_token_length,
            group_indices,
            group_maps,
        }
    }
}

impl InternalTokenFuzzer for GroupedTokenFuzzer {
    fn match_closest(&self, s: &String) -> Result<String, String> {
        if self.strings.is_empty() {
            return Err("TokenFuzzer contains no strings to match against".to_string());
        }

        // Allocation allowed for query_sig
        let mut query_sig = vec![u64::MAX; self.num_hashes];
        compute_signature(
            s,
            &self.hash_seeds,
            &mut query_sig,
            self.min_token_length,
            self.max_token_length,
        );

        let mut best_idx = 0usize;
        let mut best_score = 0usize;

        let num_groups = self.num_hashes / GROUP_SIZE;
        for g in 0..num_groups {
            let key = group_key_from_query(&query_sig, g);

            if let Some(&start_pos) = self.group_maps[g].get(&key) {
                let indices = &self.group_indices[g];

                let mut pos = start_pos;
                while pos < indices.len() {
                    let idx = indices[pos];

                    if !group_key_matches_cache(&self.tokencache, self.num_hashes, g, idx, &key) {
                        break;
                    }

                    let offset = idx * self.num_hashes;
                    let mut equal = 0usize;
                    for j in 0..self.num_hashes {
                        if self.tokencache[offset + j] == query_sig[j] {
                            equal += 1;
                        }
                    }

                    if equal > best_score {
                        best_score = equal;
                        best_idx = idx;
                    }



                    pos += 1;
                }
            }
        }

        Ok(self.strings[best_idx].clone())
    }
}

/// Build the token cache (flattened signatures) for all strings.
fn build_cache(
    strings: &[String],
    num_hashes: usize,
    seeds: &[u64],
    min_token_length: usize,
    max_token_length: usize,
) -> Vec<u64> {
    let mut cache = vec![u64::MAX; strings.len() * num_hashes];

    cache
        .par_chunks_mut(num_hashes)
        .zip(strings.par_iter())
        .for_each(|(chunk, s)| {
            compute_signature(s, seeds, chunk, min_token_length, max_token_length)
        });

    cache
}

/// Round up to nearest multiple of GROUP_SIZE (minimum GROUP_SIZE if given 0).
fn round_up_to_multiple_of_group(n: usize) -> usize {
    if n == 0 {
        GROUP_SIZE
    } else {
        ((n + GROUP_SIZE - 1) / GROUP_SIZE) * GROUP_SIZE
    }
}

fn build_groups(
    tokencache: &[u64],
    num_hashes: usize,
    num_strings: usize,
) -> (Vec<Vec<usize>>, Vec<HashMap<[u64; GROUP_SIZE], usize>>) {
    let num_groups = num_hashes / GROUP_SIZE;

    let results: Vec<(Vec<usize>, HashMap<[u64; GROUP_SIZE], usize>)> =
        (0..num_groups)
            .into_par_iter()
            .map(|g| {
                let mut indices: Vec<usize> = (0..num_strings).collect();

                indices.sort_by(|&a, &b| compare_group(tokencache, num_hashes, g, a, b));

                let mut map: HashMap<[u64; GROUP_SIZE], usize> =
                    HashMap::with_capacity(num_strings);
                for (pos, &idx) in indices.iter().enumerate() {
                    let key = group_key_from_cache(tokencache, num_hashes, g, idx);
                    map.entry(key).or_insert(pos);
                }

                (indices, map)
            })
            .collect();

    let (group_indices, group_maps): (Vec<_>, Vec<_>) = results.into_iter().unzip();
    (group_indices, group_maps)
}


fn compare_group(
    tokencache: &[u64],
    num_hashes: usize,
    group: usize,
    a: usize,
    b: usize,
) -> Ordering {
    let base_a = a * num_hashes + group * GROUP_SIZE;
    let base_b = b * num_hashes + group * GROUP_SIZE;

    for k in 0..GROUP_SIZE {
        let va = tokencache[base_a + k];
        let vb = tokencache[base_b + k];
        if va < vb {
            return Ordering::Less;
        } else if va > vb {
            return Ordering::Greater;
        }
    }
    Ordering::Equal
}

fn group_key_from_cache(
    tokencache: &[u64],
    num_hashes: usize,
    group: usize,
    idx: usize,
) -> [u64; GROUP_SIZE] {
    let base = idx * num_hashes + group * GROUP_SIZE;
    let mut key = [0u64; GROUP_SIZE];
    for i in 0..GROUP_SIZE {
        key[i] = tokencache[base + i];
    }
    key
}

fn group_key_from_query(query_sig: &[u64], group: usize) -> [u64; GROUP_SIZE] {
    let base = group * GROUP_SIZE;
    let mut key = [0u64; GROUP_SIZE];
    for i in 0..GROUP_SIZE {
        key[i] = query_sig[base + i];
    }
    key
}

fn group_key_matches_cache(
    tokencache: &[u64],
    num_hashes: usize,
    group: usize,
    idx: usize,
    key: &[u64; GROUP_SIZE],
) -> bool {
    let base = idx * num_hashes + group * GROUP_SIZE;
    for i in 0..GROUP_SIZE {
        if tokencache[base + i] != key[i] {
            return false;
        }
    }
    true
}
