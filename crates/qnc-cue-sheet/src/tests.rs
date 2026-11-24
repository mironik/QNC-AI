use super::*;

fn profile() -> ScriptProfile {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (profiles, errors) = qnc_script_profile::Profiles::load(&root);
    assert!(errors.is_empty(), "{errors:?}");
    profiles.list.into_iter().map(|(profile, _)| profile).find(|profile| profile.profile_id == "tv_prilog").unwrap()
}

fn segment(id: &str, kind: &str, frames: u64, origin: &str, lead: &str, index: u32) -> ProgramSegment {
    ProgramSegment { segment_id: id.into(), kind: kind.into(), sort_index: index, active: true, duration_frames: frames, origin: origin.into(), lead: lead.into(), fps_num: 50, fps_den: 1, ..ProgramSegment::default() }
}

fn block(id: i64, kind: &str, text: &str, fields: &[(&str, &str)]) -> Block {
    let mut block = Block { id, ..Block::written(kind, text) };
    block.fields = fields.iter().map(|(path, value)| (path.to_string(), value.to_string())).collect();
    block
}

#[test]
fn the_times_are_the_storys_and_each_element_its_blocks() {
    let profile = profile();
    let blocks = [
        block(1, "intro", "Jutarnje gužve u središtu Metkovića postale su svakodnevica.", &[]),
        block(2, "off", "Jutarnje gužve u središtu grada postale su svakodnevica, a rješenje se čeka godinama.", &[("cg", "Metković"), ("cg_kind", "location"), ("cg_time", "0:02 - 0:06"), ("shots", "Total glavne ulice\nVozač u autu"), ("notes_direction", "Pustiti 2 s IT na početku")]),
        Block { statement_id: "s1".into(), ..block(3, "sot", "Svaki dan kasnim barem petnaest minuta hitno nam treba zaobilaznica.", &[]) },
    ];
    let statements = [qnc_virtual_shots::ShortClip { shot_id: "s1".into(), clip_id: "c".into(), in_frame: 0, out_frame: 600, name: String::new(), in_still_uri: None, out_still_uri: None, still_status: String::new(), b_roll: false, caption_name: "Marko Marković".into(), caption_role: "vozač".into(), parts: Vec::new() }];
    // A lead-in of 2 s, the OFF in two segments (32 s), the statement (12 s).
    let segments = [
        segment("lead", "offovi", 100, "", "in", 0),
        segment("a", "offovi", 1000, "script:2:0", "", 1),
        segment("b", "offovi", 600, "script:2:1", "", 2),
        segment("c", "tonovi", 600, "script:3:0", "", 3),
    ];
    let facts = Facts { title: "Promet", header: vec![("Slug".into(), "PROMET_METKOVIC".into())], header_values: BTreeMap::new(), segments: &segments, story_fps: (50, 1), blocks: &blocks, profile: Some(&profile), statements: &statements, words_per_second: 2.5, picture: Some((1920, 1080, 50.0, None)) };
    let cue = cue(&facts);
    assert_eq!(cue.total, 46.0);
    assert_eq!(cue.elements.len(), 2, "one element a block");
    assert_eq!((cue.elements[0].from, cue.elements[0].to), (2.0, 34.0));
    assert_eq!((cue.elements[1].from, cue.elements[1].to), (34.0, 46.0));
    assert_eq!(cue.elements[1].what, "Marko Marković");
    assert_eq!(cue.graphics.len(), 2);
    assert_eq!((cue.graphics[0].from, cue.graphics[0].to, cue.graphics[0].text.as_str()), (4.0, 8.0, "Metković"));
    assert_eq!((cue.graphics[1].from, cue.graphics[1].to, cue.graphics[1].text.as_str()), (34.0, 40.0, "Marko Marković / vozač"));
    assert!(cue.lead.starts_with("Jutarnje gužve u središtu Metkovića"));
    let page = cue_html(&facts);
    assert!(page.contains("00:46") && page.contains("CG 02") && page.contains("1920×1080 50p 16:9"));
}
