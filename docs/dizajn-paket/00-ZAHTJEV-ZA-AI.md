# Request for AI design tools: QNC-AI user interface

> Hrvatski sažetak na dnu dokumenta.

## Who we are, what the product is

QNC-AI is a desktop video-editing and news-production application for TV journalists and editors
(news cutter on a laptop in the field, with an HDMI monitor). It runs on Windows, macOS and Linux.
It is written in **Rust with egui** (immediate-mode GUI, GPU drawn). Users work fast, under
deadline pressure, mostly by keyboard, often in dark rooms.

The current look is functional but plain ("developer grey"). We want it to look and feel like a
**professional broadcast / NLE tool** (think DaVinci Resolve, Avid Media Composer, Adobe Premiere,
Final Cut Pro, Avid iNEWS, Dalet), while staying calm, readable and fast.

## How we work with you: two phases, strictly in this order

### Phase 1: pictures only (no code)

Give us **visual proposals** (images or mockups) for the forms listed in `03-forme.md`:

1. Start with **one proposal of the visual language** shown on two screens: *Settings* and
   *AI Media Assist*. Up to **3 variants** of the visual language (e.g. "graphite neutral",
   "deep blue broadcast", "warm dark").
2. After we pick a variant, give the **same language on every form** in `03-forme.md`.
3. For each image add a short **legend**: the colours (hex), font sizes, spacing, corner radius,
   line widths you used, so the picture can be measured.

Do **not** write code in Phase 1. Do not move panels around or add/remove functions: the layout
(what is where, see `03-forme.md`) stays; we change how it looks (colours, contrast, typography,
spacing, borders, states, icons, hierarchy).

### Phase 2: values and code, only for what we accepted

After we accept a proposal, give us:

1. **Design tokens** as JSON values in the format of our contracts (see `01-tokens.md` and the
   files in `kod/contracts/`): the palette (`colors`, `media`), `theme_metrics`, the settings panel
   metrics and the form looks. We change values in these files first: one place for every form.
2. **Component specs** (`02-komponente.md` lists them): size, padding, colours of each state
   (normal, hover, pressed, selected, focused, disabled), border, radius.
3. Only where a value is not enough: **egui code** for the style blocks in `kod/crates/`
   (`qnc-settings-block`, `qnc-ui-kit`, `qnc-desktop-theme`, `qnc-theme-styles`). Keep their
   public functions (names, arguments); change what they draw. No new widget library.

## Rules that do not change (please respect them in Phase 1 already)

- **Layout stays**: the board (left column monitor + pool, right column panels, source timeline at
  the bottom), the order of tabs and groups, the functions on each form.
- **One font system** (now egui's default: Ubuntu Light for text, Hack for monospace/timecode).
  Another single font family is possible if it is open-licensed (OFL/Apache) and ships with the app.
- **Dark only** for the editing forms (a light variant is not needed now).
- **Red is reserved** for IN/OUT, cuts, errors and "over time"; do not use it for decoration.
- **Duration colours** of B-roll/slots (orange < 2 s, yellow < 3 s, green < 4 s, cyan <= 5 s,
  blue > 5 s) are a working code for journalists; keep their meaning (shades may change).
- **Readable at a glance**: text contrast at least WCAG AA (4.5:1) on its background; timecodes
  monospace.
- **egui can draw**: filled and stroked rectangles with rounded corners, lines, circles, text,
  images, simple drop shadows, transparency. It cannot do blur, gradients (only by hand-drawn
  meshes), CSS effects or animations beyond simple fades. Icons are drawn as lines or used as
  small images (SVG/PNG we ship).
- **Density**: rows are 24 px high now, controls 24 px; you may propose 26-28 px, not more. The
  timeline lanes are fixed in height (video 64 px, audio 18 px).

## What you get in this package

| File | What |
|---|---|
| `01-tokens.md` | Today's palette, typography, sizes, spacing |
| `02-komponente.md` | Every component and its states, as drawn today |
| `03-forme.md` | Every form: what is where, which screenshots we attach |
| `snimke/` | Screenshots of today's forms (names in `03-forme.md`) |
| `kod/contracts/` | The UI contracts (JSON) the values live in |
| `kod/crates/` | The egui code of the style blocks (Rust) |

## What we want back

- Phase 1: images (PNG/JPG or a Figma link) + legend per image.
- Phase 2: JSON values, component specs, and only where needed Rust/egui code of the style blocks.

---

## Hrvatski sažetak

Šaljemo paket za doradu izgleda QNC-AI-ja (Rust + egui). Rad ide u **dvije faze**:

1. **Samo slike**: najprije do 3 varijante vizualnog jezika na Postavkama i AI Media Assistu,
   zatim izabrana varijanta na svim formama, uz legendu boja i mjera. **Bez koda.** Raspored i
   funkcije se ne mijenjaju, mijenja se izgled.
2. **Tek nakon što prihvatimo**: vrijednosti (JSON tokeni u formatu naših ugovora), opis
   komponenti po stanjima, i samo gdje vrijednost nije dovoljna egui kod kocki stila.

Pravila: raspored ostaje, jedan font, samo tamna tema, crvena je rezervirana za IN/OUT, rezove i
greške, boje trajanja pokrivalica zadržavaju značenje, kontrast barem WCAG AA, egui ne zna blur ni
gradijente, redovi 24 px (najviše 28 px).
