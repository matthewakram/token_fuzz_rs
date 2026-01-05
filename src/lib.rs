pub mod hashing_funcs;
pub mod internal_token_fuzzer;
pub mod token_fuzzers;

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
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

    use crate::internal_token_fuzzer::InternalTokenFuzzer;
    use crate::token_fuzzers::indexed::IndexedTokenFuzzer;
    use crate::token_fuzzers::naive::NaiveTokenFuzzer;

    /// A MinHash-based fuzzy string matcher exposed to Python.
    ///
    /// Create an instance with a corpus of strings, then call
    /// `match_closest(query)` to retrieve the corpus string that is most
    /// similar to `query` based on MinHash signature similarity.
    ///
    /// Example (Python):
    /// ```python
    ///     f = token_fuzz_rs.TokenFuzzer(["hello world", "other text"])
    ///     best = f.match_closest("hello wurld")
    /// ```
    #[pyclass]
    pub struct TokenFuzzer {
        internal_token_fuzzer: Box<dyn InternalTokenFuzzer>,
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
        ///     method (str, optional): Which internal implementation to use. Supported
        ///         values:
        ///             - "naive": the simple in-memory implementation (default).
        ///         Unknown values will cause a panic.
        ///
        /// Returns:
        ///     TokenFuzzer: An object that can be used from Python to find closest matches.
        ///
        /// Notes:
        ///     The fuzzer computes MinHash signatures of length `num_hashes` for
        ///     each input string using a deterministic set of seeds.
        #[new]
        #[pyo3(signature = (strings, num_hashes=128, method="naive".to_string()))]
        pub fn new(strings: Vec<String>, num_hashes: usize, method: String) -> Self {
            let itf: Box<dyn InternalTokenFuzzer>= match method.as_str() {
                "naive" => Box::new(NaiveTokenFuzzer::new(strings, num_hashes)),
                "indexed" => Box::new(IndexedTokenFuzzer::new(strings, num_hashes)),
                _ => panic!("unknown method: {method}"),
            };

            TokenFuzzer {
                internal_token_fuzzer: itf,
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
            let closest = self.internal_token_fuzzer.match_closest(&s);
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
        /// ```python
        ///     f = token_fuzz_rs.TokenFuzzer(["hello world", "other text"], 128)
        ///     results = f.match_closest_batch(["hello wurld", "other txt"])
        ///     # results -> ["hello world", "other text"]
        /// ```
        pub fn match_closest_batch(&self, queries: Vec<String>) -> PyResult<Vec<String>> {
            let results: PyResult<Vec<String>> = queries
                .par_iter()
                .map(|q| self.internal_token_fuzzer.match_closest(q)) // Result<String, String>
                .map(|r: Result<String, String>| r.map_err(PyValueError::new_err)) // Result<String, PyErr>
                .collect();

            return results;
        }
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

        let fuzzer = token_fuzz_rs::TokenFuzzer::new(data, 128, "naive".to_string());

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

        let fuzzer = token_fuzz_rs::TokenFuzzer::new(data, 128, "naive".to_string());

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

        let fuzzer = token_fuzz_rs::TokenFuzzer::new(data, 128, "naive".to_string());

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
