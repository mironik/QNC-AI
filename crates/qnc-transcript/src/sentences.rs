//! The transcript shown by sentences (user 2026-11-15: "treba formatirati po rečenicama s
//! razmakom između rečenica"): the words of the lines run on and a sentence ends at a word that
//! ends with `.`, `?`, `!` or `…`; each sentence is one paragraph. A line without words stays as
//! it is. The words keep their frames, so marks, finds and the cursor are the same.

use crate::{Line, Word};

/// The lines of the transcript regrouped into its sentences, in order.
pub fn sentences(lines: &[Line]) -> Vec<Line> {
    let mut out = Vec::new();
    let mut words: Vec<Word> = Vec::new();
    let close = |words: &mut Vec<Word>, out: &mut Vec<Line>| {
        if let (Some(first), Some(last)) = (words.first(), words.last()) {
            let text = words.iter().map(|word| word.text.as_str()).collect::<Vec<_>>().join(" ");
            out.push(Line { from: first.from, to: last.to, text, words: std::mem::take(words) });
        }
    };
    for line in lines {
        if line.words.is_empty() {
            close(&mut words, &mut out);
            out.push(line.clone());
            continue;
        }
        for word in &line.words {
            words.push(word.clone());
            if word.text.trim_end_matches(['"', '\'', ')', '»', '”']).ends_with(['.', '?', '!', '…']) {
                close(&mut words, &mut out);
            }
        }
    }
    close(&mut words, &mut out);
    out
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
        let shown = sentences(&lines);
        let texts: Vec<&str> = shown.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(texts, ["Dobar dan.", "Ovo je test?", "Da"]);
        assert_eq!((shown[1].from, shown[1].to), (10, 25));
    }
}
