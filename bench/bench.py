import random
import string
import time
import pandas as pd
import matplotlib.pyplot as plt
import difflib

from rapidfuzz import process, fuzz
from token_fuzz_rs import TokenFuzzer


# -----------------------------
# CONFIG
# -----------------------------

Ns = [
    200,
    1_000,
    5_000,
    10_000,
    50_000,
    100_000,
    500_000,
    1_000_000,
    5_000_000,
    10_000_000,
]

QUERY_COUNT = 100
MIN_LEN = 50
MAX_LEN = 100
alphabet = string.ascii_lowercase


# -----------------------------
# DATA GENERATION
# -----------------------------

def random_string():
    length = random.randint(MIN_LEN, MAX_LEN)
    return "".join(random.choice(alphabet) for _ in range(length))


def mutate_two_chars(s):
    s = list(s)
    idx = random.sample(range(len(s)), 2)

    for i in idx:
        s[i] = random.choice(alphabet)

    return "".join(s)


def generate_dataset(N):

    corpus = [random_string() for _ in range(N)]

    chosen = random.sample(corpus, QUERY_COUNT)
    queries = [mutate_two_chars(s) for s in chosen]

    return corpus, queries


# -----------------------------
# BENCHMARK TARGETS
# -----------------------------

def bench_token_fuzzer(corpus, queries, method, min_token_length, max_token_length):

    fuzzer = TokenFuzzer(corpus, num_hashes=128, method=method, min_token_length=min_token_length, max_token_length=max_token_length)

    start = time.perf_counter()
    fuzzer.match_closest_batch(queries)
    end = time.perf_counter()

    return end - start


def bench_rapidfuzz(corpus, queries):

    start = time.perf_counter()

    for q in queries:
        process.extractOne(q, corpus, scorer=fuzz.ratio)

    end = time.perf_counter()

    return end - start

def bench_rapidfuzz_token(corpus, queries):

    start = time.perf_counter()

    for q in queries:
        process.extractOne(q, corpus, scorer=fuzz.token_set_ratio, score_cutoff=0.9)

    end = time.perf_counter()

    return end - start


def bench_difflib(corpus, queries):

    start = time.perf_counter()

    for q in queries:
        difflib.get_close_matches(q, corpus, n=1)

    end = time.perf_counter()

    return end - start


# -----------------------------
# RUN BENCHMARK
# -----------------------------

results = []

for N in Ns:

    print(f"\nGenerating dataset N={N:,}")

    corpus, queries = generate_dataset(N)

    row = {"N": N}

    # TokenFuzzer methods
    for method, min_token_length, max_token_length in [
        ("naive", 3, 10),
        ("indexed", 15, 25),
        ("hashed", 15, 25),
        ("grouped", 0, 8)
        ]:

        print(f"Running token_fuzz_rs ({method})")

        row[f"token_{method}"] = bench_token_fuzzer(
            corpus,
            queries,
            method,
            min_token_length=min_token_length,
            max_token_length=max_token_length,
        )
    
    if N <= 10_000_000:
        print("Running RapidFuzz")
        row["rapidfuzz"] = bench_rapidfuzz(corpus, queries)

    if N <= 1_000:
        print("Running difflib")
        row["difflib"] = bench_difflib(corpus, queries)

    results.append(row)


# -----------------------------
# PRINT TABLE
# -----------------------------

df = pd.DataFrame(results)

print("\nBenchmark Results:\n")
print(df.to_string(index=False))


# -----------------------------
# PLOT GRAPH
# -----------------------------

plt.figure()

for column in df.columns:

    if column == "N":
        continue

    plt.plot(df["N"], df[column], label=column)

plt.xscale("log")
plt.yscale("log")

plt.xlabel("Corpus Size (N)")
plt.ylabel("Total Query Time (seconds)")
plt.title("Fuzzy Search Benchmark")

plt.legend()
plt.tight_layout()

plt.savefig("fuzzy_benchmark.png")
plt.show()