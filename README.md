# token-fuzz-rs

[![PyPI version](https://img.shields.io/pypi/v/token-fuzz-rs.svg)](https://pypi.org/project/token-fuzz-rs/)
[![PyPI - Python Version](https://img.shields.io/pypi/pyversions/token-fuzz-rs.svg)](https://pypi.org/project/token-fuzz-rs/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/matthewakram/token_fuzz_rs/blob/main/LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/matthewakram/token_fuzz_rs?style=social)](https://github.com/matthewakram/token_fuzz_rs)

**The fastest** token-based fuzzy string matching in Python for **very large, static corpora**.

`token-fuzz-rs` is designed for the case where:

- You have a **very large list of (possibly very long) strings**.
- That list is **static** (or rarely changes).
- You need to run **many queries** against that list.
- You want **token-based** matching (robust to extra/missing words, small typos, etc.).

In this scenario, `token-fuzz-rs` can be **significantly faster** (often by multiple orders of magnitude) than general-purpose Python fuzzy matching libraries for token-based search.

The core is implemented in Rust for speed, but the library is intended to be used **purely from Python**, distributed on PyPI:

- PyPI: https://pypi.org/project/token-fuzz-rs/  
- Source: https://github.com/matthewakram/token_fuzz_rs

For **small to medium-sized sets or one-off matching**, you should strongly consider using [RapidFuzz](https://github.com/maxbachmann/RapidFuzz) instead – it’s feature-rich, very well maintained, and easier to integrate in many typical workloads. However, for large, static corpora with many token-based queries, `token-fuzz-rs` is focused specifically on that performance niche.

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
# Uses the default configuration:
# - num_hashes = 128
# - method = "naive"
# - min_token_length = 0
# - max_token_length = 8
fuzzer = TokenFuzzer(data)

# Single fuzzy query
print(fuzzer.match_closest("hello wurld"))            # -> "hello world"
print(fuzzer.match_closest("hello wurld I love you")) # -> "hello world"

# Batch queries (processed in parallel under the hood)
results = fuzzer.match_closest_batch([
    "hello wurld",
    "rust progrmming",
])
print(results)  # -> ["hello world", "rust programming"]
```

### Configuring the Index

You can customize the index to trade off **accuracy, speed, and memory**:

```python
from token_fuzz_rs import TokenFuzzer

fuzzer = TokenFuzzer(
    strings=[
        "hello world",
        "rust programming",
        "fuzzy token matcher",
    ],
    num_hashes=256,          # more hashes -> more accurate, more CPU/memory
    method="hashed",         # "naive" (default), "indexed", or "hashed"
    min_token_length=15,     # recommend at least 10 for indexed/hashed in many workloads, preferably more
    max_token_length=30,     # upper bound on token size (byte n-grams)
)
```

---

## When to Use `token-fuzz-rs` vs. RapidFuzz

**Use `token-fuzz-rs` when:**

- Your corpus is **large** (thousands to millions of strings).
- The corpus is **static** or changes rarely.
- You need to run **many queries** against that corpus.
- You care about **token-based similarity** and want very high throughput.

In this context, `token-fuzz-rs`:

- Builds a MinHash-based index once.
- Answers subsequent queries very quickly.
- Offers multiple internal algorithms tuned for different speed/memory profiles.
- Can outperform general-purpose libraries by **multiple orders of magnitude** on large token-based workloads.

**Use RapidFuzz when:**

- Your corpus is **small or medium-sized**.
- You don’t have an expensive one-off index build step.
- You need a rich set of similarity metrics and utilities.
- You prefer a pure-Python / standard C-extension workflow and broader feature set.

`token-fuzz-rs` is intentionally focused and minimal: one main type (`TokenFuzzer`) and two primary operations (`match_closest` and `match_closest_batch`).

---

## Internal Matching Algorithms

You can choose the algorithm via the `method` parameter to `TokenFuzzer`. All share the same external API, but differ in how they **prune candidates** and in their speed/memory trade-offs.

A key point:  
Both `"indexed"` and `"hashed"` rely on the idea that only a **small subset** of the corpus is “close enough” to be worth fully comparing. When *many* corpus entries look similar to a query (which tends to happen with **small token sizes**), these methods lose much of their advantage and can even perform worse than `"naive"`.

Because of this, `"indexed"` and `"hashed"` are typically only worth using when your **token sizes are large** – in practice, that often means a `min_token_length` of **at least 15–30 characters**, depending on your workload.

### `"naive"` (default)

- **What it does:**  
  Stores MinHash signatures in memory and performs a straightforward scan to find the best match.
- **Pros:**
  - Simple and robust.
  - Good for small to medium corpora.
  - Predictable memory usage.
  - Often the best choice when tokens are small (e.g. short n-grams or very short strings).
- **Cons:**
  - For very large corpora, queries are `O(N)` over the corpus, so other methods can be much faster *if* they can prune effectively.

### `"indexed"`

- **What it does:**  
  Builds a **small auxiliary index** to narrow down candidate strings before comparing signatures. Conceptually similar in spirit to `"hashed"`, but with a lighter-weight index.
- **Pros:**
  - Faster lookups than `"naive"` when:
    - Tokens are relatively large (`min_token_length` ≳ 10–15), and
    - Only a small fraction of the corpus is a close match to any given query.
  - Lower memory usage than `"hashed"`.
- **Cons:**
  - When tokens are small and many strings are roughly similar, the candidate set becomes large and the advantage over `"naive"` shrinks or disappears.
  - More more time needed for indexing than `"naive"` (memory overhead negligible at scale).

### `"hashed"`

- **What it does:**  
  Builds a **reverse index** that maps token hashes back to candidate strings, again aiming to prune the search space before full comparison.
- **Pros:**
  - Often the **fastest** for large, static corpora with **large tokens** (`min_token_length` ≳ 10–15) and relatively few close matches per query.
  - Queries run over a much smaller candidate set instead of the whole corpus when tokenization yields selective, informative tokens.
- **Cons:**
  - Higher memory usage due to the reverse index:
    - With large tokens, this index can be substantial; for many realistic workloads it will often **about double the data size** compared to `"naive"`.
  - As with `"indexed"`, when tokens are small and there are many near matches, the candidate sets explode and performance degrades.

**Rule of thumb:**

- If your tokens are **short** (default `min_token_length = 0`, `max_token_length = 8`), start and usually stay with `"naive"`.
- Consider `"indexed"` or `"hashed"` only when:
  - You deliberately set **large token sizes**, e.g. `min_token_length ≥ 10–15`, and
  - You expect relatively **sparse** close matches in your corpus.

---

## Token Length Parameters

Two key parameters control how strings are broken into tokens (byte n-grams):

- `min_token_length`
- `max_token_length`

These directly influence both matching behavior and performance, and they interact strongly with the choice of `method`.

### `min_token_length`

> **Intuition:** “Ignore extremely short tokens that are usually noise.”

- Tokens shorter than `min_token_length` are **ignored**.
- Small tokens (like 1–2 bytes) can:
  - Capture fine-grained similarities.
  - Useful for when your goal is to correct spelling mistakes or similar errors over short strings.
  - But also create a lot of noise and dramatically increase overlap between many strings.
- High `min_token_length` (e.g. 10–15 bytes):
  - Tokens are long, more “phrase-like”.
  - Typically produce **much fewer** overlaps, which is exactly what `"indexed"` and `"hashed"` rely on to be fast.

### `max_token_length`

> **Intuition:** “How large can a token be? Larger tokens mean fewer, more specific overlaps.”

- Tokens longer than `max_token_length` are **not** generated.
- Larger tokens:
  - Capture more context in each match (more specific).
  - Reduce the number of overlapping tokens between strings.
  - But increases the index building time, especially when the difference between min and max token length is large.

### Combined Effect and Method Choice

- **Small token window** (e.g. `min_token_length = 0`, `max_token_length = 8`):
  - Many small tokens per string.
  - Lots of overlap between many corpus strings.
  - `"naive"` often wins because pruning is ineffective for `"indexed"`/`"hashed"`.

- **Large token window** (e.g. `min_token_length = 10–15`, `max_token_length = 16–24`):
  - Fewer, longer tokens per string.
  - Overlaps become more selective.
  - `"indexed"` and `"hashed"` can prune aggressively and be much faster than `"naive"`.
  - Memory usage increases for `"hashed"` due to the reverse index (often around double the data size).

If you’re not sure where to start:

- Use the **defaults** (`min_token_length = 0`, `max_token_length = 8`, `method = "naive"`) for general use and smaller workloads.
- Only move to large tokens (`min_token_length ≥ 10–15`) and `"indexed"`/`"hashed"` when:
  - Your corpus is large and relatively static, **and**
  - You can tolerate adjusting tokenization for speed.

---

## API Reference

### Class: `TokenFuzzer`

#### Constructor

```python
TokenFuzzer(
    strings: list[str],
    num_hashes: int = 128,
    method: str = "naive",
    min_token_length: int = 0,
    max_token_length: int = 8,
) -> TokenFuzzer
```

Builds an index over the provided list of strings.

- **Parameters:**
  - `strings`:  
    List of strings to match against (the **corpus**). The corpus is treated as static after construction.
  - `num_hashes` (default: `128`):  
    Number of MinHash functions used for signature computation.
    - More hashes ⇒ more accurate similarity estimation, at the cost of more memory and CPU.
  - `method` (default: `"naive"`):  
    Internal algorithm to use. One of:
    - `"naive"`: simple in-memory scan.
    - `"indexed"`: index-based pruning; best with larger tokens and few near neighbors.
    - `"hashed"`: reverse index; often fastest with large tokens and sparse close matches, at higher memory cost.
  - `min_token_length` (default: `0`):  
    Minimum token length (in bytes) to include when generating tokens. Tokens shorter than this are ignored.
  - `max_token_length` (default: `8`):  
    Maximum token length (in bytes) to include when generating tokens.

- **Behavior:**
  - Computes MinHash-style signatures for each string (one-time cost).
  - Uses a deterministic set of hash seeds for reproducible signatures.

#### Methods

```python
match_closest(self, s: str) -> str
```

Returns the **closest-matching string** from the original corpus.

- **Parameters:**
  - `s`: query string.
- **Returns:**
  - A single string – the best match from the corpus.
- **Raises:**
  - `ValueError` if the corpus was empty at construction time.
- **Details:**
  - Similarity is measured as the fraction of matching components in MinHash signatures (a float in `0.0..1.0` under the hood).
  - If multiple corpus entries tie for best score, the first matching entry encountered is returned.

```python
match_closest_batch(self, queries: list[str]) -> list[str]
```

Returns the **closest-matching corpus string for each query**.

- **Parameters:**
  - `queries`: list of query strings to match against the indexed corpus.
- **Returns:**
  - A list of strings: for each query, the corpus string with the highest MinHash similarity.
- **Raises:**
  - `ValueError` if the corpus was empty at construction time.
- **Details:**
  - Each query is processed independently.
  - Under the hood, matching is performed in parallel using a thread pool (Rayon in Rust).
  - The order of results matches the order of input queries.

Example:

```python
from token_fuzz_rs import TokenFuzzer

corpus = ["hello world", "other text", "rust programming"]
fuzzer = TokenFuzzer(corpus)

queries = ["hello wurld", "other txt", "rust progrmming"]
results = fuzzer.match_closest_batch(queries)

print(results)
# -> ["hello world", "other text", "rust programming"]
```

---

## How It Works (High Level)

- Strings are treated as raw byte sequences.
- For each position in the string, the library builds short byte tokens (from `min_token_length` up to `max_token_length`).
- Each token is hashed with multiple independent hash functions based on SplitMix64.
- For each hash function, the minimum hash value seen over all tokens becomes one element of a **MinHash signature**.
- Similarity between two strings is approximated by the fraction of equal entries in their signatures.
- `match_closest`:
  1. Computes the MinHash signature for the query string.
  2. Compares it against all (or candidate) precomputed signatures in the corpus.
  3. Returns the string with the highest similarity score.
- `match_closest_batch` performs the same process for each query, in parallel.

This design:

- Exploits **token overlap** rather than pure character-level edit distance.
- Allows fast, approximate similarity search once signatures (and optional indexes) are precomputed.
- Scales well to large, static corpora with many queries.

---

## Notes & Limitations

- Similarity is **approximate** (MinHash-based), not exact edit distance.
- Matching is 1-to-N: both `match_closest` and `match_closest_batch` return only the **single best match per query**.
- The index is effectively **immutable** after construction; to add or remove strings, build a new `TokenFuzzer`.
- The library is intended to be used from Python; the Rust implementation is an internal detail, though I am open to releasing as crate in the future.

---

## License

This project is licensed under the **MIT License**.  
Feel free to use it, open issues, and make small PRs to improve performance or ergonomics.