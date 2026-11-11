# QNC-AI: plan AI wizarda

Datum: 2026-11-11. Odluke korisnika:
- Opcija B: zaseban repo `QNC-AI`, samostalna aplikacija.
- "Kompletno znanje i kocke mozemo koristiti, moguce promjene su dozvoljene jer smo odabrali drugi direktorij koji je
  samostalna aplikacija": QNC-AI koristi kocke QNC-a i smije ih mijenjati kod sebe, bez zamrzavanja QNC-a.
- Wizard mora raditi i na ovom racunalu: ne trazi nista vise od QNC-a.
- Pokrivalice idu po ponudi, korisnik bira; postavljaju se markerima ili Sync/B-rollom (alati Storyja).
- AI server se moze koristiti, ali ne mora; moze biti i samostalni lokalni server na laptopu koji nema veze s QNC-om.
- wav2vec2 ParlaSpeech se koristi bez obzira na nenavedenu licencu ("ako je bude trebalo uzeti rijesit cu; alat koristi
  Hrvatski sabor, mislim da je javan").

## 1. Sto je wizard

Novinar prolazi jedan put od kartice do exporta, korak po korak, a AI radi ono sto moze sam (prijepis, provjera citanja,
rez OFF-ova, ponuda pokrivalica). Svaki korak ima jasan uvjet "gotovo" i gumbe Dalje / Natrag. Wizard vodi kroz alate koji
vec postoje i sakriva ostalo. Projekt koji wizard napravi je obican QNC projekt (isti `project.db`, ista prica), pa ga QNC
moze otvoriti kad zatreba vise.

## 2. Proces i sto vec postoji

| Korak wizarda | Sto korisnik radi | Kocke (vec postoje) | Sto fali |
|---|---|---|---|
| 0. Novi prilog | Naziv, obrazac priloga | `qnc-newsroom-sheet`, `qnc-project-jobs`, AI template "Vijesti sve" | template wizarda; bez Project forme |
| 1. Uvoz | Kartica ili mapa, "uvezi sve" | `qnc-ingest-select-run`, `qnc-ingest-import-run`, pozadinski radnik | jedna radnja "uvezi sve" (Select + oznaci sve + Uvezi) |
| 2. Izjave | AI sam prepise kadrove s govorom na kanalu 1; novinar bira izjave u tekstu | `qnc-transcribe-queue`, `qnc-transcript`, izjava = slika + A1 govor, Dodaj izjavu | automatski red prijepisa za klipove s govorom nakon uvoza |
| 3. Pisanje | OFF-ovi i opaske "kadrovi koje zelim", izjave u scenarij | MA Script (`qnc-script`, `qnc-script-writer-panel`) | - |
| 4. Snimanje | Snimi prilog (Space izmedu citanja), STT provjera | `qnc-voice-take` Session, `qnc-session-readings`, `qnc-read-check` | uvjet "gotovo": svaki OFF ima citanje >= 90 % ili potvrdu novinara |
| 5. Rez OFF-ova | Automatski IN/OUT po izgovorenim rijecima, pregled | `qnc-script-draft` `take_sound_range` | pregled reza u koraku |
| 6. Prica | Kostur iz scenarija | Send to Story (`qnc-program-db` AppendSegments) | izravno u Story, bez S-Story verzija |
| 7. Pokrivalice | Ponuda za slot (tab AI pokrivalice, opaske novinara), korisnik bira i postavlja markerima ili Sync/B-rollom | Wrap redovi, M markeri, B / Cover slot, Sync/B-roll, `qnc-broll-suggest`, `qnc-ai-broll` | ponuda uz izabrani slot u istom pogledu kao prica (raspored koraka) |
| 8. Pregled | Play cijele price, HDMI, NORM, AI ton | program player, `qnc-program-output`, normalizacija, pomocnik tona | - |
| 9. Potvrda i export | Express / Original / preset, XML | `qnc-export`, `qnc-export-launch` | - |

## 3. Arhitektura

- **Repo `QNC-AI`** (ovaj direktorij), vlastiti Cargo workspace i vlastiti `AGENTS.md`. Nasljeduje nacela QNC-a (baza je
  istina, forma je samo raspored, kocke s jednom odgovornoscu, rjecnik i jezik, testovi nikad ne pisu u stvarni projekt),
  a ne nasljeduje zamrzavanja QNC-a.
- **Kocke QNC-a**: koriste se kao git ovisnost zakljucana na oznaku QNC-a (`git = "https://github.com/mironik/QNC.git",
  tag = "v-2026-11-11"`) dok ih wizard koristi kakve jesu. Kocku koju wizard treba promijeniti QNC-AI preuzme k sebi
  (`crates/` u QNC-AI, isti naziv kao u QNC-u, zapis odakle je i s koje oznake) i mijenja je tamo; `[patch]` u
  workspaceu QNC-AI tada sve ovisnosti usmjeri na njegovu verziju. Tako se QNC ne mora mijenjati, a QNC-AI ostaje
  dosljedan. Svako preuzimanje kocke zapisuje se u `AGENTS.md` QNC-AI (sto, s koje oznake, zasto).
- **Stanje wizarda u bazi projekta**: modul tablica posrednika `qnc-wizard-db` (QNC-AI), tablica `wizard_state` (korak, sto
  je potvrdeno, kada). Zatvaranje i ponovno otvaranje nastavlja gdje se stalo; QNC je ne dira.
- **Aplikacija `qnc-wizard`**: jedan samostalni exe, bez shella i bez instaliranog QNC-a. Lijevo popis koraka (gotovi,
  trenutni, zakljucani), u sredini panel koraka sastavljen od postojecih panela (monitor, prijepis, scenarij, prica,
  ponuda pokrivalica), dolje Natrag / Dalje i sto korak jos treba. Stroj ispod je isti kao MA Script i Story
  (`qnc-editorial-application`, `qnc-side-pieces`, `qnc-panel-focus`); raspored svakog koraka je ugovor rasporeda u QNC-AI.
- **Instalacija bez QNC-a**: wizard nosi sve sto treba (programe, ugovore, katalog kamera, rjecnik, FFmpeg) u svojoj mapi;
  korijen aplikacije je mapa wizarda, ne razvojni QNC.
- **AI**: na ovom racunalu kao i QNC (isti AI katalog, AI paket). AI server neobavezan: QNC-ov `qnc-ai-server` ili
  samostalni lokalni server na laptopu. Za samostalni server: server cita svoju postavku i katalog iz svoje mape, ne trazi
  QNC korijen (danas `qnc-ai-server` trazi QNC korijen; to se mijenja u QNC-AI). Wizard bez AI paketa javlja koji korak ne
  moze raditi, ne pada.

## 4. Plan izvedbe (redom)

**W0. Kostur QNC-AI**: git repo, Cargo workspace, `AGENTS.md`, `docs/razvoj`, git ovisnosti na oznaku QNC-a,
`qnc-wizard` exe s praznom formom koraka, `qnc-wizard-db` (stanje u bazi), rjecnik wizarda (hr/en). Gradnja i pokretanje na
ovom racunalu bez shella.

**W1. Samostalnost**: korijen aplikacije je mapa wizarda (ugovori, katalozi, rjecnik, postavke uz nju); wav2vec2 u AI paketu
(status `user`, odluka korisnika); samostalni lokalni AI server (postavka i katalog iz svoje mape).

**W2. Koraci 0-1**: novi prilog (obrazac, template wizarda) i "uvezi sve". Uvjet: klipovi uvezeni, zapisi dovrseni.

**W3. Korak 2**: izjave; prijepis krece sam za klipove s govorom na kanalu 1. Uvjet: barem jedna izjava ili "bez izjava".

**W4. Koraci 3-5**: pisanje, snimanje sa STT provjerom, rez OFF-ova s pregledom. Uvjet: svaki OFF ima izabrano citanje.

**W5. Koraci 6-7**: prica iz scenarija izravno u Story; pokrivalice po ponudi, korisnik postavlja markerima ili Sync/B-rollom.
Uvjet: svaki OFF pokriven ili svjesno ostavljen.

**W6. Koraci 8-9**: pregled (player, HDMI, NORM, AI ton), potvrda, export i XML.

**W7. Paket i proba**: instalacija wizarda na cisto racunalo (bez QNC-a) + AI paket; cijeli put na stvarnoj kartici (FX6,
FX3A, XDCAM), vrijeme od kartice do exporta.

**Kasnije, nije prioritet** (korisnik 2026-11-11: "na kraju mozda budemo radili i automatsko pokrivanje"): automatsko
pokrivanje OFF-ova serijom iz ponude, kao izbor uz ponudu, nikad umjesto nje.

## 5. Rizici

- Preuzete kocke se odvajaju od QNC-a: popravke iz QNC-a treba svjesno prenositi (zapis odakle je svaka kocka pomaze).
- Laptop bez jake graficke: prijepis i pregled kadrova traju; AI server (QNC-ov ili samostalni) to rjesava.
- Licenca wav2vec2: odluka korisnika; zamjena modela je promjena AI kataloga, ne koda.

## 6. Prvi korak

W0: kostur QNC-AI koji se gradi i pokrece na ovom racunalu.
