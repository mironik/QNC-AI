//! The review of the editor and the proofreader (user 2026-11-24, "Redakcijski obrazac -
//! pregled urednika i lektora"): sent while the story is written. The header of the story with
//! the status of the editor (waiting, approved, to be reworked) and of the proofreader
//! (waiting, proofread) and room for their confirmation; the anchor's lead to be checked for the
//! prompter; a table of three columns: the picture and its captions (the shots numbered through
//! the story, the CG), the text read (OFF, SYNC, STAND-UP with their lengths), and an empty
//! column for the notes of the editor and the proofreader; at the end the rules of writing for
//! the reporter. Its words are the dictionary's.

use qnc_script_profile::Role;

use crate::{html, Story};

fn t(code: &'static str) -> String {
    qnc_ui_text::text(code).to_string()
}

/// The review as an HTML page (it prints on A4).
pub fn review_html(story: &Story) -> String {
    let mut out = String::from(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\">\n<style>\nbody{font-family:Arial,Helvetica,sans-serif;font-size:11pt;margin:14mm;color:#111}\nh1{font-size:17pt;color:#0f2d5a;border-bottom:2px solid #0f2d5a;padding-bottom:4px}\nh2{font-size:12pt;color:#0f2d5a;margin-top:18px}\ntable{border-collapse:collapse;width:100%}\nth,td{border:1px solid #9aa4b2;padding:6px 8px;vertical-align:top}\nth{background:#0f2d5a;color:#fff;text-align:left}\n.k{background:#eef1f6;font-weight:bold;color:#0f2d5a;width:22%}\n.lead{border:1px solid #0f2d5a;padding:8px 10px;margin-top:8px}\n.cg{color:#b3261e;font-weight:bold}\n.mark{font-weight:bold;color:#0f2d5a}\n.said{font-style:italic}\n.notes{width:24%}\n.pic{width:30%}\n.tips{background:#f4f6f9;border-left:4px solid #0f2d5a;padding:8px 12px;margin-top:16px;font-size:10pt}\n</style>\n",
    );
    out.push_str(&format!("<title>{}</title></head><body>\n<h1>{}</h1>\n", html(&story.title), html(&t("review.title"))));

    // The header and the status of the approval.
    out.push_str(&format!("<h2>{}</h2>\n<table>\n", html(&t("review.head"))));
    let mut pairs: Vec<(String, String)> = story.header.clone();
    let total = match &story.target {
        Some(target) => format!("{} / {target}", story.total),
        None => story.total.clone(),
    };
    pairs.push((story.labels.trt.clone(), total));
    for pair in pairs.chunks(2) {
        out.push_str("<tr>");
        for (name, value) in pair {
            out.push_str(&format!("<td class=\"k\">{}</td><td>{}</td>", html(&name.to_uppercase()), html(value)));
        }
        if pair.len() == 1 {
            out.push_str("<td class=\"k\"></td><td></td>");
        }
        out.push_str("</tr>\n");
    }
    let boxes = |codes: &[&'static str]| codes.iter().map(|code| format!("&#9744; {}", html(&t(code)))).collect::<Vec<_>>().join("&nbsp;&nbsp; ");
    out.push_str(&format!(
        "<tr><td class=\"k\">{}</td><td>{}</td><td class=\"k\">{}</td><td>____________________</td></tr>\n",
        html(&t("review.editor_status")),
        boxes(&["review.waiting", "review.approved", "review.rework"]),
        html(&t("review.editor_confirm"))
    ));
    out.push_str(&format!(
        "<tr><td class=\"k\">{}</td><td>{}</td><td class=\"k\">{}</td><td>____________________</td></tr>\n</table>\n",
        html(&t("review.proof_status")),
        boxes(&["review.waiting", "review.proofread"]),
        html(&t("review.proof_confirm"))
    ));

    // The anchor's text before the story and after it.
    let placed = story.placed();
    let first_story = placed.iter().position(|(item, _)| item.in_story).unwrap_or(placed.len());
    let anchor = |range: &[(&crate::Item, crate::Place)], title: &str, out: &mut String| {
        let texts: Vec<&str> = range.iter().filter(|(item, _)| item.role == Some(Role::Anchor) && !item.text.trim().is_empty()).map(|(item, _)| item.text.trim()).collect();
        if !texts.is_empty() {
            out.push_str(&format!("<div class=\"lead\"><b>{}</b><br>{}</div>\n", html(title), texts.iter().map(|text| html(text)).collect::<Vec<_>>().join("<br>")));
        }
    };
    anchor(&placed[..first_story], &t("review.lead"), &mut out);

    // The story: the picture and its captions, the text read, the notes.
    out.push_str(&format!(
        "<h2>{}</h2>\n<table>\n<tr><th class=\"pic\">{}</th><th>{}</th><th class=\"notes\">{}</th></tr>\n",
        html(&t("review.story")),
        html(&t("review.col_picture")),
        html(&t("review.col_text")),
        html(&t("review.col_notes"))
    ));
    let mut shot = 0;
    for (item, _) in placed.iter().filter(|(item, _)| item.in_story) {
        let mut picture: Vec<String> = Vec::new();
        if item.review.shots.is_empty() || item.role == Some(Role::Clip) {
            picture.push(format!("<span class=\"mark\">{}</span>", html(&item.review.mark.to_uppercase())));
        }
        for line in &item.review.shots {
            shot += 1;
            picture.push(format!("<span class=\"mark\">{}:</span> {}", html(&qnc_ui_text::fill("writer.shot", &[("n", &shot.to_string())])), html(line)));
        }
        for (name, value) in &item.review.captions {
            picture.push(format!("<span class=\"cg\">{}:</span> {}", html(name), html(value)));
        }
        let said = item.text.trim();
        let mut text = format!("<span class=\"mark\">{}", html(&item.review.mark.to_uppercase()));
        if item.role == Some(Role::Clip) && !item.length.is_empty() {
            text.push_str(&format!(" ({})", html(&item.length)));
        }
        text.push_str(":</span> ");
        text.push_str(&if item.role == Some(Role::Clip) { format!("<span class=\"said\">&quot;{}&quot;</span>", html(said)) } else { html(said) });
        let notes: Vec<String> = item.review.notes.iter().map(|(name, value)| format!("<i>{}:</i> {}", html(name), html(value))).collect();
        out.push_str(&format!("<tr><td class=\"pic\">{}</td><td>{text}</td><td class=\"notes\">{}</td></tr>\n", picture.join("<br>"), notes.join("<br>")));
    }
    out.push_str("</table>\n");
    let last_story = placed.iter().rposition(|(item, _)| item.in_story).map_or(placed.len(), |at| at + 1);
    anchor(&placed[last_story..], &t("review.tag"), &mut out);

    // The rules of writing for the reporter.
    out.push_str(&format!(
        "<div class=\"tips\"><b>{}</b><ul><li>{}</li><li>{}</li><li>{}</li></ul></div>\n</body></html>\n",
        html(&t("review.tips")),
        t("review.tip_numbers"),
        t("review.tip_names"),
        t("review.tip_captions")
    ));
    out
}
