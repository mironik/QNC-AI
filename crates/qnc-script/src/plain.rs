//! Which messages of the script say how the work goes (done, under way), not that something is
//! wrong: the panels draw those plain and the others red (audit 2026-11-22: every message was
//! red, success too).

/// The messages that say the work goes as asked.
const PLAIN: &[&str] = &[
    "script.broll_suggested",
    "script.broll_accepted",
    "script.broll_suggesting",
    "script.drafting",
    "script.word_cut_done",
    "script.making_clip",
    "script.record_starting",
    "script.recording",
    "script.record_cancelled",
    "script.exporting",
    "script.sending_story",
    "script.sent_story",
    "script.session_sorting",
    "script.session_sorted",
    "script.undone",
    "script.redone",
    "script.words_cut_out",
    "story.msg.slip",
    "story.msg.sound_assist_kept",
];

/// Whether `message` says the work goes as asked (drawn plain); an empty message is plain.
pub fn is_plain(message: &str) -> bool {
    message.is_empty()
        || PLAIN.iter().any(|code| fits(qnc_ui_text::text(code), message))
}

/// Whether `message` is `text` with its values (`{name}`) filled in: every piece of the text
/// between the values is in the message, in order, the first at its start.
fn fits(text: &str, message: &str) -> bool {
    let mut rest = message;
    let mut first = true;
    for piece in text.split('{').enumerate().map(|(index, part)| if index == 0 { part } else { part.split_once('}').map_or(part, |(_, after)| after) }) {
        if piece.is_empty() {
            first = false;
            continue;
        }
        match rest.find(piece) {
            Some(at) if !first || at == 0 => rest = &rest[at + piece.len()..],
            _ => return false,
        }
        first = false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn done_and_under_way_are_plain_a_refusal_is_not() {
        assert!(is_plain(""));
        assert!(is_plain(qnc_ui_text::text("script.undone")));
        assert!(is_plain(&qnc_ui_text::fill("script.sent_story", &[("n", "4")])));
        assert!(!is_plain(qnc_ui_text::text("script.write_first")));
        assert!(!is_plain("disk full"));
    }
}
