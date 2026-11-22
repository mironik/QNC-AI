//! The transcript shown by sentences (user 2026-11-15: "treba formatirati po rečenicama s
//! razmakom između rečenica"; 2026-11-22: "nova rečenica, novi red"): the words of the lines run
//! on and a sentence ends at a word that ends with `.`, `?`, `!` or `…`, at a pause in speech
//! (the Croatian model writes no stops, so its sentences end where the speaker stops), or where a
//! line of the engine ends and the next begins with a capital letter. Each sentence is one
//! paragraph. A line without words stays as it is. The words keep their frames, so marks, finds
//! and the cursor are the same.

use crate::{Line, Word};

/// A pause between two words this long ends a sentence, in seconds.
pub const SENTENCE_PAUSE_SECONDS: f64 = 0.7;

/// The lines of the transcript regrouped into its sentences, in order. `pause` is a pause
/// between two words, in frames, that ends a sentence (`None`: only stops and lines end one).
pub fn sentences(lines: &[Line], pause: Option<u64>) -> Vec<Line> {
    let mut out = Vec::new();
    let mut words: Vec<Word> = Vec::new();
    let close = |words: &mut Vec<Word>, out: &mut Vec<Line>| {
        if let (Some(first), Some(last)) = (words.first(), words.last()) {
            let text = words.iter().map(|word| word.text.as_str()).collect::<Vec<_>>().join(" ");
            out.push(Line { from: first.from, to: last.to, text, words: std::mem::take(words) });
        }
    };
    let ends = |word: &Word| word.text.trim_end_matches(['"', '\'', ')', '»', '”']).ends_with(['.', '?', '!', '…']);
    let capital = |word: &Word| word.text.trim_start_matches(['"', '\'', '(', '«', '„', '“']).chars().next().is_some_and(char::is_uppercase);
    for line in lines {
        if line.words.is_empty() {
            close(&mut words, &mut out);
            out.push(line.clone());
            continue;
        }
        // A new line of the engine that begins with a capital letter is a new sentence.
        if line.words.first().is_some_and(capital) {
            close(&mut words, &mut out);
        }
        for word in &line.words {
            let paused = words.last().is_some_and(|last| pause.is_some_and(|pause| word.from.saturating_sub(last.to) >= pause));
            if paused {
                close(&mut words, &mut out);
            }
            words.push(word.clone());
            if ends(word) {
                close(&mut words, &mut out);
            }
        }
    }
    close(&mut words, &mut out);
    out
}

/// The pause that ends a sentence, in frames of a clip of `fps`.
pub fn sentence_pause(fps: u32) -> Option<u64> {
    (fps > 0).then(|| (f64::from(fps) * SENTENCE_PAUSE_SECONDS).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(words: &[(&str, u64)]) -> Line {
        let words: Vec<Word> = words.iter().map(|&(text, from)| Word { from, to: from + 5, text: text.into() }).collect();
        Line { from: words[0].from, to: words[words.len() - 1].to, text: String::new(), words }
    }

    #[test]
    fn a_sentence_ends_at_its_stop_whatever_the_lines() {
        let lines = [line(&[("Dobar", 0), ("dan.", 5), ("Ovo", 10)]), line(&[("je", 15), ("test?", 20), ("Da", 25)])];
        let shown = sentences(&lines, None);
        let texts: Vec<&str> = shown.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(texts, ["Dobar dan.", "Ovo je test?", "Da"]);
        assert_eq!((shown[1].from, shown[1].to), (10, 25));
    }

    #[test]
    fn without_stops_a_pause_or_a_new_line_with_a_capital_ends_a_sentence() {
        // The Croatian model: no stops; its lines begin with a capital letter.
        let lines = [line(&[("Za", 0), ("sada", 5), ("dominacija", 10)]), line(&[("Ajmo", 100), ("malo", 105)]), line(&[("je", 200), ("uvijek", 205), ("lijepo", 260), ("doći", 265)])];
        let shown = sentences(&lines, sentence_pause(50));
        let texts: Vec<&str> = shown.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(texts, ["Za sada dominacija", "Ajmo malo", "je uvijek", "lijepo doći"]);
        // A short break between words does not.
        let shown = sentences(&[line(&[("dobro", 0), ("je", 12)])], sentence_pause(50));
        assert_eq!(shown.len(), 1);
    }
}
