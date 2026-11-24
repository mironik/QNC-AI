//! The rough lines (Parakeet's: sentences, capitals, punctuation) corrected word by word by the
//! fine ones (the Croatian model's: better words, lower case, no punctuation) of the same part
//! (user 2026-11-19: "wav2vec bi trebao zadržati Paraketovo poravnanje, razmake... on bi trebao
//! ispraviti, a ne prepisati"; 2026-11-22: "zadržati paraketovu interpunkciju, a wav2vec dorađuje
//! overwrite riječ o riječ").
//!
//! The two lists of words are aligned as a whole, by their text and their time, not each fine
//! word by the time it falls in: by time alone a fine word said a little earlier went to the
//! rough word before it while its own rough word stayed, so the word was there twice ("A bit
//! Bit će", "Pa Pa"). A rough word aligned with a fine one is overwritten by it, written as the
//! rough one is (its capital, its punctuation, its frames); a rough word the fine engine did not
//! hear stays; a fine word no rough word has goes into the line it falls in, in its time. The
//! rough lines and their order stay.

use crate::{Line, Word};

/// A rough word and a fine one further apart than this (in seconds, between their middles) are
/// never the same word.
const FARTHEST_SECONDS: f64 = 1.5;

/// The cost of a word only one engine heard; two such cost more than one word heard differently
/// at the same time, so "jedna," and "1969." said together are one word corrected.
const ALONE: f64 = 0.9;

/// The `rough` lines with their words overwritten by the `fine` ones of the same part. `second`
/// is a second in clip frames (the time two words may be apart and still be one).
pub fn corrected(rough: &[Line], fine: &[Line], second: u64) -> Vec<Line> {
    let mut fine_words: Vec<Word> = fine.iter().flat_map(|line| line.words.iter()).flat_map(split_glued).collect();
    fine_words.sort_by_key(|word| word.from);
    let rough_words: Vec<(usize, usize)> = rough.iter().enumerate().flat_map(|(l, line)| (0..line.words.len()).map(move |w| (l, w))).collect();
    let pairs = align(&rough_words.iter().map(|&(l, w)| &rough[l].words[w]).collect::<Vec<_>>(), &fine_words, second.max(1));
    let mut lines: Vec<Line> = rough.iter().map(|line| Line { from: line.from, to: line.to, text: String::new(), words: line.words.clone() }).collect();
    let mut taken = vec![false; fine_words.len()];
    for (r, f) in pairs {
        let (l, w) = rough_words[r];
        let word = &mut lines[l].words[w];
        word.text = styled(&fine_words[f].text, &word.text);
        taken[f] = true;
    }
    for word in fine_words.iter().zip(&taken).filter(|(_, taken)| !**taken).map(|(word, _)| word) {
        let at = middle(word);
        let place = lines.iter().position(|line| line.from <= at && at < line.to.max(line.from + 1)).or_else(|| (0..lines.len()).min_by_key(|&l| lines[l].from.abs_diff(at)));
        match place {
            Some(l) => {
                let line = &mut lines[l];
                let index = line.words.iter().position(|kept| kept.from > word.from).unwrap_or(line.words.len());
                let mut word = word.clone();
                // Within a sentence the fine engine's capital (it begins each of its lines
                // with one) is not a capital.
                if index > 0 {
                    word.text = lowered_first(&word.text);
                }
                line.words.insert(index, word);
            }
            None => lines.push(Line { from: word.from, to: word.to, text: String::new(), words: vec![word.clone()] }),
        }
    }
    for line in &mut lines {
        line.text = line.words.iter().map(|word| word.text.as_str()).collect::<Vec<_>>().join(" ");
    }
    lines
}

fn middle(word: &Word) -> u64 {
    (word.from + word.to.max(word.from)) / 2
}

/// The pairs (rough index, fine index) of the cheapest alignment of the two lists, in order.
fn align(rough: &[&Word], fine: &[Word], second: u64) -> Vec<(usize, usize)> {
    let (n, m) = (rough.len(), fine.len());
    if n == 0 || m == 0 {
        return Vec::new();
    }
    let cores_r: Vec<Vec<char>> = rough.iter().map(|word| core(&word.text)).collect();
    let cores_f: Vec<Vec<char>> = fine.iter().map(|word| core(&word.text)).collect();
    let pair = |i: usize, j: usize| -> Option<f64> {
        let apart = middle(rough[i]).abs_diff(middle(&fine[j])) as f64 / second as f64;
        (apart <= FARTHEST_SECONDS).then(|| (1.0 - similarity(&cores_r[i], &cores_f[j])) + apart * 0.5)
    };
    // 0 match, 1 rough alone, 2 fine alone.
    let mut came = vec![0u8; (n + 1) * (m + 1)];
    let mut before: Vec<f64> = (0..=m).map(|j| j as f64 * ALONE).collect();
    for j in 1..=m {
        came[j] = 2;
    }
    for i in 1..=n {
        let mut row = vec![0.0; m + 1];
        row[0] = i as f64 * ALONE;
        came[i * (m + 1)] = 1;
        for j in 1..=m {
            let mut best = (before[j] + ALONE, 1u8);
            if row[j - 1] + ALONE < best.0 {
                best = (row[j - 1] + ALONE, 2);
            }
            if let Some(cost) = pair(i - 1, j - 1) {
                if before[j - 1] + cost < best.0 {
                    best = (before[j - 1] + cost, 0);
                }
            }
            row[j] = best.0;
            came[i * (m + 1) + j] = best.1;
        }
        before = row;
    }
    let (mut i, mut j, mut pairs) = (n, m, Vec::new());
    while i > 0 || j > 0 {
        match came[i * (m + 1) + j] {
            0 => {
                pairs.push((i - 1, j - 1));
                i -= 1;
                j -= 1;
            }
            1 => i -= 1,
            _ => j -= 1,
        }
    }
    pairs.reverse();
    pairs
}

/// A word's letters and digits, lower case.
fn core(text: &str) -> Vec<char> {
    text.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// How alike two words are, 0 to 1 (edit distance over the longer).
fn similarity(a: &[char], b: &[char]) -> f64 {
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 1.0;
    }
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let up = row[j + 1];
            row[j + 1] = (up + 1).min(row[j] + 1).min(diagonal + usize::from(ca != cb));
            diagonal = up;
        }
    }
    1.0 - row[b.len()] as f64 / longest as f64
}

/// A fine word whose letters run into digits with no space ("tih120", the model writes numbers
/// so) as two words over its frames, by their letters.
fn split_glued(word: &Word) -> Vec<Word> {
    let chars: Vec<char> = word.text.chars().collect();
    let Some(at) = (1..chars.len()).find(|&i| chars[i - 1].is_alphabetic() && chars[i].is_ascii_digit()) else { return vec![word.clone()] };
    let (left, right): (String, String) = (chars[..at].iter().collect(), chars[at..].iter().collect());
    let span = word.to.saturating_sub(word.from);
    let cut = word.from + span * at as u64 / chars.len() as u64;
    let mut out = vec![Word { from: word.from, to: cut, text: left }];
    out.extend(split_glued(&Word { from: cut, to: word.to, text: right }));
    out
}

fn lowered_first(text: &str) -> String {
    let mut letters = text.chars();
    letters.next().map(|first| first.to_lowercase().chain(letters).collect()).unwrap_or_default()
}

/// `said` (the fine word) written as `rough` is written: its leading and trailing punctuation and
/// its capital; the same word in another case keeps the rough one as it is (a name, an acronym).
fn styled(said: &str, rough: &str) -> String {
    let core_start = rough.find(char::is_alphanumeric).unwrap_or(rough.len());
    let core_end = rough.rfind(char::is_alphanumeric).map_or(core_start, |at| at + rough[at..].chars().next().map_or(1, char::len_utf8));
    let (before, core, after) = (&rough[..core_start], &rough[core_start..core_end], &rough[core_end..]);
    if core.to_lowercase() == said.to_lowercase() {
        return rough.to_string();
    }
    let mut said = said.trim_end_matches(|c: char| after.contains(c) && !c.is_alphanumeric()).to_string();
    // The capital is the rough one's: the fine engine begins each of its lines with one.
    let mut letters = said.chars();
    let first = letters.next();
    said = match first {
        Some(first) if core.chars().next().is_some_and(char::is_uppercase) => first.to_uppercase().chain(letters).collect(),
        Some(first) => first.to_lowercase().chain(letters).collect(),
        None => String::new(),
    };
    format!("{before}{said}{after}")
}
