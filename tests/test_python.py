from token_fuzz_rs import TokenFuzzer


def test_token_fuzzer_finds_closest_match():
    # Three strings in the data set
    data = [
        "hello world",
        "rust programming",
        "fuzzy token matcher",
    ]

    fuzzer = TokenFuzzer(data)

    # One query string
    query = "hello wurld"
    best = fuzzer.match_closest(query)

    assert best == "hello world"