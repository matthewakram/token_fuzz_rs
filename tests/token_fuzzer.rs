use std::{fs, path::Path, time};
use rand::{Rng, distr::{Alphanumeric, SampleString}};
use token_fuzz_rs::token_fuzz_rs::TokenFuzzer;

#[test]
fn token_fuzzer_matches_koeln_address() {
    // Adjust this path depending on where you place the CSV file.
    let path = Path::new("test_data/addresses.csv");

    let contents = fs::read_to_string(path).expect("failed to read tests/addresses.csv");

    // Each line is one address string.
    let addresses: Vec<String> = contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|s| s.to_string())
        .collect();

    assert!(
        !addresses.is_empty(),
        "addresses.csv must contain at least one line"
    );
    println!("read addresses");

    let start = time::Instant::now();

    let fuzzer = TokenFuzzer::new(addresses, 128, "hashed".to_string(), 15, 25);

    let query_list: Vec<String> = std::iter::repeat("aachener straße 1, köln, 50674".to_string())
        .take(300)
        .collect();
    println!("initiated fuzzer {}", start.elapsed().as_secs_f64());

    // A fake address in Köln (Koeln). Adjust if you want a specific street/postcode.
    let start = time::Instant::now();

    let best_matches = fuzzer
    .match_closest_batch(query_list)
    .expect("should not fail");
    let best_match = best_matches[0].clone();

    // let best_match = fuzzer
    //     .match_closest("Aachener Straße 1, 50674 Köln".to_string())
    //     .unwrap();

    println!("queried {}", start.elapsed().as_secs_f64());

    // let best_match = fuzzer
    //     .match_closest(query)
    //     .expect("match_closest should not fail");

    // Since we don't know the exact contents of addresses.csv here,
    // we only assert that some non-empty address is returned.
    // In your real project, replace this with a concrete expected string.
    assert!(
        !best_match.is_empty(),
        "TokenFuzzer returned an empty best match"
    );

    // For debugging, you can temporarily print the match:
    eprintln!("Best match for Köln query: {best_match}");
}

#[test]
fn token_fuzzer_random_strings_batch_queries() {
    const N: usize = 10_000_000;
    const QUERY_COUNT: usize = 100;

    let mut rng = rand::rng();

    let corpus: Vec<String> = (0..N)
        .map(|_| {
            let len = rng.random_range(50..=100);
            Alphanumeric.sample_string(&mut rng, len)
        })
        .collect();

    assert!(!corpus.is_empty(), "corpus must not be empty");

    let build_start = time::Instant::now();
    let fuzzer = TokenFuzzer::new(corpus, 128, "indexed".to_string(), 15, 25);
    println!("initiated random-string fuzzer {}", build_start.elapsed().as_secs_f64());

    let query_list: Vec<String> = (0..QUERY_COUNT)
        .map(|_| {
            let len = rng.random_range(50..=100);
            Alphanumeric.sample_string(&mut rng, len)
        })
        .collect();

    let query_start = time::Instant::now();
    let best_matches = fuzzer
        .match_closest_batch(query_list)
        .expect("random string batch query should not fail");
    println!("queried random strings {}", query_start.elapsed().as_secs_f64());

    assert_eq!(
        best_matches.len(),
        QUERY_COUNT,
        "expected one match result per query"
    );
    assert!(
        best_matches.iter().all(|m| !m.is_empty()),
        "all best matches should be non-empty"
    );
}
