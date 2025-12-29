use pyo3::prelude::*;

/// Python module `token_fuzz_rs`.
///
/// Exposes `TokenFuzzer` — a MinHash-based fuzzy string matcher implemented
/// in Rust for performance. Construct a `TokenFuzzer` with a list of strings
/// and use `match_closest` to find the most similar string to a query.
#[pymodule]
pub mod token_fuzz_rs {

    use pyo3::prelude::*;

    use pyo3::exceptions::PyValueError;
    use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
    use rayon::slice::ParallelSliceMut;

    /// A MinHash-based fuzzy string matcher exposed to Python.
    ///
    /// Create an instance with a corpus of strings, then call
    /// `match_closest(query)` to retrieve the corpus string that is most
    /// similar to `query` based on MinHash signature similarity.
    ///
    /// Example (Python):
    ///
    ///     f = token_fuzz_rs.TokenFuzzer(["hello world", "other text"])
    ///     best = f.match_closest("hello wurld")
    #[pyclass]
    pub struct TokenFuzzer {
        strings: Vec<String>,
        tokencache: Vec<u64>,
        num_hashes: usize,
        hash_seeds: Vec<u64>,
    }

    impl TokenFuzzer {
        pub fn internal_match_closest(&self, s: &String) -> Result<String, String> {
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

    #[pymethods]
    impl TokenFuzzer {
        /// Create a new `TokenFuzzer` instance.
        ///
        /// Args:
        ///     strings (List[str]): The list of strings to index for fuzzy matching.
        ///     num_hashes (int, optional): Number of MinHash functions to use when
        ///         building signatures. Defaults to 128. Larger values increase
        ///         signature resolution at the cost of more memory and CPU.
        ///
        /// Returns:
        ///     TokenFuzzer: An object that can be used from Python to find closest matches.
        ///
        /// Notes:
        ///     The fuzzer computes MinHash signatures of length `num_hashes` for
        ///     each input string using a deterministic set of seeds.
        #[new]
        #[pyo3(signature = (strings, num_hashes=128))]
        pub fn new(strings: Vec<String>, num_hashes: usize) -> Self {
            let hash_seeds = generate_seeds(num_hashes, 0x1234_5678_9abc_def0u64);
            let tokencache = build_cache(&strings, num_hashes, &hash_seeds);

            TokenFuzzer {
                strings,
                tokencache,
                num_hashes,
                hash_seeds,
            }
        }

        /// Find the closest-matching string to `s` using MinHash similarity.
        ///
        /// Args:
        ///     s (str): The query string to match against the indexed corpus.
        ///
        /// Returns:
        ///     str: The corpus string with the highest MinHash similarity to `s`.
        ///
        /// Raises:
        ///     ValueError: If the `TokenFuzzer` was created with an empty corpus.
        ///
        /// Behaviour:
        ///     Similarity is measured as the fraction of matching MinHash
        ///     signature components (a float in 0.0..1.0 under the hood). If
        ///     multiple corpus entries tie for best score, the first matching
        ///     entry encountered is returned.
        pub fn match_closest(&self, s: String) -> PyResult<String> {
            let closest = self.internal_match_closest(&s);
            match closest {
                Ok(closest_string) => Ok(closest_string),
                Err(error_msg) => Err(PyValueError::new_err(error_msg)),
            }
        }

        /// Find the closest-matching strings for a batch of query strings in parallel.
        ///
        /// Args:
        ///     queries (List[str]): A list of query strings to match against the indexed corpus.
        ///
        /// Returns:
        ///     List[str]: For each query, the corpus string with the highest MinHash similarity.
        ///
        /// Raises:
        ///     ValueError: If the `TokenFuzzer` was created with an empty corpus.
        ///
        /// Behaviour:
        ///     Each query is processed independently and in parallel using Rayon. For every
        ///     query, the MinHash signature is computed and compared against all cached
        ///     corpus signatures. The best-matching corpus string is returned for each
        ///     query, preserving the input order.
        ///
        /// Example (Python):
        ///
        ///     f = token_fuzz_rs.TokenFuzzer(["hello world", "other text"], 128)
        ///     results = f.match_closest_batch(["hello wurld", "other txt"])
        ///     # results -> ["hello world", "other text"]
        pub fn match_closest_batch(&self, queries: Vec<String>) -> PyResult<Vec<String>> {
            if self.strings.is_empty() {
                return Err(PyValueError::new_err(
                    "TokenFuzzer contains no strings to match against",
                ));
            }

            let results: PyResult<Vec<String>> = queries
                .par_iter()
                .map(|q| self.internal_match_closest(q)) // Result<String, String>
                .map(|r: Result<String, String>| r.map_err(PyValueError::new_err)) // Result<String, PyErr>
                .collect();

            return results;
        }
    }

    // ------------ Internal implementation helpers (not exposed to Python) ------------

    /// Generate `num` deterministic 64-bit seeds using a simple SplitMix64 PRNG.
    fn generate_seeds(num: usize, base_seed: u64) -> Vec<u64> {
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
    fn compute_signature(s: &str, seeds: &[u64], sig_buffer: &mut [u64]) {
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

    /// Build the token cache (flattened signatures) for all strings.
    fn build_cache(strings: &[String], num_hashes: usize, seeds: &[u64]) -> Vec<u64> {
        let mut cache = vec![u64::MAX; strings.len() * num_hashes];

        cache
            .par_chunks_mut(num_hashes)
            .zip(strings.par_iter())
            .for_each(|(chunck, s)| compute_signature(s, seeds, chunck));

        cache
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn token_fuzzer_finds_closest_match() {
        // Three strings in the data set
        let data = vec![
            "hello world".to_string(),
            "rust programming".to_string(),
            "fuzzy token matcher".to_string(),
        ];

        let fuzzer = token_fuzz_rs::TokenFuzzer::new(data, 128);

        // One query string
        let query = "hello wurld".to_string();
        let best = fuzzer.match_closest(query).unwrap();

        assert_eq!(best, "hello world");
    }

    #[test]
    fn token_fuzzer_finds_closest_match_off() {
        // Three strings in the data set
        let data = vec![
            "hello world".to_string(),
            "rust programming".to_string(),
            "fuzzy token matcher".to_string(),
        ];

        let fuzzer = token_fuzz_rs::TokenFuzzer::new(data, 128);

        // One query string
        let query = "hello wurld I love you".to_string();
        let best = fuzzer.match_closest(query).unwrap();

        assert_eq!(best, "hello world");
    }

    #[test]
    fn match_closest_batch_returns_expected_results() {
        pyo3::Python::initialize();

        // Build a small corpus and queries that clearly map to corpus entries
        let data = vec![
            "hello world".to_string(),
            "other text".to_string(),
            "rust programming".to_string(),
        ];

        let fuzzer = token_fuzz_rs::TokenFuzzer::new(data, 128);

        let queries = vec![
            "hello wurld".to_string(),
            "other txt".to_string(),
            "rust progrmming".to_string(),
        ];

        let results = fuzzer.match_closest_batch(queries.clone()).unwrap();

        assert_eq!(
            results,
            vec![
                "hello world".to_string(),
                "other text".to_string(),
                "rust programming".to_string(),
            ]
        );
    }
}
