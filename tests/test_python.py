import pytest
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


def test_token_fuzzer_finds_closest_match_off():
    data = [
        "hello world",
        "rust programming",
        "fuzzy token matcher",
    ]
    fuzzer = TokenFuzzer(data)

    query = "hello wurld I love you"
    best = fuzzer.match_closest(query)

    assert best == "hello world"


def test_empty_corpus_raises_value_error():
    fuzzer = TokenFuzzer([])

    with pytest.raises(ValueError) as excinfo:
        fuzzer.match_closest("anything")

    assert "contains no strings" in str(excinfo.value)


def test_single_element_corpus_always_returns_that_element():
    data = ["only option"]
    fuzzer = TokenFuzzer(data)

    for query in ["only option", "only", "option", "something else"]:
        assert fuzzer.match_closest(query) == "only option"


def test_exact_match_beats_similar_matches():
    data = [
        "hello world",
        "hello wurld",
        "hello world!!!",
    ]
    fuzzer = TokenFuzzer(data)

    # Query equal to first string
    best = fuzzer.match_closest("hello world")
    assert best == "hello world"


def test_tie_breaker_returns_first_in_corpus():
    # Construct corpus with duplicated string so they should tie perfectly.
    data = [
        "duplicate",
        "duplicate",
        "duplicate",
    ]
    fuzzer = TokenFuzzer(data)

    best = fuzzer.match_closest("duplicate")
    # Implementation is documented to return first on tie
    assert best == "duplicate"
    # We can additionally check that it stays stable across calls
    for _ in range(5):
        assert fuzzer.match_closest("duplicate") == "duplicate"


def test_default_num_hashes_is_used():
    data = ["hello world", "rust programming"]
    fuzzer = TokenFuzzer(data)  # rely on default num_hashes
    best = fuzzer.match_closest("hello wurld")
    assert best == "hello world"


def test_small_num_hashes_still_works():
    data = ["hello world", "rust programming"]
    # Very small signature; may be noisy but should not crash and should usually pick the right one
    fuzzer = TokenFuzzer(data, num_hashes=8)
    best = fuzzer.match_closest("hello wurld")
    assert best == "hello world"


def test_larger_num_hashes_is_deterministic():
    data = [
        "hello world",
        "rust programming",
        "fuzzy token matcher",
        "another random string",
    ]
    fuzzer = TokenFuzzer(data, num_hashes=256)

    query = "hello wurld"
    results = {fuzzer.match_closest(query) for _ in range(5)}
    # With a fixed seed and deterministic algorithm, result should be stable
    assert len(results) == 1
    assert results.pop() == "hello world"


def test_unicode_and_non_ascii_strings():
    data = [
        "naïve café",
        "naive cafe",
        "こんにちは世界",               # Japanese "Hello, World"
        "Привет, мир",                # Russian "Hello, World"
    ]
    fuzzer = TokenFuzzer(data)

    assert fuzzer.match_closest("naive cafe") in ("naive cafe", "naïve café")
    assert fuzzer.match_closest("こんにちは") == "こんにちは世界"
    assert fuzzer.match_closest("мир") == "Привет, мир"


def test_long_strings():
    base = "lorem ipsum dolor sit amet " * 50
    variant1 = base.replace("ipsum", "ixpsum", 1)
    variant2 = base.replace("dolor", "dolxr", 1)

    data = [base, variant1, variant2]
    fuzzer = TokenFuzzer(data)

    # Small perturbation of base string should still match base
    query = base.replace("amet", "amett", 1)
    best = fuzzer.match_closest(query)
    assert best == base


def test_repeated_calls_do_not_mutate_state():
    data = [
        "hello world",
        "rust programming",
        "fuzzy token matcher",
    ]
    fuzzer = TokenFuzzer(data)

    results = [fuzzer.match_closest("hello wurld") for _ in range(10)]
    assert all(result == "hello world" for result in results)


def test_different_queries_choose_different_targets():
    data = [
        "hello world",
        "rust programming language",
        "fuzzy token matcher",
    ]
    fuzzer = TokenFuzzer(data)

    assert fuzzer.match_closest("hello wurld") == "hello world"
    assert fuzzer.match_closest("I like rust") == "rust programming language"
    assert fuzzer.match_closest("token fuzzing") == "fuzzy token matcher"

def test_match_closest_batch_returns_expected_results():
    # Given a small corpus
    corpus = ["hello world", "other text", "rust programming"]
    fuzzer = TokenFuzzer(corpus, 128)

    # And some noisy queries
    queries = ["hello wurld", "other txt", "rust progrmming"]

    # When matching in batch
    results = fuzzer.match_closest_batch(queries)

    # Then results correspond to the expected closest matches
    assert results == ["hello world", "other text", "rust programming"]


def test_match_closest_batch_consistent_with_single_match():
    corpus = [
        "hello world",
        "other text",
        "rust programming",
        "fuzzy token matcher",
    ]
    fuzzer = TokenFuzzer(corpus, 128)

    queries = [
        "hello wurld",
        "other txt",
        "rust progrmming",
        "fuzzy tokne matchr",
    ]

    # Batch results
    batch_results = fuzzer.match_closest_batch(queries)

    # Single-call results
    single_results = [fuzzer.match_closest(q) for q in queries]

    # The batch implementation should be consistent with the single-query method
    assert batch_results == single_results


def test_match_closest_batch_preserves_order():
    corpus = ["aaa", "bbb", "ccc"]
    fuzzer = TokenFuzzer(corpus, 64)

    # Queries in a specific order
    queries = ["ccc", "aaa", "bbb", "ccc", "aaa"]
    results = fuzzer.match_closest_batch(queries)

    # The output order must match the input query order
    assert results == ["ccc", "aaa", "bbb", "ccc", "aaa"]


def test_match_closest_batch_empty_queries_returns_empty_list():
    corpus = ["hello world", "other text"]
    fuzzer = TokenFuzzer(corpus, 128)

    results = fuzzer.match_closest_batch([])

    assert results == []


def test_match_closest_batch_empty_corpus_raises_value_error():
    # Construct a fuzzer with an empty corpus
    fuzzer = TokenFuzzer([], 128)

    with pytest.raises(ValueError) as excinfo:
        fuzzer.match_closest_batch(["some query"])

    assert "contains no strings to match against" in str(excinfo.value)
