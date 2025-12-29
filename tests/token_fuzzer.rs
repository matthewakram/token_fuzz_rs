use std::{fs, path::Path, time};
use token_fuzz_rs::token_fuzz_rs::TokenFuzzer;

#[test]
fn token_fuzzer_matches_koeln_address() {
    // Adjust this path depending on where you place the CSV file.
    // Common pattern: put it in `tests/addresses.csv`.
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

    let fuzzer = TokenFuzzer::new(addresses, 128);

    println!("initiated fuzzer {}", start.elapsed().as_secs_f64());

    // A fake address in Köln (Koeln). Adjust if you want a specific street/postcode.
    let start = time::Instant::now();
    let query = "Aachener Straße 1, 50674 Köln".to_string();

    println!("queried {}", start.elapsed().as_secs_f64());

    let best_match = fuzzer
        .match_closest(query)
        .expect("match_closest should not fail");

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
