use super::*;

fn prepared() -> SheetSpec {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (sheets, errors) = Sheets::load(&root);
    assert!(errors.is_empty(), "{errors:?}");
    sheets.list.into_iter().map(|(sheet, _)| sheet).find(|sheet| sheet.sheet_id == PREPARED).expect("the TV package sheet")
}

fn header() -> StoryHeader {
    let mut header = StoryHeader { sheet: Some(prepared()), ..StoryHeader::default() };
    header.begin();
    header.refresh();
    header
}

#[test]
fn the_tv_package_sheet_has_every_field_of_the_header() {
    let paths: Vec<String> = prepared().fields.into_iter().map(|field| field.path).collect();
    assert_eq!(paths, ["show", "air_date", "slug", "target_seconds", "journalist", "editor", "camera_operator", "source", "notes_direction"]);
}

#[test]
fn no_name_is_guessed_a_date_is_today_and_several_names_are_kept() {
    let mut header = header();
    let field = |header: &StoryHeader, path: &str| header.view().fields.iter().find(|field| field.path == path).cloned().unwrap();
    assert!(header.view().open);
    assert!(field(&header, "journalist").names.is_empty(), "the laptop does not know who writes");
    assert_eq!(field(&header, "air_date").value, qnc_newsroom_sheet_model::date_today());
    header.act(NAME, Some("journalist\u{1f}0\u{1f}Ana Anić"));
    header.act(NAME, Some("journalist\u{1f}1\u{1f}Ivo Ivić"));
    header.act(NAME, Some("journalist\u{1f}0\u{1f}Ana Anić-Horvat"));
    assert_eq!(field(&header, "journalist").names, ["Ana Anić-Horvat", "Ivo Ivić"]);
    header.act(REMOVE_NAME, Some("journalist\u{1f}0"));
    assert_eq!(field(&header, "journalist").names, ["Ivo Ivić"]);
    header.act(TEXT, Some("target_seconds\u{1f}1:45"));
    assert_eq!(header.draft.get("target_seconds"), Some(&Value::from(105)));
    assert_eq!(field(&header, "target_seconds").value, "1:45");
    header.act(CANCEL, None);
    assert!(!header.view().open);
    assert!(field(&header, "journalist").names.is_empty(), "cancel keeps nothing");
}
