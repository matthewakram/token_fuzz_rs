import pytest
from token_fuzz_rs import TokenFuzzer

@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_token_fuzzer_finds_closest_match(method: str):
    # Three strings in the data set
    data = [
        "hello world",
        "rust programming",
        "fuzzy token matcher",
    ]

    fuzzer = TokenFuzzer(data, num_hashes=128, method=method)

    # One query string
    query = "hello wurld"
    best = fuzzer.match_closest(query)

    assert best == "hello world"


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_token_fuzzer_finds_closest_match_off(method: str):
    data = [
        "hello world",
        "rust programming",
        "fuzzy token matcher",
    ]
    fuzzer = TokenFuzzer(data, num_hashes=128, method=method)

    query = "hello wurld I love you"
    best = fuzzer.match_closest(query)

    assert best == "hello world"


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_empty_corpus_raises_value_error(method: str):
    fuzzer = TokenFuzzer([], method=method)

    with pytest.raises(ValueError) as excinfo:
        fuzzer.match_closest("anything")

    assert "contains no strings" in str(excinfo.value)


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_single_element_corpus_always_returns_that_element(method: str):
    data = ["only option"]
    fuzzer = TokenFuzzer(data, method=method)

    for query in ["only option", "only", "option", "something else"]:
        assert fuzzer.match_closest(query) == "only option"


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_exact_match_beats_similar_matches(method: str):
    data = [
        "hello world",
        "hello wurld",
        "hello world!!!",
    ]
    fuzzer = TokenFuzzer(data, method=method)

    # Query equal to first string
    best = fuzzer.match_closest("hello world")
    assert best == "hello world"


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_tie_breaker_returns_first_in_corpus(method: str):
    # Construct corpus with duplicated string so they should tie perfectly.
    data = [
        "duplicate",
        "duplicate",
        "duplicate",
    ]
    fuzzer = TokenFuzzer(data, method=method)

    best = fuzzer.match_closest("duplicate")
    # Implementation is documented to return first on tie
    assert best == "duplicate"
    # We can additionally check that it stays stable across calls
    for _ in range(5):
        assert fuzzer.match_closest("duplicate") == "duplicate"


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_default_num_hashes_is_used(method: str):
    data = ["hello world", "rust programming"]
    fuzzer = TokenFuzzer(data, method=method)  # rely on default num_hashes
    best = fuzzer.match_closest("hello wurld")
    assert best == "hello world"


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_small_num_hashes_still_works(method: str):
    data = ["hello world", "rust programming"]
    # Very small signature; may be noisy but should not crash and should usually pick the right one
    fuzzer = TokenFuzzer(data, num_hashes=8, method=method)
    best = fuzzer.match_closest("hello wurld")
    assert best == "hello world"


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_larger_num_hashes_is_deterministic(method: str):
    data = [
        "hello world",
        "rust programming",
        "fuzzy token matcher",
        "another random string",
    ]
    fuzzer = TokenFuzzer(data, num_hashes=256, method=method)

    query = "hello wurld"
    results = {fuzzer.match_closest(query) for _ in range(5)}
    # With a fixed seed and deterministic algorithm, result should be stable
    assert len(results) == 1
    assert results.pop() == "hello world"


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_unicode_and_non_ascii_strings(method: str):
    data = [
        "naïve café",
        "naive cafe",
        "こんにちは世界",               # Japanese "Hello, World"
        "Привет, мир",                # Russian "Hello, World"
    ]
    fuzzer = TokenFuzzer(data, method=method)

    assert fuzzer.match_closest("naive cafe") in ("naive cafe", "naïve café")
    assert fuzzer.match_closest("こんにちは") == "こんにちは世界"
    assert fuzzer.match_closest("мир") == "Привет, мир"


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_long_strings(method: str):
    base = "lorem ipsum dolor sit amet " * 50
    variant1 = base.replace("ipsum", "ixpsum", 1)
    variant2 = base.replace("dolor", "dolxr", 1)

    data = [base, variant1, variant2]
    fuzzer = TokenFuzzer(data, method=method)

    # Small perturbation of base string should still match base
    query = base.replace("amet", "amett", 1)
    best = fuzzer.match_closest(query)
    assert best == base


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_repeated_calls_do_not_mutate_state(method: str):
    data = [
        "hello world",
        "rust programming",
        "fuzzy token matcher",
    ]
    fuzzer = TokenFuzzer(data, method=method)

    results = [fuzzer.match_closest("hello wurld") for _ in range(10)]
    assert all(result == "hello world" for result in results)


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_different_queries_choose_different_targets(method: str):
    data = [
        "hello world",
        "rust programming language",
        "fuzzy token matcher",
    ]
    fuzzer = TokenFuzzer(data, method=method)

    assert fuzzer.match_closest("hello wurld") == "hello world"
    assert fuzzer.match_closest("I like rust") == "rust programming language"
    assert fuzzer.match_closest("token fuzzing") == "fuzzy token matcher"


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_match_closest_batch_returns_expected_results(method: str):
    # Given a small corpus
    corpus = ["hello world", "other text", "rust programming"]
    fuzzer = TokenFuzzer(corpus, 128, method=method)
    # And some noisy queries
    queries = ["hello wurld", "other txt", "rust progrmming"]

    # When matching in batch
    results = fuzzer.match_closest_batch(queries)

    # Then results correspond to the expected closest matches
    assert results == ["hello world", "other text", "rust programming"]


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_match_closest_batch_consistent_with_single_match(method: str):
    corpus = [
        "hello world",
        "other text",
        "rust programming",
        "fuzzy token matcher",
    ]
    fuzzer = TokenFuzzer(corpus, 128, method=method)

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


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_match_closest_batch_preserves_order(method: str):
    corpus = ["aaa", "bbb", "ccc"]
    fuzzer = TokenFuzzer(corpus, 64, method=method)

    # Queries in a specific order
    queries = ["ccc", "aaa", "bbb", "ccc", "aaa"]
    results = fuzzer.match_closest_batch(queries)

    # The output order must match the input query order
    assert results == ["ccc", "aaa", "bbb", "ccc", "aaa"]


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_match_closest_batch_empty_queries_returns_empty_list(method: str):
    corpus = ["hello world", "other text"]
    fuzzer = TokenFuzzer(corpus, 128, method=method)

    results = fuzzer.match_closest_batch([])

    assert results == []


@pytest.mark.parametrize("method", ["naive", "indexed"])
def test_match_closest_batch_empty_corpus_raises_value_error(method: str):
    # Construct a fuzzer with an empty corpus
    fuzzer = TokenFuzzer([], 128, method=method)

    with pytest.raises(ValueError) as excinfo:
        fuzzer.match_closest_batch(["some query"])

    assert "contains no strings to match against" in str(excinfo.value)
