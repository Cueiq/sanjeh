# What a learner's first language makes alike

`alike` compares two words by their sounds, a sound changed costing by how
far apart the two sounds are. A learner's first language makes some changes
likelier than the general costs say: a sound their language lacks is heard
as the nearest one it has. Each file here is one language, by its ISO 639-1
code, and its costs replace the general ones where they are lower.

## Format

Tab-separated, `#` for comments, one change a row:

| Column | |
|---|---|
| `kind` | `swap`: one sound heard for another, either way round. `cluster-end`: the third of three consonants at the end of a word dropped (*asked* heard as *ask*). |
| `a`, `b` | The two sounds, as `pronounce` writes them (`-` for `cluster-end`). |
| `cost` | What the change costs, from 0 (no difference) to 1. The general costs are about 0.25 for a near consonant and 0.45 for a consonant dropped at a word's end. |
| `evidence` | What the row rests on: `inventory` (the language lacks the sound or the contrast), `perception` (learners were measured not telling the two apart), `syllables` (the language's syllables don't allow it). |
| `source` | The key of a source below. |

The costs are a first guess from the evidence, not measured. A benchmark of
learners' real mishearings is the way to tune them; a row that moves a
benchmark's results the wrong way should go.

## Persian (`fa.tsv`)

- Persian has no /w/, /θ/ or /ð/, and /ŋ/ only before /k/ and /g/; its six
  vowels have no long and short pair (`inventory`).
- Iranian learners heard /iː/ and /ɪ/, and /uː/ and /ʊ/, as one vowel, and
  heard /ʌ/ with the low back vowels /ɑ/ and /ɔ/ (`perception`).
- Persian syllables end in at most two consonants, and learners find longer
  English clusters hard (`syllables`).

## Sources

- **majidi-ternes-1991**: Mohammad-Reza Majidi and Elmar Ternes, "Persian
  (Farsi)", Illustrations of the IPA, *Journal of the International Phonetic
  Association* 21(2), 96–98, 1991; also the *Handbook of the International
  Phonetic Association*, Cambridge University Press, 1999, 124–125.
- **afshar-2022**: Naeimeh Afshar, doctoral dissertation on the perception of
  American English vowels by Iranian learners of English, University of
  Pannonia, 2022.
  <https://library.uni-pannon.hu/doktori/2022/Naeimeh_Afshar_dissertation.pdf>
- **hall-thesis**: Mahnaz Hall, *Phonological characteristics of Farsi speakers
  of English and L1 Australian English speakers' perceptions of proficiency*,
  master's thesis, Curtin University.
  <https://asian-efl-journal.com/thesis/phonological-characteristics-of-farsi-speakers-of-english-and-l1-australian-english-speakers-perceptions-of-proficiency/index.htm>
