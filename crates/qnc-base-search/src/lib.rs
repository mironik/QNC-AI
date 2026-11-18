//! The archive of card bases searched (user 2026-11-15: "velike kuće ne brišu baze, one ostaju
//! arhiva"; "to bi moglo služiti i za brzo pronalaženje kadrova u arhivi"; "pretragu bi dodao kao
//! samostalnu kocku koja bi se pozivala u obliku overlay prozora"). A base is found when every
//! word asked is in its name, place, description, analysis (the shot analysis' terms and
//! captions) or its card's label, whatever the case and the Croatian marks. The bases are read
//! once per search, in a thread of their own; nothing is written.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

use qnc_ingest_base::IngestBase;

/// A base found: the base and the fields its words were found in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub base: IngestBase,
    /// The lines of its analysis that say a word asked (the shots it holds of it).
    pub lines: Vec<String>,
}

/// The bases of `bases` that say every word of `query`, in their order. An empty query finds all.
pub fn find(bases: &[IngestBase], query: &str) -> Vec<Hit> {
    let words: Vec<String> = query.split_whitespace().map(plain).filter(|word| !word.is_empty()).collect();
    bases
        .iter()
        .filter_map(|base| {
            let fields = [base.title.as_str(), base.location.as_str(), base.description.as_str(), base.analysis.as_str(), base.label()];
            let said = plain(&fields.join(" \n "));
            if !words.iter().all(|word| said.contains(word.as_str())) {
                return None;
            }
            let lines = base
                .analysis
                .split([',', ';', '\n'])
                .map(str::trim)
                .filter(|piece| !words.is_empty() && words.iter().any(|word| plain(piece).contains(word.as_str())))
                .map(str::to_string)
                .collect();
            Some(Hit { base: base.clone(), lines })
        })
        .collect()
}

/// Lower case without the Croatian marks, so "cavtat" finds "Cavtat" and "luka" finds "Luka",
/// "zupa" finds "Župa".
fn plain(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'č' | 'ć' => 'c',
            'đ' => 'd',
            'š' => 's',
            'ž' => 'z',
            other => other,
        })
        .collect()
}

/// A search running off the caller's thread: the bases are read anew for each.
#[derive(Default)]
pub struct Search {
    job: Option<Receiver<Result<Vec<Hit>, String>>>,
}

impl Search {
    /// Searches the bases of `root` for `query`; an earlier search still running is dropped.
    pub fn start(&mut self, root: &Path, query: &str) {
        let (root, query): (PathBuf, String) = (root.to_path_buf(), query.to_string());
        let (send, receive) = channel();
        std::thread::spawn(move || {
            let _ = send.send(qnc_ingest_base::all(&root).map(|bases| find(&bases, &query)));
        });
        self.job = Some(receive);
    }

    /// The hits once the search is done.
    pub fn poll(&mut self) -> Option<Result<Vec<Hit>, String>> {
        let found = self.job.as_ref()?.try_recv().ok()?;
        self.job = None;
        Some(found)
    }

    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(title: &str, location: &str, analysis: &str) -> IngestBase {
        IngestBase {
            id: title.into(),
            name: format!("Ingest {title}"),
            uri: "u".into(),
            project_name: None,
            active: false,
            title: title.into(),
            location: location.into(),
            description: String::new(),
            analysis: analysis.into(),
            operator: String::new(),
        }
    }

    #[test]
    fn a_base_is_found_by_every_word_in_its_fields_whatever_the_case_and_marks() {
        let bases = [base("Konavle", "Cavtat", "U kadrovima: Luka, Brod\nKadrovi: A boat in a harbour"), base("Župa", "Mlini", "U kadrovima: Trg")];
        let hits = find(&bases, "cavtat brod");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].base.title, "Konavle");
        assert_eq!(hits[0].lines, ["Brod"], "the analysis' piece that says the word");
        assert_eq!(find(&bases, "zupa")[0].base.title, "Župa", "without the marks");
        assert_eq!(find(&bases, "").len(), 2, "nothing asked finds all");
        assert!(find(&bases, "stadion").is_empty());
    }
}
