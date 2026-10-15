#!/usr/bin/env python3
"""Build Sanjeh's pronouncing table from two open pronouncing dictionaries.

    python3 scripts/pronouncing/build.py <folder>

reads `britfone.main.3.0.1.csv` (British, MIT) and `cmudict.dict` (American,
BSD) from that folder — each checked against the hash it was built from, so
the table is never remade from another release by accident — and writes
`data/pronouncing/pronouncing.tsv.gz`: one line a word, sorted,
`word<TAB>us<TAB>gb<TAB>differs`, each side its sounds in IPA — several
separated by `|`, the dictionary's first first — or empty where that
dictionary doesn't have the word; `differs` is `1` for a word whose readings
say different things (`readings_differ`), else empty.

Both are written the way learners' dictionaries write them: length marks,
`r` and `g`, `e` for the vowel of *dress*, `ʌ` for *cut*, `i` and `u` for the
unstressed vowels of *happy* and *influence*; the stress mark before the
syllable it stresses, none on a word of one syllable, and one main stress a
word — a second the dictionary marks main is written secondary, the last
main. Britfone lacks most contractions and possessives (*I'm*, *it's*,
*Tom's*); those the American dictionary has are made from the British
word and Britfone's own clitic sounds (`derive_british`).
"""

import gzip
import hashlib
import re
import sys
from pathlib import Path

# The releases the table is made from: another one is a decision, not an
# accident (review, 2026-10-06).
SOURCES = {
    'britfone.main.3.0.1.csv': '59f197e98520856d1cc88e380beb54e4314d8712efc4d588b6778819c502d920',
    'cmudict.dict': '81917843c7f44ce2b094ac63873c2c7a4cf802040792c455ba3ca406891c3d22',
}

ARPABET_VOWELS = {
    'AA': 'ɑː', 'AE': 'æ', 'AO': 'ɔː', 'AW': 'aʊ', 'AY': 'aɪ', 'EH': 'e',
    'EY': 'eɪ', 'IH': 'ɪ', 'IY': 'iː', 'OW': 'oʊ', 'OY': 'ɔɪ', 'UH': 'ʊ', 'UW': 'uː',
}
ARPABET_CONSONANTS = {
    'B': 'b', 'CH': 'tʃ', 'D': 'd', 'DH': 'ð', 'F': 'f', 'G': 'g', 'HH': 'h',
    'JH': 'dʒ', 'K': 'k', 'L': 'l', 'M': 'm', 'N': 'n', 'NG': 'ŋ', 'P': 'p',
    'R': 'r', 'S': 's', 'SH': 'ʃ', 'T': 't', 'TH': 'θ', 'V': 'v', 'W': 'w',
    'Y': 'j', 'Z': 'z', 'ZH': 'ʒ',
}

# The consonants a syllable may begin with inside a word: the stress mark
# goes before the longest of these the consonants before its vowel end with.
# Not `sf`: *transfer* is /trænsˈfɝː/. A word's first consonants are its
# first syllable's, whatever they are (*sphere*).
ONSETS = {
    'pl', 'pr', 'pj', 'bl', 'br', 'bj', 'tr', 'tj', 'tw', 'dr', 'dj', 'dw',
    'kl', 'kr', 'kj', 'kw', 'gl', 'gr', 'gj', 'gw', 'fl', 'fr', 'fj', 'θr',
    'θw', 'ʃr', 'sl', 'sw', 'sp', 'st', 'sk', 'sm', 'sn', 'mj', 'nj',
    'vj', 'hj', 'lj', 'spl', 'spr', 'str', 'skr', 'skw', 'spj', 'stj', 'skj',
}

# Long vowels written short where nothing stresses them: *happy*, *influence*.
UNSTRESSED = {'iː': 'i', 'uː': 'u'}


def is_onset(consonants):
    if len(consonants) == 1:
        return consonants[0] != 'ŋ'
    return ''.join(consonants) in ONSETS


def is_vowel(phone):
    return phone[1] is not None


def one_main_stress(phones):
    """A word has one main stress, its last: one a dictionary marks main
    before it is secondary (CMU's *okay* has two)."""
    mains = [i for i, phone in enumerate(phones) if phone[1] == 1]
    return [(symbol, 2 if stress == 1 and i != mains[-1] else stress)
            for i, (symbol, stress) in enumerate(phones)]


def render(phones):
    """Phones as (symbol, stress) — stress None for a consonant, 0 for an
    unstressed vowel, 1 or 2 for a stressed one — as one IPA string."""
    phones = [(UNSTRESSED.get(symbol, symbol) if stress == 0 else symbol, stress) for symbol, stress in one_main_stress(phones)]
    vowels = [i for i, phone in enumerate(phones) if is_vowel(phone)]
    marks = {}
    if len(vowels) > 1:
        previous = -1
        for i in vowels:
            stress = phones[i][1]
            if stress in (1, 2):
                before = [symbol for symbol, _ in phones[previous + 1:i]]
                take = len(before) if previous == -1 else next(
                    (size for size in range(len(before), 0, -1) if is_onset(before[len(before) - size:])), 0)
                marks[i - take] = 'ˈ' if stress == 1 else 'ˌ'
            previous = i
    return ''.join(marks.get(i, '') + symbol for i, (symbol, _) in enumerate(phones))


def from_arpabet(text):
    phones = []
    for token in text.split():
        match = re.fullmatch(r'([A-Z]+)([012]?)', token)
        if not match:
            return None
        base, digit = match.groups()
        if base in ARPABET_CONSONANTS:
            phones.append((ARPABET_CONSONANTS[base], None))
            continue
        stress = int(digit or 0)
        if base == 'AH':
            symbol = 'ʌ' if stress else 'ə'
        elif base == 'ER':
            symbol = 'ɝː' if stress else 'ɚ'
        elif base in ARPABET_VOWELS:
            symbol = ARPABET_VOWELS[base]
        else:
            return None
        phones.append((symbol, stress))
    return phones or None


BRITFONE_SYMBOLS = {'ɹ': 'r', 'ɐ': 'ʌ', 'ɛ': 'e', 'ɛə': 'eə', 'ɡ': 'g'}
BRITFONE_VOWEL = re.compile('[aeiouɑɒɔəɛɜɪʊʌæɐ]')


def from_britfone(text):
    phones = []
    for token in text.split():
        stress = None
        if token[0] in 'ˈˌ':
            stress = 1 if token[0] == 'ˈ' else 2
            token = token[1:]
        symbol = BRITFONE_SYMBOLS.get(token, token)
        if BRITFONE_VOWEL.search(symbol):
            phones.append((symbol, stress or 0))
        else:
            phones.append((symbol, None))
    return phones or None


def variants_of(word):
    """'read(2)' → 'read'."""
    return re.sub(r'\(\d+\)$', '', word)


def strong(phones):
    """A reading with a main stress: not a weak form (*to* as /tə/)."""
    return any(stress == 1 for _, stress in phones)


def main_syllable(phones):
    """Which syllable the main stress is on, counting from the first."""
    stresses = [stress for _, stress in one_main_stress(phones) if stress is not None]
    return stresses.index(1) if 1 in stresses else None


def syllables(phones):
    return sum(1 for phone in phones if is_vowel(phone))


def readings_differ(us, gb):
    """Whether a word's readings say different things, so the table can't
    tell which a line means (review, 2026-10-06): its main stress moves
    between its full readings in either dictionary — *perfect*, *suspect*,
    *record* — or, a word of one syllable, both dictionaries give it full
    readings that differ — *read*, *live*, *close*. Weak forms (*to* as /tə/)
    never count; nor do the many words of more syllables whose readings
    differ only in a vowel, which say the same word either way (*either*)."""
    for side in (us, gb):
        if len({main_syllable(phones) for phones in side if strong(phones)}) > 1:
            return True
    full = [{tuple(phones) for phones in side if strong(phones)} for side in (us, gb)]
    one_syllable = all(syllables(phones) == 1 for side in (us, gb) for phones in side)
    return one_syllable and all(len(readings) > 1 for readings in full)


SIBILANTS = {'s', 'z', 'ʃ', 'ʒ', 'tʃ', 'dʒ'}
VOICELESS = {'p', 't', 'k', 'f', 'θ'}
# A clitic's vowel merges with the long vowel before it: *we're* /wɪə/.
MERGED_RE = {'iː': 'ɪə', 'eɪ': 'eə', 'uː': 'ʊə'}


def with_clitic(base, clitic):
    """A British word with a clitic after it, as Britfone writes the clitics
    on their own ('s z or s, 'd, 'll, 've, 'm, 're): the vowel they have
    after a consonant, none after a vowel; 's as English adds it."""
    last_symbol, last_stress = base[-1]
    after_vowel = last_stress is not None
    if clitic == "'s":
        if last_symbol in SIBILANTS:
            return base + [('ɪ', 0), ('z', None)]
        return base + [('s' if last_symbol in VOICELESS else 'z', None)]
    if clitic == "'m":
        return base + [('m', None)]
    if clitic in ("'d", "'ll", "'ve"):
        consonant = {"'d": 'd', "'ll": 'l', "'ve": 'v'}[clitic]
        vowel = not after_vowel and (clitic != "'d" or last_symbol in {'t', 'd'})
        return base + ([('ə', 0)] if vowel else []) + [(consonant, None)]
    if clitic == "'re":
        if after_vowel and last_symbol in MERGED_RE:
            return base[:-1] + [(MERGED_RE[last_symbol], last_stress)]
        return base + [('ə', 0)]
    return None


def derive_british(us, gb):
    """The contractions and possessives the American dictionary has and
    Britfone lacks, made from the British word: *I'm* from *I*, *it's* from
    *it*, *Tom's* from *Tom*, a plural's *dogs'* as *dogs*. From the word's
    first full reading — *it* as /ɪt/, not its weak form."""
    derived = {}
    for word in us:
        if word in gb or "'" not in word:
            continue
        if word.endswith("s'") and word[:-1] in gb:
            derived[word] = gb[word[:-1]]
            continue
        base, clitic = word.rsplit("'", 1)
        if base not in gb or not clitic:
            continue
        full = [phones for phones in gb[base] if strong(phones)] or gb[base]
        made = with_clitic(full[0], "'" + clitic)
        if made:
            derived[word] = [made]
    return derived


def checked(folder, name):
    path = folder / name
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != SOURCES[name]:
        sys.exit(f'{path}: not the release the table is made from (sha256 {digest})')
    return path.read_text(encoding='utf-8')


def add(readings, word, phones):
    entry = readings.setdefault(word, [])
    if phones not in entry:
        entry.append(phones)


def main(folder):
    folder = Path(folder).expanduser()
    us, gb = {}, {}
    for line in checked(folder, 'cmudict.dict').splitlines():
        line = line.split('#', 1)[0].strip()
        if not line:
            continue
        word, _, text = line.partition(' ')
        phones = from_arpabet(text)
        if phones:
            add(us, variants_of(word).lower(), phones)
    for line in checked(folder, 'britfone.main.3.0.1.csv').splitlines():
        word, _, text = line.partition(',')
        phones = from_britfone(text.strip())
        if phones:
            add(gb, variants_of(word.strip()).lower(), phones)
    derived = derive_british(us, gb)
    gb.update(derived)

    def side(readings):
        sounds = []
        for phones in readings:
            sound = render(phones)
            if sound not in sounds:
                sounds.append(sound)
        return '|'.join(sounds)

    out = Path(__file__).resolve().parents[2] / 'data' / 'pronouncing' / 'pronouncing.tsv.gz'
    words = sorted(set(us) | set(gb))
    differs = 0
    lines = []
    for word in words:
        flag = readings_differ(us.get(word, []), gb.get(word, []))
        differs += flag
        lines.append(f"{word}\t{side(us.get(word, []))}\t{side(gb.get(word, []))}\t{'1' if flag else ''}\n")
    # mtime 0: the same dictionaries always make the same bytes.
    with open(out, 'wb') as raw, gzip.GzipFile(fileobj=raw, mode='wb', mtime=0) as file:
        file.write(''.join(lines).encode('utf-8'))
    print(f'{len(words)} words ({len(us)} American, {len(gb)} British, {len(derived)} of them made), '
          f'{differs} whose readings differ -> {out}')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        sys.exit('usage: build.py <folder holding britfone.main.3.0.1.csv and cmudict.dict>')
    main(sys.argv[1])
