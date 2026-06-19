# banlex

**Multilingual content-risk lexicon and classifier for content-moderation pipelines.**

`banlex` is a small Rust library and CLI that scores input text against
configurable risk **categories** using a data-driven, multilingual lexicon
(term → category + weight + language tag). It returns per-category scores, the
exact terms that matched, and an overall risk verdict gated on a threshold.

It is built for **defensive / moderation use only**: routing or flagging
content for human review, triage queues, and pre-filters. It does not generate
content and ships only mild, generic *indicator* terms.

## What it does

- Loads a lexicon (built-in default, or your own JSON via `--lexicon`).
- Tokenizes and normalizes text (Unicode-aware lowercasing, word boundaries).
- Matches single words **and multi-word phrases** (longest match wins per span).
- Aggregates **per-category scores** and records every matched term.
- Produces a verdict (`CLEAR` / `FLAGGED`) against a `--threshold`, with an
  exit-code gate suitable for CI / pipeline use.

### Built-in default categories

| Category             | Purpose                                                  |
|----------------------|----------------------------------------------------------|
| `spam`               | promotional / unsolicited-bulk indicators                |
| `scam`               | fraud / advance-fee / credential-bait indicators         |
| `harassment`         | hostile / threatening tone indicators                    |
| `self-harm-keywords` | distress indicators (route to a human / help resources)  |

The default lexicon is an original, hand-authored short list spanning English,
Spanish, French, and German. It is a demonstration set, not a comprehensive
moderation wordlist — supply your own lexicon for production use.

## Install / build

Requires a stable Rust toolchain.

```sh
cargo build --release
```

The binary is produced at `target/release/banlex`.

## CLI usage

```sh
# Scan text from arguments
banlex scan "you have won, send gift card now"

# Scan a file, custom threshold, JSON output
banlex scan --file message.txt --threshold 1.5 --json

# Use a custom lexicon
banlex scan --lexicon examples/lexicon.json "claim your prize, free money!"

# List the categories in the active lexicon
banlex categories
banlex categories --lexicon examples/lexicon.json --json
```

### Exit codes (`scan`)

| Code | Meaning  | Condition                          |
|------|----------|------------------------------------|
| `0`  | CLEAR    | total score below threshold        |
| `1`  | FLAGGED  | total score `>=` threshold         |
| `2`  | ERROR    | usage / IO / lexicon error         |

This makes `banlex` usable as a gate in a moderation or CI pipeline:

```sh
if banlex scan --file comment.txt --threshold 1.5 >/dev/null; then
  echo "ok"
else
  echo "needs review"
fi
```

### Example output

```
$ banlex scan "you have won, please verify your account"
verdict: FLAGGED
total score: 2.400 (threshold 1.000, 7 tokens)
categories:
  scam                   2.400  (2 matches)
matched terms:
  [scam] "you have won" (w=1.30, lang=en, @tok 0)
  [scam] "verify your account" (w=1.10, lang=en, @tok 4)
```

The total score is the **maximum** single-category score (the strongest signal),
not the sum across categories.

## Lexicon format

A lexicon is JSON:

```json
{
  "name": "example-marketing-risk",
  "entries": [
    { "term": "free money", "category": "scam", "weight": 2.0, "lang": "en" },
    { "term": "order now",  "category": "spam", "weight": 0.9, "lang": "en" }
  ]
}
```

- `term` — single word or multi-word phrase; normalized like the input.
- `category` — any string; categories are derived from the entries present.
- `weight` — finite, non-negative number added per match.
- `lang` — optional, informational language tag.

See [`examples/lexicon.json`](examples/lexicon.json) and the sample texts in
[`examples/texts/`](examples/texts/).

## Library API

```rust
use banlex::{default_lexicon, Lexicon, Scanner};

// Built-in lexicon, threshold 1.0
let scanner = Scanner::new(default_lexicon(), 1.0);
let result = scanner.scan("Please click here and buy now!");
println!("{} ({:.2})", result.verdict(), result.total_score);
for c in &result.category_scores {
    println!("{}: {:.2} ({} matches)", c.category, c.score, c.match_count);
}

// Load a custom lexicon from JSON
let lex = Lexicon::from_json(r#"{ "entries": [
    { "term": "free money", "category": "scam", "weight": 2.0 }
] }"#).unwrap();
let r = Scanner::new(lex, 1.5).scan("get free money today");
assert!(r.flagged);
```

## Testing

```sh
cargo test
```

Unit tests cover tokenization, single-word and phrase scoring, the
longest-match / no-double-count rule, multilingual matching, and lexicon
validation. Integration tests in `tests/cli.rs` drive the compiled binary for
the threshold gate, JSON output, custom-lexicon loading, file input, and error
exit codes.

## Scope and limitations

- Lexicon matching is a **heuristic pre-filter**, not a judgment. It will miss
  obfuscation, sarcasm, and context, and will over-match benign uses. Always
  keep a human in the loop, especially for the `self-harm-keywords` category,
  which should route to support resources rather than punitive action.
- The default lexicon is intentionally tiny and illustrative.

## License

License: COCL 1.0

## Maintainer

Cognis Digital — `cognis-digital/banlex`
