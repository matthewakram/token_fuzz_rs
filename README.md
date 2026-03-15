# token-fuzz-rs

[![PyPI version](https://img.shields.io/pypi/v/token-fuzz-rs.svg)](https://pypi.org/project/token-fuzz-rs/)
[![PyPI - Python Version](https://img.shields.io/pypi/pyversions/token-fuzz-rs.svg)](https://pypi.org/project/token-fuzz-rs/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/matthewakram/token_fuzz_rs/blob/main/LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/matthewakram/token_fuzz_rs?style=social)](https://github.com/matthewakram/token_fuzz_rs)

**The fastest token-based fuzzy string matching in Python for very large, static corpora.**  
Rust core, Python-first API, distributed on PyPI.

- PyPI: https://pypi.org/project/token-fuzz-rs/  
- Source: https://github.com/matthewakram/token_fuzz_rs

Use this when you have a **large, mostly static list of strings** and need to run **many token-based queries** quickly.  
For smaller/one-off matching, use [RapidFuzz](https://github.com/maxbachmann/RapidFuzz).

**Token-based fuzzy matching** treats strings as collections of tokens (e.g., byte n‑grams), rather than as raw character sequences. In effect, it favors **shared fragments and word-level overlap**, making it more tolerant of reordered words, missing words, or small local edits. Traditional edit-distance-style fuzzing focuses on the exact character sequence, so it tends to penalize word reordering and long insertions more harshly.

---

## Install

```bash
pip install token-fuzz-rs
```

```python
from token_fuzz_rs import TokenFuzzer
```

---

## Quick Start

```python
from token_fuzz_rs import TokenFuzzer

data = [
    "hello world",
    "rust programming",
    "fuzzy token matcher",
]

fuzzer = TokenFuzzer(data)

print(fuzzer.match_closest("hello wurld"))            # -> "hello world"
print(fuzzer.match_closest("hello wurld I love you")) # -> "hello world"

results = fuzzer.match_closest_batch([
    "hello wurld",
    "rust progrmming",
])
print(results)  # -> ["hello world", "rust programming"]
```

---

## Configuration

```python
fuzzer = TokenFuzzer(
    strings=data,
    num_hashes=256,
    method="hashed",     # "naive" (default), "indexed", "hashed", or "grouped"
    min_token_length=15,
    max_token_length=30,
)
```

**Key knobs:**
- `num_hashes`: accuracy vs CPU/memory.
- `min_token_length` / `max_token_length`: token size window (byte n-grams).
- `method`: internal search strategy.

---

## When to Use `token-fuzz-rs`

**Great fit if:**
- Corpus is large (thousands → millions).
- Corpus is static or rarely changes.
- You run lots of queries.
- Token overlap matters more than strict edit distance.

**Not ideal if:**
- Small/medium corpora.
- You need many different matching metrics.
- You need dynamic inserts/updates.

---

## Alternatives (When to Use Them)

- **[RapidFuzz](https://github.com/maxbachmann/RapidFuzz)**  
  Best all‑around choice for small/medium corpora, rich metrics, and easy integration.

- **[TheFuzz (fuzzywuzzy)](https://github.com/seatgeek/thefuzz)**  
  Simple, widely known API; good for quick prototyping or compatibility with older code.

- **[textdistance](https://github.com/life4/textdistance)**  
  Huge collection of distance/similarity metrics; good for experimentation and research.

- **[python-Levenshtein](https://github.com/ztane/python-Levenshtein)**  
  Fast edit-distance primitives; good if you want raw distances and will build your own logic.

---

## Methods (Internal Algorithms)

All methods share the same API; they differ in how they prune candidates.

### `"naive"` (default)
- Scans all signatures.
- Predictable, robust.
- Best when token sizes are **small** or corpora aren’t huge.

### `"indexed"`
- Lightweight pruning index.
- Faster than naive **when tokens are long** and matches are sparse.
- Minimal extra memory.

### `"hashed"`
- Reverse index (larger memory).
- Often fastest for **large tokens** and sparse matches.
- Memory can be ~2× naive.

### `"grouped"` (new)
- **Fastest** when token sizes are **small** *and* matches are **very close**.
- Works best when **~90%+** of signature components match.
- If queries are not highly similar, it can be **less precise**.
- When the threshold is met, can be **~50× faster** than naive.

**Rule of thumb:**
- Small tokens (default 0–8): start with **`naive`**, use **`grouped`** only if you expect very high similarity.
- Large tokens (≥10–15): consider **`indexed`** or **`hashed`**.

---

## Token Length Parameters

Tokens are byte n-grams.  
These two parameters heavily affect behavior:

- `min_token_length`: ignores short tokens (less noise) (exclusive).
- `max_token_length`: caps token size (more context per token) (inclusive).

**Small window (4–8):**
- Many tokens per string.
- High overlap across corpus.
- **naive** often best.

**Large window (15–30):**
- Fewer, more selective tokens.
- Pruning becomes effective.
- **indexed/hashed** often best.

---

## API

### `TokenFuzzer`

```python
TokenFuzzer(
    strings: list[str],
    num_hashes: int = 128,
    method: str = "naive",
    min_token_length: int = 0,
    max_token_length: int = 8,
) -> TokenFuzzer
```

### `match_closest`

```python
match_closest(self, s: str) -> str
```

Returns the single closest corpus string.

### `match_closest_batch`

```python
match_closest_batch(self, queries: list[str]) -> list[str]
```

Batch version (parallelized internally).

---

## Benchmark

We compare token-fuzz-rs to the most popular fuzzy search library, RapidFuzz, and another popular library (difflib).
The query script can be found under /bench/bench.py. 
We present here the time it takes to make 100 queries over corpora of varying sizes.
To be very conservative, we set the similarity cuttoff at 90% so RapidFuzz can significantly prune its candidate pool, and also only use the levenshtein distance algorithm, which is significantly faster than their implementation of token-based fuzzy search.
In summary, token-fuzz-rs shows a performance increase of roughly 4 orders of magnitude over RapidFuzz.
The hashed algorithm has great performance but runs out of memory on the largest benchmark, causing significantly slower swap lookups.

```bash
Benchmark Results:

       N  token_naive  token_indexed  token_hashed  token_grouped  rapidfuzz   difflib
     200     0.002772       0.001654      0.001675       0.001373   0.002673  2.604417
    1000     0.002342       0.002267      0.002272       0.002030   0.012994 12.107110
    5000     0.003304       0.001493      0.001420       0.001499   0.076178       NaN
   10000     0.005966       0.001686      0.002015       0.001620   0.140908       NaN
   50000     0.052331       0.002283      0.002131       0.001591   0.756459       NaN
  100000     0.121471       0.002476      0.001733       0.001744   1.466600       NaN
  500000     0.677942       0.002526      0.002356       0.001565   6.710559       NaN
 1000000     1.553939       0.003203      0.002182       0.001832  14.486500       NaN
 5000000     8.621556       0.010324      0.002516       0.003704  68.900339       NaN
10000000    16.574239       0.019260      0.367714       0.005032 144.028963       NaN
```

![](bench/fuzzy_benchmark.png)

---

## How It Works (High Level)

- Strings → byte n-grams
- Tokens → MinHash signatures
- Similarity ≈ fraction of equal signature components
- One-time build, fast queries

---

## Notes

- Approximate similarity (MinHash), not edit distance.
- Index is immutable: rebuild to add/remove items.
- Python API only (Rust is internal, for now).

---

## License

MIT License. Contributions and issues welcome.
