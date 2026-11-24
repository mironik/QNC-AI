# Components and their states (as drawn today)

Every component is drawn by one style block and used on every form; a change here changes all
forms. States to design for each: **normal, hover, pressed, selected/on, keyboard focus,
disabled**, and where it applies **taken** (an IN/OUT, marker, lane picked up to be moved) and
**error**.

## Window chrome

| Component | Today | Code |
|---|---|---|
| Title bar (frameless) | 32 px, title left; right: search, settings (gear), minimise, maximise, close (close hover red #C42B1C) | `qnc-title-bar` |
| Status line (footer) | One line of text; plain / alert (red) / done (green); a link to a file is underlined | `qnc-status-line`, `qnc-shell-footer` |
| Alert bar | Red bar, top centre, upper case text, 3.5 s | `qnc-alert` |
| Choice box / confirm dialog | Centred popup, title, text, buttons (default focused) | `qnc-choice-box`, `qnc-confirm-dialog` |

## Settings style block (`kod/crates/qnc-settings-block`)

| Component | Today |
|---|---|
| Tabs (General, User, Project, AI, Export) | Text-width selectable labels in a row, 24 px high, a 1 px rule under them |
| Left menu | Group heading (upper case, 12 px, muted, bold); items one tab (16 px) in, full width, selected = filled |
| Group heading | Left, 12 px text size of the row, bold |
| Row | One tab in; label column 168 px (left aligned, 12 px muted), field column (rest), button column 120 px; 24 px high |
| Text field | Filled `raised`, 1 px border, 24 px |
| Choice (drop-down) | egui ComboBox, 24 px, truncated text |
| Check box | 24 px row, box + words |
| Primary button | Filled `accent`, dark text |
| Action button | Filled `raised`, `text` |
| Hint | 12 px muted |
| Said (result line) | Green (ok) or red (error) text |
| Scrolling list (labels, rules) | Fixed number of rows, own scroll bar |

## Media components

| Component | Today | Code |
|---|---|---|
| Pool tabs | Text tabs above the media pool (All, SOT/Statements, Transcript, AI B-roll…) | `qnc-pool-tabs-panel`, `qnc-media-pool-head` |
| Media card | Poster 16:9 (or a microphone icon for a voice take), name, duration; top-right tick (selected) or X/regroup on the chosen card; status dot (ready, original ready, pending, error); chosen = red frame; keyboard focus = thicker focus frame | `qnc-media-card` |
| Monitor | 16:9 black, the player's picture; poster with a message bar at the bottom while preparing | `qnc-monitor`, `qnc-monitor-panel` |
| Transport row | Play, IN, OUT, Auto/On (HDMI) | `qnc-transport-panel` |
| Level meter + NORM / AI / KEEP | 240 × 22, A1/A2 bars, zones green/yellow/red, buttons right of it | `qnc-level-bar`, `qnc-level-meter` |
| Source timeline | Header: clip name, IN / OUT / Duration timecodes (monospace), buttons right; lanes: V (filmstrip), A1, A2 (waves), 28 px label column with lane buttons; red IN/OUT, white playhead, red when hovered in reach; dimmed outside IN-OUT; level marks -18/-9/-1 dBFS; channel picker column on a taken lane | `qnc-timeline`, `qnc-source-dock`, `qnc-source-timeline-view` |
| Wrap rows (story) | One timeline row per segment: filmstrip or narrow video, A1 and A2 bands; M markers (thin lines), slots with B-roll strips coloured by duration, J/L sides in red; panel bar with Undo/Redo, Cover slot, Overwrite, Export | `qnc-segment-panel` |
| Trim zoom | Popup in the middle: 2 or 4 pictures around a cut (cut picture framed red), A1/A2 waves with words under them, red cut line, white playhead; Undo / Save / Close | `qnc-trim-overlay` |
| Transcript panel | Lines of words, the word at the playhead highlighted, IN..OUT words underlined; search field + n of m + Next + Copy at the bottom | `qnc-transcript-panel` |
| Script panel | Story blocks as a table: time column, text, join column; record buttons, takes T1..Tn with % read; editor at the bottom | `qnc-script-panel`, `qnc-script-writer-panel` |
| Video AI panel | Usable parts (TC, steady/pan, caption), coloured by duration | `qnc-shot-panel` |
| Card ingest | Cards with radius 8: background work (rows: name, bar 6 px, n/m, time, ×), base fields | `apps/qnc-ai-card-ingest` |
| Tray menu | System tray menu (OS drawn) with tick items | `qnc-tray` |

## egui basics behind all of them (`qnc-desktop-theme::apply_settings_look`, `qnc-ui-kit`)

- `visuals.widgets.*`: fills `raised`/`surface_alt`, hover `surface_alt`, strokes `border`.
- Selected = filled with `accent` at low alpha or `raised`; focus = 1 px `focus` frame.
- One font family; sizes 12 / 13 / 14 / 20.
