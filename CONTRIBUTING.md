# Contributing to Sanjeh

Thank you for helping. Sanjeh is meant to be improved by people who know
language teaching, linguistics or speech, not only by people who write Rust:
a better word list, a fairer rule, a few hundred honest ratings or a new
benchmark can matter more than code.

- [What Sanjeh is](#what-sanjeh-is)
- [Getting set up](#getting-set-up)
- [Ways to help, and where to start](#ways-to-help-and-where-to-start)
- [Adding a feature](#adding-a-feature)
- [Adding or improving a model](#adding-or-improving-a-model)
- [Adding a benchmark](#adding-a-benchmark)
- [Rating lines](#rating-lines)
- [Pull requests](#pull-requests)
- [Style](#style)

## What Sanjeh is

Sanjeh measures what language-learning material asks of a learner: how an
English word sounds, where a line's words melt into each other, how hard a
line is to hear and to say, what level a word is. None of these can be
proven right; they can only be made to agree better with learners and
teachers. So the project is built from three kinds of thing, each changed in
its own way:

| | What it is | Where | How it changes |
|---|---|---|---|
| **Features** | numbers computed from the input by deterministic, tested code | `crates/sanjeh/src/` | code and tests |
| **Models** | small, versioned weight files that turn features into a measure | `models/` | a new file, fitted and benchmarked |
| **Benchmarks** | labelled items, a fixed split, and a card saying where they came from | `benchmarks/` | data, with its licence |

A better measure is shown, not argued: on a benchmark's `test` split, per
level, beyond noise.

## Getting set up

You need [Rust](https://rustup.rs) (1.85 or newer) and, for fitting models,
Python 3 with numpy.

```bash
git clone https://github.com/Cueiq/sanjeh && cd sanjeh
cargo test                                   # everything should pass
echo '{"text": "Can I get a coffee, please?", "duration_ms": 1500}' | cargo sanjeh line measure -
cargo sanjeh line bench benchmarks/example/items.jsonl --split all
python3 -m pip install -r tools/requirements.txt
```

`cargo sanjeh …` is an alias for `cargo run -q --bin sanjeh -- …`
(`.cargo/config.toml`). `benchmarks/example/` is twelve lines with invented
labels, there only so every command runs on a fresh clone: its numbers mean
nothing.

## Ways to help, and where to start

- **Issues labelled [`good first issue`](https://github.com/Cueiq/sanjeh/labels/good%20first%20issue)**
  are small and self-contained.
- **Rate lines** for a benchmark, at your own level ([below](#rating-lines)).
- **Find where a measure is wrong**: a line measured too easy or too hard,
  a melt the engine misses, a word at the wrong level. Open an issue with the
  input, what Sanjeh said, and what you expected and why.
- **Bring data**: an openly licensed word list, corpus or recording set that
  could become a benchmark (open a *benchmark* issue first).
- **Improve a rule or a model**, as below.

If you are unsure whether something is wanted, open an issue and ask.

## Adding a feature

Line features live in `crates/sanjeh/src/line/features.rs`.

1. **Name it.** The table in `Features`' documentation is the contract with
   the models: a feature's name and meaning never change once a model uses
   it. Add a row for yours.
2. **Compute it** in `facts()`, from the line and what the pronouncing table,
   the word lists and the melt engine say. Keep it deterministic: the same
   line always gives the same number.
3. **Test it** in `crates/sanjeh/tests/line.rs`, on real speech: a line where
   the feature should be high, one where it should be low.
4. **Say why it matters** where it's useful to a learner: a `Reason` (in
   `line/mod.rs`) lets an app tell the learner what makes the line hard.
5. **Fit and bench** a model with it (next section). A feature no model can
   use to do better is noise; say what the bench showed in your pull request.

## Adding or improving a model

Models are JSON weight files in `models/line/` (`LineModel`): for each skill
(`ear`, `mouth`), a linear model of the line's level on the CEFR scale and a
logistic model of its worth, over standardised features.

```bash
cargo sanjeh line features benchmarks/<set>/items.jsonl > features.jsonl
python3 tools/fit_line.py features.jsonl models/line/<name>.json --name <name> --about "what it was fitted on, and how"
cargo sanjeh line bench benchmarks/<set>/items.jsonl --model models/line/<name>.json --split test
cargo sanjeh line bench benchmarks/<set>/items.jsonl --model models/line/v1.json --split test
```

- Fit on `fit` only; judge on `test` only. Never tune by looking at `test`.
- `v0-hand` (hand-made rules) is the baseline to beat, `v1` the current default.
- Report every level the benchmark has, not an average: a model that helps
  B2 learners and hurts A1 learners is not better.
- Any tool may fit a model, as long as it writes the same JSON.
  `tools/fit_line.py` explains its method (an ordinal model over each rater's
  own level) in its docstring.
- Making a model the default (`line::default_model`) is its own pull request,
  with the bench results on every public benchmark.

## Adding a benchmark

A benchmark is a folder in `benchmarks/` with `items.jsonl` (the format is
in [benchmarks/README.md](benchmarks/README.md)) and a `card.md`:

- **Sources and licences**, per file. Public sets hold only openly licensed
  text and audio (CC0, CC BY, CC BY-SA, MIT and the like). Lines from films,
  series or anything else you may not share stay in `benchmarks/private/`,
  which git ignores.
- **How items were chosen**, and **how labels were collected**: who rated,
  their levels and first languages, what they were asked.
- **Items and labels per level and per skill.**
- **The split**: which items are `test`. It never changes once published; a
  new split is a new benchmark.

Open a *benchmark* issue before collecting much: others may know of data,
or of a problem with it.

## Rating lines

A rater says, **for themselves**, whether a line was *too easy*, *just
right* or *too hard*, and whether it was worth practising — once for writing
it down from the audio (`ear`) and once for saying it with the speaker
(`mouth`). Your own level turns your answer into a statement about the line,
so you don't need to know CEFR levels: only your own, roughly. Rate every
line you are given, the dull ones too, and don't look anything up first.

## Pull requests

- One change a pull request: a feature, a model, a benchmark, a fix.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test`
  pass (CI checks them).
- When a feature or a model changes, paste the `bench` output for each
  public benchmark, before and after.
- When data is added, its licence is in the card and the README's data table.
- Commit messages say what changed and why, in plain English.

## Style

- **Plain English** in documentation and comments: what the code does and
  why, with an example from real speech (*gonna* for *going to*) rather than
  an abstract one.
- **Every rule with its reason**: a threshold says where it came from; a
  word list says what it lists and why.
- Rust: `cargo fmt` (120 columns), no `unwrap` outside tests unless the data
  is bundled and checked by a test.
- No network access, no randomness, nothing slow at run time: the measures
  run inside apps.

By contributing you agree that your contribution is licensed under the
project's MIT licence, and that data you add is under the licence its card
states.

Be kind; see the [code of conduct](CODE_OF_CONDUCT.md).
