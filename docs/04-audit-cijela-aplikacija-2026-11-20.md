# Audit cijele aplikacije: sve forme lanca (2026-11-20)

Nastavak `03-audit-kocke-2026-11-20.md` (AI Media Assist). Ovdje: tray i Ingest Wizard,
Project/Postavke/Projekti/Admin/Ask, Story/S-Story/MA Script/MA Audio/MA Video, i ono što
reže kroz cijelu aplikaciju. Sve provjereno u kodu; ništa nije mijenjano.
`AI` = QNC-AI/crates, `SH` = QNC-shared/crates.

## 1. Mjere

- 414 paketa: 159 u QNC-AI (116 kocki, 15 aplikacija, 28 alata), 255 u QNC-shared.
- 16 paketa s 20+ qnc ovisnosti: `qnc-ingest-worker` 37, `qnc-ingest-application` 36,
  `qnc-script` 36, `qnc-panel-focus` 30, `qnc-editorial-application` 29, `qnc-program-segments`
  25, `qnc-ingest-select` 25, `qnc-export` 24, `qnc-editorial-blocks` 24, `qnc-project-blocks`
  23, `qnc-source-preview` 23, `qnc-source-timeline` 23, `qnc-subsegment-edit` 22,
  `qnc-ingest-import-worker` 22, `qnc-ai-card-ingest` (aplikacija!) 21, `qnc-ai-broll` 20.
- 25 kopija iste pomoćne `fn log` (11 s provjerom dijagnostike, 14 bez; `&str` ili `String`).
- Conformance: 13 prekršaja (C6, C10, C12, C15), isti kao prije današnjih promjena.
- Hardkodiranih `C:\` putanja u radnom kodu nema.
- Mrtve kocke: `AI/qnc-wizard-flow` (267 redaka, nitko je ne koristi), `StartAsk`/`Ask` u
  `qnc-card-ingest`, `AutoSelect::take_skipped`, privremena datoteka
  `%TEMP%\qnc-ai-ingest-choice-<pid>.json` (`qnc-story-start:74`) koju nitko ne čita.

## 2. Tray i Ingest Wizard (zaključani, ali auditirani)

**Aplikacije drže pravila (forma nije samo raspored):**
- `apps/qnc-ai-card-ingest` (719 redaka): naziv baze obavezan prije zatvaranja + auto-naziv
  nakon 30 s (:188-212), pravilo "kopirano" i prelazak na breaking news (:217-229), politika
  izbacivanja kartice (eject.rs :11-119), "je li kartica još potrebna" (eject.rs :122-135),
  zadrži/obriši djelomični uvoz ponovnim pokretanjem samog sebe s `--delete-base` (:420-479),
  parsiranje naziva baze (:570-575) koje ponavlja `qnc-ingest-base` `stamp()`/`label()`.
- `apps/qnc-ai-tray` (526): kartica se "slegne" 5 s, 5 pokušaja čitanja, katalog svakih 60 s,
  tri grane za poznatu karticu (:308-332), stop/obriši s 10 s odgode i popisom baza u
  memoriji (:161-211, :405), odredište mape Da/Ne (:231-262), pokretanje exe-a s ručno
  sastavljenom putanjom (:444-505). `qnc-ingest-orchestrator` je za tray samo prolaz
  (`card_base` = `new_base`).

**Isto napisano više puta:**
- Skeniranje diskova svake 2 s: tray `:274`, `qnc-attached-cards:46-81`, `eject.rs:85`
  (dva procesa skeniraju istodobno).
- Učitavanje kataloga kamera s ponavljanjem: tray (60 s), attached-cards (2 s), AutoPlay.
- "Je li kartica već u bazi": tray `known_card`, attached-cards `known_card_elsewhere`,
  oboje nad `qnc-ingest-base:647-673`, s različitim ishodima.
- Nit koja prati napredak (1 s): `qnc-card-ingest:216-251` i `qnc-work-progress:278-307`.
- "Je li moja baza još aktivna": card-ingest `:225`, worker `run.rs:293`, `main.rs:140`.
- Pokretanje exe-a na 5 načina: tray, card-ingest, `qnc-tray/windows.rs:343` (literal
  `"qnc-ai-ask.exe"`), `qnc-wizard-open`, `qnc-worker-wake`; samo zadnja dva kroz
  `qnc_plugin_process::beside_application`.
- "Gotovo/kopirano" formula 4 puta: work-progress `:58-71`, card-ingest `:219`,
  eject `:41`, work-info `:114-119`.
- `QNC_NEW_STORY` definiran dvaput (orchestrator `:48`, story-start `:212`).

**Baza nije jedina istina:**
- `qnc-ingest-base` piše `project.db` **sirovim** `rusqlite::Connection::open` (`:70, :159,
  :596, :684, :730`), mimo posrednika (`qnc-db-broker`) kroz koji pišu Select i uvoz; isti
  SQLite ima dva kanala pisanja. Migracije (`ALTER TABLE`) radi pri upisu (`:157-171`).
- Isti red `ingest_base_info` pišu dva procesa (aplikacija `BaseInfo::keep`, worker
  `describe_base`).
- `make()` privremeno prepisuje globalni `projects_root` pa ga vraća (`:566, :582`): utrka
  između procesa.
- Stanje procesa u JSON-u i memoriji: `data/tray-mode.json` čitaju tri procesa (card-ingest
  **svaki frame**, `:220`; worker iz njega mijenja redoslijed koraka `run.rs:148-166`, iako
  dokument orkestratora kaže "redoslijed je templateov"), `ingest-orchestrators.json`, popis
  baza za brisanje u memoriji traya.

**Raspoređivanje posla napisano više puta:** `SH/qnc-background-jobs` (koriste samo
filmstrip i wave), `tools/qnc-ingest-worker` vlastiti raspoređivač (nit po koraku s 1 s
petljom, nadzorna nit s 2/30/10 s, pravila prioriteta `run.rs:139-172`, hardkodirani koraci
`:115-125`, 20 ms petlja artefakata do sat vremena `main.rs:233-284`), transcribe petlja 2 s.

**Spojene kocke:** `qnc-tray` (ikona + upozorenja + dijalozi + IPC `Signal` + `OneCopy` +
izgled izbornika; card-ingest treba samo `OneCopy` i `Signal`), `qnc-ingest-base` (1280
redaka, 73 fn: registar baza, info + migracije, veza s pričom, poznata kartica, indeks
kartice, zatvaranje, sirova čitanja registra), `qnc-ingest-application` (1661, 110 fn),
`qnc-ingest-import-worker` (1316, 74 fn).

## 3. Project / Postavke / Projekti / Admin / Ask

- `qnc-project-store` (2128 radnih redaka + 1387 testova, 18 pub fn, ~14 odgovornosti):
  registar, baza projekta, templatei/seed, slijed aplikacija, **poslovni defaulti** postavki
  (products, crossfade, brzina čitanja, glasnoća, lead-in/out, timecode, bitrate `:1792-1899`),
  mape projekta, zaštita od brisanja po OS-u (icacls/attrib/chflags; **ponovno zaključava sve
  projekte pri svakom `open()`**, `:167`), brisanje, pretraga, vlastita kopija
  `qnc-settings-path` (`:882-916`). "Slijed počinje grupom a" provjereno 3 puta
  (`:546, :1750, :1914`). Fallback templatea dvaput (store `:1659`, jobs `:170`).
- Forma i sesija odlučuju: `qnc-project-session` (routiranje koja lokacija ide u registar,
  u postavke računala ili u nacrt `:319-355, :552-575`; čita postavke računala u
  konstruktoru `:146`; `"exports/projekti"` ponovljen `:153`), `qnc-project-blocks`
  (zaglavlje kaže "blokovi samo crtaju", a `project_advanced.rs:101,114,133` pišu nacrt;
  pravilo "može kreirati" `settings.rs:276,412`; `"NE"/"DA"`, `"custom"`, `"X"` izvan
  rječnika; `list.rs` crta isti red dvaput `:44-62` i `:248-254`).
- **Druga tema:** `qnc-project-blocks/src/theme.rs` ima vlastiti `Theme` i `rgb()`
  (`:6, :32`), kopija `SH/qnc-desktop-theme` (`:11, :187`); `shell.layout.json` se parsira
  dvaput u dvije teme (`layout_contract.rs:31, :46`).
- Aplikacije: isti boilerplate (OneCopy, root, FormLook, NativeOptions, stil) u
  settings/projects/admin umjesto `qnc-form-window::options`, koju koriste druge forme.
  `qnc-ai-ask` ima vlastite boje, font, veličine i zadane gumbe (`"yes=Da,no=Ne"`).
  Dva sustava teksta: aplikacije `qnc_wizard_text`, kocke `qnc_ui_text`.
- `qnc-form-look`: vlastita crvena `from_rgb(230,70,60)` umjesto `danger()`, vlastiti egui
  gumbi, `get()` vraća 0.0 za nepoznat ključ. Conformance ga već prijavljuje.
- Put do registra `data/qnc-projects.db` sastavljen u **5 kocki** (`qnc-ingest-base:290,302,
  1056`, `qnc-project-store:793`, `SH/qnc-project-close:53`, `SH/qnc-work-settings:160`,
  `main.rs:42`; `qnc-desktop:837`).
- Isti `{folder}` zapis postavke računala napisan dvaput (`SH/qnc-base-cards:18-40`,
  `SH/qnc-ai-catalog:22-56`); dva načina građenja putanje postavki; `ui-language.json` dva
  parsera u istoj kocki; dva pravila pretrage (`ProjectRow::matches` i `qnc-base-search`);
  slobodan id generiran 3 puta.

## 4. Story / S-Story / MA Script / MA Audio / MA Video

- `qnc-script` (2270 radnih redaka, 45 pub fn, 30 polja, 38 ovisnosti): ~20 odgovornosti
  (CRUD blokova, nacrti polja, snimanje OFF-a, snimanje cijelog priloga, provjera čitanja,
  markeri, rez po riječima/slovima, undo, prijedlog pokrivalica, export, **Send to S-Story
  s pravilima priče** `:813-883` i literalom `"offovi"` `:857`, delegacija složene izjave,
  čitanje baze svake 2 s, red upisa, koordinacija 3 pozadinska posla). `act_own` je jedan
  `match` od ~260 redaka. Ponovno izvozi ~30 stavki iz `qnc-subsegment-edit`.
- `SH/qnc-program-segments` (4836 radnih redaka u 23 modula, 95 pub fn, struktura sa ~75
  polja): udomljuje stanje 10 kocki koje već postoje (zoom, lanes, level, assist, sync,
  export, loudness, waves, filmstrips, undo). `slip.rs:31` ručno ograničava umjesto
  `qnc_edit_edge::Range::slipped` koju koristi subsegment-edit.
- Isti automat stanja dvaput (`qnc-program-segments` ↔ `qnc-subsegment-edit`): slip
  (`SLIP` konstanta i `slip_key`), save-and-keep, rub (trim), slide/slot move, level line,
  assist, Esc/Enter, poruke; **undo dvaput** s dvije tablice (`story_undo` 190 redaka,
  `script_undo` 91) pod istim akcijama.
- Pravila u panelima: `qnc-script-panel` (klik za vrijeme snimanja = marker `:53`, prag
  preko cilja `:113`, mapiranje metra −48 dB `:266` **isto u writer-panelu `:145`**),
  `qnc-script-writer-panel` (najava samo jednom `:72`, numeracija blokova `next_mark`
  `:327` ponavlja `view.rs:453` drukčije), `SH/qnc-segment-panel` (prepisuje akciju
  `program.rs:223`, `TakeMarkerPicture→TakeMarker` `:226` uz komentar "panel ne odlučuje").
- **Layout ugovori:** `AI/contracts/ui/editorial.layout.json` nitko ne čita u radu (samo
  conformance); istina je `SH/contracts/ui/ai-editorial.layout.json` (`include_str` u
  `SH/qnc-ai-editorial-layout`, alias `qnc-editorial-layout` u `AI/Cargo.toml:308`). Danas su
  isti (md5), ali mogu se razići. `qnc-ai-editorial-layout` (620) je kopija
  `qnc-editorial-layout` (517) s 113 razlikovnih redaka.
- Čitači baze na sat u otvorenoj formi: Script 2 s, SkeletonVersions 2 s, ZoomWords 3 s
  (glasovi, koje Script već čita), AiBroll 3 s, Transcript 1 s, ClipTranscripts 1 s (×2-3),
  četiri `qnc-clip-follow` (1-3 s), transcribe-queue 250 ms, transcribe-run 200 ms,
  lease 1 s: **12-15 neovisnih petlji** po formi. Sama priča (`program-segments`) nema sat.

## 5. Što reže kroz sve

- Jedna tablica, dva pisca: `app_settings` (`qnc-project-store:698`, `SH/qnc-project-close:218`;
  dopušteno iznimkom u AGENTS §3, ali je drugi pisac izvan vlasnika), `project_settings`
  (store + `qnc-ingest-select/test_support` pod feature `test-support`, koju **uključuju**
  `qnc-ingest-application` i `qnc-ingest-import-worker` u produkciji).
- Sirovo otvaranje SQLite mimo posrednika: `qnc-ingest-base` (7 mjesta), `qnc-project-store`
  (3), `qnc-camera-profiles`, `qnc-ingest-store/validate.rs`, `qnc-shot-captions`,
  `qnc-shot-examples`, `qnc-shot-vocabulary`.
- Fallback na `current_dir()` kad korijen nije nađen kopiran u 11 aplikacija.
- `qnc-ingest-select/test_support.rs` ugrađuje fixture iz drugog repoa relativnom putanjom
  (`../../../../QNC-shared/...`).
- 78 kocki istog imena postoji i u klasičnom QNC-u; 8 s drugim popisom datoteka, 46 s istim
  popisom ali drugim sadržajem, 24 identične. Kad se QNC vrati na zajedničke kocke, to je
  razlika koju treba pomiriti.

## 6. Zaključak

Pravilo "jedna radnja = jedna kocka" napušteno je na tri načina, svugdje isto:
1. **Aplikacije i orkestratori odlučuju** (tray, card-ingest, panel-focus, project-session,
   editorial-application, worker).
2. **Velike kocke udomljuju tuđe stanje** (script, program-segments, source-preview,
   project-store, ingest-base, tray).
3. **Isto pravilo napisano više puta** (skeniranje diskova, napredak, pokretanje exe-a,
   prag glasa, undo, slip/trim/level/assist, tema, put do registra, log).

Plus "baza je jedina istina" probijena JSON datotekama stanja, sirovim pisanjem mimo
posrednika i stanjem u memoriji procesa.

## 7. Preporučeni redoslijed (nastavak na 03-audit, §H)

Nakon koraka 1-7 iz 03-audita (AI Media Assist):
8. **Jedan pisac po bazi**: `qnc-ingest-base` prelazi na posrednika; `test-support`
   izlazi iz produkcijskih ovisnosti.
9. **Jedna kocka za pokretanje pomoćnih programa** (`qnc-plugin-process`), svih 5 mjesta.
10. **Jedna kocka za diskove i karticu** (attached-cards) koju tray i card-ingest samo zovu;
    jedna kocka napretka; "gotovo/kopirano" jednom.
11. **Tray i card-ingest bez pravila**: slijeganje, pokušaji, poznata kartica, stop/obriši,
    eject u kocke; `qnc-tray` rastaviti (IPC, OneCopy, dijalozi).
12. **Stanje procesa u bazu**: tray-mode i izbor orkestratora kao zapis u registru; worker
    uzima redoslijed iz templatea; privremena datoteka story-start van.
13. **`qnc-project-store` rastaviti** (zaštita od brisanja, defaulti postavki, templatei,
    mape); zaštita ne pri svakom otvaranju; jedno mjesto za put do registra.
14. **`qnc-script` i `qnc-program-segments` rastaviti**; jedan undo; Send to S-Story u kocku
    priče; jedan automat za slip/trim/level/assist.
15. Jedna tema (`qnc-project-blocks/theme.rs` van), jedan tekst sustav, jedan `fn log`,
    jedan boilerplate aplikacije (`qnc-form-window`), jedan layout ugovor (ukloniti AI kopiju
    ili je učiniti istinom).
16. Obrisati mrtvo: `qnc-wizard-flow`, `StartAsk`, `take_skipped`, temp datoteka.

Svaki korak: zlatni test ponašanja prije, promjena, isti test poslije, commit. Tray i Ingest
Wizard su zaključani: koraci 8-12 traže izričito otključavanje.
