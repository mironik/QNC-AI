# Forms: what is where, and the screenshots

The layout of each form stays (what is where); only the look changes. Put the screenshots in
`snimke/` under the names below (PNG, the whole window, at 100 % scale).

## Editing board (AI Media Assist, MA Script, MA Audio, MA Video, S-Story, Story, Ingest)

All share one board (`ai-editorial.layout.json`, `ingest.layout.json`):

```
┌──────────────────── title bar ─────────────────────────────────────────┐
│ LEFT COLUMN (36.5 %, surface)    │ RIGHT COLUMN (bg)                    │
│ ┌──────────────────────────────┐ │ the form's right panel:              │
│ │ monitor 16:9                 │ │  transcript / script / segments /    │
│ └──────────────────────────────┘ │  Video AI / directory browser …      │
│ pool tabs        │ transport     │                                      │
│ media pool (cards grid)          │                                      │
├──────────────────────────────────┴──────────────────────────────────────┤
│ source timeline: header (clip, IN, OUT, Duration, buttons) + V/A1/A2    │
└──────────────── status line ────────────────────────────────────────────┘
```

| Form | Right panel | Screenshots |
|---|---|---|
| Ingest | Directory browser (Computer / LAN / Internet / Bases), Select / Cancel | `ingest-browser.png`, `ingest-cards.png` |
| AI Media Assist (e) | Tabs Audio AI (transcript) / Video AI (shots) | `ai-media-assist-transcript.png`, `ai-media-assist-video-ai.png`, `ai-media-assist-trim-zoom.png` |
| MA Script (g) | Script writer over the whole form; media in an overlay window | `ma-script.png`, `ma-script-media-overlay.png` |
| MA Audio (h) | Script panel (blocks, takes, record) | `ma-audio.png` |
| MA Video (l) | Empty (reserved) | `ma-video.png` |
| S-Story (k) | Draft story segments (Wrap rows) | `s-story.png` |
| Story (o) | Segments (Wrap rows, markers, slots, B-roll), export | `story-segments.png`, `story-trim-zoom.png`, `story-export.png` |

## Settings (1120 × 700)

```
┌ title bar ───────────────────────────────────────────────┐
│ General | User | Project | AI | Export        (tabs)     │
│ ──────────────────────────────────────────────────────── │
│ MENU (288)        │ GROUP CHOSEN                          │
│  THIS COMPUTER    │  Heading                              │
│   Locations       │   label (168) │ field │ button (120)  │
│   System          │   label       │ field │ button        │
│   …               │                                       │
└──────────────────────────────────────────────────────────┘
```

Screenshots: `settings-general-locations.png`, `settings-general-system.png`,
`settings-general-tray.png`, `settings-user.png`, `settings-project.png`, `settings-ai-templates.png`,
`settings-ai-labels.png`, `settings-export.png`.

## Project (new project, template, story sheet)

Screenshots: `project.png`, `project-story-sheet.png` (the form of a new story, keyboard driven).

## Projects (list of projects, 640 wide)

Screenshot: `projects.png`.

## Card ingest (540 × 745, opens when a card is put in)

"Ingest in progress": base name, time; card "Background work" with rows (Import, Posters,
Filmstrip, Wave, AI shots, Content, Captions, Transcripts: bar, n/m, time, ×); card "Ingest base"
(name, place, description); Cancel.

Screenshots: `card-ingest-running.png`, `card-ingest-done.png`.

## Administration (1100 × 600)

Ingest bases: list, delete, open. Screenshot: `admin.png`.

## Tray menu (system tray)

Status, Stop all, modes (Local · Breaking news, Local · Story, Net) with ticks, Import from
folder, Settings, Administration, Open application, Quit. Screenshot: `tray-menu.png`.
