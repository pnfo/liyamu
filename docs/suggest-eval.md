# Suggestion quality (Easy Singlish)

Measured with `liyamu eval 150` (`cargo run --release -p liyamu-cli -- eval 150`) on sentences held out
of training (`data/corpus/heldout.txt`: every 20th sentence of the pitaka.lk books and of OpenSubtitles;
150 of each here). Each word is typed in Singlish letter by letter, a third of the words with doubled long
vowels, and in the second half of the table 30% of the words of four letters or more get one typo (a vowel
left out, two letters swapped, or a neighbouring key).

- **first / in first 3**: the word is the first suggestion (what Space commits) / among the first three,
  once all its letters are typed.
- **after 2 / 3 letters**: the word is among the first three when only its first 2 / 3 letters are typed.
- **keys saved**: keystrokes saved against typing every letter and Space, picking the word as soon as it is
  offered (arrows count); a word never offered counts as typed twice.
- **new words kept**: words the lexicon does not have, typed without typos, that are left as typed
  rather than replaced by a known word.

| typing | engine | first | in first 3 | after 2 letters | after 3 letters | keys saved | new words kept | ms/key |
|---|---|---|---|---|---|---|---|---|
| clean | old (all readings) | 88.4% | 95.6% | 1.6% | 2.5% | -7.2% | 91.4% | 0.15 |
| clean | real words + typos | 88.4% | 98.3% | 9.8% | 24.4% | 12.8% | 86.0% | 0.68 |
| clean | + previous word | 89.7% | 98.4% | 17.4% | 33.9% | 20.7% | 86.0% | 0.90 |
| 1 typo in 30% of words | old (all readings) | 69.0% | 74.7% | 2.0% | 1.5% | -29.3% | 91.0% | 0.15 |
| 1 typo in 30% of words | real words + typos | 76.0% | 92.2% | 9.6% | 20.6% | 6.1% | 85.1% | 0.67 |
| 1 typo in 30% of words | + previous word | 77.1% | 92.7% | 16.0% | 28.8% | 14.8% | 85.1% | 1.02 |

## A small neural model (tried, removed)

A 1.46 M-parameter next-word model (20,000-word vocabulary, one GRU layer, 1.8 MB) trained on the same text
and mixed with the bigram model improved every measure by about half a point (keys saved 20.7% → 21.1%).
Not worth its size and slower first key; the trainer and inference code were removed. The amount of
training text is the limit, not the model.

## Names, places, dictionary words, shorthand and shortcuts

Added after the above: 9,900 words of place and people names, 550 hand-written names, 32,400 dictionary
headwords (lexicon 103,566 → about 146,000 words), shorthand (vowels left out), ඹ ඳ ඟ typed b d g, and the
shortcut table. The held-out text has few names and the simulated typist never types shorthand, so this
table only shows that ordinary typing did not get worse; what was added is covered by the golden rows.

| typing | engine | first | in first 3 | after 2 letters | after 3 letters | keys saved | new words kept | ms/key |
|---|---|---|---|---|---|---|---|---|
| clean | + previous word | 89.9% | 98.9% | 17.2% | 32.9% | 20.6% | 84.6% | 1.28 |
| 1 typo in 30% of words | + previous word | 78.1% | 93.2% | 16.8% | 28.5% | 15.0% | 84.6% | 1.27 |

## Misspellings demoted

3,550 corpus spellings that the dictionary headwords show to be wrong are now counted as the right word
(`data/lexicon/misspellings.tsv`). The held-out text still has those spellings, and the measurement counts
"corrected to the dictionary spelling" as a new word not kept, which is why that column falls.

| typing | engine | first | in first 3 | after 2 letters | after 3 letters | keys saved | new words kept | ms/key |
|---|---|---|---|---|---|---|---|---|
| clean | + previous word | 90.0% | 98.9% | 17.2% | 33.0% | 20.6% | 80.2% | 1.27 |
| 1 typo in 30% of words | + previous word | 78.2% | 93.2% | 16.8% | 28.6% | 14.9% | 79.7% | 1.26 |
