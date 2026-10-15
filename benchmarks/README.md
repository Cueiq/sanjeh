# Benchmarks

A benchmark is labelled items, a fixed split, and a card saying where they
came from. Models are fitted on `fit` and judged on `test`, never the other
way round; results are reported **per level**.

## Line benchmarks

A JSON Lines file, an item a line (`sanjeh::bench`):

```json
{"id": "e06-73", "group": "e06", "split": "test",
 "input": {"text": "Hey, guys, I'm gonna throw something out there.", "duration_ms": 2403,
           "accent": "us", "voices": 1, "heard": {"tiny": […], "base": […], "small": […]},
           "entries": [], "names": ["stuart", "denise"]},
 "labels": [{"rater": "r1", "rater_level": "B2", "skill": "ear", "level": "right", "worth": "yes"}]}
```

- `input` is what `sanjeh::line::measure` takes (`LineInput`). `heard` is
  optional: each word whisper's tiny, base and small English models heard
  in the line's clip, with the model's confidence (`p`).
- A label is one rater's view, **for themselves**: the line was `easy`,
  `right` or `hard` for them, and worth practising (`yes`, `meh`, `no`).
  Their own level turns that into the line's: a B2 rater's `right` is a B2
  line. `skill` is `ear` (written down from the film) or `mouth` (said with
  the actor).

```bash
cargo run -q --bin sanjeh -- line features <items.jsonl> > features.jsonl
python3 tools/fit_line.py features.jsonl models/line/<name>.json --name <name>
cargo run -q --bin sanjeh -- line bench <items.jsonl> --model models/line/<name>.json --split test
```

`bench` reports for each skill:

- **votes matched**: the share of votes the model's level makes exactly;
- **level AUC**: whether its levels put the lines a rater found easy below
  the rest (0.5 is chance, 1 perfect) — the ranking, whatever the scale;
- **worth AUC**: the same for lines rated worth it;
- **at their level, right**: of the lines it puts at the rater's level, the
  share they found right — what a learner would be given;
- **called worth it, yes**: of the lines it calls worth it, the share rated so;
- votes matched **by the level it gave**.

## Sets

| Set | Lines | Raters | Levels | Shared |
|---|---|---|---|---|
| `example` | 12 written for this repository | 1, invented (B1) | — | yes: **not a benchmark**, for trying the tools |
| `private/stuart` | 104 (fit 60, test 44) from *Stuart Fails to Save the Universe*, episodes 1–2 and 6 (6 rated blind) | 1 (B2, knew episodes 1–2 well) | B2 | **no**: the lines and audio are the show's |

Public sets, from openly licensed audio (Common Voice, Tatoeba) and raters at
every level, are the next step: every level from A1 to C2 needs raters at it.

## Results

`private/stuart`, test split (episode 6):

| Model | Ear: votes · level AUC · worth AUC | Mouth: votes · level AUC · worth AUC |
|---|---|---|
| `v0-hand` | 29% · 0.51 · 0.66 | 32% · 0.58 · 0.08 |
| `v1` | 39% · 0.61 · 0.55 | 39% · 0.52 · 0.65 |

One rater and 28 rated lines a skill: differences under about 0.1 are noise.
