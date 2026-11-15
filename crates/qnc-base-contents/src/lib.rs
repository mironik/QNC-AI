//! What a card base holds, from its shot analysis (user 2026-11-15: "naziv i opis je pomoć
//! korisniku da zna što je u njoj, jer velike kuće ne brišu baze, one ostaju arhiva; u opis dodaj
//! podatke koje kreira alat za analizu kadrova, bez ponavljanja istih pojmova i opisa"; "to bi
//! moglo služiti i za brzo pronalaženje kadrova u arhivi").
//!
//! The text has two lines: the vocabulary's terms of every looked-at shot (the same two best terms
//! the AI B-roll cards show), each once, the most frequent first; and the captions of the shots
//! (Florence-2's first sentence), each once. It is made from what the analysis wrote and is written
//! beside the user's own description (`qnc-ingest-base` `set_analysis`); nothing is decided here.

use std::collections::HashMap;
use std::path::Path;

use qnc_db_broker::ProjectDbTarget;

/// At most this many terms and captions in the text: enough to know and to find a base by.
pub const MOST_TERMS: usize = 60;
pub const MOST_CAPTIONS: usize = 40;

/// The text of what the base behind `target` holds (its `project_uri` keys the captions of this
/// computer). Empty when nothing was looked at yet.
pub fn describe(root: &Path, target: &ProjectDbTarget, project_uri: &str) -> Result<String, String> {
    let model = qnc_ai_catalog::installed_shot_content()?.model_name();
    let content = qnc_shot_content_db::read_all(target, &model)?;
    let mut terms: Vec<String> = Vec::new();
    if let (Some(set), Ok(list)) = (qnc_shot_vocabulary::read(root, &model)?, qnc_shot_vocabulary::installed()) {
        for part in content.iter().flat_map(|clip| &clip.parts) {
            terms.extend(qnc_shot_vocabulary::best_terms(&set, &list, &part.vector, 2).into_iter().filter_map(|(code, _)| list.text_of(&code).map(str::to_string)));
        }
    }
    let mut captions = Vec::new();
    if let Ok(captioner) = qnc_ai_catalog::installed_shot_captioner() {
        let captioner = captioner.model_name();
        for clip in content.iter().filter(|clip| !clip.parts.is_empty()) {
            captions.extend(qnc_shot_captions::read_clip(root, &captioner, project_uri, &clip.clip_id)?.into_iter().map(|caption| caption.text));
        }
    }
    Ok(text(&terms, &captions))
}

/// The two lines from the terms and captions as they came, each once.
pub fn text(terms: &[String], captions: &[String]) -> String {
    let terms = most_frequent(terms, MOST_TERMS);
    let captions = once(captions.iter().map(|caption| first_sentence(caption)), MOST_CAPTIONS);
    let mut lines = Vec::new();
    if !terms.is_empty() {
        lines.push(format!("{}: {}", qnc_wizard_text::text("base.analysis_terms"), terms.join(", ")));
    }
    if !captions.is_empty() {
        lines.push(format!("{}: {}", qnc_wizard_text::text("base.analysis_captions"), captions.join("; ")));
    }
    lines.join("\n")
}

/// Each term once, the most frequent first (equal ones in the order they first came).
fn most_frequent(terms: &[String], most: usize) -> Vec<String> {
    let mut counted: Vec<(String, usize, usize)> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    for term in terms.iter().map(|term| term.trim()).filter(|term| !term.is_empty()) {
        let key = term.to_lowercase();
        match at.get(&key) {
            Some(&index) => counted[index].1 += 1,
            None => {
                at.insert(key, counted.len());
                counted.push((term.to_string(), 1, counted.len()));
            }
        }
    }
    counted.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));
    counted.into_iter().take(most).map(|(term, _, _)| term).collect()
}

/// Each text once (whatever its case and closing stop), in the order they came.
fn once(texts: impl Iterator<Item = String>, most: usize) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    texts
        .map(|text| text.trim().trim_end_matches('.').trim().to_string())
        .filter(|text| !text.is_empty() && seen.insert(text.to_lowercase()))
        .take(most)
        .collect()
}

fn first_sentence(caption: &str) -> String {
    caption.split_terminator(['.', '\n']).next().unwrap_or(caption).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_term_and_caption_is_said_once_the_most_frequent_terms_first() {
        let terms: Vec<String> = ["Luka", "Brod", "luka", "Trg", "Brod", "Luka"].map(String::from).to_vec();
        let captions: Vec<String> = ["A boat in a harbour. Two men.", "a boat in a harbour", "A man speaks into a microphone."].map(String::from).to_vec();
        let lines: Vec<String> = text(&terms, &captions).lines().map(str::to_string).collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with(": Luka, Brod, Trg"), "{}", lines[0]);
        assert!(lines[1].ends_with(": A boat in a harbour; A man speaks into a microphone"), "{}", lines[1]);
        assert_eq!(text(&[], &[]), "", "nothing looked at yet");
    }
}
