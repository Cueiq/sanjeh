# Placement: a learner's level for listening and speaking

*Status: design, 2026-10-15. Nothing here is built yet; the issues that build
it are listed [at the end](#the-work).*

## Goal

Tell, in about five minutes, a learner's level of English for **listening**
(the *ear*: understanding and writing down what is said) and for
**speaking** (the *mouth*: saying it), each to **within one CEFR level** —
and keep that estimate up to date as they practise.

**Not goals**: a certificate, reading or writing levels (a later measure may
add them), or anything that needs a model at run time beyond what Sanjeh's
other measures already use. A placement is a starting point for choosing
material, not a judgement of the learner, and must be easy to redo.

## What a level is here

Each skill has its own **ability**, θ, a number on a logit scale: a learner
at θ gets an item of difficulty *b* right with chance

```text
P(right) = 1 / (1 + e^-(a·(θ − b)))
```

where *a* (discrimination) says how sharply the item separates learners
below and above it (the two-parameter logistic model; with every *a* = 1 it
is the Rasch model). θ maps to a CEFR level by **cut-off points**
([Mapping to CEFR](#mapping-to-cefr)). The ear and the mouth are separate
abilities: a learner often understands far more than they can say.

The scale of θ is chosen so that it lines up with the scale `line` already
measures on (A1 = 1 … C2 = 6): an item's difficulty can start from its
measured level, and a learner's θ is directly comparable with a line's.

## The test

Three short parts, about five minutes in all:

| Part | Task | Items | Time | Tells |
|---|---|---|---|---|
| Vocabulary | *Is this an English word?* yes/no, real words and made-up ones | 30–40 | ~1.5 min | a quick, strong first estimate of overall level: the prior for both skills |
| Listening | hear a short line, write it down | 6–12, adaptive | ~2 min | the ear |
| Speaking | say a short line, read or after hearing it | 4–8, adaptive | ~1.5 min | the mouth |

A learner may skip speaking (no microphone, a quiet room): the mouth then
starts from the vocabulary and listening estimate, with a wide uncertainty
that practise narrows.

### Vocabulary yes/no

The fastest strong signal of level known: learners say which strings are
English words. Real words are drawn by **frequency band**; **made-up words**
(about one for every two real ones) that look and sound like English keep it
honest, since saying *yes* to everything scores nothing. Scored with a
correction for guessing (the Δm or ISDT scores of Huibregtse, Admiraal and
Meara, 2002), as LexTALE does (Lemhöfer and Broersma, 2012). Its result is
the **prior** for both skills, not a skill of its own.

Made-up words must follow English sound and spelling patterns and must not
be words — a generator in the spirit of Wuggy (Keuleers and Brysbaert, 2010),
checked against the pronouncing table and the word lists.

### Listening items

A line from **openly licensed audio** (Common Voice, Tatoeba), heard once or
twice, written down. Score: the share of its words caught, by the caller's
answer matching. A line is many small chances to be right, so it is treated
as one Bernoulli trial per word, all at the line's difficulty — more
information per item than a single right/wrong.

An item's difficulty **starts** from `line`'s ear level for it and is
replaced by its calibrated difficulty once enough learners have answered it
([Calibration](#calibration)).

### Speaking items

A short line, said after hearing it (or read, for the shortest test). Score:
the share of words a speech recogniser hears (a first, rough measure); later,
rhythm, intonation and sounds ([#9](https://github.com/its-saeed/sanjeh/issues/9)).
Difficulty starts from `line`'s mouth level.

## Estimating the level

After each answer, θ is estimated by **expected a posteriori** (EAP; Bock
and Mislevy, 1982): the mean of the posterior over a grid of θ values, with
its standard error (SE). The prior is normal: centred on the vocabulary
result (or on a level the learner says they have), wide enough (SD ≈ 1.2
levels) that a few answers can move it a long way. EAP, unlike maximum
likelihood, gives an estimate after a run of all-right or all-wrong answers.

## Choosing the next item

The next item is the one with the most **Fisher information** at the
current θ — the item the learner has about even odds of getting right —
among items of the part being given, not yet taken. To keep the most
informative items from being shown to everyone, it is drawn at random from
the best few (*randomesque*, Kingsbury and Zara, 1989), with a **seed the
caller gives**, so a test is reproducible: Sanjeh itself never uses hidden
randomness.

## Stopping

A part stops when the SE of its θ falls below about 0.35 (roughly a third of
a level), or after its most items; never before its fewest, so one lucky
answer can't end it. All three numbers are parameters, set from simulation
([Validation](#validation)).

## The item bank

Items are data, like benchmarks: a folder with `items.jsonl` and a card.

```json
{"id": "listen-cv-0412", "skill": "ear", "task": "dictation",
 "content": {"text": "Can I get a coffee, please?", "audio": "cv/0412.mp3"},
 "words": 6, "difficulty": 3.4, "discrimination": 1.0,
 "calibrated": false, "source": "Common Voice", "licence": "CC0"}
```

`calibrated: false` means the difficulty is a starting estimate (from a
measure), not yet from learners' answers. Only openly licensed content goes
in a public bank.

## Calibration

Once a bank has answers from enough learners (a few hundred per item for a
2PL; fewer for Rasch), each item's difficulty and discrimination are
estimated from them (marginal maximum likelihood), the starting estimates
replaced, and items that fit badly (that strong learners miss, or that
discriminate nothing) flagged for review. Calibration runs offline, like
fitting a line model, and its result is a new version of the bank.

## Mapping to CEFR

θ is a number; a level is a decision. The cut-off between, say, B1 and B2 is
set by **standard setting**: learners whose level is known by other means
(a certificate, a teacher's assessment) take the test, and the cut-offs are
placed where they best separate them (the Council of Europe's manual for
relating examinations to the CEFR, 2009, describes the methods). Until such
data exists, the cut-offs sit where the items' starting difficulties put
them, and the levels are **provisional** — said so wherever they are shown.

## Keeping the level up to date

Practice is a stream of answers to items of known difficulty: a line written
down, a line shadowed, a vote on a line. Each updates θ by the same model:
a small step toward what the answer says (an online EAP update, or an
Elo-style step for speed), the step shrinking as answers accumulate. A
placement only starts this; practice does most of the work.

## Validation

- **Simulation** (no learners needed): invented learners at known θ answer
  by the model; report how often the test lands within one level and within
  half a level, per level, and how many items it takes. This sets the
  stopping rule and the bank's size, and catches mistakes in the engine.
  Simulated results are labelled as such, never reported as accuracy.
- **Real learners**: a benchmark of learners with an outside level who take
  the test (and, for some, take it again a week later): agreement within one
  level per level, and test–retest consistency.

## Interface

```rust
pub struct Item { pub id: String, pub skill: Skill, pub task: Task, pub difficulty: f64, pub discrimination: f64, pub words: u32 }
pub struct Answer { pub item: String, pub right: u32, pub of: u32 }   // words right of words asked
pub struct Estimate { pub theta: f64, pub se: f64 }

pub fn estimate(answers: &[Answer], bank: &Bank, prior: Prior) -> Estimate;
pub fn next_item<'a>(bank: &'a Bank, now: &Estimate, taken: &[String], part: Task, seed: u64) -> Option<&'a Item>;
pub fn finished(now: &Estimate, asked: usize, rule: &StopRule) -> bool;
pub fn level(theta: f64, cuts: &Cuts) -> Level;
pub fn update(now: &Estimate, item: &Item, answer: &Answer) -> Estimate;   // keeping it up to date
```

Scoring an answer (which words were right) is the caller's: it depends on
how the answer was given (typed, spoken) and is not part of placement.

## Privacy

Placement needs only answers: item ids and how many words were right. No
audio, no typed text, nothing about the learner, is part of its input or of
any benchmark built from it without their consent.

## Open questions

- How many made-up words, and how they are scored, for learners whose first
  language is close to English (they recognise cognates) or far from it.
- Whether listening items should allow one replay, as practice does.
- How much a learner's own stated level should weigh in the prior.

## References

- Bock, R. D. and Mislevy, R. J. (1982). Adaptive EAP estimation of ability
  in a microcomputer environment. *Applied Psychological Measurement* 6(4).
- Council of Europe (2009). *Relating Language Examinations to the Common
  European Framework of Reference for Languages: A Manual.*
- Huibregtse, I., Admiraal, W. and Meara, P. (2002). Scores on a yes–no
  vocabulary test: correction for guessing and response style. *Language
  Testing* 19(3).
- Keuleers, E. and Brysbaert, M. (2010). Wuggy: A multilingual pseudoword
  generator. *Behavior Research Methods* 42(3).
- Kingsbury, G. G. and Zara, A. R. (1989). Procedures for selecting items
  for computerized adaptive tests. *Applied Measurement in Education* 2(4).
- Lemhöfer, K. and Broersma, M. (2012). Introducing LexTALE: A quick and
  valid Lexical Test for Advanced Learners of English. *Behavior Research
  Methods* 44(2).
- Nation, I. S. P. (1990). *Teaching and Learning Vocabulary*; Schmitt, N.,
  Schmitt, D. and Clapham, C. (2001). Developing and exploring the behaviour
  of two new versions of the Vocabulary Levels Test. *Language Testing* 18(1).
- van der Linden, W. J. and Glas, C. A. W., eds. (2010). *Elements of
  Adaptive Testing.* Springer.

## The work

Tracked in [#10](https://github.com/its-saeed/sanjeh/issues/10); each step
is its own issue. The first three need no data; calibration, mapping and the
real benchmark need learners, so they wait on the public benchmark and the
rating tools ([#1](https://github.com/its-saeed/sanjeh/issues/1),
[#2](https://github.com/its-saeed/sanjeh/issues/2)).

| # | Step | Section |
|---|---|---|
| [#11](https://github.com/its-saeed/sanjeh/issues/11) | Review the design and settle its open questions | [Open questions](#open-questions) |
| [#12](https://github.com/its-saeed/sanjeh/issues/12) | The adaptive test engine: estimate, choose the next item, stop | [Estimating](#estimating-the-level), [Choosing](#choosing-the-next-item), [Stopping](#stopping) |
| [#13](https://github.com/its-saeed/sanjeh/issues/13) | Simulated learners, to test the engine before any data | [Validation](#validation) |
| [#14](https://github.com/its-saeed/sanjeh/issues/14) | The item bank format | [The item bank](#the-item-bank) |
| [#15](https://github.com/its-saeed/sanjeh/issues/15) | Vocabulary yes/no items, scored with a correction for guessing | [Vocabulary yes/no](#vocabulary-yesno) |
| [#16](https://github.com/its-saeed/sanjeh/issues/16) | A made-up word generator | [Vocabulary yes/no](#vocabulary-yesno) |
| [#17](https://github.com/its-saeed/sanjeh/issues/17) | Listening items from openly licensed audio | [Listening items](#listening-items) |
| [#18](https://github.com/its-saeed/sanjeh/issues/18) | Speaking items | [Speaking items](#speaking-items) |
| [#19](https://github.com/its-saeed/sanjeh/issues/19) | Calibrating items from learners' answers | [Calibration](#calibration) |
| [#20](https://github.com/its-saeed/sanjeh/issues/20) | Mapping ability to CEFR levels | [Mapping to CEFR](#mapping-to-cefr) |
| [#21](https://github.com/its-saeed/sanjeh/issues/21) | A real benchmark of learners with a known level | [Validation](#validation) |
| [#22](https://github.com/its-saeed/sanjeh/issues/22) | Keeping the level up to date from practice | [Keeping the level up to date](#keeping-the-level-up-to-date) |
| [#23](https://github.com/its-saeed/sanjeh/issues/23) | `sanjeh placement simulate` and `try` | [Interface](#interface) |
