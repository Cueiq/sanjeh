#!/usr/bin/env python3
"""Fit a line model to a benchmark's `fit` split.

    cargo run -q --bin sanjeh -- line features <items.jsonl> > features.jsonl
    python3 tools/fit_line.py features.jsonl models/line/<name>.json --name <name> [--about "..."]

Level is fitted as an ordinal model over each rater's own level: a line's
level L = bias + Σ w·z (z the standardised features), and a rater at level
R votes it

    easy  with chance 1 − σ(s·(L − R)),
    hard  with chance σ(s·(L − R − 1)),
    right otherwise,

so a B2 rater's "right" says the line is B2, whatever the rater's level.
A rater's level shifts with how well they know an episode (one they have
practised feels easier), so each rater and episode has its own offset, fitted
beside the weights: the weights learn only which lines are harder than which.
Where the scale sits comes from the baseline model (`--anchor`, the word
lists' scale): the bias makes the fitted levels average what it gives the
same lines. A learner's own level, learnt from their tries, does the rest.
Worth is a logistic model of the chance a line is rated worth it ("yes").
Both are held near zero (ridge), as a few hundred ratings can't carry many
weights. Needs numpy.
"""
import argparse, json
import numpy as np

RANK = {'A1': 1, 'A2': 2, 'B1': 3, 'B2': 4, 'C1': 5, 'C2': 6}
FEATURES = {
    'ear': {
        'level': ['vocab', 'hard_words', 'common', 'words', 'sps', 'sentences', 'ear_melts', 'weak_forms',
                  'entries', 'names', 'miss_tiny', 'miss_base', 'miss_small', 'unsure'],
        'worth': ['words', 'entries', 'formula', 'common', 'names', 'question', 'exclaim', 'ear_melts',
                  'weak_forms', 'vocab', 'miss_base'],
    },
    'mouth': {
        'level': ['vocab', 'hard_words', 'syllables', 'sps', 'sentences', 'ear_melts', 'melts', 'hard_sounds',
                  'entries', 'names', 'miss_tiny', 'miss_small'],
        'worth': ['words', 'entries', 'formula', 'common', 'names', 'question', 'exclaim', 'contractions',
                  'hard_sounds', 'vocab', 'miss_base'],
    },
}
SCALE = 2.5  # how sharply a vote turns at a rater's level


def sig(x):
    return 1 / (1 + np.exp(-x))


def standardise(rows, names):
    X = np.array([[r['features'].get(n, 0.0) for n in names] for r in rows], float)
    mean, sd = X.mean(0), X.std(0)
    sd[sd == 0] = 1.0
    return (X - mean) / sd, mean, sd


def linear(model, features):
    return model['bias'] + sum(w * (features.get(n, 0.0) - m) / (s or 1.0)
                               for n, m, s, w in zip(model['features'], model['mean'], model['sd'], model['weights']))


def fit_level(rows, names, ridge, anchor):
    data = [(r, l) for r in rows for l in r['labels'] if l.get('level')]
    Z, mean, sd = standardise([r for r, _ in data], names)
    groups = sorted({(l['rater'], r['group']) for r, l in data})
    G = np.array([groups.index((l['rater'], r['group'])) for r, l in data])
    offset = np.zeros(len(groups))
    R0 = np.array([RANK[l['rater_level']] for _, l in data], float)
    vote = np.array([{'easy': 0, 'right': 1, 'hard': 2}[l['level']] for _, l in data])
    w, b = np.zeros(len(names)), float(R0.mean())
    for _ in range(6000):
        R = R0 + offset[G]
        L = b + Z @ w
        a, c = sig(SCALE * (L - R)), sig(SCALE * (L - R - 1))
        d = np.where(vote == 0, SCALE * a,
            np.where(vote == 2, -SCALE * (1 - c),
                     -SCALE * (a * (1 - a) - c * (1 - c)) / np.maximum(a - c, 1e-9)))
        w -= 0.05 * (Z.T @ d / len(d) + ridge * w)
        b -= 0.05 * d.mean()
        # the offsets move against the line: a rater shifted up sees it easier
        offset -= 0.05 * (np.bincount(G, weights=-d, minlength=len(groups)) / len(d) + 0.01 * offset)
        offset -= offset.mean()
    # where the scale sits: the anchor's average over the same lines
    target = np.mean([linear(anchor, r['features']) for r, _ in data])
    b = float(target)
    print('    rater offsets:', ', '.join(f'{g[1]} {o:+.2f}' for g, o in zip(groups, offset)))
    return {'features': names, 'mean': mean.tolist(), 'sd': sd.tolist(), 'weights': w.round(4).tolist(), 'bias': round(b, 4)}


def fit_worth(rows, names, ridge):
    data = [(r, l) for r in rows for l in r['labels'] if l.get('worth')]
    Z, mean, sd = standardise([r for r, _ in data], names)
    y = np.array([1.0 if l['worth'] == 'yes' else 0.0 for _, l in data])
    w, b = np.zeros(len(names)), 0.0
    for _ in range(6000):
        g = sig(Z @ w + b) - y
        w -= 0.1 * (Z.T @ g / len(y) + ridge * w)
        b -= 0.1 * g.mean()
    return {'features': names, 'mean': mean.tolist(), 'sd': sd.tolist(), 'weights': w.round(4).tolist(), 'bias': round(b, 4)}


def main():
    p = argparse.ArgumentParser()
    p.add_argument('features')
    p.add_argument('out')
    p.add_argument('--name', required=True)
    p.add_argument('--about', default='')
    p.add_argument('--ridge', type=float, default=0.3)
    p.add_argument('--anchor', default='models/line/v0-hand.json', help='the model whose scale the levels keep')
    a = p.parse_args()
    rows = [json.loads(l) for l in open(a.features) if l.strip()]
    anchor = json.load(open(a.anchor))
    fit = [r for r in rows if r['split'] == 'fit']
    model = {'name': a.name, 'about': a.about or f'fitted by tools/fit_line.py on {len(fit)} lines (ridge {a.ridge})'}
    for skill in ('ear', 'mouth'):
        mine = [dict(r, labels=[l for l in r['labels'] if l['skill'] == skill]) for r in fit]
        model[skill] = {'level': fit_level(mine, FEATURES[skill]['level'], a.ridge, anchor[skill]['level']),
                        'worth': fit_worth(mine, FEATURES[skill]['worth'], a.ridge)}
    json.dump(model, open(a.out, 'w'), indent=1)
    for skill in ('ear', 'mouth'):
        for part in ('level', 'worth'):
            m = model[skill][part]
            top = sorted(zip(m['features'], m['weights']), key=lambda t: -abs(t[1]))[:5]
            print(f"{skill} {part}: bias {m['bias']:+.2f} · " + ', '.join(f'{n} {x:+.2f}' for n, x in top))


if __name__ == '__main__':
    main()
