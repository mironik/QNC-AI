# Design tokens today (QNC-AI)

Where each value lives is in brackets; changing the value there changes every form.

## Palette (`contracts/ui/ai-shell.layout.json` → `colors`)

| Token | RGB | Hex | Used for |
|---|---|---|---|
| `bg` | 15, 16, 18 | #0F1012 | Window background, right column, panels |
| `surface` | 23, 24, 27 | #17181B | Left column (monitor + pool), cards |
| `raised` | 32, 34, 38 | #202226 | Buttons, fields, hover, `surface_alt` |
| `border` | 38, 40, 44 | #26282C | Lines, frames, dividers |
| `text` | 237, 237, 237 | #EDEDED | Text |
| `muted` | 138, 139, 143 | #8A8B8F | Labels, hints, secondary text |
| `accent` | 200, 201, 204 | #C8C9CC | Strong buttons, selected |
| `focus` | 154, 155, 160 | #9A9BA0 | Keyboard focus frame |

Media (`colors.media`):

| Token | RGB | Used for |
|---|---|---|
| `lane` | 36, 38, 43 | Audio lane fill |
| `wave` | 128, 130, 137 | Audio wave |
| `wave_zoom` | 140, 142, 149 | Wave in the trim zoom |
| `timeline_focus`, `lane_taken` | 237, 237, 237 | Timeline in focus, a lane taken |
| `meter` | (128,160,140) (200,184,120) (214,82,74) | Level meter: under -12, -12..-3, over -3 dBFS |
| `dot_ready`, `dot_original_ready`, `dot_pending`, `dot_error` | 138,139,143 / 200,201,204 / 70,72,76 / 214,82,74 | Card status dots |
| `check_on` | 237, 237, 237 | Ticked box |
| `level_marks` | -, -, (214,82,74,40) | Level marks on lanes (-18 / -9 / -1 dBFS) |
| `level_curve` | 150, 152, 158 | Level curve line |
| `lane_divider` | 23, 24, 27 | Between lanes |

Fixed colours in code (`qnc-desktop-theme`):

| Name | RGB | Used for |
|---|---|---|
| `danger` | 239, 68, 68 | Errors, refusals (red bar on top), over time |
| `done` | 34, 197, 94 | "Exported" / done status |
| `total_text` | 255, 120, 120 | Story total over the approved length |
| IN/OUT, cut line | red | Source marks, cut in the zoom |

Duration colours of B-roll / slots (`ai-editorial.layout.json` → `cover_lengths`):
orange #FF8C00 (< 2 s), yellow #FACC15 (< 3 s), green #22C55E (< 4 s), cyan #22D3EE (<= 5 s),
blue #3B82F6 (> 5 s).

## Typography (`ai-shell.layout.json` → `theme_metrics`)

| Token | Value |
|---|---|
| Font family | egui default: Ubuntu Light (text), Hack (monospace) |
| `font_ui` | 14 px |
| `font_timecode` | 13 px (monospace) |
| Settings labels (`label_font_size`) | 12 px |
| Group titles in Settings (`group_title_font_size`) | 12 px, upper case, muted, bold |
| Form titles (form looks `title_size`) | 20 px |

## Sizes and spacing

Chrome (`theme_metrics`): row 28 px, control 24 px, padding 8 × 2 px.

Settings panel (`project.layout.json` → `right_settings_panel`, `forms/settings.look.json`):

| Token | Value |
|---|---|
| Window | 1120 × 700 |
| `panel_pad` | 20 |
| `inner_pad_x` / `inner_pad_y` | 8 / 8 (one "tab" of indent = 16) |
| `section_gap` | 8 |
| Label column | 168 |
| Button column | 120 |
| Column gap | 8 |
| Field min width | 160 |
| Row height | 24 |

Editorial and Ingest board (`ai-editorial.layout.json` / `ingest.layout.json` → `shell` / `board`):
left column 36.5 % (min 280), divider 5, right column min 200, margins 8, block padding 10,
gap 8. Monitor 16:9, contain, black face. Media cards min width 160, text height 34, gap 10.

Timeline (`qnc-timeline` `TimelineMetrics`): label column 28, video lane 64, audio lane 18
(expanded 52), no gap between lanes, border 1.

Forms (`contracts/ui/forms/*.look.json`):

| Form | Window | Notes |
|---|---|---|
| Settings | 1120 × 700 | values above |
| Card ingest | 540 × 745 | cards with radius 8, frame 1, padding 18; progress bars 6 thick, row 26 |
| Administration | 1100 × 600 | margins 24, title 20, buttons 90 × 24 |
| Projects | 640 wide | rows 60 high, gap 10 |

## Shapes today

Corner radius: mostly 0-4 px (cards in card ingest 8). Lines 1 px. No shadows except egui's
popups. Buttons flat, filled `raised`, text `text`; strong button filled `accent`.
