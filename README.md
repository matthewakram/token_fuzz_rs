# token-fuzz-rs

**The fastest** token-based fuzzy string matching in Python for **very large, static corpora**.

`token-fuzz-rs` is designed for the case where:

- You have a **very large list of (possibly very long) strings**.
- That list is **static** (or rarely changes).
- You need to run **many queries** against that list.
- You want **token-based** matching (robust to extra/missing words, small typos, etc.).

In this scenario, `token-fuzz-rs` can be **significantly faster** (often by multiple orders of magnitude) than general-purpose Python fuzzy matching libraries for token-based search.

For **small to medium-sized sets or one-off matching**, you should strongly consider using [RapidFuzz](https://github.com/maxbachmann/RapidFuzz) instead – it’s feature-rich, very well maintained, and easier to integrate in many typical workloads. However, for large, static corpora with many token-based queries, `token-fuzz-rs` is focused specifically on that performance niche.

The core is implemented in Rust for speed, but the library is intended to be used **purely from Python** via its PyPI package.

---

## Why Token-Based Matching?

Token-based matching treats strings as **bags of tokens** (e.g. words or byte n-grams) rather than as plain character sequences. This has several advantages:

- **Robust to word order**  
  `"New York City"` vs. `"City of New York"` can still match well.

- **Robust to extra or missing words**  
  `"hello world"` vs. `"hello wurld I love you"` still yield a high similarity because the important tokens overlap.

- **More tolerant of local edits**  
  Small insertions/deletions don’t completely destroy similarity as they might with naive edit-distance-based approaches.

- **Good for partial overlaps**  
  Useful when strings share important keywords but differ in prefixes/suffixes.

`token-fuzz-rs` implements a MinHash-style token similarity over byte-based tokens, making it efficient and scalable for very large corpora.

---

## Installation

Install from PyPI:

```bash
pip install token-fuzz-rs
```

Then import it in Python as:

```python
from token_fuzz_rs import TokenFuzzer
```

---

## Quick Start

```python
from token_fuzz_rs import TokenFuzzer

# Your corpus of strings (can be very large)
data = [
    "hello world",
    "rust programming",
    "fuzzy token matcher",
]

# Build the index (one-off cost; optimized for many subsequent queries)
fuzzer = TokenFuzzer(data)

# Fuzzy queries
print(fuzzer.match_closest("hello wurld"))            # -> "hello world"
print(fuzzer.match_closest("hello wurld I love you")) # -> "hello world"
print(fuzzer.match_closest("rust progmming"))         # -> "rust programming"
```

---

## When to Use `token-fuzz-rs` vs. RapidFuzz

**Use `token-fuzz-rs` when:**

- Your corpus is **large** (thousands to millions of strings).
- The corpus is **static** or changes rarely.
- You need to run **many queries** against that corpus.
- You care about **token-based similarity** and want very high throughput.

In this context, `token-fuzz-rs`:

- Builds a compact MinHash-based index once.
- Answers subsequent queries very quickly.
- Can outperform general-purpose libraries by **multiple orders of magnitude** on large token-based workloads.

**Use RapidFuzz when:**

- Your corpus is **small or medium-sized**.
- You don’t have an expensive one-off index build step.
- You need a rich set of similarity metrics and utilities.
- You prefer a pure-Python / standard C-extension workflow and broader feature set.

`token-fuzz-rs` is intentionally focused and minimal: one main type (`TokenFuzzer`) and one main operation (`match_closest`).

---

## API Reference

### Class: `TokenFuzzer`

#### Constructor

```python
TokenFuzzer(strings: list[str]) -> TokenFuzzer
```

Builds an index over the provided list of strings.

- `strings`: list of strings to match against (the “corpus”).
- Internally, computes MinHash-style signatures for each string (one-time cost).

#### Methods

```python
match_closest(self, s: str) -> str
```

Returns the closest-matching string from the original corpus.

- `s`: query string.
- Returns: a single string – the best match from the corpus.
- Raises: `ValueError` if the corpus was empty at construction time.

---

## How It Works (High Level)

- Strings are treated as byte sequences.
- For each position in the string, the library builds short byte tokens (up to 8 bytes).
- Each token is hashed with multiple independent hash functions based on SplitMix64.
- For each hash function, the minimum hash value seen over all tokens becomes one element of a **MinHash signature**.
- Similarity between two strings is approximated by the fraction of equal entries in their signatures.
- `match_closest`:
  1. Computes the MinHash signature for the query string.
  2. Compares it against all precomputed signatures in the corpus.
  3. Returns the string with the highest similarity score.

This design:

- Exploits **token overlap** rather than pure character-level edit distance.
- Allows fast, approximate similarity search once signatures are precomputed.
- Scales well to large, static corpora with many queries.

---

## Notes & Limitations

- Similarity is **approximate** (MinHash-based), not exact edit distance.
- Matching is 1-to-N: it returns only the **single best match**.
- The index is **immutable** after construction; to add or remove strings, build a new `TokenFuzzer`.
- The library is intended to be used from Python; the Rust code is an internal implementation detail.

---

## License

This project is licensed under the **MIT License**.
Feel free to use it, make small PRs, or open issues to make feature requests.
