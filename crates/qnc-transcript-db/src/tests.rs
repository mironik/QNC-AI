use super::*;
use rusqlite::Connection;

const PROJECT_DB: &str = "qnc://local/db/project_db/p1";

fn project() -> (tempfile::TempDir, ProjectDbTarget) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("project.db");
    Connection::open(&file)
        .unwrap()
        .execute_batch(
            "CREATE TABLE project_settings (project_id TEXT);
             INSERT INTO project_settings (project_id) VALUES ('p1');
             CREATE VIEW public_project_settings AS SELECT project_id FROM project_settings;",
        )
        .unwrap();
    (dir, ProjectDbTarget::from_owner_binding(&file, PROJECT_DB).unwrap())
}

fn transcript(from: u64, to: u64, said: &str) -> ClipTranscript {
    let word = Word { from, to: from + 10, text: said.into() };
    ClipTranscript {
        clip_id: "c1".into(),
        from_frame: from,
        to_frame: to,
        language: "hr".into(),
        engine: "qnc.transcribe.whisper@0.1.0 ggml-large-v3-turbo.bin".into(),
        lines: vec![Line { from, to: from + 10, text: said.into(), words: vec![word] }],
        fine: false,
    }
}

#[test]
fn a_kept_transcript_is_read_back_and_a_range_said_again_replaces_it() {
    let (_dir, target) = project();
    assert!(read_clip(&target, "c1").unwrap().is_empty(), "a project without transcripts reads none");
    save(&target, transcript(100, 200, "Dobar")).unwrap();
    save(&target, transcript(0, 50, "Vidite")).unwrap();
    save(&target, transcript(100, 200, "Dobar dan")).unwrap();
    let kept = read_clip(&target, "c1").unwrap();
    assert_eq!(kept.iter().map(|t| t.from_frame).collect::<Vec<_>>(), [0, 100], "in the order of their ranges");
    assert_eq!(kept[1].lines[0].text, "Dobar dan");
    assert!(read_clip(&target, "c2").unwrap().is_empty());
}

#[test]
fn a_clip_forgotten_loses_its_transcripts_and_its_queue_entry_but_not_while_it_runs() {
    let (_dir, target) = project();
    save(&target, transcript(0, 100, "a")).unwrap();
    save(&target, transcript(200, 300, "b")).unwrap();
    let entry = Queued { clip_id: "c1".into(), from_frame: 0, to_frame: 100, quality: QueueQuality::Fast, state: QueueState::Waiting, error: String::new(), took_ms: 0 };
    enqueue(&target, vec![entry.clone()]).unwrap();
    set_queued(&target, &entry, QueueState::Running, "", 0).unwrap();
    assert!(forget_clip(&target, "c1").is_err(), "it is being transcribed");
    assert_eq!(read_clip(&target, "c1").unwrap().len(), 2);
    set_queued(&target, &entry, QueueState::Done, "", 0).unwrap();
    forget_clip(&target, "c1").unwrap();
    assert!(read_clip(&target, "c1").unwrap().is_empty());
    assert!(read_queue(&target).unwrap().is_empty());
    assert!(read_transcribed(&target).unwrap().is_empty());
}

#[test]
fn a_word_outside_its_range_is_refused_and_nothing_is_kept() {
    let (_dir, target) = project();
    let mut outside = transcript(100, 200, "Dobar");
    outside.lines[0].words[0].to = 250;
    assert!(save(&target, outside).is_err());
    assert!(read_clip(&target, "c1").unwrap().is_empty());
}

fn asked(clip_id: &str, quality: QueueQuality) -> Queued {
    Queued { clip_id: clip_id.into(), from_frame: 0, to_frame: 500, quality, state: QueueState::Waiting, error: String::new(), took_ms: 0 }
}

#[test]
fn queued_clips_are_read_in_order_and_asking_a_clip_again_waits_it_again() {
    let (_dir, target) = project();
    assert!(read_queue(&target).unwrap().is_empty(), "a project without a queue reads none");
    enqueue(&target, vec![asked("a", QueueQuality::Fast), asked("b", QueueQuality::Best)]).unwrap();
    set_queued(&target, &asked("a", QueueQuality::Fast), QueueState::Done, "", 4200).unwrap();
    set_queued(&target, &asked("b", QueueQuality::Best), QueueState::Failed, "no sound", 0).unwrap();
    let queue = read_queue(&target).unwrap();
    assert_eq!(queue[0].took_ms, 4200, "how long a done clip took is kept");
    assert_eq!(queue.iter().map(|q| (q.clip_id.as_str(), q.state, q.quality)).collect::<Vec<_>>(), [("a", QueueState::Done, QueueQuality::Fast), ("b", QueueState::Failed, QueueQuality::Best)]);
    assert_eq!(queue[1].error, "no sound");
    enqueue(&target, vec![asked("b", QueueQuality::Fast)]).unwrap();
    let rows: Vec<_> = read_queue(&target).unwrap().into_iter().filter(|q| q.clip_id == "b").map(|q| (q.state, q.quality)).collect();
    assert_eq!(rows, [(QueueState::Failed, QueueQuality::Best), (QueueState::Waiting, QueueQuality::Fast)], "a request of another engine is a row of its own");
    enqueue(&target, vec![asked("b", QueueQuality::Best)]).unwrap();
    let again = read_queue(&target).unwrap().into_iter().find(|q| q.clip_id == "b" && q.quality == QueueQuality::Best).unwrap();
    assert_eq!((again.state, again.error.as_str()), (QueueState::Waiting, ""), "the same request asked again waits again");
}

#[test]
fn every_piece_of_a_long_clip_is_a_row_of_its_own() {
    let (_dir, target) = project();
    let piece = |from: u64, to: u64| Queued { from_frame: from, to_frame: to, ..asked("long", QueueQuality::Fast) };
    enqueue(&target, vec![piece(0, 3000), piece(3000, 6000), piece(6000, 9000)]).unwrap();
    assert_eq!(read_queue(&target).unwrap().len(), 3, "audit 2026-11-19: only the last piece was kept");
}

/// A statement transcribed again (live 2026-11-08, Mironik 3609 992..1072): the old line
/// 992..1168 ran past it and stayed whole, its words there twice. Now only its words outside
/// the new range stay.
#[test]
fn a_newer_transcript_takes_the_words_of_an_old_line_that_runs_past_it() {
    let (_dir, target) = project();
    let word = |from: u64, to: u64, text: &str| Word { from, to, text: text.into() };
    let mut old = transcript(0, 2000, "");
    old.lines = vec![Line {
        from: 992,
        to: 1168,
        text: "Ovo je prije brkova, a onda".into(),
        words: vec![word(992, 1000, "Ovo"), word(1001, 1010, "je"), word(1011, 1040, "prije"), word(1041, 1070, "brkova,"), word(1100, 1120, "a"), word(1130, 1168, "onda")],
    }];
    save(&target, old).unwrap();
    let mut new = transcript(992, 1072, "");
    new.lines = vec![Line { from: 993, to: 1070, text: "Ovo je prije brkova".into(), words: vec![word(993, 1070, "Ovo je prije brkova")] }];
    save(&target, new).unwrap();
    let kept = read_clip(&target, "c1").unwrap();
    let old = kept.iter().find(|t| t.from_frame == 0).unwrap();
    assert_eq!(old.lines.iter().map(|l| (l.from, l.to, l.text.as_str())).collect::<Vec<_>>(), [(1100, 1168, "a onda")], "only the old words after the statement stay");
    // A row kept before that rule (its old line whole again) is read right all the same.
    let conn = rusqlite::Connection::open(_dir.path().join("project.db")).unwrap();
    let whole = serde_json::to_string(&vec![Line {
        from: 992,
        to: 1168,
        text: "Ovo je prije brkova, a onda".into(),
        words: vec![word(992, 1000, "Ovo"), word(1001, 1010, "je"), word(1011, 1040, "prije"), word(1041, 1070, "brkova,"), word(1100, 1120, "a"), word(1130, 1168, "onda")],
    }])
    .unwrap();
    conn.execute("UPDATE clip_transcripts SET lines_json = ?1 WHERE from_frame = 0", [whole]).unwrap();
    drop(conn);
    let read = read_clip(&target, "c1").unwrap();
    let mut words: Vec<(u64, String)> = read.iter().flat_map(|t| &t.lines).flat_map(|l| &l.words).map(|w| (w.from, w.text.clone())).collect();
    words.sort();
    assert_eq!(words.into_iter().map(|(_, text)| text).collect::<Vec<_>>(), ["Ovo je prije brkova", "a", "onda"], "no word twice");
}

#[test]
fn a_clip_asked_again_while_it_runs_is_not_marked_done_by_the_old_run() {
    let (_dir, target) = project();
    let first = asked("a", QueueQuality::Fast);
    enqueue(&target, vec![first.clone()]).unwrap();
    set_queued(&target, &first, QueueState::Running, "", 0).unwrap();
    // Asked again (another range, the Croatian pass) while the first one runs.
    let again = Queued { from_frame: 100, to_frame: 200, quality: QueueQuality::Best, ..first.clone() };
    enqueue(&target, vec![again.clone()]).unwrap();
    set_queued(&target, &first, QueueState::Done, "", 10).unwrap();
    let queue = read_queue(&target).unwrap();
    let state = |quality| queue.iter().find(|q| q.quality == quality).map(|q| (q.state, q.from_frame));
    assert_eq!(state(QueueQuality::Fast), Some((QueueState::Done, 0)), "the first run marks its own request");
    assert_eq!(state(QueueQuality::Best), Some((QueueState::Waiting, 100)), "the new request still waits");
    set_queued(&target, &again, QueueState::Done, "", 10).unwrap();
    assert!(read_queue(&target).unwrap().iter().all(|q| q.state == QueueState::Done));
}

#[test]
fn transcribed_clips_are_listed_and_done_entries_leave_the_queue_failed_ones_stay() {
    let (_dir, target) = project();
    assert!(read_transcribed(&target).unwrap().is_empty(), "a project without transcripts lists none");
    save(&target, transcript(0, 50, "Vidite")).unwrap();
    save(&target, transcript(100, 200, "Dobar")).unwrap();
    assert_eq!(read_transcribed(&target).unwrap(), ["c1"], "a clip once, however many ranges");
    // A clip without speech is transcribed (nothing heard) but has no words for the tab.
    save(&target, ClipTranscript { clip_id: "quiet".into(), lines: Vec::new(), ..transcript(0, 300, "") }).unwrap();
    assert_eq!(read_transcribed(&target).unwrap(), ["c1", "quiet"]);
    assert_eq!(read_spoken(&target).unwrap(), ["c1"], "only a transcript with words");
    enqueue(&target, vec![asked("a", QueueQuality::Fast), asked("b", QueueQuality::Fast)]).unwrap();
    set_queued(&target, &asked("a", QueueQuality::Fast), QueueState::Done, "", 100).unwrap();
    set_queued(&target, &asked("b", QueueQuality::Fast), QueueState::Failed, "no sound", 0).unwrap();
    clear_done(&target).unwrap();
    assert_eq!(read_queue(&target).unwrap().iter().map(|q| q.clip_id.as_str()).collect::<Vec<_>>(), ["b"]);
}

#[test]
fn a_newer_transcript_is_what_was_said_over_its_range() {
    let (_dir, target) = project();
    let mut whole = transcript(0, 1000, "staro");
    whole.lines = vec![
        Line { from: 100, to: 200, text: "prije".into(), words: Vec::new() },
        Line { from: 400, to: 500, text: "unutra".into(), words: Vec::new() },
        Line { from: 800, to: 900, text: "poslije".into(), words: Vec::new() },
    ];
    save(&target, whole).unwrap();
    save(&target, transcript(300, 600, "novo")).unwrap();
    let kept = read_clip(&target, "c1").unwrap();
    let texts: Vec<&str> = kept.iter().flat_map(|t| t.lines.iter().map(|l| l.text.as_str())).collect();
    assert_eq!(texts, ["prije", "poslije", "novo"], "the older line inside the new range goes");
    save(&target, transcript(0, 1000, "sve")).unwrap();
    let kept = read_clip(&target, "c1").unwrap();
    assert_eq!(kept.len(), 1, "a transcript inside the newer one's range goes");
}

#[test]
fn statements_are_kept_in_the_order_added_and_one_can_go() {
    let (_dir, target) = project();
    assert!(read_quotes(&target).unwrap().is_empty(), "a project without statements reads none");
    let quote = |from: u64, text: &str| Quote {
        id: 0,
        clip_id: "c1".into(),
        clip_name: "Mironik 2680.MXF".into(),
        from_frame: from,
        to_frame: from + 100,
        start_tc: "00:21:56:17".into(),
        end_tc: "00:21:58:17".into(),
        length: "00:00:02:00".into(),
        text: text.into(),
    };
    add_quote(&target, quote(500, "Pa dosad njih")).unwrap();
    add_quote(&target, quote(100, "igrača je")).unwrap();
    assert!(add_quote(&target, quote(200, " ")).is_err(), "a statement without text");
    let kept = read_quotes(&target).unwrap();
    assert_eq!(kept.iter().map(|q| q.text.as_str()).collect::<Vec<_>>(), ["Pa dosad njih", "igrača je"]);
    delete_quote(&target, kept[0].id).unwrap();
    assert_eq!(read_quotes(&target).unwrap().iter().map(|q| q.from_frame).collect::<Vec<_>>(), [100]);
}

#[test]
fn a_word_cut_into_a_range_ends_at_its_edges() {
    let word = |from: u64, to: u64, text: &str| Word { from, to, text: text.into() };
    // "Pa" before the range, "dosad" across its start (middle inside), "njih" across its end.
    let line = Line { from: 220, to: 340, text: "Pa dosad njih".into(), words: vec![word(220, 230, "Pa"), word(232, 250, "dosad"), word(314, 336, "njih")] };
    let cut = crate::lines_cut(&[line], 236, 328, true);
    assert_eq!(cut.len(), 1);
    assert_eq!((cut[0].from, cut[0].to), (236, 328));
    assert_eq!(cut[0].words.iter().map(|w| (w.from, w.to)).collect::<Vec<_>>(), vec![(236, 250), (314, 328)]);
}

#[test]
fn the_fine_pass_hears_what_only_the_rough_one_heard() {
    let span = |clip: &str, from: u64, to: u64, engine: &str| Span { clip_id: clip.into(), from_frame: from, to_frame: to, engine: engine.into() };
    let spans = [
        span("a", 0, 1000, "parakeet m"),
        span("a", 200, 300, "wav2vec2 parlaspeech"),
        span("a", 290, 600, "wav2vec2 parlaspeech"),
        span("b", 0, 500, "wav2vec2 parlaspeech"),
        span("c", 0, 30, "parakeet m"),
    ];
    assert_eq!(crate::not_heard_by(&spans, "parlaspeech", 25), vec![("a".into(), 0, 200), ("a".into(), 600, 1000), ("c".into(), 0, 30)]);
}

#[test]
fn a_fast_transcript_made_later_never_says_over_a_fine_one() {
    let (_dir, target) = project();
    // Heard while the clip played, by the fine engine: frames 100..200.
    save(&target, ClipTranscript { fine: true, ..transcript(100, 200, "fino") }).unwrap();
    // The fast engine later hears the whole clip; it says other words over the same frames.
    let mut whole = transcript(0, 300, "brzo");
    whole.lines.push(Line { from: 150, to: 160, text: "krivo".into(), words: vec![Word { from: 150, to: 160, text: "krivo".into() }] });
    save(&target, whole).unwrap();
    let said: Vec<String> = read_clip(&target, "c1").unwrap().iter().flat_map(|t| t.lines.iter().map(|l| l.text.clone())).collect();
    assert!(said.contains(&"fino".to_string()), "{said:?}");
    assert!(said.contains(&"brzo".to_string()), "the fast one fills what the fine one lacks: {said:?}");
    assert!(!said.contains(&"krivo".to_string()), "nothing of the fast one over the fine one: {said:?}");
    // The same range heard fast again keeps the fine one.
    save(&target, transcript(100, 200, "opet brzo")).unwrap();
    let said: Vec<String> = read_clip(&target, "c1").unwrap().iter().flat_map(|t| t.lines.iter().map(|l| l.text.clone())).collect();
    assert!(said.contains(&"fino".to_string()) && !said.contains(&"opet brzo".to_string()), "{said:?}");
}

#[test]
fn the_fine_words_correct_the_rough_ones_and_keep_their_sentences() {
    let w = |text: &str, from: u64, to: u64| Word { from, to, text: text.into() };
    let line = |words: Vec<Word>| Line { from: words[0].from, to: words.last().unwrap().to, text: words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" "), words };
    // Parakeet: sentences, capitals, punctuation; one wrong word and one missed.
    let rough = [line(vec![w("Kad", 0, 10), w("je", 10, 20), w("jedna,", 20, 40)]), line(vec![w("Zagreb", 50, 70), w("promocija.", 80, 100)])];
    // The Croatian model: lower case, no punctuation, the right words, the missed "bila".
    let fine = [line(vec![w("kad", 0, 10), w("je", 10, 20), w("1969.", 21, 39), w("zagreb", 50, 70), w("bila", 71, 79), w("promocija", 80, 100)])];
    let lines = corrected(&rough, &fine, 50);
    let said: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
    assert_eq!(said, ["Kad je 1969.,", "Zagreb bila promocija."]);
}

#[test]
fn a_fine_word_a_little_off_its_rough_one_overwrites_it_and_is_not_there_twice() {
    let w = |text: &str, from: u64, to: u64| Word { from, to, text: text.into() };
    let line = |words: Vec<Word>| Line { from: words[0].from, to: words.last().unwrap().to, text: words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" "), words };
    // Parakeet heard "A bit će teško." the fine engine "a bit će teško" each word a little
    // earlier, its "bit" inside Parakeet's "A"; and "tih 120" glued.
    let rough = [line(vec![w("A", 0, 10), w("Bit", 10, 20), w("će", 20, 30), w("teško.", 30, 50)]), line(vec![w("Kroz", 60, 70), w("tih", 70, 80), w("sto,", 80, 100)])];
    let fine = [line(vec![w("A", 0, 4), w("bit", 5, 9), w("će", 15, 25), w("tesko", 26, 46)]), line(vec![w("Kroz", 60, 70), w("tih120", 70, 100)])];
    let lines = corrected(&rough, &fine, 50);
    let said: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
    assert_eq!(said, ["A Bit će tesko.", "Kroz tih 120,"]);
}

#[test]
fn a_word_heard_twice_at_the_same_time_is_read_once_a_word_said_again_stays() {
    let word = |from: u64, to: u64, text: &str| Word { from, to, text: text.into() };
    let words = vec![word(3649, 3657, "ne"), word(3654, 3658, "ne"), word(3657, 3677, "ne"), word(3841, 3865, "igrača,"), word(3841, 3860, "igrača")];
    let line = Line { from: 3649, to: 3865, text: String::new(), words };
    let read = without_echoes(&[line]);
    assert_eq!(read[0].text, "ne ne igrača,", "the echo goes, the 'ne' said again after it stays");
}
