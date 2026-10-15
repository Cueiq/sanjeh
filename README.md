# Sanjeh

[![CI](https://github.com/its-saeed/sanjeh/actions/workflows/ci.yml/badge.svg)](https://github.com/its-saeed/sanjeh/actions/workflows/ci.yml)
[![Licence: MIT](https://img.shields.io/badge/licence-MIT-blue.svg)](LICENSE)

*Sanjeh* (سنجه) is Persian for "a measure". It measures what English
learning material asks of a learner, at every level from A1 to C2: how a word
sounds, where a line's words melt into each other, how hard a line is to hear
and to say, what level a word is.

None of this can be proven right; it can only be made to agree better with
learners and teachers. So every measure comes with **benchmarks**, results
are reported **per level**, and anyone can improve a measure by showing it
does better. [Contributions are welcome](CONTRIBUTING.md), from linguists,
teachers and learners as much as from programmers.

## What it measures

| Measure | Question it answers | State |
|---|---|---|
| `pronounce` | How does this word or phrase sound, in American or British English (IPA)? | done |
| `connected` | Where do this line's words melt into each other (*gonna*, *the-rare* for *there are*)? | done |
| `levels` | What CEFR level is this word, by the open word lists? | done |
| `line` | How hard is this line to **hear** (written down from the audio) and to **say** (shadowed with the speaker), and how much is it worth practising? Why? | first model, `v1` |
| spoken try | How close did a learner's try come to the speaker's rhythm, intonation and sounds? | [#9](https://github.com/its-saeed/sanjeh/issues/9) |
| two voices | Split a subtitle line said by two people into one line each | [#6](https://github.com/its-saeed/sanjeh/issues/6) |
| `words`, `placement` | A word's level in its sentence; a learner's level, by a short adaptive test | planned |

## Try it

You need [Rust](https://rustup.rs) 1.85 or newer.

```bash
git clone https://github.com/its-saeed/sanjeh && cd sanjeh
cargo test
echo '{"text": "Can I get a coffee, please?", "duration_ms": 1500}' | cargo sanjeh line measure -
```

For each skill, `ear` and `mouth`, you get whether the line can be practised,
its level on the CEFR scale (A1 = 1 … C2 = 6), its worth (0–1), and the
reasons: its pace, the words that melt (*the-rare* for *there are*), the
sounds learners find hard, the words above B1.

As a library:

```rust
use sanjeh::line::{LineInput, default_model, measure};

let line: LineInput = serde_json::from_str(r#"{"text": "Where did you put the keys?", "duration_ms": 1400}"#)?;
let measured = measure(&line, default_model());
println!("{:.1} to hear, {:.1} to say", measured.ear.level, measured.mouth.level);
```

and `sanjeh::pronounce("thought", sanjeh::Accent::Gb)`,
`sanjeh::connected::melts("pick it up", sanjeh::Accent::Us)`,
`sanjeh::levels::word_level("grieving")`.

## How it is built

- **Features** are computed by deterministic, tested code: a line's length,
  speed, melts, word levels, hard sounds, and what weaker speech recognisers
  missed in it (the *weak ear*: learners' mistakes rise where recognisers
  err).
- **Models** are small versioned weight files in `models/`, plain JSON, so
  the same file is used in Rust, in an app or in a notebook. A better model is
  a new file; a better feature is code.
- **Benchmarks** in `benchmarks/` hold labelled items, a fixed split into
  fitting and test data, and a card saying where they came from and under
  what licence. See [benchmarks/README.md](benchmarks/README.md).

```text
crates/sanjeh/src/
  pronounce.rs        the pronouncing table: IPA, US and GB
  connected/          the twelve rules of connected speech and the engine that finds them
  levels.rs           CEFR levels from the open word lists
  line/               a line's features, its reasons, its model, the weak ear
  bench.rs            benchmark items and how a model is judged
  bin/sanjeh.rs       the command line: cargo sanjeh line (features | measure | bench)
models/line/          v0-hand (the baseline), v1 (the default)
benchmarks/           README (the format), example/ (invented labels, for trying the tools)
tools/fit_line.py     fits a line model (needs numpy)
data/                 the bundled tables, each with its notice
```

## Data and licences

Sanjeh's code is under the [MIT licence](LICENSE). The data it bundles keeps
its own:

| Data | Source | Licence |
|---|---|---|
| `data/pronouncing/` | [Britfone](https://github.com/JoseLlarena/Britfone) 3.0.1 (British), [CMU Pronouncing Dictionary](https://github.com/cmusphinx/cmudict) (American), rewritten in IPA by `scripts/pronouncing/build.py` | MIT, BSD (see its `NOTICE.md`) |
| `data/connected-speech.json` | the twelve rules of connected speech, their word lists and examples | MIT |
| `data/levels/` | [CEFR-J Wordlist](http://www.cefr-j.org/) 1.5 (A1–B2), Octanove Vocabulary Profile C1/C2 1.0, via [Open Language Profiles](https://github.com/openlanguageprofiles/olp-en-cefrj) | free with citation; CC BY-SA 4.0 (see its `NOTICE.md`) |

If you use the word levels, cite the CEFR-J Wordlist as its notice asks.

## Contributing

Start with [CONTRIBUTING.md](CONTRIBUTING.md): how to set up, add a feature,
fit and judge a model, add a benchmark, or rate lines. Issues labelled
[good first issue](https://github.com/its-saeed/sanjeh/labels/good%20first%20issue)
are a good way in; the most needed help is
[a public benchmark rated at every level](https://github.com/its-saeed/sanjeh/issues/1).
Please follow the [code of conduct](CODE_OF_CONDUCT.md).
