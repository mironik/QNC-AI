//! The cue sheet of the gallery (user 2026-11-24, "tehnička najava priloga za režiju": "režiji se
//! šalje kad se prilog završi, uzimaju se stvarni TC podaci i trajanje"): written from the
//! finished Story. Its times are the Story's (the segments in order, each its length at the
//! story's rate, the lead-in counted as the exported file has it); each segment finds the block
//! of the script it was made of by its origin (`script:<block>:<part>`), which gives its mark,
//! its shots, its key words, its caption and its note for the gallery.
//!
//! The page: the header (show and date, slug, journalist, editor, the real length, source and
//! format, sound channels, picture), the anchor's lead with its reading time, the IN and OUT cues
//! for the director, the graphics (CG) with their real in and out, the run of the story (time,
//! element, key words, note) and the special notes of the reporter. It reads and writes
//! nothing: the caller gives what it read.

use std::collections::{BTreeMap, HashMap};

use qnc_program_db::ProgramSegment;
use qnc_script_db::Block;
use qnc_script_profile::{Role, ScriptProfile};
use qnc_virtual_shots::ShortClip;

pub const MODULE_ID: &str = "qnc.module.cue-sheet";

/// The paths of the fields the cue sheet reads in a block (the TV package profile has them).
const CG: &str = "cg";
const CG_KIND: &str = "cg_kind";
const CG_TIME: &str = "cg_time";
const NOTES: &str = "notes_direction";
/// The field of the header's notes for the gallery.
const SHEET_NOTES: &str = "notes_direction";
/// How long a CG stands when the script gives no time.
const CG_SECONDS: f64 = 6.0;
/// The words of a key phrase.
const KEY_WORDS: usize = 6;

/// What the cue sheet is written from.
pub struct Facts<'a> {
    pub title: &'a str,
    /// The header as people read it (name, value), and its values by code.
    pub header: Vec<(String, String)>,
    pub header_values: BTreeMap<String, String>,
    /// The Story's segments (its program) and its rate.
    pub segments: &'a [ProgramSegment],
    pub story_fps: (u32, u32),
    pub blocks: &'a [Block],
    pub profile: Option<&'a ScriptProfile>,
    pub statements: &'a [ShortClip],
    /// Words read in a second (the anchor's lead is timed by it).
    pub words_per_second: f64,
    /// The picture of the project: width, height, frames a second, fields (`None` progressive).
    pub picture: Option<(u32, u32, f64, Option<bool>)>,
}

/// One element of the run of the story.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub from: f64,
    pub to: f64,
    pub mark: String,
    /// What is seen or who speaks.
    pub what: String,
    pub first_words: String,
    pub last_words: String,
    pub said: bool,
    pub note: String,
    pub shots: Vec<String>,
}

/// One graphic.
#[derive(Debug, Clone, PartialEq)]
pub struct Graphic {
    pub from: f64,
    pub to: f64,
    pub kind: String,
    pub text: String,
}

/// The cue sheet before it is written out.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cue {
    pub total: f64,
    pub lead: String,
    pub lead_seconds: f64,
    pub elements: Vec<Element>,
    pub graphics: Vec<Graphic>,
}

/// The cue sheet of `facts`.
pub fn cue(facts: &Facts<'_>) -> Cue {
    let rate = f64::from(facts.story_fps.0.max(1)) / f64::from(facts.story_fps.1.max(1));
    let blocks: HashMap<i64, &Block> = facts.blocks.iter().map(|block| (block.id, block)).collect();
    let marks = marks(facts);
    let mut active: Vec<&ProgramSegment> = facts.segments.iter().filter(|segment| segment.active).collect();
    active.sort_by_key(|segment| segment.sort_index);
    let (mut at, mut elements): (f64, Vec<(Option<i64>, Element)>) = (0.0, Vec::new());
    for segment in active {
        let seconds = segment.duration_frames as f64 / rate;
        let (from, to) = (at, at + seconds);
        at = to;
        if !segment.lead.is_empty() {
            continue;
        }
        let block_id = block_of(&segment.origin);
        // Segments of one block one after another are one element.
        if let Some((Some(last), element)) = elements.last_mut() {
            if Some(*last) == block_id {
                element.to = to;
                continue;
            }
        }
        let element = match block_id.and_then(|id| blocks.get(&id)) {
            Some(block) => element_of(block, marks.get(&block.id).cloned().unwrap_or_default(), from, to, facts),
            None => Element {
                from,
                to,
                mark: qnc_ui_text::text(if segment.kind == "offovi" { "cue.kind_off" } else { "cue.kind_sot" }).to_string(),
                what: String::new(),
                first_words: String::new(),
                last_words: String::new(),
                said: segment.kind != "offovi",
                note: String::new(),
                shots: Vec::new(),
            },
        };
        elements.push((block_id, element));
    }
    let mut graphics = Vec::new();
    for (block_id, element) in &elements {
        if let Some(block) = block_id.and_then(|id| blocks.get(&id)) {
            graphics.extend(graphic_of(block, element, facts));
        }
    }
    // The anchor's text before the first block of the story.
    let first = facts.blocks.iter().position(|block| kind_role(facts, block) != Some(Role::Anchor)).unwrap_or(facts.blocks.len());
    let lead: Vec<&str> = facts.blocks[..first].iter().map(|block| block.text.trim()).filter(|text| !text.is_empty()).collect();
    let lead = lead.join(" ");
    let lead_seconds = if facts.words_per_second > 0.0 { lead.split_whitespace().count() as f64 / facts.words_per_second } else { 0.0 };
    Cue { total: at, lead, lead_seconds, elements: elements.into_iter().map(|(_, element)| element).collect(), graphics }
}

fn kind_role(facts: &Facts<'_>, block: &Block) -> Option<Role> {
    facts.profile.and_then(|profile| profile.kind(&block.kind)).map(|kind| kind.role)
}

/// The block of a segment's origin `script:<block>:<part>`.
fn block_of(origin: &str) -> Option<i64> {
    origin.strip_prefix("script:")?.split(':').next()?.parse().ok()
}

/// The numbered mark of each block (`OFF 2`, `IZJAVA 1`), numbered among the blocks of its kind.
fn marks(facts: &Facts<'_>) -> HashMap<i64, String> {
    let mut numbers: HashMap<&str, usize> = HashMap::new();
    facts
        .blocks
        .iter()
        .map(|block| {
            let number = numbers.entry(block.kind.as_str()).or_insert(0);
            *number += 1;
            let cue = facts.profile.map(|profile| profile.cue_label_with(&block.kind, *number, &block.fields)).unwrap_or_default();
            let mark = if cue.is_empty() { facts.profile.and_then(|profile| profile.kind(&block.kind)).map_or_else(|| block.kind.to_uppercase(), |kind| kind.short.clone()) } else { cue };
            (block.id, mark)
        })
        .collect()
}

/// The caption of a clip block: its statement's, else its own caption fields.
fn caption(block: &Block, facts: &Facts<'_>) -> (String, String) {
    let field = |path: &str| block.fields.get(path).map(|value| value.trim().to_string()).unwrap_or_default();
    match facts.statements.iter().find(|statement| !block.statement_id.is_empty() && statement.shot_id == block.statement_id) {
        Some(statement) => (statement.caption_name.trim().to_string(), statement.caption_role.trim().to_string()),
        None => (field("caption_name"), field("caption_role")),
    }
}

fn element_of(block: &Block, mark: String, from: f64, to: f64, facts: &Facts<'_>) -> Element {
    let clip = kind_role(facts, block) == Some(Role::Clip);
    let shots: Vec<String> = block.fields.get("shots").map(|text| text.lines().map(str::trim).filter(|line| !line.is_empty()).map(str::to_string).collect()).unwrap_or_default();
    let what = if clip { caption(block, facts).0 } else { shots.join(", ") };
    let words: Vec<&str> = block.text.split_whitespace().collect();
    let first_words = words.iter().take(KEY_WORDS).copied().collect::<Vec<_>>().join(" ");
    let last_words = words.iter().skip(words.len().saturating_sub(KEY_WORDS)).copied().collect::<Vec<_>>().join(" ");
    Element { from, to, mark, what, first_words, last_words, said: clip, note: block.fields.get(NOTES).cloned().unwrap_or_default(), shots }
}

/// The graphic of an element: the CG text of an OFF (its kind by its name), the caption of a
/// statement or a stand-up; in and out from the block's `cg_time` (`m:ss - m:ss` from the
/// element's start), else its first six seconds.
fn graphic_of(block: &Block, element: &Element, facts: &Facts<'_>) -> Option<Graphic> {
    let profile = facts.profile;
    let (kind, text) = if kind_role(facts, block) == Some(Role::Clip) {
        let (name, role) = caption(block, facts);
        if name.is_empty() {
            return None;
        }
        let kind = qnc_ui_text::text(if block.kind == "standup" { "cue.cg_reporter" } else { "cue.cg_guest" }).to_string();
        (kind, if role.is_empty() { name } else { format!("{name} / {role}") })
    } else {
        let text = block.fields.get(CG).map(|text| text.trim().to_string()).filter(|text| !text.is_empty())?;
        let code = block.fields.get(CG_KIND).cloned().unwrap_or_default();
        let kind = profile
            .and_then(|profile| profile.kind(&block.kind))
            .and_then(|kind| kind.fields.iter().find(|field| field.path == CG_KIND))
            .and_then(|field| profile?.options.get(field.options.as_ref()?)?.iter().find(|(choice, _)| *choice == code).map(|(_, name)| name.clone()))
            .unwrap_or(code);
        (kind, text)
    };
    let (start, end) = block.fields.get(CG_TIME).and_then(|time| span(time)).unwrap_or((0.0, CG_SECONDS));
    let from = (element.from + start).min(element.to);
    Some(Graphic { from, to: (element.from + end).min(element.to).max(from), kind, text })
}

/// `m:ss - m:ss` (or seconds) as seconds from the start.
fn span(text: &str) -> Option<(f64, f64)> {
    let (from, to) = text.split_once('-')?;
    Some((seconds(from)?, seconds(to)?)).filter(|(from, to)| to > from)
}

fn seconds(text: &str) -> Option<f64> {
    let text = text.trim();
    match text.split_once(':') {
        Some((minutes, rest)) => Some(minutes.trim().parse::<f64>().ok()? * 60.0 + rest.trim().parse::<f64>().ok()?),
        None => text.parse().ok(),
    }
}

/// Seconds as `mm:ss`.
pub fn clock(seconds: f64) -> String {
    let whole = seconds.max(0.0).round() as u64;
    format!("{:02}:{:02}", whole / 60, whole % 60)
}

fn t(code: &'static str) -> String {
    qnc_ui_text::text(code).to_string()
}

fn html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The cue sheet as an HTML page (it prints on A4).
pub fn cue_html(facts: &Facts<'_>) -> String {
    let cue = cue(facts);
    let mut out = String::from(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\">\n<style>\nbody{font-family:Arial,Helvetica,sans-serif;font-size:10.5pt;margin:12mm;color:#111}\n.top{background:#1f2a37;color:#fff;padding:10px 14px}\n.top h1{margin:0;font-size:16pt}\n.top div{font-size:9pt;color:#cbd5e1}\nh2{background:#334155;color:#fff;font-size:10.5pt;padding:5px 10px;margin:14px 0 0}\ntable{border-collapse:collapse;width:100%}\nth,td{border:1px solid #9aa4b2;padding:5px 8px;vertical-align:top}\nth{background:#e2e8f0;text-align:left}\n.k{background:#e2e8f0;font-weight:bold;width:18%}\n.box{border:1px solid #9aa4b2;padding:7px 10px}\n.cue{display:flex;gap:10px;background:#fff7e6;border:1px solid #f0a030;padding:8px}\n.cue div{flex:1}\n.tag{background:#1f2a37;color:#fff;padding:1px 6px;font-size:9pt;font-weight:bold}\n.out{background:#b45309}\n.said{font-style:italic}\n.notes{background:#f1f5f9;border:1px solid #cbd5e1;padding:8px 10px;margin-top:14px}\n</style>\n",
    );
    out.push_str(&format!("<title>{}</title></head><body>\n<div class=\"top\"><h1>{}</h1><div>{}</div></div>\n", html(facts.title), html(&t("cue.title")), html(&t("cue.subtitle"))));

    // The header with the real length, the format and the sound channels.
    let mut pairs: Vec<(String, String)> = facts.header.clone();
    let notes = facts.header_values.get(SHEET_NOTES).cloned().unwrap_or_default();
    pairs.retain(|(_, value)| *value != notes || notes.is_empty());
    pairs.push((t("cue.trt"), clock(cue.total)));
    if let Some((width, height, fps, fields)) = facts.picture {
        let scan = match fields {
            Some(_) => format!("{}i", (fps * 2.0).round()),
            None => format!("{}p", fps.round()),
        };
        let aspect = if height > 0 && (f64::from(width) / f64::from(height) - 16.0 / 9.0).abs() < 0.05 { "16:9" } else { "" };
        pairs.push((t("cue.picture"), format!("{width}×{height} {scan} {aspect}").trim().to_string()));
    }
    pairs.push((t("cue.sound"), t("cue.sound_lanes")));
    pairs.push((t("cue.state"), format!("&#9744; {}", t("cue.ready"))));
    out.push_str("<table>\n");
    for pair in pairs.chunks(2) {
        out.push_str("<tr>");
        for (name, value) in pair {
            let value = if value.starts_with("&#9744;") { value.clone() } else { html(value) };
            out.push_str(&format!("<td class=\"k\">{}</td><td>{value}</td>", html(&name.to_uppercase())));
        }
        if pair.len() == 1 {
            out.push_str("<td class=\"k\"></td><td></td>");
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</table>\n");

    // 1. The anchor's lead.
    out.push_str(&format!("<h2>{}</h2>\n<div class=\"box\">", html(&t("cue.lead"))));
    if cue.lead.is_empty() {
        out.push_str("&mdash;");
    } else {
        out.push_str(&format!("{} <i>({} ~{})</i>", html(&cue.lead), html(&t("cue.lead_length")), clock(cue.lead_seconds)));
    }
    out.push_str("</div>\n");

    // 2. IN and OUT for the director.
    out.push_str(&format!("<h2>{}</h2>\n<div class=\"cue\">", html(&t("cue.in_out"))));
    match (cue.elements.first(), cue.elements.last()) {
        (Some(first), Some(last)) => {
            let picture = first.shots.first().cloned().unwrap_or_else(|| first.what.clone());
            out.push_str(&format!(
                "<div><span class=\"tag\">{}</span> <b>{} – {}</b><br><b>{}:</b> {}<br><b>{}:</b> {} {}</div>",
                html(&t("cue.in")),
                clock(first.from),
                clock(first.to),
                html(&t("cue.picture_of")),
                html(&picture),
                html(&t("cue.sound_of")),
                html(&first.mark),
                html(&first.first_words)
            ));
            let last_picture = last.shots.last().cloned().unwrap_or_else(|| format!("{} {}", last.mark, last.what).trim().to_string());
            out.push_str(&format!(
                "<div><span class=\"tag out\">{}</span> <b>{}</b><br><b>{}:</b> <span class=\"said\">&quot;...{}&quot;</span><br><b>{}:</b> {}</div>",
                html(&t("cue.out")),
                clock(cue.total),
                html(&t("cue.last_words")),
                html(&last.last_words),
                html(&t("cue.last_shot")),
                html(&last_picture)
            ));
        }
        _ => out.push_str(&html(&t("cue.no_story"))),
    }
    out.push_str("</div>\n");

    // 3. The graphics.
    out.push_str(&format!(
        "<h2>{}</h2>\n<table>\n<tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>\n",
        html(&t("cue.graphics")),
        html(&t("cue.number")),
        html(&t("cue.in_out_tc")),
        html(&t("cue.graphic_kind")),
        html(&t("cue.graphic_text"))
    ));
    for (index, graphic) in cue.graphics.iter().enumerate() {
        out.push_str(&format!("<tr><td><b>CG {:02}</b></td><td>{} – {}</td><td>{}</td><td><b>{}</b></td></tr>\n", index + 1, clock(graphic.from), clock(graphic.to), html(&graphic.kind), html(&graphic.text)));
    }
    out.push_str("</table>\n");

    // 4. The run of the story.
    out.push_str(&format!(
        "<h2>{}</h2>\n<table>\n<tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>\n",
        html(&t("cue.run")),
        html(&t("cue.time")),
        html(&t("cue.element")),
        html(&t("cue.key_words")),
        html(&t("cue.note"))
    ));
    for (index, element) in cue.elements.iter().enumerate() {
        let what = if element.what.is_empty() { String::new() } else { format!(" ({})", html(&element.what)) };
        let words = if element.first_words.is_empty() {
            String::new()
        } else if element.said {
            format!("<b>IN:</b> <span class=\"said\">&quot;{}...&quot;</span><br><b>OUT:</b> <span class=\"said\">&quot;...{}&quot;</span> ({})", html(&element.first_words), html(&element.last_words), clock(element.to - element.from))
        } else {
            format!("<span class=\"said\">&quot;{}...&quot;</span> &rarr; <span class=\"said\">&quot;...{}&quot;</span>", html(&element.first_words), html(&element.last_words))
        };
        let mut note = html(&element.note);
        for (number, graphic) in cue.graphics.iter().enumerate() {
            if graphic.from >= element.from && graphic.from < element.to {
                note.push_str(&format!("{}{}", if note.is_empty() { "" } else { "<br>" }, html(&qnc_ui_text::fill("cue.play_cg", &[("n", &format!("{:02}", number + 1)), ("at", &clock(graphic.from))]))));
            }
        }
        if index + 1 == cue.elements.len() {
            note.push_str(&format!("{}<b>{}</b>", if note.is_empty() { "" } else { "<br>" }, html(&t("cue.end"))));
        }
        out.push_str(&format!("<tr><td>{} – {}</td><td><b>{}</b>{what}</td><td>{words}</td><td>{note}</td></tr>\n", clock(element.from), clock(element.to), html(&element.mark)));
    }
    out.push_str("</table>\n");
    if !notes.trim().is_empty() {
        out.push_str(&format!("<div class=\"notes\"><b>{}:</b> {}</div>\n", html(&t("cue.notes")), html(&notes)));
    }
    out.push_str("</body></html>\n");
    out
}

#[cfg(test)]
mod tests;
