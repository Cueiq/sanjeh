# Sanjeh

*Sanjeh* (سنجه) is Persian for "a measure". It measures what language-learning
material asks of a learner, for English, at every level from A1 to C2:

| Measure | Question it answers | State |
|---|---|---|
| `pronounce` | How does this word or phrase sound, in American or British English? | done |
| `connected` | Where do this line's words melt into each other (*gonna*, *pi-ki-tup*)? | done |
| `levels` | What CEFR level is this word, by the open word lists? | done |
| `line` | How hard is this film line to **hear** (dictation) and to **say** (shadowing), and how much is it worth practising? | first model, `v1` |
| `words` | What level is this word or phrase, in its sentence (not only by the lists)? | planned |
| `placement` | What level is this learner, for listening and for speaking? | planned |

None of these can be proven right; each can only be measured against data.
So every measure comes with **benchmarks**, and anyone can improve it:

- **Features** are computed by deterministic, tested code: a line's length,
  speed, melts, word levels, what weaker speech recognisers missed.
- **Models** are small versioned weight files (linear, logistic, or IRT for
  placement) in `models/`, evaluated the same way in Rust, in an app, or in a
  notebook. A better model is a new weights file; a better feature is code.
- **Benchmarks** in `benchmarks/` hold items with labels, a fixed split into
  fitting and test data, and a data card (source, licence, who labelled it,
  how many items at each level). Results are reported **per level**, so a
  change that helps B2 learners but hurts A1 learners shows.

## Data

| File | Source | Licence |
|---|---|---|
| `data/pronouncing/` | Britfone 3.0.1 (British), CMU Pronouncing Dictionary (American), rewritten in IPA by `scripts/pronouncing/build.py` | MIT, BSD (see its `NOTICE.md`) |
| `data/connected-speech.json` | The twelve rules of connected speech, their word lists and examples | MIT, as Sanjeh |
| `data/levels/` | CEFR-J Wordlist 1.5 (A1–B2), Octanove Vocabulary Profile C1/C2 1.0 | free with citation; CC BY-SA 4.0 (see its `NOTICE.md`) |

## Building and measuring

```bash
cargo test
cargo run -q --bin sanjeh -- line measure <lines.jsonl> --model models/line/v1.json
```

See [benchmarks](benchmarks/README.md) for fitting and judging a model.
