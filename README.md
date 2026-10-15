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
cargo sanjeh melts "I'm going to tell him."
```

`cargo sanjeh` runs the command-line tool; every example below is real
output from it. To use Sanjeh as a library, add it from Git:

```toml
[dependencies]
sanjeh = { git = "https://github.com/its-saeed/sanjeh" }
```

## The measures

### How a word sounds — `pronounce`

The sound of a word or phrase in IPA, in American or British English, from
two open pronouncing dictionaries rewritten the way learners' dictionaries
write them: length marks, one main stress, `ɚ` and `ə`.

```console
$ cargo sanjeh pronounce water
ˈwɔːtɚ
$ cargo sanjeh pronounce --gb water
ˈwɔːtə
$ cargo sanjeh pronounce a cup of tea
ə kʌp ʌv tiː
$ cargo sanjeh pronounce read
read more than one way: red, riːd
```

A word whose readings mean different things (*read* now and before) is
*ambiguous*: the table won't choose for you, but `readings()` gives them all.

```rust
sanjeh::pronounce("water", sanjeh::Accent::Gb)   // Pronounced::Known("ˈwɔːtə")
sanjeh::readings("with", sanjeh::Accent::Gb)     // ["wɪð", "wɪθ"]
```

### Where words melt — `connected`

In real speech words run into each other, and that, more than unknown
words, is why learners who can read a line can't hear it. Sanjeh finds where
a line's words melt by twelve rules of connected speech, from each word's
sound in the learner's accent: **linking** (*pick it* → *pi-kit*), **weak
forms** (*to* → *tuh*), **fusions** (*going to* → *gonna*), **elision** (*and
shine* → *an shine*), **yod** (*did you* → *didja*), **dropped h** (*tell him*
→ *te-lim*), **dropped g** (*getting* → *gettin'*), ***'em*** (*tell them* →
*te-lem*), the American **soft t** (*water* → *wader*), the British **cut-off
t**, **assimilation** (*in my* → *im my*) and the British **linking r**.

```console
$ cargo sanjeh melts "Did you eat?"
Didja eat?
  yod           didja
  linking       you-weat
$ cargo sanjeh melts "Get a lot of water."
Get a lotta wader.
  fusion        a lotta
  flap-t        wader
$ cargo sanjeh melts --gb "Tell them to come in."
Te-lem tuh co-min.
  them          te-lem
  weak-form     tuh
  linking       co-min
```

The accent matters: *Pick it up* is *Pi-ki-tup* in British English and
*Pi-ki-dup* in American. Nothing melts across punctuation, where a speaker
pauses. `melts()` gives each melt with its rule, words and IPA; `melted()`
draws them into the stretches a learner reads. The rules, their names and
explanations in English and Persian, and their word lists are data:
`data/connected-speech.json`.

### What level a word is — `levels`

A word's CEFR level, by the open CEFR-J Wordlist (A1–B2) and the Octanove
Vocabulary Profile (C1–C2), found through its forms: *went* is *go*, *grieving*
is *grieve*.

```console
$ cargo sanjeh level go went grieving devastating toughie
go: A1
went: A1
grieving: B2
devastating: B1
toughie: not in the lists
```

The lists grade words, not meanings, and miss much spoken English
(*toughie*, *geez*): a word's level in its sentence is a measure still to
come.

### How hard a line is to hear and to say — `line`

A line from a film or a series, measured for two skills: the **ear**
(writing it down from the audio: dictation) and the **mouth** (saying it with
the speaker: shadowing). For each, whether it can be practised at all, its
level on the CEFR scale, how much it is worth practising, and the reasons, so
an app can tell a learner *why*.

```console
$ cargo sanjeh line show --ms 2400 "He reckons the whole thing's a waste of time."
ear   B2 (4.2) · worth 0.28 · can be practised
      9 words in 2.4 s
      reckons (B2)
      90% everyday words
mouth B1 (3.7) · worth 0.68 · can be practised
      9 words in 2.4 s
      the → thuh (weak-form)
      thing's a → thing-za (linking)
      a → uh (weak-form)
      waste of → was-tof (linking)
      of → uv (weak-form)
      th in thing's
      w in waste
      reckons (B2)
      90% everyday words
```

Some lines can't be practised, and say why: a sentence cut off, two people
speaking, a line too short to hold anything, or a word said wrong on purpose,
which leaves no answer to hold a learner to:

```console
$ cargo sanjeh line show --ms 2500 "You are weawy getting on my nerves."
ear   B1 (3.7) · worth 0.33 · can't be practised: said wrong on purpose (weawy)
…
```

The level and worth come from a **model** in `models/line/`: weights over
the line's features (its pace, length, melts, hard words, the sounds learners
find hard such as *th* and *w*, and the *weak ear*: what weaker speech
recognisers missed in the clip, given as `heard`, since learners' mistakes
rise where recognisers err). The default, `v1`, is a first model, fitted to
one learner's ratings: its levels bunch around B1 and B2. A benchmark rated
by learners at every level ([#1](https://github.com/its-saeed/sanjeh/issues/1))
is what will spread them out.

```bash
echo '{"text": "Where did you put the keys?", "duration_ms": 1400}' | cargo sanjeh line measure -   # JSON, for programs
```

```rust
use sanjeh::line::{LineInput, default_model, measure};

let line: LineInput = serde_json::from_str(r#"{"text": "Where did you put the keys?", "duration_ms": 1400}"#)?;
let measured = measure(&line, default_model());
println!("{:.1} to hear, {:.1} to say", measured.ear.level, measured.mouth.level);
```

### Measures to come

- **A spoken try** ([#9](https://github.com/its-saeed/sanjeh/issues/9)): how
  close a learner's try came to the speaker's rhythm, intonation, stress and
  sounds, judged against experts' scores.
- **Two voices** ([#6](https://github.com/its-saeed/sanjeh/issues/6)): a
  subtitle line said by two people split into one line each.
- **The weak ear built in** ([#5](https://github.com/its-saeed/sanjeh/issues/5)):
  Sanjeh running the speech recognisers itself.
- **Placement**: a learner's level for listening and for speaking, by a
  five-minute adaptive test, kept up to date by their practice — see
  [the design](docs/placement.md).
- **A word's level in its sentence**, not only by the word lists.

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
