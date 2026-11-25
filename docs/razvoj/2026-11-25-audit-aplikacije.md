# Audit ponašanja cijele aplikacije QNC-AI, 2026-11-25

Korisnik: "Moraš napraviti audit cijele aplikacije, pronaći ponašanje svakog dijela i svake kocke";
"da si u Agents.md pisao pravila, sad ne bi trebao raditi audit". Samo čitanje, kod nije mijenjan.

Područja (šest pregleda):
1. Story i S-Story (detalj: `2026-11-25-audit-story.md`);
2. platforma, baze i alati;
3. Ingest, kartice, tray i wizard;
4. izvor, player, valovi, prerender i export;
5. AI, scenarij, prijepis, glas i izjave;
6. forme, ploče, tipke, miš i postavke.

Pravila koja iz ovoga slijede (i ona koja je korisnik odredio) upisana su u `AGENTS.md`, odjeljak
**3a. Pravila ponašanja**. Ovdje je ono što kod danas radi drukčije ili nedorečeno: **popis za
odluku korisnika**. Ništa s ovog popisa nije popravljeno.

## A. Ista radnja radi različito (treba jedno pravilo)

A1. **Tipke source timelinea idu kroz dva usmjeravanja**: Story/MA kroz `qnc-panel-focus`
    (`route.rs`), Ingest kroz `qnc-ingest-application` (`dispatch.rs`) i `qnc-focus-walk`.
    Razlike:
    - Ctrl+1/2 u Ingestu ne pušta uzeti IN/OUT;
    - linija razine (Ctrl+Shift+1/2, desni klik na liniju) u Ingestu ne stiže nikamo;
    - Shift+I (IN i OUT za duljinu slota) samo u Storyju; u Ingestu je Shift+I Uvezi;
    - gore/dolje na uzetoj oznaci (ključna slika) samo u Storyju;
    - zoom reza i povećana izjava isključeni u Ingestu;
    - kotačić s uzetom krivuljom razine radi u Ingestu i Storyju različito;
    - Home / klik na V: Story vraća i klip dok svira priča, Ingest samo cue;
    - u Ingestu bez playera oznake, trake i rez rade, a tipke playera ne.
A2. **Alt/Tab+strelice i klik u panel** daju fokus panelu samo u Story/MA formama. U Ingestu
    su tipke vezane, a ništa ih ne obrađuje; u Projectu, Postavkama i prozorima nema hoda Tabom.
A3. **Enter**:
    - lista u Ingestu: otvara, a bez potvrde ide na sljedeći panel;
    - Project: otvara izabrani projekt;
    - obrazac priče i choice box: sljedeće polje, zatim potvrda;
    - povijest projekta: vraća projekt;
    - tekst izjave u scenariju: izreže (piše u bazu);
    - složena izjava u AI Media Assistu: drži nacrt koji se ne zapisuje (vidi C1);
    - potvrdni dijalog i Export prozor: ništa.
A4. **Esc**:
    - Story/source timeline: `clear_focus`;
    - Ingest: Odustani u pregledniku, pa Esc ne zatvara povijest projekta;
    - ostali preseti tipki: Esc daje `clear_focus` i `close_player` zajedno;
    - potvrdni dijalog, choice box, Export prozor, obrazac priče, Postavke: ništa.
A5. **Delete i Backspace**:
    - QNC preset: Backspace undo, Delete briše uzeti marker;
    - ostali preseti: Delete i Backspace okidaju `delete_part` i `delete_marker` zajedno, a
      pomoć F1 kaže Backspace = Undo;
    - Ingest: Backspace = Gore;
    - kartice, popis projekata, templatei: samo X (×) klikom, bez tipke;
    - izjave, citati i blokovi scenarija brišu se X-om bez potvrde: blok scenarija ima Undo,
      citati i izjave nemaju;
    - u scenariju, kad su uzeti i M marker i riječi, Delete briše samo marker.
A6. **Klik i dvoklik**:
    - klik na redak projekta odmah otvara projekt;
    - dvoklik u povijesti odmah vraća projekt, bez potvrde;
    - dvoklik u Export prozoru izvozi;
    - dvoklik na riječ prijepisa postavlja IN/OUT (ne sprema);
    - dvoklik na riječ scenarija odmah upisuje dio u bazu;
    - desni klik na riječ: u scenariju uzima riječ ili postavlja M, u prijepisu ništa.
A7. **Redoslijed i nazivi gumba u prozorima**: NE|DA, Export|Cancel, Cancel|Create, Go back|Close,
    Da|Ne. Pet zasebnih izvedbi prozora (confirm, choice box, export box, povijest, ai-ask) i
    obrazac priče, svaki sa svojim tipkama.
A8. **Ostali preseti tipki (Edius, Resolve, Premiere, Final Cut, Avid) gube tipke.** Zamjena
    je po cijelom opsegu, ne po radnji, pa izbor takvog preseta tiho uklanja Enter, J/K/L,
    povijest, Undo/Redo, snapping, tabove poola i hod panelima.
A9. **IN/OUT za prijepis**: osvjetljenje riječi traži oba korisnikova markera, a dodavanje
    citata, ponovni prijepis i Transcribe uzimaju i zadani IN/OUT (početak i kraj klipa).
A10. **"Izjava" su tri različita zapisa**:
     - citat prijepisa (`transcript_quotes`);
     - izjava u poolu (`virtual_shots`, složena s dijelovima);
     - blok scenarija s dijelovima (`script_blocks`).

     Izmjena jednog ne stiže na druge.
A11. **Tekst OFF-a u MA Audiju i MA Scriptu sprema se različito**:
     - MA Audio: Dodaj + Zamijeni; Zamijeni upisuje i prazan tekst i ne sprema bilješku B-rolla;
     - MA Script: Spremi, s provjerom;
     - NAJAVA samo jednom vrijedi samo u MA Scriptu;
     - broj sljedećeg OFF-a u MA Scriptu računa se drugom formulom nego oznaka bloka.
A12. **Story / S-Story** (detalj u auditu Storyja):
     - u S-Storyju radnje slota mijenjaju V1 (`clip_slots.rs`);
     - u Storyju se spoj dva segmenta uzima gestama markera i piše V1/A1;
     - prazan slot nema traku trajanja i ne može se obrisati (vidi D1);
     - brisanje M-a briše pokrivalice oba slota;
     - odbijanje "rupa u pokrivalici" blokira i izmjene V1.

## B. Podaci mimo vlasnika ili posrednika

B1. `qnc-ingest-base` piše izravnim SQL-om u baze projekata i sam stvara tablice:
    - `ingest_base_closing`, `ingest_base_info`, `story_clips`, `story_ingest_bases`;
    - `ingest_base_sources`, `ingest_base_links`, `ingest_base_only_new`.

    Uz to piše `QNCCARD.JSON` i zapise u bazu kartice iz projekta. Opis tvrdi "ništa drugo se ne
    piše".
B2. `qnc-content-store` je drugi, usporedni posrednik baze projekta; `program-db` i
    `virtual-shots` otvaraju vlastiti `BEGIN IMMEDIATE`, što nije sigurno pod najmom.
B3. Oko 20 tablica nema ugovor baze; dio ugovora ne odgovara kodu:
    - analiza kadrova je u `ai.db`;
    - URI `virtual-shots`;
    - `story_undo` i `story_revision` priče;
    - nazivi potvrda zapisa medija.
B4. Upisi bez transakcije ili s niti sučelja:
    - `qnc-stopped-work` (čitaj–izmijeni–piši);
    - `qnc-newsroom-sheet` piše profil scenarija iz niti sučelja;
    - `qnc-base-statements` kopira složenu izjavu u više zasebnih upisa;
    - registar Ingesta se piše sinkrono u dispatchu.
B5. Aktivni projekt čita se na 4 načina; `qnc-ingest-base` čita `data/qnc-projects.db` izravno i
    ne poštuje pribadani projekt (`QNC_WORK_PROJECT`).
B6. `qnc-prerender-launch` provjerava najam zadanog projekta, a pokreće prerender aktivnog.

## C. Pokvareno ili napola napravljeno

C1. **[POPRAVLJENO 2026-11-25]** **Složena izjava u AI Media Assistu**: `StatementPlay` baca izmjene (`take_writes` se
    odbacuje); Update radi samo za jednostavnu izjavu. Dijelovi složene izjave ne mijenjaju se
    nigdje. `StatementPlay` uzima tipke i kad nije nacrtan.
C2. **[POPRAVLJENO 2026-11-25]** **Povijest projekta u Ingestu**: Ctrl+Shift+H otvara povijest, ali se prozor ne crta
    (`theme: None`), a ona od tada guta svaku tipku.
C3. **AI ograničenja vrijede samo u sučelju**: tipka ili radnja iz kataloga zaobilazi sivi gumb
    (snimka OFF-ova, Smooth, prijedlog pokrivalica).
C4. **Scenarij**:
    - nakon neuspjelog Spremi sljedeći uspješan upis prazni uređivač;
    - dodani blok briše nespremljeni nacrt;
    - snimanje takea ne provjerava je li u tijeku snimanje cijelog priloga;
    - promjena projekta ruši take koji se snima.
C5. Spremljena XML postavka exporta (`export-xml.qncxml`) nikad se ne koristi.
C6. Tray pri zatvaranju gasi i QNC formu u kojoj korisnik radi (zastavica "outlives" ne vrijedi
    uz job objekt).
C7. Mrtav kod:
    - `qnc-wizard-flow`, `qnc-wizard-db`, `qnc-wizard-steps` (nitko ih ne koristi);
    - `StartAsk`, `stored()`;
    - `show_dock` / `show_timeline_dock`, `show_head`, `pick_directory`;
    - `apply_user_overrides_json` (korisničke izmjene tipki nisu spojene);
    - opsezi tipki `timeline_modal` i `off` u svih 6 preseta.
C8. Radnje bez tipke ili bez pomoći:
    - F1 samo u Storyju;
    - 28 vezanih radnji nije u pomoći;
    - Export prozor, preglednici lokacija Projecta, Close project, Postavke i obrazac priče
      (Odustani) nemaju tipku;
    - id-evi izvan kataloga: `project_history_light`, `statement_caption_save/cancel`.
C9. Opisi kocki koji tvrde suprotno od koda ("ništa ne piše", a piše):
    - `qnc-source-timeline`, `qnc-source-preview`, `qnc-prerender`, `qnc-export`;
    - `qnc-ai-broll`, `qnc-ingest-base`, tray.
C10. Prijedlog pokrivalica na čitanju s kartice na mreži dobiva lokalni URI (`card-readers`).
C11. `qnc-take-heard` nije vezan uz AI mogućnost projekta i u dvije otvorene forme može prepisati
     isti take dvaput.

## D. Pravila korisnika koja kod još ne slijedi

D1. **Prazan slot se može obrisati** (V2): odlazi s IN/OUT-om, OUT prije i IN poslije postaju
    jedan M, slotovi iza se pomaknu naprijed. Danas: nema operacije, Delete javlja poruku.
D2. **Traka trajanja selektira slot**, i prazan: danas je prazan slot nema.
D3. **Sve na V2 ne dira V1/A1**: S-Story (`clip_slots.rs`) to krši (A12).

## E. Isti kod napisan više puta (pravilo: jednom, u kocki)

- Pokretanje programa: oko 6 načina.
- Format timecodea i trajanja: oko 5.
- Nit + kanal + poll: desetak ručnih kopija; `qnc-background-jobs` koriste samo dva radnika.
- `wait_for_player`, `scratch_dir`, `log` u AI radnicima.
- Dva čitača diskova istodobno.
- Dva puta čitanja zvuka (val i prijepis).
- Katalog tipki ugrađen 5 puta (`include_str!`).
- Pet izvedbi prozora; šest vrsta gumba.
- Spremljene postavke exporta na dva mjesta.
- Preset tipki se bira na dva mjesta.
