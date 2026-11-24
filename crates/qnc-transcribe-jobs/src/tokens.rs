//! Tokens of a transducer (Parakeet) into words and segments: one rule for every program that
//! hears with one (sherpa-onnx on the processor, ONNX Runtime on the graphics card).

use crate::{Segment, Word};

/// A pause longer than this ends a segment.
const PAUSE_SECONDS: f64 = 1.0;

/// One token as the engine gives it: its text (a leading space begins a word), where it starts
/// and how long it lasts (seconds from the start of the window), its log probability.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub text: String,
    pub start: f32,
    pub duration: f32,
    pub log_prob: Option<f32>,
}

/// The words of `tokens`, their times from the start of the WAV (`offset` seconds before them).
pub fn token_words(tokens: &[Token], offset: f64) -> Vec<Word> {
    let mut words: Vec<(Word, f32, usize)> = Vec::new();
    for token in tokens {
        let start = offset + f64::from(token.start);
        let end = start + f64::from(token.duration.max(0.0));
        let starts_word = token.text.starts_with(' ') || words.is_empty();
        // A letter the model has no token for (capital Đ) is <unk>: left out, not invented.
        let piece = token.text.trim_start().replace("<unk>", "");
        let piece = piece.as_str();
        if starts_word && !(is_punctuation(piece) && !words.is_empty()) {
            words.push((Word { start, end, text: piece.to_string(), p: 1.0 }, 0.0, 0));
        } else if let Some((word, _, _)) = words.last_mut() {
            word.text.push_str(piece);
            word.end = word.end.max(end);
        }
        if let (Some((_, sum, n)), Some(lp)) = (words.last_mut(), token.log_prob) {
            *sum += lp;
            *n += 1;
        }
    }
    words
        .into_iter()
        .map(|(mut word, sum, n)| {
            if n > 0 {
                word.p = (sum / n as f32).exp();
            }
            word
        })
        .filter(|word| !word.text.is_empty())
        .collect()
}

fn is_punctuation(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_punctuation())
}

/// The words in segments: one ends after . ? ! or before a pause of more than a second.
pub fn segments(words: &[Word]) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current: Vec<Word> = Vec::new();
    for (index, word) in words.iter().enumerate() {
        current.push(word.clone());
        let ends_sentence = word.text.ends_with(['.', '?', '!']);
        let pause = words.get(index + 1).is_some_and(|next| next.start - word.end > PAUSE_SECONDS);
        if ends_sentence || pause || index + 1 == words.len() {
            let text = current.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
            segments.push(Segment { start: current[0].start, end: current[current.len() - 1].end, text, words: std::mem::take(&mut current) });
        }
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(text: &str, start: f32) -> Token {
        Token { text: text.into(), start, duration: 0.1, log_prob: Some(0.0) }
    }

    #[test]
    fn tokens_become_words_and_words_sentences() {
        let tokens = [token(" vol", 0.0), token("i", 0.2), token(".", 0.3), token(" I", 0.5), token(" sad", 3.0)];
        let words = token_words(&tokens, 10.0);
        let text: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(text, ["voli.", "I", "sad"]);
        assert_eq!(words[0].start, 10.0);
        assert!((words[0].end - 10.4).abs() < 1e-6);
        let segments = segments(&words);
        let lines: Vec<&str> = segments.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(lines, ["voli.", "I", "sad"], "a sentence ends at the full stop, a pause ends the next");
    }
}
