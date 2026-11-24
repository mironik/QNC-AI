# QNC-AI - zakon repoa

QNC-AI je samostalna aplikacija: AI wizard koji novinara vodi od kartice do exporta
(`docs/01-wizard-plan.md`). Kod kocki koje vec postoje u QNC-u kopira se ovamo za samostalni rad.

## 1. Odnos prema QNC-u

- QNC i QNC-AI nemaju nikakve veze (korisnik 2026-11-11). QNC-AI ne dohvaca nista iz QNC-a
  (nema git ovisnosti) i u radu ne koristi nista iz mape QNC-a: svoj korijen (ova mapa), svoja
  `data/` (postavke racunala, baze, privremeni JSON kartica), svoji `catalogs/` i `contracts/`.
- Kocka QNC-a koja treba wizardu kopira se u `crates/` (isti naziv, kod s oznake QNC-a) i od tada
  je kocka QNC-AI-a; mijenja se ovdje. Svako kopiranje zapisuje se u odjeljku 4: sto, s koje
  oznake, zasto.
- Zamrzavanja iz `AGENTS.md` QNC-a ne vrijede za QNC-AI. Nacela vrijede:
  - baza je jedina istina i ona je u QNC-AI-u; dok prica nema projekt, ono sto je procitano s
    kartice je privremeni JSON u `data/cards/`;
  - forma je samo raspored; sve sto radi je u kockama s jednom odgovornoscu;
  - svaki natpis ima sifru u rjecniku: wizardov u `contracts/wizard-text`, kopiranih kocki u
    `contracts/text`; engleski je osnovni jezik;
  - testovi, mjerenja i probe nikad ne pisu u stvarni projekt, kartica je samo za citanje;
  - samo Rust, bez Pythona.
- Projekt koji wizard napravi je obican QNC projekt: QNC ga moze otvoriti.

## 2. Odluke korisnika

- 2026-11-11: opcija B, zaseban repo; kocke QNC-a se smiju koristiti i mijenjati ovdje.
- 2026-11-11: wizard radi i na laptopu, ne trazi nista vise od QNC-a.
- 2026-11-11: pokrivalice po ponudi, korisnik bira i postavlja ih markerima ili Sync/B-rollom.
  Automatsko pokrivanje mozda kasnije, nije prioritet.
- 2026-11-11: AI server nije obavezan; moze biti samostalni lokalni server bez QNC-a.
- 2026-11-11: wav2vec2 ParlaSpeech se koristi bez obzira na nenavedenu licencu.
- 2026-11-20: Tray (`apps/qnc-ai-tray`, `contracts/wizard-text`) i Ingest Wizard
  (`apps/qnc-ai-card-ingest`, pozadinski radnik `tools/qnc-ingest-worker`) su samostalni projekti
  i ZAKLJUCANI su (commit ba707ca). Ne mijenjaju se bez izricitog korisnikova otkljucavanja koje ih
  imenuje. Kocka koju dijele s drugim formama (npr. `qnc-transcribe-run`, `qnc-sound-range`,
  kocke u QNC-shared) mijenja se samo ako se time ne mijenja rad traya i Ingest Wizarda; inace
  stati i pitati.
- 2026-11-20: AI Media Assist vracen na kod od 2026-11-19 12:00 (commit 810e1b6, u QNC-shared
  10462cb); rad poslije podne (slozene izjave, potpisi, prozor izjave 10 %) ostaje u povijesti.
- 2026-11-22: korisnik je odlucio nastaviti razvoj u QNC-AI (ne u QNC-pro) i OTKLJUCAO tray i Ingest
  Wizard (`apps/qnc-ai-tray`, `apps/qnc-ai-card-ingest`, `tools/qnc-ingest-worker`). Plan: ispraviti
  greske iz audita (`QNC-pro/docs/08-cjeloviti-audit-qnc-ai-2026-11-22.md`, odjeljak 12) i prenijeti
  iz QNC-pro ono sto je ondje bolje (citanje samo iz kopije, evidencija kartica).

## 3. Rad

- Commit po zavrsenom koraku (testovi prolaze, zapis ovdje i u `docs/razvoj/GGGG-MM-DD.md`),
  push na GitHub na kraju bloka rada. Force push samo uz izricitu dozvolu.
- Izolirani korijen za probu: `cargo run -p qnc-wizard-flow --example iso_root -- <QNC> <novi>`
  (kopija registra i baze aktivnog projekta), zatim `QNC_ROOT=<novi>`.

## 4. Kocke QNC-a preuzete u QNC-AI

Nijedna (2026-11-11).

## 5. Zapis

Izvedeno 2026-11-11 (W0, kostur): Cargo workspace s kockama QNC-a na oznaci `v-2026-11-11`
(git, `.cargo/config.toml` dohvat kroz git ovog racunala). Kocke: `qnc-wizard-text` (rjecnik
wizarda en/hr, jezik racunala iz QNC-ove postavke `ui-language.json`), `qnc-wizard-steps` (10
koraka iz `contracts/wizard-steps.json`, jedno pravilo napredovanja: korak se otvara kad su svi
prije njega gotovi, Dalje trazi gotov prikazani korak, Natrag uvijek), `qnc-wizard-db` (modul
tablica QNC-ova posrednika baze projekta, tablica `wizard_steps`, citanje u pozadini svake 2 s,
pisanje kroz serijskog pisca), `qnc-wizard-application` (aktivni projekt kroz QNC-ov port, bez
crtanja), `qnc-wizard-board` (crtanje: koraci lijevo, prikazani korak, Natrag/Dalje; boje QNC
ljuske iz `contracts/wizard.layout.json`), program `qnc-wizard`. Koraci jos nemaju sadrzaj:
"Oznaci kao gotovo" zamjenjuje uvjet koraka dok se korak ne napravi (W2-W6).

Izmjena 2026-11-11 (korisnik: "wizard bi trebao voditi korisnika, a ne da korisnik bira sto ce
raditi"; "racunalo za montazu treba imati pokrenut daemon u trayu; kad korisnik ubaci karticu, ako
je ima u popisu, pokrece import wizard"): plan prepisan (`docs/01-wizard-plan.md`), izbornik koraka
iz W0 izbacen: koraci su samo traka napretka na koju se ne klika, prvi korak je "Kartica i kadrovi"
(11 koraka). Izvedeno W1: kocka `qnc-tray` (ikona u trayu, izbornik, obavijest, jedan primjerak
programa; Windows adapter kroz windows-sys, drugi sustavi kazu da trayja jos nema), kocka
`qnc-card-watch` (novi disk od zadnjeg pogleda, disk spojen prije prvog pogleda ne pokrece nista;
kartica po QNC-ovom katalogu kamera iz `data/ingest-transport.json`, samo citanje), program
`qnc-ai-tray` (svake 2 s pogled na diskove; kartica poznate kamere: obavijest i `qnc-wizard --card
<disk> --camera <kamera>`; nepoznat disk samo obavijest; nečitljiv disk se pokusava 5 puta), wizard s
`--card` pokazuje karticu i ne cita projekt. Provjereno: testovi (11), lazna Sony kartica u
privremenoj mapi prepoznata stvarnim katalogom, uzivo `subst X:` na laznu karticu i stvarna kartica
D: (FX6) pokrenule su wizard sa svojim diskom. Otvoreno: druga kartica dok je wizard otvoren otvara
drugi wizard (pitanje "dodaj u prilog / novi prilog" je W3); ikona je zadana ikona Windowsa;
automatsko pokretanje s prijavom (W8, uz dozvolu).

Izmjena 2026-11-11 (korisnik: "ne zelim formu vizarda, vizard moze biti sastavni dio traya tako da
nije vidljiv cijelo vrijeme i da ne treba svoju formu"; "klik na tray moze pokazati tray izbornik"):
wizard nema prozor. Uklonjeni program `qnc-wizard`, kocke `qnc-wizard-board` i
`qnc-wizard-application` i ugovor `contracts/wizard.layout.json`. Nova kocka `qnc-wizard-flow` (na
kojem je koraku prica: kartica bez projekta je korak 1, s projektom koraci iz baze projekta; bez
crtanja). `qnc-ai-tray` je wizard: kartica pokrece pricu (obavijest s iducim korakom), izbornik ikone
(klik bilo kojom tipkom) pokazuje gdje je prica (siv redak), "Nastavi: <korak>" (otvorit ce formu
koraka; do W2 samo obavijest) i Izlaz; opis ikone isto. `qnc-tray`: `set_menu`, siva stavka, lijevi
klik otvara izbornik. Provjereno: testovi; uzivo tray na izoliranoj kopiji prepoznaje `subst X:` i
karticu D: bez otvaranja prozora (log). Opazeno: citac FX6 kartice postane citljiv oko 9 s nakon
pokretanja traya, pa ga tray vidi kao tek ubacenu karticu.

Izmjena 2026-11-11 (korisnik je ubacio dvije kartice, izbornik je pokazao samo zadnju): prica drzi sve
ubacene kartice dok nema projekt, izvadena kartica izlazi iz price (`qnc-card-watch` javlja i izvadene
diskove, `Found::Gone`).

Izvedeno 2026-11-11 (korisnik: "treba dodati sigurno izbacivanje kartice"): kocka `qnc-card-eject`
(disk zakljucan, odspojen, uklanjanje dopusteno, medij izbacen, kao "Eject" sustava; disk koji nesto
drzi otvorenim se odbija kao "u upotrebi", nikad na silu; samo korijen diska; Windows adapter).
Izbornik traya ima "Izbaci: <kartica>" za svaku karticu price; izbacivanje u svojoj niti, ishod kao
obavijest. Test samo odbijanje mape koja nije korijen diska (proba uzivo: korisnik sa stvarnom karticom).

Izvedeno 2026-11-11 (korisnik: "info error prozor: vratite karticu (naziv kartice), koristi se"; "to se
mora automatski otvoriti, ili je bolje da se na sredini ekrana pojavi error info overlay"): kartica
price izvadena bez izbacivanja ostaje u prici kao izvadena (`qnc-wizard-flow` `card_out`; kartica je
odredena serijskim brojem diska, pa vracena na drugo slovo je ista) i odmah se otvara prozor greske u
sredini ekrana, iznad svega: "Vratite karticu <naziv> (D:\): koristi se" (`qnc-tray` `alert`,
sistemski prozor poruke). Vracena kartica: obavijest. Izbacena preko izbornika izlazi iz price bez
greske; odbijeno izbacivanje ("u upotrebi") je takoder prozor greske. Naziv kartice je oznaka diska i
slovo.

Popravak 2026-11-11 (korisnik: "ne otvara se na sredini ekrana nego minimizirana"): sistemski prozor
poruke iz programa u trayu Windows ne smije dovesti naprijed. Zamijenjen vlastitim overlayem greske
(`qnc-tray` `alert.rs`): prozor u sredini ekrana, iznad svih prozora, prikazan bez aktiviranja (zato se
pokaze odmah), u bojama QNC ljuske s crvenom trakom i tipkom "U redu". Provjereno snimkom zaslona
(primjer `qnc-tray/examples/alert.rs`).

Popravak 2026-11-11 (korisnik: "kartica se sama ne zatvara kad sam vratio karticu"): prozor greske ima
kljuc (`qnc-tray` `alert(key, ...)`, `close_alert(key)`; isti kljuc zamijeni stari prozor); "vratite
karticu" ima kljuc serijskog broja kartice i zatvara se sam kad je ta kartica opet unutra. Provjereno
primjerom: prozor nestane cim stigne `close_alert`.

Izvedeno 2026-11-11 (W2, prvi dio; korisnik: "B, kreni s W2"): pregled kartice prije projekta u
privremenoj radnoj bazi. Kocka `qnc-card-workspace`: privremeni QNC korijen u privremenoj mapi
sustava (`%TEMP%\QNC-AI\work-*`) s kopijama ugovora i kataloga QNC korijena, njegovim postavkama
racunala (`ingest-transport.json`, jezik, dijagnostika) i jednim pomocnim projektom koji napravi
QNC-ova Project kocka (`qnc-project-store`); brise ga ista kocka (`delete_project` skida zastitu od
brisanja), pa mapu; ostaci zaustavljenog traya brisu se na pokretanju. Kocka `qnc-card-preview`:
korak kartice price, radna baza u pozadini i QNC-ov Ingest kao zaseban program u njoj (`QNC_ROOT` =
radna baza; programi QNC-a iz `QNC_PROGRAMS`, inace mapa traya); kad prica nema kartica i Ingest je
zatvoren, radna baza se brise. Tray otvara Ingest na novu karticu i na "Nastavi". Stvarni projekt se
ne dira. Provjereno: test radne baze (napravljena s aktivnim pomocnim projektom za 0,9 s, obrisana s
projektom). Nije izvedeno: Ingest se sam ne postavlja na karticu (korisnik bira karticu u
pregledniku), oznaceni kadrovi jos ne idu dalje (W3).

Izvedeno 2026-11-11 (W2, drugi dio; korisnik: "bljeska kao da se nesto pokrece, ali nista se ne otvori;
nastavi s automatskim odabirom kartice u Ingestu"): uzrok: QNC-ov `qnc-ingest.exe` u `target/release`
je stara gradnja bez hrvatskog rjecnika i rusi se na postavci jezika `hr`. Novi program `qnc-ai-ingest`:
QNC-ov Ingest iz istih kocki kao QNC-ov (aplikacija, raspored, ploca, form host; oznaka `v-2026-11-11`)
plus kocka `qnc-card-autoselect` koja kroz Ingestove radnje (diskovi racunala, disk kartice po
serijskom broju, Odaberi) sama ide na kartice price redom, kad Ingest zavrsi prethodnu. Tray mu
predaje serijske brojeve kartica (`--card`). Za razvoj `qnc-ai-ingest.exe` se kopira uz QNC-ove
programe (`QNC/target/release`), jer odande pokrece player i pomocne programe; paket u W8.

Popravak 2026-11-11 (korisnik: "bljeskao je 5-6 puta i onda se otvorio; dodavanje druge kartice nije
registrirao"): (1) QNC-ovi pomocni programi (radnik, dekoderi, player) su konzolni i svaki je otvarao
svoj prozor konzole; tray sada pokrece Ingest sa skrivenom konzolom (`CREATE_NO_WINDOW`) koju oni
dijele. (2) Kartice price idu kroz bazu: `qnc-wizard-db` tablica `wizard_cards` (`SetCards`,
`ReadCards`, `project_target(root)`), tray ih upisuje u pomocni projekt radne baze cim se promijene
(`qnc-card-preview` `set_cards`), `qnc-ai-ingest` ih cita svake sekunde i ide i na karticu ubacenu
kasnije (`qnc-card-autoselect` `want`, svaka kartica jednom). Argumenti `--card` vise nisu potrebni.

Izmjena 2026-11-11 (korisnik: "mislim da treba pojednostaviti"; "iingest ima svoju stalnu bazu, svi
drugi citaju iz nje"; "baza je u QNC-AI"; "nemaju nikakve veze QNC i QNC-AI"; "moze biti i privremeni
json dok se ne kreira projekt"; "QNC-AI je samostalan, on samo kopira kod kocki koje postoje za
samostalni rad"): zamjenjuje tri prethodna zapisa (radna baza, `qnc-ai-ingest`, kartice kroz bazu).
Uklonjeni: `qnc-card-workspace`, `qnc-card-preview`, `qnc-ingest-launch`, `qnc-card-autoselect`,
`apps/qnc-ai-ingest` i tablica kartica u `qnc-wizard-db` (git povijest ih cuva). QNC-AI nema git
ovisnost na QNC: 35 kocki QNC-a kopirano je u `crates/` s oznake `v-2026-11-11` (one koje citanje
kartice treba i njihove ovisnosti: `qnc-scanner`, `qnc-ingest-cameras`, `qnc-camera-*`,
`qnc-sony-metadata`, `qnc-source-*`, `qnc-media-metadata`, `qnc-ui-text`, `qnc-dev-diagnostics`,
`qnc-db-broker`, `qnc-active-project-read`, `qnc-work-settings` i ostale koje one povlace;
`cargo tree` ih pokazuje). Uz njih kopirani su katalog kamera (`catalogs/camera-patterns`, revizija
2026.10.22.1), profili kamera (`catalogs/cameras`), katalog formata zapisa i rjecnici QNC kocki
(`contracts/text`); wizardov rjecnik je presao u `contracts/wizard-text`. Korijen u radu je mapa
QNC-AI-a (`data/ingest-transport.json` ovog racunala imenuje katalog kamera; `data/` nije u gitu).
Nova kocka `qnc-card-read`: kartica s popisa cita se samo za citanje (indeks kamere i zapisi svakog
klipa kroz kopirane kocke kamera, bez probea) u privremeni JSON `data/cards/<serijski>.json` (klip:
naziv, original, proxyji, slicica kamere, trajanje; upozorenja). Tray je pokrece u pozadini cim
prepozna karticu i javlja koliko je klipova procitano. Forma uvoza koja pokazuje te klipove sljedeci
je korak. Provjereno: gradnja bez ijedne veze na QNC (`cargo tree`), testovi wizardovih kocki,
`qnc-card-read` (zapis i citanje JSON-a). Na stvarnoj kartici nije provjereno (nije bila spojena).

Izvedeno 2026-11-11 (korisnik: "ne pokrece ingest"): forma uvoza QNC-AI-a. Kocka `qnc-card-pick`
(bez sucelja): klipovi svih procitanih kartica iz njihovih privremenih JSON-a (cita ih ponovno svake
sekunde), posteri sa slicica kamere (kartica samo citana, u pozadini) i oznaceni kadrovi (oznaka
zivi dok program radi; projekt ih dobiva u W3). Program `qnc-ai-ingest` samo crta: traka (kartice,
klipovi, oznaceno, Oznaci sve, Odznaci sve, "Dalje: novi prilog" jos iskljuceno) i mreza kartica
kopiranom kockom `qnc-media-card`; klik na karticu je oznaka. Mjere iz
`contracts/ui/card-pick.layout.json`, boje iz `contracts/ui/shell.layout.json`; jedan primjerak.
Kopirane jos kocke QNC-a: `qnc-media-card`, `qnc-clip-row`, `qnc-media-thumbnail`, `qnc-ui-kit`,
`qnc-key-list`, `qnc-pool-pointer`, `qnc-desktop-theme`, `qnc-board`. Tray otvara formu cim procita
karticu i na "Nastavi" u koraku kartice. `qnc-card-watch`: prvi pogled uzima sve spojene diskove kao
nove (kartica koja je unutra kad se tray pokrene pokrece pricu); disk koji nije kartica tada se samo
zapise. Provjereno uzivo: tray pokrenut s karticama D: (FX6) i G: (FX3A) procitao je 30 i 26 klipova
i otvorio formu; drugo otvaranje nije pokrenulo drugi primjerak.

Izmjena 2026-11-11 (korisnik: "zasto nisi koristio modificirani Ingest koji ima source timeline i
monitor... ne trazim da pises ponovo nego da prilagodimo"; "sto ako su kadrovi na internom disku ili
na mrezi"; "na nivou aplikacije imamo globalni direktorij u kojem se kreiraju ingest baze kao
samostalne jedinice; kada korisnik kreira svoj projekt taj projekt uzima tu bazu za svoju ingest
bazu"): zamjenjuje prethodni zapis (forma `qnc-ai-ingest`, `qnc-card-pick`, `qnc-card-read`,
privremeni JSON; uklonjeni). QNC-ov Ingest kopiran je cijeli: program `apps/qnc-ingest`, kocke
Ingesta, playera, monitora, source timelinea, preglednika (Racunalo/LAN/Intranet), pozadinskog
radnika i njihove ovisnosti (ukupno 218 kocki u `crates/`), programi `tools/qnc-player-runner`,
`tools/qnc-ingest-worker`, `tools/qnc-decoder-ffmpeg`, `tools/qnc-program-output-host`, te
`contracts/`, `catalogs/` i `seed/` QNC-a s oznake `v-2026-11-11`. FFmpeg (LGPL, build
`n8.1.3-17-gc389840894-20261011`) uz programe iz arhive na ovom racunalu
(`C:\Users\miron\tools\qnc-ffmpeg-archive`), ne iz QNC-a. Nova kocka `qnc-ingest-base`: Ingest baze
aplikacije u `data/ingest`, svaka samostalna jedinica koju napravi kopirana Project kocka
(`qnc-project-store`): vlastita mapa i baza s radnim postavkama izabranog templatea, aktivna dok
Ingest radi u njoj; trenutna je aktivna Ingest baza, inace se napravi nova. Tray je postavi pa
pokrene Ingest (skrivena konzola za pomocne programe, jedan primjerak koji je tray pokrenuo).
Provjereno uzivo: tray s karticama D: i G: napravio je Ingest bazu i otvorio Ingest nad njom. Nije
izvedeno: Ingest se sam ne postavlja na karticu; projekt price jos ne zapisuje svoju Ingest bazu (W3);
izrada baze traje oko 4 s na niti traya.

Izvedeno 2026-11-11 (korisnik: "ingest cim se ucita skenira karticu, ne ceka korisnika; ako se ubaci
nova kartica i ona se skenira"): kopirani Ingest (`qnc-ingest-desktop` `IngestApp`) sam ide na svaku
spojenu karticu poznate kamere. Nova kocka `qnc-attached-cards` (u pozadini svake 2 s: spojeni
diskovi koji su kartice po katalogu kamera, samo citanje); vracena kocka `qnc-card-autoselect`
(kroz Ingestove radnje: diskovi racunala, disk kartice, Odaberi; sljedeca kartica kad Select
prethodne zavrsi; svaka kartica jednom). Bljeskanje: kopirana Project kocka pokrece `icacls` i
`attrib` bez prozora konzole, mapa Ingest baza zakljucava se jednom. Provjereno uzivo: s karticom
G: (FX3A) Ingest se otvorio, sam otvorio disk kartice i pokrenuo Odaberi; Ingest baza je dobila
zapise, pozadinski radnik je radio.

Izmjena 2026-11-11 (korisnik: "u ingestu se cijelo vrijeme provlaci greska, koristimo windows
oznacavanje diskova, a aplikacija mora biti multi OS neutralna"; "globalni direktorij ingest baza
dodati kao direktni izbornik"; "u ingest bazu dodati naziv projekta, to ce dopisati projekt forma"):
(1) `qnc-source-bindings` `volume_label`: spojeni disk se imenuje svojom oznakom (volume label),
bez nje "Disk <serijski>" (rjecnik `source.volume_unnamed`), nikad slovom; i registrirani izvor
dobiva taj naziv kad je spojen. Put mape u pregledniku vec koristi naziv izvora. (2) Izbornik Baze
uz Racunalo/LAN/Intranet (`SourceKind::Bases`, radnja `ingest_source_kind_bases`, Alt+4 u QNC
presetu, ugovor rasporeda `kinds`, `empty_bases`, fokus): popis Ingest baza iz `data/ingest` (aktivna
s oznakom, uz svaku projekt koji ju je uzeo ili "bez projekta"); klik ili Odaberi je otvara kao bazu
u kojoj Ingest radi (`qnc-ingest-base` `BaseBrowse`: popis i otvaranje izvan niti sucelja; Ingest
zatim ucita bazu kao projekt). (3) Ingest baza ima tablicu `ingest_base_project` (jedan red: naziv
projekta, vrijeme); `qnc_ingest_base::set_project_name` je zove Project forma kad projekt uzme bazu
(W3). Provjereno: testovi `qnc-source-bindings`, `qnc-ingest-base`, `qnc-ingest-application`,
`qnc-keyboard-shortcut`, `qnc-ingest --check-contracts`. Uzivo nije provjereno.

Izvedeno 2026-11-11 (W3, prvi dio; korisnik: "nakon Selektiraj sve kada klikne UVEZI mora se otvoriti
overlay obrazac za kreiranje projekta; kad se kreira projekt naziv projekta se dodaje u ingest
bazu"): nova kocka `qnc-story-start` (bez sucelja): Uvezi u Ingest bazi bez projekta otvara obrazac
novog projekta (kopirani QNC obrazac priloga: `qnc-newsroom-sheet`, `qnc-newsroom-sheet-model`,
`qnc-settings-form`, `qnc-field-spec`); Kreiraj u pozadini napravi projekt price
(`qnc-ingest-base` `take_for_story`: kopirana Project kocka, mapa `projects/`, ne `data/ingest`;
projekt u svojoj bazi zapise koju je Ingest bazu uzeo, tablica `story_ingest_base`; baza dobije
naziv projekta; Ingest ostaje u bazi), pa uvoz krene; Odustani ne uvozi nista. Baza koja vec ima
projekt uvozi odmah; projekt koji nije Ingest baza (`active_is_base`, brzo citanje registra) uvozi
kao prije. Dok je obrazac otvoren, tipke idu obrascu, ne Ingestu. Radnje `story_sheet_confirm`,
`story_sheet_cancel`. Naziv price ne smije pocinjati s "Ingest". Provjereno: testovi
`qnc-ingest-base` (projekt price uzima bazu, baza ostaje aktivna i zna naziv),
`qnc-ingest-application`. Uzivo nije provjereno. Nije izvedeno: vrijednosti obrasca jos se ne
spremaju u bazu projekta (samo naziv).

Dopuna 2026-11-11 (korisnik: "QNC-ov NEW PROJEKT otvara, ali u njega treba dodati drop down izbornik
templatea"; "templatei se citaju iz baze"): obrazac novog projekta (kopirani `qnc-newsroom-sheet`,
`qnc-newsroom-sheet-model`) ima zadnji red Template: padajuci izbornik templatea iz registra
QNC-AI-a (`qnc-ingest-base` `templates`: `list_project_templates`, izabran je onaj koji baza drzi
izabranim), strelice ga mijenjaju kao red Obrazac; projekt price se kreira s izabranim
templateom (`take_for_story(.., template_id)`). Pokusaj otvaranja cijele Project aplikacije na
Uvezi napusten na zahtjev korisnika (nije commitan).

Izvedeno 2026-11-11 (korisnik: "iz QNC project izdvoji Postavke kao samostalni obrazac, u trayu dodaj
link Postavke; to se veoma rijetko koristi zato treba biti odvojen od wizarda"; "nedostaju ti dropdown
izbornici"): (1) Kopirana QNC Project forma (`qnc-project-desktop`, `qnc-project-blocks`,
`qnc-project-session`, `qnc-project-application`, `qnc-project-jobs` i njihove kocke) ima nacin samo
postavke (`ProjectApp::settings_only`, `qnc_project_blocks::show_settings`: desna strana preko
cijelog prozora, bez popisa projekata i bez Novi projekt, Advanced otvoren). Novi program
`qnc-ai-settings` (jedan primjerak) je otvara u korijenu QNC-AI-a; tray ima stavku Postavke. (2)
Obrazac priloga bez padajucih izbornika: QNC-AI nije imao izabran obrazac pa se otvarao prvi po
abecedi (agencija/online, bez polja s izborom); postavka racunala `data/story-sheet.json` sada bira
`commercial` kao QNC. Provjereno: Postavke se otvaraju (prozor "QNC AI: Postavke").

Izvedeno 2026-11-11 (korisnik: "u postavke treba dodati lokaciju globalnih baza, a projektna baza ide u
projektni direktorij kao i u QNC"; "lokacija ingest baza nije ista kao i lokacija projekta"): Postavke
imaju trece polje lokacije "Ingest baze (globalne)" s istim preglednikom kao lokacija projekata i
mapa exporta (`ProjectBrowserTarget::BasesRoot`, radnje `project_pick_bases_root` i
`project_bases_root_browser_*`; otvoren je samo jedan preglednik). Izbor se pamti kao postavka
racunala `data/ingest-bases.json` (`qnc-ingest-base` `folder`, `set_folder`; bez nje `data/ingest`).
Ingest baza se kreira u toj mapi, a lokacija projekata iz Postavki se vrati kakva je bila; projekt
price se kreira u lokaciji projekata iz Postavki (njegova baza u mapi projekta, kao u QNC-u), vise
ne u `projects/` koju je wizard sam postavljao. Lokacija projekata koju je prijasnja verzija
ostavila na mapi baza vraca se na `projects` (`keep_projects_apart`, i pri otvaranju Postavki).
Test: baza u mapi baza, baza price u mapi projekata, lokacija projekata nepromijenjena, druga mapa
baza. Napomena: jedna izmjena komentara u `qnc-ingest-base` napravljena je `sed`-om umjesto Edit
alatom (pravilo: samo Edit).

Pravilo 2026-11-11 (korisnik: "kod mora biti odvojen od GUI kako bi se GUI mogao naknadno mijenjati
preko GUI tools"): sve sto radi je u kockama bez sucelja (bez egui/eframe); forma samo crta prikaz
koji joj kocka da i salje radnje natrag. Kocka koja crta ne cita bazu, karticu ni datoteke.

Izvedeno 2026-11-12 (korisnik: "obriši staru bazu"; "pogledaj možeš li diskove razlikovati još po čemu,
jer glupo je dvaput pisati isti broj"): (1) `qnc-ingest-base` `forget(root, id)` brise Ingest bazu s mapom
(kopirano Project spremiste skida zastitu), nikad projekt price; primjer `examples/forget_base.rs`. Stara
baza "Ingest 2026-11-11 21:21:08 UTC" obrisana (Ingest je prije toga zaustavljen; sljedeca kartica
pravi novu bazu). (2) `qnc-source-volumes` `describe(root)`: vrsta diska (kartica/USB, disk, mrezni,
opticki) i velicina, samo citanje. Disk bez oznake zove se po vrsti ("Kartica / USB", "Disk") umjesto
"Disk <serijski>" (`qnc-source-bindings` `volume_label`, `kind_label`); serijski je u svom stupcu, a
treci stupac preglednika je velicina kao na naljepnici (decimalni GB, `size_label`; `qnc-ingest-select`
`browser`), ne ponovljena oznaka. Rjecnik `source.kind_*` umjesto `source.volume_unnamed`. Testovi
`qnc-source-volumes`, `qnc-source-bindings`, `qnc-ingest-select`. Uzivo nije provjereno.

Izvedeno 2026-11-12 (korisnik: "nazovi karticu po kameri"; "to može biti dodatni podatak"): uz velicinu
diska preglednik pokazuje kameru kartice ("Sony ILME-FX3A · 128 GB"); naziv i serijski ostaju.
`qnc-card-watch` `camera_of`: indeks i prvi zapis klipa kartice (samo tekst) i profili kamera
(`catalogs/cameras`, citani bez upisa u bazu) daju model; bez prepoznatog modela proizvodac rasporeda.
`qnc-attached-cards` (pozadinska nit, svake 2 s) daje karticu s kamerom, kamera se cita jednom po kartici;
`qnc-ingest-application` `set_cameras` je dopisuje u popis diskova. Izmjereno (primjer
`qnc-card-watch/examples/cameras.rs`, samo citanje): D: Sony ILME-FX6V 2,3 s, G: Sony ILME-FX3A 1,1 s.
Nalaz uz pitanje korisnika o nazivu i proizvodacu kartice: obje kartice nemaju oznaku volumena; sustav za
D: (CFexpress u USB citacu) daje proizvodaca i model kartice "DAJINGYU CX256GCFA", a za G: (SD) samo
citac "Generic- SD/MMC/MS PRO". Uzivo nije provjereno.

Izvedeno 2026-11-12 (korisnik: "kad na ingestu (ako ima selektiranih kadrova) korisnik klikne UVEZI,
ingest se zatvara, a Novi Projekt se otvara; kad se popunjena forma Novi Projekt zatvori, otvara se AI
Media Assist"): zamjenjuje obrazac preko Ingesta od 2026-11-11. Uvezi odmah upise odabir i probudi
pozadinski uvoz (radnik radi bez Ingesta); kad je odabir predan radniku, Ingest otvara program
`qnc-ai-new-project` i zatvara se. Taj program (`apps/qnc-ai-new-project`) crta QNC-ov obrazac novog
projekta (`qnc-newsroom-sheet`, s izborom templatea) u svom prozoru preko kocke `qnc-story-start`;
Kreiraj pravi projekt price koji uzima bazu i otvara AI Media Assist, Odustani zatvara prozor; baza koja
vec ima projekt (ili aktivni projekt koji nije baza) odmah otvara AI Media Assist. Pokretanje sljedeceg
programa je nova kocka `qnc-ai-open` (forma ne pokrece procese, test Ingesta). AI Media Assist kopiran iz
QNC-a (oznaka v-2026-11-11): `apps/qnc-media-assist-audio-ai` i 71 kocka koja je falila
(`qnc-editorial-*`, `qnc-panel-focus`, `qnc-script*`, `qnc-transcript*`...); `--check-contracts` prolazi.
Iz Ingesta uklonjeni obrazac, `story_sheet_*` radnje i ovisnosti `qnc-story-start`,
`qnc-newsroom-sheet(-model)`. Testovi `qnc-ingest-application` (31), `qnc-ai-open`, `qnc-story-start`.
Uzivo nije provjereno.

Popravak 2026-11-12 (korisnik: "sad je s ingesta preskočilo na AI Media assist"): aktivna baza vec je
imala projekt (napravljen ranije starim obrascem), pa je "Novi projekt" otisao ravno u AI Media Assist.
`qnc-ingest-base` `current`: baza koju je projekt preuzeo ne sluzi sljedecem Ingestu, tray pravi novu.
Test prosiren; primjer `examples/list_bases.rs` (samo citanje).

Izvedeno 2026-11-12 (korisnik: "treba onda u obrascu Novi projekt dodati Option dugme Aktivni projekt ili
novi projekt; ako projekt postoji onda korisnik može nastaviti dalje ili kreirati novi projekt"): kad je
prije postojao projekt price, prozor "Novi projekt" prvo pita (`qnc-choice-box`): "Nastavi: <projekt>",
"Novi projekt", "Odustani". Nastavi pridruzuje bazu tog Ingesta postojecem projektu i otvara AI Media
Assist; Novi projekt otvara obrazac. `qnc-ingest-base`: `last_story` (projekt koji je preuzeo najnoviju
drugu bazu), `continue_story`; projekt price pamti sve svoje baze u `story_ingest_bases` (vise redova,
umjesto jednoredne `story_ingest_base`, koja ostaje neiskoristena u starim projektima). `qnc-story-start`
`offer`, `go_on`, `new_story`. Tekstovi `new_project.*` (en, hr). Testovi `qnc-ingest-base`. Uzivo nije
provjereno.

Izmjena pristupa 2026-11-12 (korisnik: "globalna baza kao ingest na velikim ingest sustavima: korisnik
ubaci karticu, tray provjeri poznaje li karticu i odmah radi Select All i ingestira materijal po osnovnim
postavkama (link media); ingest proces koji pokrece tray je kopiranje postera, filmstrip i gruba wave";
"ingest formi je uvijek izvor globalna baza"; "info prozor Ingest proces u tijeku, kada zavrsi daje dugme
Pokreni QNC"; "obrazac Novi projekt tretiraj kao overlay prozor Ingest forme"; "projekt koristi dvije
baze: globalnu ingest bazu (vrsta media servera, izvorni podaci) i lokalnu projekt bazu (ono sto mi radimo
s tim podacima)"). Zamjenjuje zaseban prozor "Novi projekt" od istog dana (`apps/qnc-ai-new-project`
uklonjen) i Ingest koji sam skace po karticama. (1) Tray na poznatu karticu pokrece
`apps/qnc-ai-card-ingest` (jedan primjerak): kocka `qnc-card-ingest` vozi Ingest aplikaciju bez prozora
(Select svake kartice u globalnu bazu, Odaberi sve, uvoz po postavkama, `import_into_base`), radnik kopira
postere i radi filmstrip i val; prozor pokazuje "Ingest u tijeku", kamere, broj klipova, a kad radnik
stane gumb "Pokreni QNC" otvara Ingest formu. (2) Ingest forma vise ne prati kartice; cita aktivnu
globalnu bazu. Uvezi s odabranim klipovima pita pricu preko forme (panel `qnc-story-overlay`: Nastavi /
Novi projekt / Odustani, pa QNC-ov obrazac s templateom); odabrani klipovi idu u projekt
(`qnc-ingest-base` `take_clips`, tablica `story_clips` u bazi projekta, izvorni podaci ostaju u globalnoj
bazi; `story_clips`, `story_id`), zatim se otvara AI Media Assist i Ingest se zatvara. Testovi
`qnc-ingest-base`, `qnc-ingest-application` (31). Nije izvedeno: AI Media Assist jos radi nad aktivnom
globalnom bazom (svi njezini klipovi), a ne nad projektom s dvije baze; prozor Postavki (preklapanje).
Uzivo nije provjereno.

Dopuna 2026-11-12 (korisnik: "puni wave ne cekati, ujedno potreban je progres bar po funkciji koju
trenutno radi"): pozadinski ingest je gotov kad svaki klip baze ima poster, filmstrip (status ready) i
barem grubi val; puni val radnik dovrsava sam. `qnc-card-ingest` `Progress` (klipovi, posteri,
filmstripovi, valovi) cita iz baze jednom u sekundi u svojoj niti (`qnc-content-read` `clip_posters`,
`qnc-artifact-db`); "gotovo" tek nakon prvog pregleda kartica i Selecta svake. Prozor
`qnc-ai-card-ingest` ima traku napretka za Postere, Filmstrip i Val (n / ukupno). Uzivo: kartica D:
(30 klipova) Selecting pa Done, 30/30/30/30.

Brisanje 2026-11-12 (korisnik: "odabrana je pogresna putanja za globalne i lokalne baze, to je u razvojnom
direktoriju; izbrisi sve postojece kreirane globalne i lokalne baze"): sve globalne baze i projekti
obrisani (primjer `qnc-ingest-base/examples/forget_all.rs`, kroz Project spremiste koje skida zastitu).

Popravak Postavki 2026-11-12 (korisnik: "postavke nisu u funkciji"; "ne radi ni kreiranje template"):
lokacije (projekti, Ingest baze, export) su u Postavkama prve i uvijek vidljive; upisana putanja sprema
se na Enter ili izlazak iz polja (`ProjectIntent::CommitLocation`, `ProjectSession::commit_typed`).
Predlozak nije isao jer QNC-AI nije imao katalog aplikacija (`data/application-catalog.json`, trazi
aplikaciju grupe a); kopiran alat `tools/qnc-app-catalog`. AI napomene u Postavkama prelamaju se.
Snimci zaslona iz PowerShella bez DPI svijesti bili su odrezani: prozor je stao u sirinu.

Kopija cijelog QNC-a 2026-11-12 (korisnik: "mozda ti je najlakse kopirati cijeli QNC projekt u QNC-AI i na
njemu raditi prepravke"): kopirano 52 sto je falilo (shell `qnc-app`, Project, Story, S-Story, MA Script,
MA Audio, MA Video, desktop adapteri, AI programi i alati) s oznake v-2026-11-11; vec prepravljeno u
QNC-AI zadrzano; `tools/diagnostics` (Python i JS) izostavljen. Katalog aplikacija ima svih osam (Project a).

Zajednicki AI direktorij 2026-11-12 (korisnik: "za AI posebni direktorij koji mogu pozivati obje verzije
aplikacija"): modeli su u `C:\Users\Public\QNC-AI-share\models`, katalog AI-ja ih imenuje `share:models/...`
(`qnc-ai-catalog` `share_dir`, `model_path`; AI paket pod `share/`). Ista izmjena u QNC-u. AI biblioteke uz
programe (onnxruntime, sherpa-onnx, rife, vcomp) u `target/release` su tvrde veze na QNC-ove. Debug build
obrisan u oba projekta (korisnik: "ocisti nepotrebne kes i debug datoteke").
Dopuna (korisnik: "direktoriji ne smiju biti hardkodirani"): AI direktorij je postavka racunala
`data/ai-share.json` (`qnc-ai-catalog` `share_dir`, `set_share`, `chosen_share`; bez postavke greska, bez
zadane mape). Postavke imaju polje "AI direktorij (zajednicki)" (`LocationTarget::AiShare`,
`ProjectBrowserTarget::AiShare`, radnje kataloga `project_pick_ai_share` i `project_ai_share_browser_*`,
`ProjectSession::keep_computer_folder` za Ingest baze i AI direktorij). Upisano `C:\Users\Public\QNC-AI-share`.

Otvoreno (korisnik: "direktorij zajednickih kocki da ono sto je zajednicko ne bude duplicirano; jedino GUI
verzije su drugacije"; "korisniku izbor nacina rada: kroz Wizard ili klasicni nacin"; "tray bi koristila i
jedna i druga verzija"): predlozen jedan repozitorij (QNC-AI glavni, QNC arhiv) s jednim skupom kocki, dva
nacina rada i zajednickim trayem; ceka odluku.

Dopuna 2026-11-12 (isto pravilo, "direktoriji ne smiju biti hardkodirani"): mapa Ingest baza nema zadanu
vrijednost (`data/ingest` uklonjen, `BASES_DIR`): `qnc-ingest-base` `folder` vraca gresku kad nije izabrana u
Postavkama, pa tray ne radi ingest i javlja to; `keep_projects_apart` vise sam ne postavlja projekte u
`projects` korijena, nego javlja da se mapa projekata mora izabrati. Test prosiren. Ostaje: Project spremiste
(QNC-ov kod) ima svoju zadanu mapu projekata kad registar nema izbor.

Izvedeno 2026-11-12 (korisnik: "kocke su shared-zajednicke... ako neka aplikacija trazi promjenu kocke, orginalna
ostaje, a ta aplikacija u svoj direktorij ukljucuje promjenjenu kocku"; "GUI je samostalan po aplikaciji, AI je
shared"; "1, ugovori zajednicki"): zajednicke kocke QNC-a i QNC-AI-ja su u repou `C:\Users\miron\Projects\QNC-shared`
(247 kocki istih u oba repoa, ne GUI, s ugovorima i katalozima koje ugradjuju ili im testovi citaju). Ovaj repo ih
uzima kao path ovisnost (`qnc-x = { path = "../QNC-shared/crates/qnc-x" }` u korijenskom `Cargo.toml`); premjestene
kocke uklonjene iz `crates/`. Kocka koju wizard treba drukciju: kopija u `crates/` ovog repoa i ovisnost na nju,
original ostaje u QNC-shared. Conformance kopiran iz QNC-a (vidi kocke izvan korijena). Wizard i dalje krsi pravila
koja conformance provjerava (Ingest ovisi o Project spremistu i pricama, `qnc-ingest-base` i `qnc-wizard-db` pisu
bazu same, nedostaju docs/07 i docs/11), od prije. Detalj u AGENTS.md QNC-a. Provjereno: `cargo check --workspace`,
release gradnja s `libav`, testovi (pada samo `without_a_lease_and_without_its_executable_launching_says_so`, okolina).

Izvedeno 2026-11-12 (korisnik: "tray je jedan"; "u QNC ingestu dodati izbor ulaza Baza... tretira se kao kartica"):
klasicni QNC Ingest ima tab Baze s karticama baza koje je napravio ovaj tray; klik otvara karticu, Select je kupi u
QNC projekt. Mapa baza je jedna postavka racunala (`data/ingest-bases.json`), koju sada cita zajednicka kocka
`qnc-base-cards`; `qnc-ingest-base` vise nema svoju kopiju (`folder`, `set_folder` su njezini). Detalj u AGENTS.md QNC-a.
Izvedeno 2026-11-12 (korisnik: "na info prozoru za svaku radnju dodaj ikonu X što obustavlja taj proces, ako korisniku
nije potreban"; "dodaj u tray brisanje baze"): info prozor uvoza kartice ima uz svaku traku X. Obustavljene funkcije su
jedan zapis baze baze (`ingest_runtime` kljuc `stopped_work`, kroz javni pisac), nova kocka `crates/qnc-stopped-work`
(+ ugovor modula); radnik (`tools/qnc-ingest-worker`) ga cita svake sekunde: svaka funkcija ima svoju zastavicu koja je
njezin cancel (uvoz, posteri, filmstrip i val (radnici se ponovno slazu bez obustavljene vrste), AI kadrovi, sadrzaj,
opisi), obustavljena se ne pokrece; napravljeno ostaje; obustava vrijedi za tu bazu. Prozor ne ceka obustavljenu
funkciju (`Progress::complete`). Tray ima "Obrisi Ingest bazu": pitanje Da/Ne (`qnc-tray` `confirm`), zaustavi uvoz pa
obrise bazu i njezinu mapu (`qnc_ingest_base::forget`), kartica se ne dira. Uzivo nije provjereno.
Izvedeno 2026-11-12 (korisnik: "nisam obrisao bazu, traj je ponovo pokreno ingest u novom projektu... to treba
onemogućiti"; snimatelj nije obrisao karticu prije novog snimanja; "glupo je ponovo uvoziti ono što je uvezeno"):
tray prije uvoza provjerava je li kartica vec u nekoj Ingest bazi. Originali na kartici (`qnc-card-watch`
`card_originals`, samo citanje) usporeduju se s originalima baza (`qnc-content-read` `clip_originals_of_file` u
QNC-shared, jedini citac kataloga); `qnc-ingest-base` `known_card`: najnovija baza koja ima barem jedan original
kartice i sto je na kartici novo. Kartica u bazi u kojoj Ingest radi sada (koju prica nije uzela) ide kao prije.
Inace: bez novih klipova info prozor (8 s), nista se ne uvozi i baza se ne pravi; s novima pitanje Da / Ne / Odustani
(`qnc-tray` `ask`, `Answer`): Da dodaje nove u tu bazu (red uvoza uzima samo neuvezene), Ne pravi novu bazu samo s
novima (`new_with_only_new`, zapis `ingest_base_only_new` u bazi; prozor uvoza nakon "oznaci sve" odznaci klipove
koje stara baza ima, `not_taken`, i broji samo nove), Odustani ne radi nista. Stari klipovi ostaju u katalogu nove
baze neuvezeni (Select cita cijelu karticu). Test `a_card_a_base_has_is_known_and_a_base_of_its_new_clips_leaves_the_old`.
Uzivo: kartica D: prepoznata u bazi 22:55:02 (30 klipova, nema novih), nova baza nije napravljena. Pitanje s novim
klipovima nije provjereno uzivo.
Izvedeno 2026-11-12 (korisnik: "trebala bi ponuditi otvaranje postojećih projekata u vlasništvu korisnika... iz stare
QNC forme Projects preuzeti panel koji je prikazivao arhivu projekata (koji nisu obrisani)"; "u traju možemo dodati
link na zadnji projekt"): nova forma `apps/qnc-ai-projects` (exe `qnc-ai-projects`, jedan primjerak): popis projekata
iz kopirane Project forme (`qnc-project-blocks` `show_projects`, `qnc-project-desktop` `projects_only`, `take_opened`),
samo price koje je napravio korisnik ovog racunala (`qnc-ingest-base` `my_stories`: vlasnik iz zapisa porijekla, bez
Ingest baza; `qnc-project-session` `only`). Pretraga i X za brisanje kao u Projectu. Otvaranje (klik, Enter):
`qnc-ingest-base` `open_story` vraca najnoviju Ingest bazu te price kao bazu u kojoj se radi (prica bez baze postaje
aktivna sama), pa se otvara AI Media Assist kao nakon Uvezi, a forma se zatvara. Tray: stavke "Zadnji projekt: naziv"
(`last_story`, cita se svakih 10 s; otvara isto) i "Projekti"; kartica vec uvezena bez novih klipova pita "Otvoriti vase
projekte?" (Da otvara formu Projekti). Uzivo: forma se otvorila, nasla 1 projekt korisnika; otvaranje projekta i
stavke traya nisu provjerene uzivo.
Popravak 2026-11-12 (korisnik: "transkript najduže izjave napravljen je značajno lošije nego u QNC, puno nedostaje";
"kad je riječ o miši... taj tekst"): u bazi 22:03 prijepisu Mironik 3608 falilo je 1444-4266 (56 s). Red prijepisa je
dugi klip dvaput prekinuo za kraće (1463, 4268); nastavak usred klipa slusa sekundu sire (`padded`) i u tom nacinu
`qnc-transcribe-run` je retke davao tek na kraju, pa je dio koji red zapise pri ustupanju (`keep_part`) bio bez
njih, a sljedeci nastavak krenuo je iza njega. Sada se retci daju cim su cuti, odrezani na trazeni raspon
(`lines_cut`). Postavke prijepisa (CH1, hr, Parakeet, prag) iste su kao u QNC-u. Isti klip u bazi 22:55 (jedno
ustupanje) ima potpun tekst. Primjer `qnc-transcript-db/examples/line_words.rs` (samo citanje) u QNC-shared.
Prijepis u bazi 22:03 nije napravljen ponovno.
Izmjena 2026-11-12 (korisnik: "timecode broji trajanje od početka, a trebao bi davati timecode playholdera, a ne
trajanje; njega može davati kao informaciju; za izjave je bitan timecode"): traka source timelinea (zajednicka kocka
`qnc-source-dock` u QNC-shared, vrijedi i za klasicni QNC) prvo pokazuje TC playheada (timecode izvora), pa IN, OUT i
Trajanje kao do sada. Aplikacije izgradjene 2026-11-13 (desktop i Story s `libav`).
Ponovni prijepis 2026-11-13 (korisnik: "izgradi i prepiši 3608"): Mironik 3608 u bazi 22:03 prepisan je ponovno
kroz isti red prijepisa (primjer `qnc-transcribe-queue/examples/again.rs`, kopiran uz aplikaciju jer red trazi
programe uz sebe), 105 s u jednom prolazu. Tekst 1444-4266 ("kad je riječ o miši...") je tu; ostale stanke 2-3 s.
Popravak 2026-11-13 (korisnik: "u tab Prijepisi stavio si i klipove koji nemaju prijepis"): uvoz kroz tray prepise
svaki klip, a klip bez govora dobije prazan prijepis (0 redaka). Tab Prijepis uzimao je svaki klip s prijepisom.
Sada uzima samo klipove ciji prijepis ima rijeci (`qnc-transcript-db` `read_spoken`, nova operacija `ReadSpoken` u
QNC-shared; `qnc-transcript`). Radnik i dalje koristi `read_transcribed` (klip bez govora se ne prepisuje ponovno).
Test prosiren. AI Media Assist izgradjen 2026-11-13.
Izmjena 2026-11-13 (korisnik: "početni TC, položaj playholdera, završni TC i prikaz trajanja odabranog dijela klipa
(novog virtualnog SubClipa), njegovo trajanje, ne TC"; "nepotreban je prikaz IN/OUT, on je zapisan u bazi";
"povećaj prikaz nivoa tona, prekratak je, na svim formama istih dimenzija"): traka source timelinea (`qnc-source-dock`
u QNC-shared) iza naziva klipa pokazuje TC pocetka klipa, TC playheada, TC kraja klipa (i kad je izjava povecana:
`SourceTimeline::with_clip_frames`, `qnc-source-timeline-view`) i trajanje dijela IN-OUT kao trajanje
(sekunde:slike, od minute minute:sekunde:slike), bez IN/OUT; zamjenjuje "Trajanje do playheada" od 16. 10. Mjerac
razine (`qnc-level-bar`) je 200 px (prije 78 px), gumbi NORM, AI, ZADRZI isti; jedna kocka, iste mjere u svakoj formi.
Klasicni QNC dobiva istu traku (bez `with_clip_frames`: kraj je kraj prikazanog dijela).
Dopuna 2026-11-13 (korisnik: "da ne bi netko krivo shvatio duljinu odabranog dijela, prikaz broja frejmova neka bude
druge nijanse boje"): u trajanju je broj slika (iza zadnje dvotocke) u boji fokusa teme timelinea.
Izmjena 2026-11-13 (korisnik: "moraš formama zadati da se otvore jer se ne vidi dio UI"): Ingest, AI Media Assist,
MA Script, MA Audio, MA Video, S-Story i Story otvaraju se maksimizirane (`with_maximized(true)`, kao desktop i Project).
Popravak istog dana (korisnik: "i dalje se otvara djelomično"): forme su vec imale `with_maximized(true)`, ali eframe
po zadanom pamti zadnju velicinu prozora i vraca je pri otvaranju; sada `persist_window: false` kao desktop.
Popravak 2026-11-13 (korisnik: "i dalje ne prikazuje na punom ekranu"; izmjereno: AI Media Assist 1294x798 na ekranu
1536x864, `IsZoomed` false): zastavica `with_maximized` u graditelju prozora na Windowsima nije bila dovoljna. Forme
pri pokretanju salju `ViewportCommand::Maximized(true)`, a pocetna velicina 1280x760 je uklonjena. Provjereno
pokretanjem MA Video: `IsZoomed` true, prozor preko cijelog ekrana.
Rjesenje istog dana (korisnik: "najprije se otvorio manji pa maksimizirao"; "to je zakrpa, nije rješenje"): uzrok je u
winitu. eframe drzi wgpu prozor skrivenim do prvog crtanja, a winit ga prikazuje sa `SW_SHOWNOACTIVATE`, sto na
Windowsima prikaze prozor u njegovoj normalnoj velicini i tek ga onda opet maksimizira. Nova kocka
`crates/qnc-form-window`: normalna velicina prozora forme je radna povrsina ekrana (`SPI_GETWORKAREA`, procitana prije
nego winit ukljuci svijest o mjerilu ekrana, dakle u logickim tockama), uz maksimiziranje i bez pamcenja zadnje
velicine; normalni i maksimizirani prozor su isti, nista manje se ne prikazuje. Sedam formi (Ingest, AI Media Assist,
MA Script, MA Audio, MA Video, S-Story, Story) predaju joj samo naslov i aplikaciju (`qnc_form_window::run`); naredba
`Maximized` i skrivanje uklonjeni. Izmjereno (MA Video, svakih 25 ms): prozor se pojavi 1550x878 i odmah je
maksimiziran. Ostali OS: samo maksimiziranje (radna povrsina se zasad cita samo na Windowsima).
Ispravak istog dana (korisnik: "i dalje se pokaže manji prazan prozor a onda maximize"; izmjereno svim prozorima
procesa bez pauze: prikazan maksimiziran, skriven, prikazan u normalnoj velicini i prazan, skriven, prikazan):
prozor stvoren kao maksimiziran Windows odmah prikaze (`SW_MAXIMIZE`), eframe ga skriva do prvog crtanja, winit ga
ponovno prikazuje u normalnoj velicini. `qnc-form-window` vise ne stvara prozor maksimiziran: stvara ga na mjestu i u
velicini radne povrsine, eframe ga prikaze jednom, nacrtan, a dvije slike kasnije trazi se `Maximized`. Izmjereno: prozor
se pojavi jednom (1550x902), za 112 ms je maksimiziran (1550x878); bez skrivanja i bez manjeg prozora.
Dopuna istog dana (korisnik: "uskladi normalni prozor s maksimiziranim"): normalni prozor dobiva mjesto koje Windows
daje maksimiziranom: okvir tik izvan radne povrsine, unutrasnjost = radna povrsina ispod naslovne trake. Okvir i
naslovna traka citaju se pri stvarnom mjerilu ekrana (`AdjustWindowRectExForDpi`, nit privremeno svjesna mjerila;
pri 125 % okvir je 9 px, a ne 8 tocaka x 1,25) i daju u tockama. Izmjereno u fizickim pikselima: normalni i
maksimizirani prozor 1938x1098 na -9,-9, unutrasnjost 1920x1051; maksimiziranje ne mijenja nista vidljivo.
Izmjena 2026-11-13 (korisnik: "info prozor importa ostaje istih dimenzija, nije dio desktopa; dok se odvija import
korisnik moze raditi nesto drugo; ako ga minimizira, kad import zavrsi treba se otvoriti iznad drugih aplikacija"):
prozor uvoza kartice (`apps/qnc-ai-card-ingest`) kad je uvoz gotov vraca se iz minimiziranog, dolazi iznad ostalih
aplikacija (`WindowLevel::AlwaysOnTop`, `Focus`, treptanje na traci zadataka) i ostaje iznad dok korisnik u njemu ne
klikne ili tipka; tada je obican prozor.
Odluka 2026-11-13 (korisnik: "zasebni prozori ostaju za sada, naknadno ćemo odlučiti finalno rješenje"): forme
QNC-AI-ja ostaju zasebni prozori (maksimizirani); desktop (`qnc-app`, jedan prozor s launch barom) se ne uvodi dok
korisnik ne odluci. Otvoreno pitanje za tada: launch bar bi citao slijed aplikacija price kojoj baza pripada (aktivna
je baza, a njezin slijed je samo Project).
Izvedeno 2026-11-13 (korisnik: "ako nešto ima A1 i tretman Izjave... ako se kadar potpuno promijeni to treba analizirati
jer se iz toga mogu izvući pokrivalice"; "ne označavaš odzume ili zumove"): (1) Zum i odzum. `qnc-shot-detect` uz pomak
cijele slike trazi i povecanje oko sredine (`zoom_scale`: mjerila 1±k/100, bilinearno uzorkovanje, srednji dio slike, prihvaca
se samo kad je barem 15 % bolje od mjerila 1); par slika koji raste 3 %+ u sekundi je zum, koji se smanjuje odzum
(`Camera::ZoomIn`, `ZoomOut`, imena `zoom_in`, `zoom_out`); dio je niz parova iste vrste, najmanje 3 s. Baza analize,
pregled sadrzaja i opis kadra uzimaju ih kao upotrebljive dijelove; natpisi `shots.camera.zoom_in/zoom_out` (zum, odzum) u
Video AI panelu i AI pokrivalicama. Test `a_take_gives_its_zoom_in_and_its_zoom_out`. (2) Izjava s promjenom kadra. Analiza
kadrova uzima i izjave (kad je prijepis gotov); pregled sadrzaja uzima izjavu samo kad je analiza nasla barem dva kadra
(`tools/qnc-ingest-worker` `AiTakes`: `shots`, `content`; `qnc-shot-content-run` `look_missing` filtrira po `pace.takes`).
Izjava se vise ne odlucuje za cijeli klip nego po kadru (`qnc-shot-content-run` `statement_shots`): kadar koji izgleda kao
izjava i ima govor na A1 ostaje cijeli (vrsta `statement`), ostali kadrovi istog klipa daju svoje dijelove za AI pokrivalice.
Klipovi vec analizirani ne dobivaju zum dok se ne analiziraju ponovno (`ANALYSIS_VERSION` nije podignut); izjave dosad nisu
bile analizirane, pa ih radnik uzima sam. Testovi `qnc-shot-detect`, `qnc-shot-db`, `qnc-shot-content-run`,
`qnc-shot-caption-run`, release gradnja radnika. Uzivo nije provjereno; pravilo `statement_shots` nema vlastiti test.
Izmjena 2026-11-13 (korisnik: "da, podigni verziju analize"): `qnc-shot-db` `ANALYSIS_VERSION` 2 -> 3 i `qnc-shot-content-db`
`CONTENT_VERSION` 3 -> 4 (sadrzaj kadra se veze na dijelove analize, pa se radi iznova uz nju): svi klipovi aktivnog projekta
analiziraju se i pregledavaju jednom ponovno i dobivaju zum, odzum i izjavu po kadru. Opisi kadrova (Florence-2) su po kljucu
kadra u bazi racunala, pa novi kadrovi dobivaju svoje. Testovi `qnc-shot-db`, `qnc-shot-content-db`, `qnc-shot-content-run`,
release gradnja radnika, `cargo check` QNC-a. Uzivo nije provjereno.
Popravak 2026-11-13 (korisnik: "pogledaj log, imamo specificnu situaciju, dvije kartice ubacene"; log i baza 15:32:19 22f0,
samo citanje): G: (FX3A) se uvozila, a D: (FX6V) ubacena dok je prozor uvoza radio usla je u istu bazu i svih 30 klipova
dobilo je "Izvor medija nije povezan u konfiguraciji": pozadinski proces procitao je izvore jednom pri pokretanju, kad D:
nije bila spojena. D: je zatim na 3 s nestala i vratila se, a prozor uvoza je nije ponovno odabrao (pamtio ju je kao
obradenu), pa neuspjeli klipovi nisu vraceni u red. (1) `qnc-ingest-import-worker` `ConfigMediaOpener::of_root`: izvor
koji nije na popisu cita se ponovno iz korijena (diskovi spojeni sada) i trazi jos jednom; `tools/qnc-ingest-worker`
(`sources_now`): dovrsetak zapisa, posteri i analiza kadrova isto. (2) `qnc-card-autoselect` `want`: kartica koje vise
nema zaboravlja se, pa se po povratku ponovno odabire i Uvezi vraca neuspjele klipove u red (test
`a_card_taken_out_and_back_is_gone_to_again`). Primjer `qnc-content-read/examples/import_errors.rs` (QNC-shared, samo
citanje). Testovi `qnc-card-autoselect`, `qnc-card-ingest`; `qnc-ingest-import-worker` 20/22 (dva testa padaju i bez ove
izmjene). Radnik nije zamijenjen (radio je i drzao datoteku). Otvoreno: kartica koju neka baza vec ima, ubacena dok prozor
uvoza radi, ulazi u tekucu bazu bez provjere; PMW-300 (E:) dala je 0 klipova; QNC ima svoju kopiju uvoza s istom greskom.
Izmjena 2026-11-13 (korisnik: "ako je baza uvezena wizard ispise pored naziva kamere da je uvezena vec.. dodaje se samo linkovi
na prethodno uvezenu bazu, ponovo se ne uvozi"): `qnc-attached-cards` uz svaku spojenu karticu jednom trazi raniju bazu (ne
onu u kojoj Ingest sada radi) koja vec ima njezine klipove (`AttachedCard.elsewhere`; `qnc-ingest-base` `known_card_elsewhere`,
zajednicka provjera `known_among`). Prozor uvoza (`qnc-card-ingest`): tekuca baza dobiva vezu na tu bazu (`qnc-ingest-base`
`link_active_to`, tablica `ingest_base_links` u bazi, u zasebnoj niti); kartica koju ta baza ima cijelu se ne odabire,
kartica s novim klipovima odabire se, a klipovi koje povezana baza ima ne uvoze se (`not_taken` broji i povezane baze); prica
koja uzme bazu uzima i povezane (`write_story_base`). Prozor pise svaku karticu u svom retku, a poznatu uz kameru "vec uvezeno
(<baza>), dodana je veza na tu bazu" ili "... uvoze se samo nova: n" (`Status.cards`, `CardLine`; rjecnik wizarda
`card_ingest.already_imported`, `_new`). Testovi `qnc-attached-cards`, `qnc-card-ingest`, `qnc-card-autoselect`,
`qnc-wizard-text`; release gradnja radnika i prozora uvoza (tray je radio, nije zamijenjen). `qnc-ingest-base` veze nemaju test.
Uzivo nije provjereno.
Izmjena 2026-11-13 (korisnik: "ako nema baza, brisanje baze bi trebalo biti neaktivno (blijeda slova)"): stavka trayja "Obrisi
bazu" je neaktivna kad nema nijedne Ingest baze; broj baza cita se uz zadnji projekt (svakih 10 s) i odmah nakon brisanja baze
(`apps/qnc-ai-tray` `menu(.., has_bases)`); baze koje se ne mogu procitati ne gase stavku (brisanje tada kaze zasto). Tray
izgraden (release).
Izmjena 2026-11-13 (korisnik: "daj tray-u ikonu QNC"): nova kocka `crates/qnc-app-mark` (`qnc_mark(size)`: bijelo "QNC"
fontom Hack koji QNC vec nosi, na plavom zaobljenom kvadratu, RGBA bez predmnozene prozirnosti, crtano cetiri puta vece pa
smanjeno). `qnc-tray` `Tray::start(.., icon: Option<fn(u32) -> Vec<u8>>)`: Windows adapter trazi sliku u velicini male
ikone zaslona (`SM_CXSMICON`) i od nje pravi ikonu (`CreateIconIndirect`); bez slike ili kad je sustav ne uzme, opca
ikona aplikacije. Tray je predaje. Primjer `qnc-app-mark/examples/save_mark.rs` sprema PNG-ove za pregled. Test
`the_mark_has_its_size_clear_corners_and_white_letters_in_the_middle`, release gradnja trayja. Popravak: maska ikone popunjena
nulama (bitmapa bez bitova imala je sto god je bilo u memoriji); drugi primjerak trayja u log pise da prvi radi. Izmjena
(korisnik: "ikona se vidi, podebljaj slova, neka kocka bude crna"): kvadrat crn, slova podebljana sirenjem za 2,8 % kvadrata
(`thicker`). Uzivo: ikona se vidi u traci.
Izmjena 2026-11-13 (korisnik: "neka uvoz po vremenu nastanka"; izabrao to umjesto "najveci najprije"): red uvoza uzima
klipove redom kojim su snimljeni: `qnc-content-store` (QNC-shared) ClaimNext slaze po vremenu snimanja (`created_at_utc`,
iz `creation_time` zapisa kamere) kao trenutku, bez obzira na zonu koju je kamera zapisala; klip bez vremena dolazi iza, po
nazivu. Zamjenjuje "najkraci prvi" od 12. 11. Vrijedi za QNC i QNC-AI (ista kocka). Test
`the_import_takes_the_clips_in_the_order_they_were_recorded`. Uzivo nije provjereno.
Izmjena 2026-11-13 (korisnik: "u trayu otvaranje izbornika direktorija, ako izvor nije kartica nego neki direktorij"): tray
ima stavku "Uvoz iz direktorija…" (`INGEST_FOLDER`); ona otvara prozor uvoza s `--folder` u bazi koju daje
`qnc_ingest_base::current` (dok prozor uvoza vec radi, tray kaze da je uvoz u tijeku). Prozor (`qnc-ai-card-ingest`) tada
pokazuje Ingestov preglednik izvora (Racunalo / LAN / Internet / Baze, mape, Odaberi / Odustani) istom kockom kao Ingest forma
(`qnc-ingest-blocks` `source_browser`, novi javni ulaz na postojeci panel). `qnc-card-ingest` `start_folder`, `view`,
`choose`, `Phase::Choosing`: spojene kartice se ne odabiru; Odaberi direktorija je Ingestov Select tog direktorija (karta
kamere ili pojedinacne datoteke, kao u Ingestu), zatim isto kao kartica (svi klipovi, uvoz po postavkama, posteri, valovi,
filmstrip, AI). Odustani zatvara prozor (baza koju je `current` napravila ostaje prazna). Rjecnik wizarda
`tray.ingest_folder`, `tray.ingest_runs`, `card_ingest.choose_folder`. Testovi `qnc-card-ingest`, `qnc-wizard-text`, release
gradnja trayja i prozora uvoza. Uzivo nije provjereno.
Dopuna 2026-11-13 (korisnik: "dok uvozi s jedne kartice ne dozvoljava pristup direktorija uvozu"): prozor uvoza je i dalje
jedan, ali uvoz iz direktorija radi i dok on uvozi karticu: tray pokrene prozor s `--folder`, a taj primjerak, kad prvi vec
radi, da mu signal i zavrsi (`qnc-tray` `Signal`: imenovani dogadaj sesije, samo "sad", bez stanja; `listen`, `fired`, `fire`).
Prozor koji radi tada pokaze preglednik (`qnc-card-ingest` `pick_folder`); preglednik uzme tek kad nijedna kartica nije u
Selectu, a dok ga korisnik drzi kartice se ne odabiru; uvoz, posteri, valovi i AI vec odabranih klipova idu dalje u pozadini.
Odaberi direktorij dodaje njegove klipove u istu bazu; Odustani vraca na uvoz kartice (`cancel_folder`), a prozor pokrenut
samo za direktorij zatvara. Uzivo nije provjereno.
Popravak 2026-11-13 (korisnik: "kaze da je kartica vec uvezena, a ja sam obrisao bazu prije toga"; log: potvrda brisanja u
23:42:48, Izlaz iz trayja 8 s poslije, bez "base deleted"): brisanje ceka da pozadinski posao pusti bazu (do 60 s), a Izlaz je
zavrsio tray usred toga, pa je baza ostala i kartica je u sljedecem pokretanju bila poznata. Sada tray odmah javi da brise
(`tray.base_deleting`), a Izlaz dok brisanje traje kaze da ce izaci kad baza bude obrisana i izade tek tada
(`tray.quit_after_delete`, `quitting`). Uzivo nije provjereno.
Izmjena 2026-11-13 (korisnik: "ako prepoznajes kameru, dodaj i naziv kamere pored broja kartice, jer sami brojevi ne govore
puno"; log: brisanje baze 658c ovaj put zavrseno, preglednik direktorija tijekom uvoza otvoren): tray kameru kartice imenuje
po profilima kamera (`qnc_card_watch::camera_of`: model, npr. "Sony ILME-FX3A"; bez prepoznatog profila obitelj iz kataloga
kao prije); poruke (vracena, vec uvezena, izbacena, vratite karticu), stavke izbornika i status wizarda pisu karticu s
kamerom (`qnc-wizard-flow` `Card::name_with_camera`: "XAVC (G:\) · Sony ILME-FX3A"); log `card ... camera=`. Uzivo: log
pokazuje "camera=Sony ILME-FX3A" za karticu G:.
Izmjena 2026-11-13 (korisnik: "cim je zavrsen Prijepis ucini dostupnim dugme Pokreni QNC, a preostali procesi neka nastave u
pozadini"): u prozoru uvoza "Pokreni QNC" je dostupan cim su svi klipovi uvezeni i im je prvi kanal cut (prijepis gotov,
zaustavljen ili iskljucen u postavkama; `qnc-card-ingest` `Progress::ready_for_qnc`), ne tek kad je sve gotovo. Klik otvara
QNC i zatvara prozor; filmstrip, valovi i AI rade dalje u pozadinskom procesu, koji ne ovisi o prozoru. Test
`qnc_can_start_once_every_clip_is_imported_and_heard_the_rest_goes_on`, release gradnja prozora. Uzivo nije provjereno.
Izmjena 2026-11-13 (korisnik, slika preglednika u prozoru uvoza: "na ovom direktorij panelu treba takoder pisati naziv
kamere"): u popisu diskova uz disk spojene kartice pise model kamere ("Kartica / USB · Sony ILME-FX3A"). `qnc-card-ingest`
cita spojene kartice i kad je pokrenut samo za direktorij (`card_cameras`, po serijskom broju volumena; za uvoz se kartice i
dalje uzimaju samo uz uvoz kartice), `qnc-ingest-blocks` `source_browser(.., cameras)`; Ingest forma predaje prazan popis
(nema promjene). Uzivo nije provjereno.
Izmjena 2026-11-14 (korisnik: "bazi mozes umjesto teksta Ingest dati naziv kamere, i u trayu dodati podatke s koje lokacije
se uvozi (kad je odabran iz direktorija)"): nova baza se zove po kameri kartice (`qnc-ingest-base` `current_for`,
`new_with_only_new(.., what)`; ime ostaje s prefiksom "Ingest" po kojem se baze prepoznaju, a prikazuje se oznaka bez njega:
"Sony ILME-FX3A 2026-11-14 00:01:45 7c4c"); baza za direktorij "Direktorij <vrijeme>" (`tray.folder_base`). Izabrani
direktorij prozor uvoza upisuje u bazu (`add_source`, tablica `ingest_base_sources` u bazi, put kako ga pokazuje preglednik),
a tray ga cita svake 2 s (`active_sources`) i pokazuje u izborniku i opisu ikone ("Uvoz iz: E:\", `tray.source`). Testovi
`qnc-ingest-base` (ime po kameri), release gradnja trayja i prozora. Uzivo nije provjereno.
Odluka 2026-11-14 (korisnik: "razlicite kartice se trebaju uvoziti u svoje baze, ne u zajednicku bazu"; izabrao opciju 2):
svaki izvor (kartica ili direktorij) ide u svoju bazu i uvoze se istodobno: svaka baza ima svoj prozor uvoza i svoj pozadinski
proces, neovisno o aktivnom projektu.
Prosirenje pravila 2026-11-14 (korisnik odobrio: "da, odobravam, kreni"; dopunjuje pravilo 3 i 4.1 root AGENTS.md QNC-a):
proces moze dobiti projekt na kojem radi (`qnc-work-settings` `PINNED_PROJECT_ENV` = `QNC_WORK_PROJECT`, `pinned_project`,
`SettingsReader::with_project`); tada mu je to "aktivni projekt" umjesto onog u `app_settings.active_project_id`. Taj projekt
nikad nije izmisljen: bira ga program koji pokrece proces iz registra (tray QNC-AI-ja iz registra baza). Samo lokalni registar
(preko mreze jasna greska); server postavki uvijek daje aktivni projekt iz baze. Korisnik 2026-11-14: baze su kartice s
kojima QNC radi (kopirani klipovi ili linkovi na klipove na kartici, po postavkama); projekt nastaje tek u Ingest formi, gdje
korisnik odlucuje sto ide u projekt; tray samo uvozi kartice, u pozadini, da je pri montazi sve vec na raspolaganju.
Izvedeno: tray (`Windows`) drzi prozor uvoza po bazi: nova kartica dobiva novu bazu nazvanu po kameri (`qnc-ingest-base`
`new_base`, aktivni projekt ostaje kakav je bio), direktorij svoju bazu "Direktorij"; prozor se pokrece s `QNC_WORK_PROJECT`
= ta baza i `--card <serijski broj>` (prozor uvozi samo svoju karticu, `qnc-card-ingest` `only_card`; jedan prozor po bazi,
`OneCopy` po bazi; naslov prozora je oznaka baze). Kartica vracena dok njezin prozor radi pripada tom prozoru. Pozadinski
proces koji prozor probudi nasljeduje bazu. `qnc-ingest-base` "radna baza" je zadana baza procesa, inace aktivna (`working`).
Brisanje baze: tray zavrsi njezin prozor i upise u bazu da se brise (`mark_closing`, tablica `ingest_base_closing`);
prozor i pozadinski proces te baze tada zavrse (`closing`). "Pokreni QNC" ne mijenja aktivni projekt (Ingest forma baze
vidi kao kartice, tab Baze). Testovi `qnc-work-settings` (`a_reader_given_a_project_reads_it_whichever_the_database_marks_active`),
`qnc-ingest-base`, `qnc-card-ingest`, `qnc-active-project-read`, release gradnja trayja. Uzivo nije provjereno. Nije izvedeno:
`new_with_only_new` (nova baza samo s novim klipovima poznate kartice) jos cini bazu aktivnom.
Popravak 2026-11-14 (korisnik: "iako sam brisao, stare baze nisu obrisane"; log: "Obrisi bazu" uzima aktivnu bazu, pa su
baze koje nisu aktivne ostajale): izbornik trayja ima stavku "Obrisi bazu: <naziv>" za svaku Ingest bazu (`DELETE_BASE_OF`
+ redni broj), brise se izabrana; dok se jedna brise, druga ceka (`tray.delete_base_busy`). Bez baza ostaje siva stavka.
Usput iz loga: drugi prozor se zatvarao jer je radio stari program prozora (nova gradnja nije mogla zamijeniti pokrenuti),
pa je stari prozor obje kartice uzeo u istu bazu; s novim programom rade dva prozora i dva pozadinska procesa istodobno.
Popravak 2026-11-14 (korisnik: "klik na prozor tray ingesta taj prozor prebaci u front; vise prozora s pomakom"; log: prvi
gotov prozor ostao je iznad svih i skrio drugi): prozor koji je zavrsio i dosao naprijed vraca se na obicnu razinu i kad
izgubi fokus (nakon 1 s), ne samo na klik u njega. Tray prozoru daje `--place <broj otvorenih>`, prozor se otvara 36 px
dolje-desno po svakom (najvise 8 mjesta).
Izmjena 2026-11-14 (korisnik: "na tray ingest prozoru ne treba biti dugme Pokreni QNC, nego info izbornik koji kaze sta je
uvezeno, sta je jos u tijeku, s pitanjem zelite li da pokrenem QNC ili cekamo da kompletan Ingest zavrsi"): dugme uklonjeno
iz prozora uvoza. Tray svake 2 s u pozadini cita napredak svake baze koju je otvorio (`qnc-card-ingest` `progress_of`,
`qnc-ingest-base` `not_taken_of`; baza ostaje na popisu i kad je prozor zatvoren). Pravilo pitanja je kocka `StartAsk`
(testovi): jedno pitanje za sve baze kad je prva spremna za montazu (uvezena i prepisana), a druga jos radi (Da: pokreni QNC,
ostalo ide u pozadini; Ne: cekamo); ako korisnik ceka, ponovno kad je sve gotovo; nova kartica pocinje novi krug. Da otvara
`qnc-ingest` (`qnc-ai-open`). Uz pitanje: popis baza s brojem uvezenih klipova i stanjem (sve gotovo, spremno za montazu, u
tijeku). Uz to (korisnik: "kada su kartice uvezene ne treba vise javljati da su kartice potrebne"): kartica ciji su klipovi
svi uvezeni, izvadena bez izbacivanja, vise ne javlja "vratite karticu"; ponovno ubacena provjerava se ima li novih klipova
(postojeci `card_check`). Uzivo nije provjereno.
Izvedeno 2026-11-14 (korisnik: "istrazi moze li se u istom prolazu raditi AI kadrovi, filmstrip, val"; "bitno je ukupno
vrijeme cekanja"; "da, kreni s 1 i 2"): mjereno na kopijama FX6V klipova (samo citanje): citanje zvuka za val 1 GB za 1,5 s,
AI kadrovi jednog klipa 283 intra slike za 27 s (oko 95 ms po slici, dekodiranje 4:2:2 10-bit), 4 klipa istodobno 2,1x brze;
zajednicki prolaz bi ustedio samo 13 slika filmstripa po klipu, pa nije izveden. U logu uvoza 103 klipa AI kadrovi su cekali
prijepis svakog klipa i zavrsili 7,5 min iza njega, jedan klip po jedan. (1) `qnc-shot-analysis` (QNC-shared) dekodira do 4
serije od 16 slika odjednom (`MOST_DECODERS`, najvise pola niti racunala), svaka u svojoj mapi; isti klip 27 s -> 13,5-16 s,
isti kadrovi i dijelovi. (2) `tools/qnc-ingest-worker` `ai_takes`: AI kadrovi uzimaju svaki kopirani klip, ne cekaju prijepis
(pregled sadrzaja i dalje ceka, za izjave). Uzivo nije provjereno.
Izmjena 2026-11-14 (korisnik: "15 minuta cekanja za prijepis je neupotrebljivo, montaza ne moze poceti bez prijepisa";
"prijepis mora biti u priblizno vrijeme filmstripa i vala"; "SigLIP, pa tek zatim opisi; opisi nam vise sluze za treniranje
SigLIP-a"): log: prijepis (Parakeet, ispod normalnog prioriteta) radio je 2,6-3x stvarnog vremena, a sam na racunalu oko 8x,
jer su AI dekoderi slika (do 4 ffmpega, normalni prioritet), SigLIP i Florence (ispod normalnog, kao prijepis) dijelili
procesor s njim. Redoslijed prioriteta sada: player, sucelje, prijepis (ispod normalnog), a pozadinske slike najnize (idle):
dekoder slika filmstripa i AI kadrova (`qnc-ffmpeg-decode` `run_filmstrip_command` `background`, QNC-shared; slike zooma reza
ostaju normalne), SigLIP i Florence (`tools/qnc-shot-content-siglip`, `tools/qnc-shot-caption-florence`). Opisi (Florence) ne
rade vise usporedno s uvozom, nego tek na kraju rada pozadinskog procesa, poslije SigLIP-a. Uzivo nije provjereno.
Popravak 2026-11-14 (korisnik: "ne da brisati bazu ni izbaciti karticu ni iskljuciti tray"; log: pozadinski proces radio je
opise za bazu u brisanju): nit koja prati bazu (zatvorena, drugi najam, baza se brise) radila je samo dok traje uvoz, pa zadnji
AI krug poslije uvoza nitko nije zaustavio; brisanje je cekalo bazu, a Izlaz traya brisanje. `tools/qnc-ingest-worker` prati
bazu cijeli rad (`watching`). Tray napredak baza cita svake 10 s (svake 2 s trosio je oko cetvrtine jezgre).
Dopuna 2026-11-14 (korisnik: "aktivirati sistemsko izbacivanje kartice kada korisnik iz traya iskljucuje karticu"; log: citac
je karticu nakon izbacivanja medija odmah ponovno spojio): `qnc-card-eject` poslije zakljucavanja, odmontiranja i izbacivanja
medija pusti disk i zatrazi od sustava uklanjanje uredaja diska, a kad ga sustav ne pusti, uredaja na kojem visi (citac), kao
"Sigurno ukloni hardver" (SetupDi po broju diska, `CM_Request_Device_EjectW`, razlog odbijanja u gresci). Uzivo nije
provjereno: kartica izbacena tako vraca se tek kad se citac ili kartica ponovno utaknu.
Izvedeno 2026-11-14 (korisnik: "kad se ubacuje kartica koju poznaje tray, da se ne otvara File Explorer, nego import
wizard"): nova kocka `crates/qnc-autoplay-guard`: dok radi, odgovara na pitanje Windowsa smije li AutoPlay otvoriti nesto za
ubaceni disk (`IQueryCancelAutoPlay` u running object tableu, klasni moniker `CLSID_QueryCancelAutoPlay`, vlastita nit s COM-om
i petljom poruka); odgovor daje pozivatelj. Postavke Windowsa se ne mijenjaju: kad tray ne radi, AutoPlay je kakav ga je korisnik
postavio. Tray: za disk koji je kartica poznate kamere (`qnc-card-watch` `what_is`) AutoPlay se otkaze i otvara se prozor uvoza
kao i dosad; za svaki drugi disk AutoPlay radi; odluka ide u log (`autoplay disk= card=`). Test registracije i otpustanja.
Uzivo nije provjereno.
Izmjena izgleda 2026-11-14 (korisnik, AI Media Assist: "iznad timelinea tri podjele vertikalno: lijevo media pool, desno media
pool kreiranih izjava, u srednjem dijelu gore monitor, ispod prepis odabranog fajla"; "idealno da se paneli mogu siriti po
korisnikovim potrebama"; "za QNC neka ostane stari za sada"): ugovor rasporeda grupe e (`contracts/ui/editorial.layout.json`)
ima `statements_pool` (naslov "Izjave"); tab izjava je iz lijevog poola uklonjen. `qnc-editorial-layout` (QNC-shared): grupa sa
`statements_pool` ima ploču od tri stupca iznad source timelinea (lijevo tabovi poola, kartice i traka Prijepis; u sredini
monitor, transport i desni panel grupe, tj. prijepis s tabovima Audio AI / Video AI; desno naslov i kartice izjava;
`STATEMENTS_HEAD`, `STATEMENT_CARDS`, `ColumnSizes`, `board_layout_sized`); ostale grupe i klasicni QNC nepromijenjeni.
`qnc-board` `drag_edge`: granicu stupca i granicu ispod monitora korisnik povlaci misem; velicine su izbor prozora dok radi
(egui memorija), nikad u bazi; nijedan stupac ispod desetine sirine. `qnc-editorial-blocks` crta pool izjava (klik otvara
izjavu, X je brise) i rucke. `qnc-editorial-application`: spremanje izjave prebacuje tab samo kad pool ima tab izjava. Uzivo nije
provjereno.
Popravak 2026-11-14 (korisnik: "i dalje otvara staru"): ugovor rasporeda ugradjen je u kocku (`include_str!`) iz repoa u kojem
kocka zivi, pa je program citao QNC-shared ugovor. Po pravilu zajednickih kocki (original ostaje, aplikacija uzima kopiju u
svoj direktorij) QNC-AI ima vlastitu kopiju `qnc-ai-editorial-layout` (ime knjiznice isto, `qnc_editorial_layout`); korisnik:
"stavi je u isti direktorij kao i ostale, u share, samo joj daj novi naziv": kocka je u `QNC-shared/crates/qnc-ai-editorial-layout`
i ugradjuje svoj ugovor `QNC-shared/contracts/ui/ai-editorial.layout.json` (kopija QNC-AI `contracts/ui/editorial.layout.json`); zajednicka `qnc-editorial-layout` vracena je na stanje prije tri stupca, pa je klasicni QNC
nepromijenjen. `qnc-board` `drag_edge` ostaje u zajednickoj kocki (opca, samo dodana).
Izmjena 2026-11-15 (korisnik: "nije sretno rjesenje, vratiti stari izgled, a na desnoj strani ostaviti izjave, ako postoje";
"neka ostane mogucnost promjene dimenzija svih panela, a trenutni neka bude default"): tri stupca uklonjena. Ploca je opet
standardna (lijevo monitor, tabovi i transport, media pool; desno prijepis), a desno od prijepisa je stupac izjava samo dok
postoji izjava (`qnc-ai-editorial-layout` `board_layout_with`, sirina iz ugovora `statements_pool.width` 420). Granice se
povlace misem: izmedju lijevog stupca i prijepisa, ispod monitora i pred stupcem izjava (`PanelSizes`, `drag_edge` crta crtu u
boji ruba teme, u boji fokusa dok je mis na njoj); bez povlacenja raspored je kakav ugovor kaze. Velicine su izbor prozora dok
radi (egui memorija), nikad u bazi. Vrijedi za sve forme ploce u QNC-AI (Story, S-Story, MA); klasicni QNC nepromijenjen.
Uzivo nije provjereno.
Dopuna 2026-11-15 (korisnik, AI Media Assist: "oznaceni tekst kompletan markiraj"): u prijepisu AI Media Assista tekst izmedju
IN i OUT prikazanog klipa osvijetljen je cijeli, jednom bojom (boja fokusa teme, prozirna), i razmaci izmedju oznacenih rijeci;
bez zagrada (`qnc-transcript-panel`). Uzivo nije provjereno.
Ispravak 2026-11-15 (korisnik: "samo tekst prijepisa koji korisnik odabere treba biti markiran"; "default sirinu taba
Izjava suzi na sirinu jednog postera"; "zasto su se prijepisi i AI opisi vratili na 0"): u prijepisu je oznacen samo tekst
izmedju IN-a i OUT-a koje je korisnik postavio; bez njegovih oznaka nista (`qnc-transcript`: `source_in_frame` /
`source_out_frame` projekcije, ne `visible_source_marks`, koji bez oznaka daje cijeli klip). Stupac izjava je zadano siroko
jedan poster (`statements_pool.width` 190 u oba ugovora rasporeda). Tray broji prepisane klipove baze iz baze projekta
(`read_transcribed` i gotovi ili pali u redu), ne iz reda, koji AI Media Assist cisti kad se isprazni (`qnc-card-ingest`
`progress_in`). AI opisi stoje na 0 dok SigLIP ne zavrsi, po redoslijedu. Testovi, release gradnja. Uzivo nije provjereno.
Izmjena 2026-11-15 (korisnik: "dodaj oznaku (zagradu) na pocetku i kraju selektiranog teksta, jer ne postoji indikacija je li
dvoklik prihvacen"; "nakon kreiranja izjave fokus ostaje na tekstu, ne prelazi na kreiranu izjavu"): u prijepisu AI Media
Assista tekst izmedju korisnikova IN-a i OUT-a ima crvene zagrade na pocetku i kraju uz osvijetljenu podlogu
(`qnc-transcript-panel`). Stupac Izjave je panel tipki (`Panel::Statements`, `qnc-focus-walk` `STATEMENTS`, `tab_order` grupe e
iza poola): nova izjava i klik na izjavu stavljaju tipkovnicu na stupac (okvir fokusa), kartica nove izjave je u fokusu i
dovodi se u pogled (`qnc-media-card`: kad se izabrana kartica promijeni, fokus ide na nju), strelice biraju, Enter otvara.
Testovi, release gradnja AI Media Assista. Uzivo nije provjereno.
Popravak 2026-11-15 (korisnik: "marker treba postaviti kad se napravi dvoklik, ne cekati da se odabere drugi"): prvi dvoklik na
rijec prijepisa (postavljen samo IN) odmah oznaci tu rijec zagradama i podlogom; drugi dvoklik prosiri oznaku do OUT-a
(`qnc-transcript`). Uzivo nije provjereno.
Ispravak 2026-11-15 (korisnik: "fokus se ne smije prebacivati na desni panel Izjave kad korisnik klikne na Dodaj izjavu; fokus
mora ostati na zadnjem kliknutom panelu"): Dodaj izjavu ne mijenja panel tipkovnice; nova izjava je samo izabrana u stupcu.
Klik na izjavu i dalje stavlja tipkovnicu na stupac Izjave, a klik na tekst prijepisa na source timeline (tipke teksta su tipke source timelinea; zadnji kliknuti panel).
Izvedeno 2026-11-15 (korisnik: "u tekst panelu nemamo kursora, pa ne mozemo selektirati tekst tipkovnicom"; "kocka treba biti
univerzalna, treba moci raditi u bilo kojoj formi gdje ima teksta (npr. u skripti)"): nova javna kocka `QNC-shared/crates/qnc-text-cursor`
(+ ugovor `contracts/modules/text-cursor.module.json`): kursor na bilo kojem tekstu rijeci s vremenima (retci rijeci `[from, to)`):
kursor je rijec pod playheadom (ili zadnja na koju je postavljen dok playhead ne ode drugamo), strelice lijevo/desno za rijec,
gore/dolje za redak, Shift+strelice oznacavaju od rijeci na kojoj je oznacavanje pocelo; sto oznaka znaci odlucuje pozivatelj.
U AI Media Assistu: klik na tekst prijepisa stavlja tipkovnicu na tekst (tipke su i dalje tipke source timelinea), strelice
pomicu kursor i playhead na rijec, Shift+strelice postavljaju IN/OUT od-do rijeci (`qnc-source-timeline` `select_words`), pa se
vide zagrade i podloga; klik izvan teksta ili drugi panel vraca strelice playheadu (`qnc-panel-focus` `text.rs`). Katalog: radnje
`transcript_select_left/right/up/down` (Shift+strelice) u opsegu source_timeline svih preseta, grupa pomoci "Prijepis".
Popravak usput: klik na rijec nije mijenjao panel tipkovnice (rijec se obradivala prije). Skripta jos ne koristi kocku. Testovi;
uzivo nije provjereno.
Dopuna 2026-11-15 (korisnik: "panel prijepis nemas u tab putanji"): tekst prijepisa je panel tipki `Panel::Transcript` u krugu
Taba grupe e (iza dugmadi source timelinea) s okvirom fokusa; klik na tekst ili Tab ga uzima. Strelice i Shift+strelice idu
kursoru teksta, ostale tipke (Space, I, O, Esc...) su tipke source timelinea, a tipkovnica ostaje na tekstu.
Popravak 2026-11-15 (korisnik: "kursor preskace redove teksta u prikazu"; "treba formatirati po recenicama s razmakom izmedju
recenica, nova recenica novi red"): prijepis se prikazuje po recenicama: rijeci teku preko redaka zapisa, recenica zavrsava na
rijeci koja zavrsava s . ? ! ili …, svaka recenica je svoj odlomak s razmakom izmedju (`qnc-transcript` `sentences`; rijeci
zadrzavaju svoja vremena, pa oznake, pretraga i kursor rade isto). Kursor ide po redovima kakvi se vide na ekranu: panel pri
crtanju zapisuje redove (`qnc-text-cursor` `ShownRows`, `TranscriptView.rows`), gore/dolje idu red po red; rijec kursora dovodi
se u pogled kad se pomakne. Uzivo nije provjereno.
Izmjena 2026-11-15 (korisnik: "kursor bi unutar recenice trebao skakati s razmaka na razmak, ne s rijeci na rijec"): kursor
teksta stoji na razmaku kao u editoru teksta: prije rijeci ili iza zadnje rijeci reda (`qnc-text-cursor` `Gap`), crta se kao
okomita crta (`qnc-transcript-panel`, `ShownRows` caret); strelice idu s razmaka na razmak (iza zadnje rijeci reda pa na pocetak
sljedeceg), gore/dolje na isto mjesto u redu iznad/ispod, playhead ide na vrijeme razmaka; Shift+strelice oznacavaju rijeci
izmedju razmaka gdje je oznacavanje pocelo i kursora (IN/OUT). Klik na rijec stavlja kursor ispred nje. Uzivo nije provjereno.
Promjena pravila unosa 2026-11-15 (korisnik: "velika zbrka s tab tipkovnicom; Alt + strelice za izbor panela, strelice ostaju
za setnju unutar panela; nema kruga, korisnik odredjuje gdje zeli"; "da, slazem se s prijedlozima"): mijenja zakljucano pravilo od
2026-10-20 u dijelu "Alt+ skace". Alt + strelice biraju panel u tom smjeru na ekranu (nova javna kocka
`QNC-shared/crates/qnc-panel-map` + ugovor `contracts/modules/panel-map.module.json`: najblizi panel cija je sredina u tom
smjeru, uz vise preklapanja; nema kruga). Klik bilo gdje u panelu je ista radnja (`focus_panel`, cilj panel): ploca pri crtanju
zapisuje mjesta panela (`qnc-board` `panel_places`; koja su mjesta paneli zna ugovor rasporeda, `focus_place`), a klik koji nista
drugo ne napravi postaje `focus_panel`. Skok playheada na sljedeci objekt je Ctrl+Alt + strelice (katalog QNC preseta, opsezi
source_timeline i ingest; pomoc F1). Tab i Shift+Tab ostaju kao dodatni nacin. Enter na tekstu prijepisa ide dalje od teksta, ne
na dugmad source timelinea. Vrijedi za Story, S-Story, MA i AI Media Assist (`qnc-panel-focus` `choose_panel`); Ingest ima
tipke u katalogu, ali ih njegov krug (`qnc-focus-walk`) jos ne obradjuje (sljedeci korak), a u MA Scriptu prozor medija nema mjesta
panela. Testovi; uzivo nije provjereno.
Dopuna 2026-11-15 (korisnik: "Alt+ je u redu, ali polozaj Alt tipke nije povoljan za brzi rad; Tab + strelica mi se cini
elegantniji, a ako korisnik u brzini pogrijesi manja je steta"): Tab drzan je modifikator kao Ctrl i Alt (`qnc-keyboard-shortcut`:
`"tab": true` u vezi tipke, `ShortcutEvent.tab`): Tab + strelica bira panel u tom smjeru (iste radnje `panel_*` kao Alt +
strelice, koje ostaju). Tab pritisnut sam vrijedi kad se pusti, ako uz njega nije pritisnuta druga tipka, pa i dalje ide na
sljedeci panel (Shift+Tab na prethodni). Posljedica i u klasicnom QNC-u (dijeli kocku): Tab djeluje pri pustanju, ne pri pritisku.
Popravci iz loga istog dana: Alt+lijevo s teksta islo je na source timeline (panel ispod cijele sirine racunao se "lijevo");
panel u smjeru mora biti cijeli s te strane (`qnc-panel-map`). Skok na objekt bio je jos u dva opsega na Alt; sada svugdje
Ctrl+Alt. Testovi (Tab drzan i Tab sam kroz egui okvire); uzivo nije provjereno.
Ispravak 2026-11-15 (korisnik: "kursor u tekstu koji je izmedju rijeci je indikator polozaja; ne rijeci markirati dok ne postoje
dva markera in/out"): ukida ispravak "prvi dvoklik odmah oznaci rijec": rijeci su oznacene (zagrade, podloga) tek kad postoje i IN
i OUT; polozaj pokazuje kursor teksta.
Izmjena 2026-11-15 (korisnik: "otvara se sa selektiranim tabom Sve, ali ne da se pomaknuti nigdje; trebao bi se otvarati s tabom
Prijepisi jer je njena glavna funkcija napraviti izjave iz prijepisa"): u novom projektu prijepisa i izjava jos nema, pa su ti
tabovi bili sivi (pravilo praznih tabova) i Sve je bio jedini. Ugovor rasporeda dobio je `opening_tab` (tab s kojim se forma
otvara, uvijek dostupan; `qnc-ai-editorial-layout`, `FocusLayout.opening_tab`, `FocusView.opening_tab`); grupa e (AI Media Assist)
se otvara na Prijepisima. Usput: ulazak u panel (Alt/Tab + strelica, klik) javlja i krugu tipki gdje je tipkovnica.
Izmjena 2026-11-15 (korisnik: "prozor koji daje obavijest da je baza spremna izgleda kao sistemski i ne dolazi na vrh, pa se ne
vidi"): pitanja traya (spremno za montazu / pokreni QNC, potvrde) vise nisu sistemski MessageBox nego program `qnc-ai-ask` uz
tray (`apps/qnc-ai-ask`): prozor u QNC temi, uvijek na vrhu, trazi paznju; gumbi iz rjecnika (`ask.yes`, `ask.no`, `ask.cancel`
u `contracts/wizard-text`), Enter daje zadani odgovor (kod potvrde Ne, kao prije), Esc zadnji, zatvaranje je Odustani; odgovor
ide trayu na stdout (`qnc-tray` `qnc_ask`). Kad program ne moze krenuti, pita sistemski prozor i log kaze zasto. Uzivo nije
provjereno (tray radi, pa nije izgradjen).
Popravak 2026-11-15 (korisnik: "projekt izbornik se otvori minimiziran, pa se ne vidi"): nova kocka `crates/qnc-window-front`
(prozor doveden iznad ostalih: nije minimiziran, na vrhu, fokus, trazi paznju, dok korisnik ne klikne u njega ili ode na drugi
prozor). Prozor Projekti (`qnc-ai-projects`, otvara ga tray) koristi je cim se otvori; prozor kartice (`qnc-ai-card-ingest`) je
koristi na kraju uvoza umjesto vlastite kopije istog koda. Uzivo nije provjereno.
Izvedeno 2026-11-15 (korisnik: "UI cijele aplikacije u minimalistickom premium stilu"; "ne zelim ukrase, cisto"; "kreni s temom
za QNC-AI, s primjerom"): tema QNC-AI-ja je vlastita kopija ugovora ljuske `QNC-shared/contracts/ui/ai-shell.layout.json` s cistom
grafitnom paletom (podloga 15,16,18; panel 23,24,27; aktivno 32,34,38; linija 38,40,44; tekst 237; sporedni 138,139,143; isticanje
i fokus neutralne sive 200 / 154), jedina boja ostaje crvena za IN/OUT. Ucitava je `qnc-ai-editorial-layout`, pa vrijedi za forme
ploce QNC-AI-ja (AI Media Assist kao primjer, MA, S-Story, Story); klasicni QNC i njegov `shell.layout.json` nisu dirani. Nije jos
izvedeno: boje upisane u kockama (zeleni val i trake zvuka, mjerac razine, boje trajanja pokrivalica), ljuska QNC-AI-ja, prozori
traya i kartice (`qnc-ingest-layout`), okvir fokusa jos je tanka siva crta, ne svjetlija nijansa panela. Uzivo nije provjereno.
Izvedeno 2026-11-15 (korisnik: "imamo bazu, nemamo projekt vezan za tu bazu; formi QNC AI Projekti nedostaje dugme Kreiraj novi
projekt koji poziva Ingest"; "napravi odmah u premium stilu"): baza dobiva projekt tek na Uvezi u Ingestu; kad je pokretanje QNC-a
odbijeno, baza sa 103 klipa ostala je bez projekta, a Projekti je nisu vidjeli. Prozor Projekti ima na vrhu "+ Novi projekt" (cist:
samo tekst u bojama prozora, bez okvira i pozadine, svjetliji pod misem): Ingest se otvara na najnovijoj bazi bez projekta koja
nije veza na drugu bazu (`qnc-ingest-base` `newest_without_story`), a Uvezi daje obrazac novog projekta; kad svaka baza ima
projekt, Ingest se otvara za karticu ili mapu. Tekst `projects.new` u `contracts/wizard-text`. Uzivo nije provjereno.
Popravak 2026-11-15 (korisnik: "ovo nije premium stil, a dugme Novi projekt je nevidljivo; forma mora biti jasna nekome tko prvi
put sjeda za montazu"): cista paleta upisana je i u QNC-AI-jev vlastiti `contracts/ui/shell.layout.json` (isti kao
`ai-shell.layout.json`), pa je imaju i prozori koji ga citaju: Projekti, Postavke, Ingest i prozor kartice, pitanja traya, tray i
ljuska. "Novi projekt" je jedina primarna radnja prozora Projekti: svijetli gumb s tamnim tekstom (180x38). Uzivo nije provjereno.
Izmjena 2026-11-15 (korisnik: "bijelo dugme strši, položaj neprirodan, margine, paneli, dimenzije; ako imaju tri projekta, čemu
prazni prostor ispod; logika čitanja od vrha prema dnu: ako u spisku ne nalazi projekt koji treba, šta iza toga slijedi"):
prozor Projekti se čita odozgo: naslov s pretragom, projekti, a odmah ispod zadnjeg projekta red "+ Novi projekt" iste visine kao
red projekta (tanki okvir u boji ruba, tekst u boji teksta, pod mišem ispuna izabranog reda), bez bijelog gumba na vrhu i bez
statusa "Spremno.". Prozor je visok koliko sadržaj (do 8 projekata bez trake za listanje, više se lista na istom mjestu).
`qnc-project-blocks` `show_projects` vraća i `ProjectsAlone` (red novog projekta kliknut, visina sadržaja); `qnc-project-desktop`
`with_new_label`, `take_new_asked`, `content_height`; `qnc-ai-projects` mijenja visinu prozora kad se sadržaj promijeni.
Uživo nije provjereno.
Izmjena 2026-11-15 (korisnik: AI Media Assist "ne vidim da si išta promijenio"; "da" na prebacivanje boja u temu): boje medija više
nisu upisane u zajedničke kocke nego su dio teme: trake i valovi A1/A2, uvećani val, fokus timelinea (narančasti NORM, trajanje,
uzeta oznaka), uzeta traka, zone mjerača razine i točkice i kvačica kartice (QNC-shared `qnc-desktop-theme` `MediaColors`,
`ShellColors.media`, `Theme.media`; `qnc-theme-styles` ih koristi). Ugovor ljuske bez `media` daje boje klasičnog QNC-a (nepromijenjeno);
QNC-AI (`contracts/ui/shell.layout.json`, QNC-shared `ai-shell.layout.json`) daje sive: trake [28,30,34], val [150,152,158], fokus i
uzeta traka bijelo-sivi, mjerač prigušeni zeleni/žuti/crveni, točkice sive (greška prigušeno crvena). Crveni IN/OUT, rez i greška
ostaju. Test `qnc_ai_draws_its_media_in_graphite_not_in_qncs_colours`. AI Media Assist i ostale forme ponovno izgrađene. Uživo nije
provjereno.
Popravak 2026-11-15 (korisnik nakon probe: "timeline je kaotičan, crvene linije, sive linije, strši bijeli ton, grozno"; log: klik
na A1 otvorio je prošireni prikaz trake): linije razina -18/-9/-1 dBFS i linija razine tona također su boje teme (`MediaColors`
`level_marks`, `level_curve`; klasični QNC iste kao prije). QNC-AI: linije razina gotovo nevidljive (bijele 12/20 alfa, -1 dBFS
prigušeno crvena 45), linija razine siva, trake [24,25,28], val [104,106,112], prošireni val [118,120,126]. Uživo nije provjereno.
Izmjena 2026-11-15 (korisnik: "val je u ovom obliku neprecizan i nije lako čitljiv; bolje samo amplitudu kao Premiere i Final Cut,
manje linija"; "ne trebaš pokazivati linije nivoa, nego signal koji je viši bojaj u odgovarajućoj boji"; "ostaje samo jedna linija,
za podešavanje nivoa tona"): QNC-AI crta val kao amplitudu od dna trake prema gore u dB mjerilu (-54 dBFS dno, puna skala vrh),
pa se tihi govor čita jednako kao glasan; dio jači od -9 dBFS je u drugoj boji mjerača (prigušeno žuto), jači od -1 dBFS u trećoj
(prigušeno crveno); linije razina -18/-9/-1 se ne crtaju, ostaje samo linija razine tona. QNC-shared `qnc-wave-view` `WaveShape`
(`Mirrored`, `Amplitude`), `paint_wave_shape`, `paint_level_marks_shape`, `height_of`; `TimelineTheme.wave_shape`;
`MediaColors.wave_amplitude` (zadano false: klasični QNC zadržava zrcalni val i linije razina). Test
`the_amplitude_stands_on_a_db_scale_from_the_bottom`. Uživo nije provjereno.
Popravak 2026-11-15 (korisnik: "kada se podešava nivo tona wave ne prati povećanje ili smanjenje"; log: uzeta linija razine A1 i A2
na source timelineu, strelice gore/dolje): val source timelinea (traka i prošireni prikaz) sada se množi linijom razine te trake na
svakoj slici (QNC-shared `qnc-timeline` `level_lines::with_curves`, nacrt uzete linije odmah); Wrap redovi već imaju krivulju u svom
valu i ne množe se još jednom. Vrijedi i za klasični QNC (do sada je tamo val pratio samo razinu trake). Test
`a_curve_raises_and_lowers_the_wave_under_it_only`. Uživo nije provjereno.
Izmjena 2026-11-15 (korisnik: "audio trakovima treba dodati pozadinu, ne smije biti crna jer se stapa s UI; između audio traka i
filmstripa treba linija odvajanja"): QNC-AI trake A1/A2 imaju pozadinu [36,38,43] (svjetliju od podloge), val [128,130,137],
prošireni [140,142,149], a između trake i slike je linija od 2 px u boji video trake timelinea [23,24,27] (korisnik: "ne vidim crnu liniju"; svijetlosiva 1 px se uz sliku nije vidjela) (`MediaColors.lane_divider`, `TimelineTheme.lane_divider`;
A1 iznad slike, ostale ispod). Klasični QNC bez linije, kao prije. Uživo nije provjereno.
Izmjena 2026-11-15 (korisnik: "linija koja odvaja filmstrip od tona nije vidljiva na A1, proširi je"; "ide filmstrip, linija, A1 ton;
ton se crta iznad linije"; "predi i tray u premium stilu, povećaj font"): (1) Linija između trake i slike je 3 px na rubu trake prema
slici, a val trake stoji uz nju, nikad ispod nje (prije ju je val A1, koji raste od dna, prekrivao). (2) Tray izbornik više nije
sistemski bijeli: crta ga sam tray (`crates/qnc-tray/src/menu_look.rs`, owner-draw, WM_MEASUREITEM/WM_DRAWITEM): podloga [23,24,27],
red pod mišem [38,40,44], tekst [237,237,237], stavka koja nije izbor prigušena, Segoe UI 12 pt (prije sistemskih 9 pt), redovi 26 pt.
Tray poruke (`alert.rs`) u istoj sivoj paleti. Uživo nije provjereno.
Popravak 2026-11-15 (korisnik: "prekrio si i dugmad A1 i A2, to ne smiješ"; "samo boja odabranog A1 ili A2 treba biti svijetla,
inače malo prigušena, jedino tako znamo koji wave je odabran za prikaz preko filmstripa"): linija između trake i slike ide samo preko
trake, ne preko gumba A1/A2; gumb trake koja nije uzeta ima prigušeni tekst i okvir, uzeta (val preko slike) svijetli tekst i okvir
(`MediaColors.lane_quiet`, `TimelineTheme.lane_quiet`, `LaneLook.quiet`; vrijedi i u trim zoomu). Klasični QNC bez promjene.
Uživo nije provjereno.
Izmjena 2026-11-15 (korisnik: "dugme Dodaj izjavu nema nikakav pokazatelj da je prihvatilo klik, provjeri i drugu dugmad"; "dugo se
čeka da se prikaže poster kreirane izjave, ne čekaj zapis u bazu"): (1) Gumbi QNC-AI-ja odgovaraju: pod mišem malo svjetliji, dok
se drže svjetliji, a nakon klika kratko (0,35 s) zasvijetle; pokazivač ruke (gui-kit `qnc_ui_kit::press_feedback`; chrome, jaki i
gumbi forme, gumbi docka i reda ispod monitora). Uključuje ga tema (`MediaColors.press_feedback`, QNC-AI da; klasični QNC ne).
(2) Dodaj izjavu odmah stavlja karticu izjave u stupac Izjave, sa slikom IN-a koju je monitor već prikazao (inače poster klipa),
dok se zapis piše u pozadini (AGENTS §5: klip se smije prikazati dok se sprema); red iz baze je zamijeni i zadrži tu sliku dok se
ne pročita vlastita slika izjave; neuspjeli zapis je uklanja (QNC-shared `qnc-virtual-short-cards` `saving`, `drop_saving`;
`qnc-shot-stills` `poster_now`). Aplikacijski sloj nije narastao. Testovi. Uživo nije provjereno.
Popravak 2026-11-15 (korisnik: "kreirao sam novi projekt, nije mi ponudio upis podataka o nazivu, odmah je otišao s Ingesta na Media
Assist... od iste baze se mora moći kreirati više projekata"): Uvezi iz baze koja već ima svoj projekt više ne dodaje klipove tom
projektu bez pitanja. Kao i za bazu bez projekta, nudi se nastavak projekta baze (inače zadnjeg projekta) ili novi projekt s obrascem
(`qnc-story-start` `import_asked`, `poll`). Novi projekt iste baze dobiva bazu kao jednu od svojih (`story_ingest_bases`), a baza
pamti zadnji projekt. Uz to Ingest zapisuje trajanje svakog koraka pokretanja u player log (`start-*`, `ingest-start app made`).
Uživo nije provjereno.
Izmjena 2026-11-15 (korisnik: "traje dugo i otvara postojeći projekt; baze kartica ne bi trebale biti vezane s projektima, nego projekti
s bazom"; "status baze kartica je da je baza zamjena za karticu; projekti se linkuju na baze, kao u QNC-u opcija Link"): baza kartice
više ništa ne zapisuje o projektu (tablica `ingest_base_project` i `set_project_name` uklonjeni iz koda; stari zapisi u bazama se ne
čitaju). Projekt u svojoj bazi drži baze na koje je linkan (`story_ingest_bases`, uz vrijeme zadnjeg linka); jedna baza može biti
link više projekata. `IngestBase.project_name` je projekt koji je bazu linkao zadnji, pročitan iz projekata (`qnc-ingest-base`
`takers`, `story_bases`); `last_story` je projekt koji je zadnji linkao neku bazu; otvaranje projekta (`open_story`) otvara njegovu
najnoviju bazu i bilježi je kao zadnji link. Popis Baze u Ingestu pokazuje bazu kao karticu (kamera i vrijeme), ne projekt. "Novi
projekt" iz prozora Projekti otvara Ingest za novi projekt (`qnc-ai-open` `open_for_new_story`, `QNC_NEW_STORY`): Uvezi tada odmah
otvara obrazac novog projekta, bez ponude postojećeg (`qnc-story-start`). Test u `qnc-ingest-base` (dva projekta iz iste baze, baza
bez zapisa projekta). Uživo nije provjereno; tray nije ponovno izgrađen (bio je pokrenut).
Izmjena 2026-11-15 (korisnik: "bazi trebamo dodati pored datuma i vremena kreiranja i opis... postojeće polje koje smo koristili za
naziv projekta koristit ćemo za naziv baze; polje za upis naziva baze na tray-ingest vizardu koju korisnik treba upisati prije
zatvaranja vizarda"): baza kartice ima svoj naziv i opis u vlastitoj bazi (tablica `ingest_base_info`; `qnc-ingest-base` `set_info`,
`IngestBase.title`, `description`, `shown()` = "naziv · kamera datum vrijeme"). Tray Ingest prozor (`qnc-ai-card-ingest`) ima polja
Naziv baze i Opis, zapisuje ih kad se polje napusti i pri zatvaranju; zatvaranje bez naziva se odbija uz crvenu poruku. Popis Baze u
Ingestu i tray izbornik pokazuju naziv baze uz kameru, datum i vrijeme. Nije izvedeno: opis predložen iz analize kadrova. Test u
`qnc-ingest-base`. Uživo nije provjereno.
Ispravak 2026-11-22 (audit QNC-pro docs/08, greska 12.5; tray i wizard otkljucani): filmstrip ili val klipa koji ne
uspije dva puta odustaje se za taj rad radnika (`qnc-background-jobs` `failed`, `gave_up`, `TRIES` = 2; QNC-shared).
Prije se klip s greskom vracao u red pri svakom sync-u i servis artefakata radio je do roka od 1 sata
(`worker.artifacts_timeout`). Sljedeci rad radnika (novi proces) pokusava ga ponovno. Testovi `qnc-background-jobs`
(novi `a_clip_that_failed_twice_is_given_up_until_another_project`), `qnc-filmstrip-worker`, `qnc-wave-worker`.
Uzivo nije provjereno.
Ispravak 2026-11-22 (audit greska 12.6): kopija uvoza i dovrsetak zapisa klipova cekaju samo dok player priprema ili
svira, vise ne i dok korisnik montira (korak, oznaka). Nova `qnc-playback-activity::playback_plays` (QNC-shared;
`playback_pause` ostaje za klasicni QNC); koriste je `tools/qnc-ingest-worker` (zapisi) i
`qnc-ingest-import-worker` (kopija). Test `the_copy_waits_while_a_player_plays_not_while_the_user_edits`. Uzivo nije
provjereno.
Ispravak 2026-11-22 (audit greske 12.7 i 12.13): (1) u Breaking newsu analiza kadrova zadrzava radnike i dok prijepis
radi (`tools/qnc-ingest-worker` `make_shots`: pravilo "0 radnika dok traje prijepis" vrijedi samo izvan Breaking
newsa, jer tamo prijepis ide zadnji). (2) Preview klipa ciji zapis jos nije gotov (`qnc-source-preview`
`retry_incomplete_record`): radnik se budi odmah i zatim svakih 5 s po svom satu (prije je pokusaj svakih 250 ms
pomicao isto vrijeme, pa se radnik nikad nije ponovno probudio), a klip se otvara na slici na kojoj je bio otvoren,
ne na pocetku. Testovi `qnc-source-preview`, gradnja radnika. Uzivo nije provjereno.
Ispravak 2026-11-22 (audit greska 12.1): Breaking news radi i sa sistemskim predloskom. Predlozak bez
`workspace.application_selection` (nijedan sistemski ga nema) uzima aplikacije instalirane na ovom racunalu iz kataloga
aplikacija (`data/application-catalog.json`), prvu iz svake prioritetne grupe (`qnc-breaking-news`
`installed_applications`). Prije je Breaking news s takvim predloskom javljao "the chosen template has no applications"
i nije nastajao. Test `a_template_without_applications_takes_the_first_installed_of_each_group`. Uzivo nije provjereno.
Ispravak 2026-11-22 (audit greska 12.2): brisanje projekta (`qnc-project-store` `delete_project`) najprije u jednom
koraku premjesti mapu projekta u stranu (`<mapa>.deleting-<pid>-<ms>`; dok program drzi datoteku u njoj, premjestanje
se odbija i projekt ostaje cijel), zatim potvrdi brisanje u registru, i tek onda brise datoteke. Prije su se datoteke
brisale prije potvrde, pa je greska ostavljala projekt u popisu bez datoteka. Mapa koja se nakon potvrde ne da
obrisati ostaje u strani (zapis na stderr). Testovi `qnc-project-store` (43). Uzivo nije provjereno.
Provjera 2026-11-22 (audit greska 12.3): zastita mapa vec se postavlja samo kad je nema (`icacls_has`), a sistemski
predlosci se namjerno ponovno upisuju iz seeda. Ispravljeno samo to da se `attrib +h` vise ne pokrece za svaki
projekt pri svakom otvaranju: mapa koja je vec skrivena se preskace (`set_project_hidden`, atribut iz metapodataka).
Testovi `qnc-project-store` (43).
Ispravak 2026-11-22 (audit greska 12.4): novi projekt iz Projecta s imenom koje pocinje s "Ingest" se odbija
(`qnc-project-jobs` Create, tekst `project.msg.name_is_base`), jer bi postao Ingest baza (baza se razlikuje samo po
imenu). Ingest baze i dalje nastaju kroz `qnc-ingest-base`. Test u `a_new_project_keeps_its_story_sheet_in_its_own_database`.
Odluka 2026-11-22 (audit 12.12): prozor izjave i uvecani slot su 1/13 sa svake strane, ne 70/15/15 (korisnik:
"filmstrip ima 13 slicica, pa sa svake strane, ispred IN i iza OUT, ostaje jedna slicica";
`qnc-timeline-model` `enlarged`). Raniji zapisi koji kazu 70/15/15 su zastarjeli; nije greska.
Ispravak 2026-11-22 (audit greska 12.18): poruke panela scenarija (MA Audio, MA Script) vise nisu sve crvene. Poruka
koja kaze da posao ide kako treba (izvezeno, poslano u S-Story, ponisteno, snima se, trazim pokrivalice...) crta se
obicnom bojom, ostale crveno (`qnc-script` `is_plain`, popis sifri u `plain.rs`, poruka s vrijednostima se prepoznaje
po dijelovima teksta). Test `done_and_under_way_are_plain_a_refusal_is_not`.
Ispravak 2026-11-22 (audit greska 12.16): prva snimka OFF-a dobiva ime `<projekt>_<oznaka>` samo kad nijedna datoteka
mape glasa vec ne pocinje tim imenom; inace `_2`, `_3`... (`qnc-voice-take` `free_name`). Prije je drugi OFF s istom
oznakom (profil bez rednog cuea, ili blokovi prebrojani nakon dodavanja ili brisanja) prepisivao `_T1` datoteke i
klip kataloga prvoga. Snimke koje blok vec ima zadrzavaju svoje ime. Test `a_name_whose_files_are_there_is_taken_with_a_number`.
Ispravak 2026-11-22 (audit greska 12.11): XML export vise ne pada cijeli kad neki klip nije na ovom racunalu
(kartica nije spojena). Takav klip ulazi u XML kao offline, s putanjom kakva je na kartici, pa ga program za montazu
ponovno veze po imenu; zapis `export xml offline` u player logu (`tools/qnc-export`). Gradnja; uzivo nije provjereno.
Ispravak 2026-11-22 (audit greska 12.8): kljuc (otisak) dijela prerendera sadrzi samo ono sto mijenja sliku i zvuk:
kodek, velicinu, stopu, poluslike, format zvuka i bitrate (`qnc-program-render` `Delivery::key`, QNC-shared). Prije je
sadrzavao cijeli blok exporta, pa je novi pocetni timecode, mapa exporta ili ime preseta trazio da se svi dijelovi
renderiraju iznova. Postojeci dijelovi jednom se renderiraju ponovno (novi oblik kljuca). Test
`only_what_changes_the_picture_and_sound_changes_a_units_fingerprint`. Vrijedi i za klasicni QNC.
Odluke 2026-11-22: (1) proxy nije prioritet (snimatelji ga rijetko koriste; postaje vazan tek u Intranet okruzenju
TV kuce), pa greske 12.9 i 12.20 cekaju. (2) Enter je potvrda (spremanje). (3) OFF koji nije procitan ide i u XML i u
S-Story kao prazno mjesto duljine citanja.
Ispravak 2026-11-22 (audit greska 12.14): Enter na source timelineu s uzetim IN-om ili OUT-om sprema oznaku gdje jest i
pusta je (`qnc-panel-focus` route: `keep_marks` umjesto `clear_focus`); prije ju je vracao na prvi polozaj. Esc i dalje
vraca. Testovi `qnc-panel-focus` (zlatna tablica tipki prolazi).
Ispravak 2026-11-22 (audit greska 12.17, odluka korisnika): OFF koji nije snimljen ide u S-Story kao prazno mjesto
duljine citanja, kao u Nacrtu XML (prije se izostavljao). U bazi price to je Off klipa susjednog dijela (iza, inace
ispred) s iskljucenim A1, od slike 0 i duljine u slikama price, bez provjere duljine klipa (kao pauza slozene izjave):
`qnc-program-db` `AppendSegment.pause_millis`, `qnc-script-draft` `SkeletonPart.pause_millis`, `qnc-script`
salje ga. "Posalji u Story" iz S-Storyja salje takvo mjesto opet kao pauzu (`qnc-program-segments` writes). Testovi
`an_off_not_read_yet_is_an_empty_place_of_its_reading_time_held_by_a_clip`,
`the_skeleton_is_the_script_as_story_segments_with_an_empty_place_for_an_off_not_recorded`. Napomena: klasicni QNC
(`QNC/crates/qnc-script`, zamrznut) se s danasnjim QNC-sharedom vec nije gradio (nedostaje polje `fine` u
`qnc-transcribe-run`), a sada mu treba i `pause_millis`; nije diran. Uzivo nije provjereno.
Izmjena 2026-11-22 (korisnik nakon zive probe: "ingest qnc-pro bio je znatno brzi... pravilo ocito nije bilo dobro,
koristi system koji koristi qnc-pro"): u Breaking newsu radnik (`tools/qnc-ingest-worker` `breaking_held`) radi klip
po klip kao QNC-pro: valovi i filmstrip uzimaju svaki klip cim je kopiran, nikad ne cekaju kopiju cijele kartice;
AI kadrovi kad valovi nemaju posla, SigLIP iza kadrova, opisi zadnji. Prije su filmstrip i AI kadrovi cekali kraj
kopije svih klipova (log: 118 klipova, kopija ~30 MB/s kao QNC-pro, filmstrip 0 od 118 kroz cijelu kopiju).
Brzina kopije je ista (citac kartice).
Izmjena 2026-11-22 (korisnik): u zoomu izjave (IN slika, player, OUT slika) klik na IN ili OUT sliku uzima tu oznaku i
stavlja playhead na nju, kao Ctrl+I / Ctrl+O (`select_mark_in`, `select_mark_out`); klik na srednju sliku (player)
pusta uzetu oznaku gdje jest (`zoom_save`). Prije je klik samo pomicao playhead (`qnc-program-segments`
`source_stills`, QNC-shared). Test `a_click_on_the_in_or_out_picture_takes_the_mark_the_player_lets_it_go`.
Izmjena 2026-11-22 (korisnik): uzeti IN ili OUT na source timelineu crta se crveno (`TimelineTheme.cut`), da se vidi
da je oznaka u uredivanju; prije boja fokusa (`qnc-timeline` `paint_in_out_handles`, QNC-shared).
Ispravak 2026-11-22 (ziva proba): zoom za zivu sliku (player u zoomu izjave) vise ne dekodira slike unaprijed
(`qnc-trim-stills` `ahead`); prolazi od 25 slika trcali su za playerom i usporavali prelazak s oznake na oznaku.
Ispravljeno isti dan: klik na player u zoomu izjave pusta oznaku, playhead ide na IN, a zoom ostaje otvoren bez uzete
oznake dok se ne zatvori s Esc ili Zatvori (`qnc-program-segments` `statement_zoom`, `close_statement_zoom`;
`qnc-panel-focus` `zoom_save` s vrijednoscu IN). Test `a_statements_zoom_stays_open_after_its_mark_is_let_go_until_it_is_closed`.
Izmjena 2026-11-22 (korisnik: "pokušaj srediti prijepis.. nova rečenica novi red"): baza ingest projekta (samo čitanje) pokazala je da retci hrvatskog modela (wav2vec2) nemaju točke, pa su se u panelu prijepisa rečenice lijepile preko pauza u dugi odlomak. Rečenica (novi red u panelu prijepisa) sada završava i na pauzi između riječi od 0,7 s (`SENTENCE_PAUSE_SECONDS`, u slikama klipa iz timecodea) i kad novi redak motora počinje velikim slovom (`qnc-transcript` `sentences(lines, pause)`, `sentence_pause`). Točka, upitnik i uskličnik i dalje završavaju rečenicu. Test `without_stops_a_pause_or_a_new_line_with_a_capital_ends_a_sentence`. Uživo nije provjereno.
Popravak 2026-11-22 (korisnik: "istraži i popravi spajanje... zadržati paraketovu interpunkciju, a wav2vec dorađuje overwrite riječ o riječ"): ispravak Parakeetova prijepisa hrvatskim modelom (`qnc-transcript-db` `corrected`, sada modul `correct.rs` u QNC-shared) dodjeljivao je riječi samo po vremenu u koje padaju: riječ hrvatskog modela izgovorena malo ranije išla je prethodnoj Parakeetovoj riječi, a njezina je ostala, pa je riječ bila dvaput ("A bit Bit će", "Pa Pa"). Sada se dva popisa riječi poravnavaju cijeli, po tekstu (sličnost slova) i vremenu (najviše 1,5 s između sredina): poravnata Parakeetova riječ dobiva riječ hrvatskog modela s Parakeetovim velikim slovom, interpunkcijom i vremenom; riječ koju hrvatski model nije čuo ostaje; riječ koju Parakeet nema ulazi u svoj redak po vremenu, malim slovom unutar rečenice. Slova slijepljena sa znamenkama u izlazu hrvatskog modela ("tih120") dijele se u dvije riječi. Ponovni prijepis istog dijela oslanja se i na već ispravljeni tekst (`qnc-transcribe-run` `corrected_by_fine`), pa drugi prolaz ne gubi interpunkciju. Postojeći prijepisi se ne mijenjaju dok se taj dio ne prepiše ponovno. Testovi `the_fine_words_correct_the_rough_ones_and_keep_their_sentences`, `a_fine_word_a_little_off_its_rough_one_overwrites_it_and_is_not_there_twice`. Uživo nije provjereno.
Izvedeno 2026-11-22 (korisnik: "u formi Postavke dodati putanju Externog AI Agenta i LLM modela, te putanju importa iz, ili kroz čitač kartica spojen na udaljenoj lokaciji"; odluke: AI agent je adresa AI servera, LLM se zasad samo zapisuje, čitač je mrežna dijeljena mapa): forma Postavke (QNC-AI Settings) ispod orkestratora ima blok `qnc-machine-settings-view` (+ nove kocke `qnc-llm-place` i `qnc-card-readers`). Vanjski AI agent: adresa:port, otisak certifikata i token AI servera ovog računala (`qnc-ai-place` `set_server_of` u QNC-shared, datoteka `data/ai-place.json`; prazna adresa briše postavku i AI radi na ovom računalu). LLM model: putanja ili adresa, samo se zapisuje u `data/llm.json` (QNC-AI ga još ne koristi). Čitači kartica na mreži: naziv i dijeljena mapa (npr. `\192.168.100.28\kartica`) u `data/card-readers.json`; svaki je izvor Ingesta `qnc://lan/<host>/source/reader-<naziv>` (`qnc-source-bindings` `with_card_readers`, u `load`, s probeom lokalnih izvora), pa ga Ingest pokazuje pod LAN i Select ga čita kao karticu (mapa se samo čita); pogrešna postavka čitača se zapisuje u log, ostali izvori ostaju. Postavke AI-ja se pišu na Spremi, čitači odmah na Dodaj i Ukloni. Testovi `qnc-ai-place`, `qnc-llm-place`, `qnc-card-readers`, `qnc-source-bindings` (`a_card_reader_on_the_network_is_a_lan_source_read_from_its_folder`). Uživo nije provjereno; dijeljenje kartice na Jetsonu (Samba) još nije postavljeno.
Popravak 2026-11-22 (korisnik: "dodao sam jetson, pogledaj log"; Ingest LAN: "Nema podešenih LAN izvora"): izvor čitača imao je u adresi naziv računala s točkama (`qnc://lan/192.168.100.28/...`), a izvor smije imati samo slova, znamenke, `-` i `_`, pa se cijeli popis izvora nije učitao i Ingest je ostao na starom bez riječi. `qnc-card-readers` `host`: točka i drugi znakovi postaju `-` (`qnc://lan/192-168-100-28/source/reader-jetson`); Ingest sada pokazuje grešku kad se izvori ne mogu pročitati (`qnc-ingest-application` `browse.rs`). Primjer `qnc-ingest-select/examples/sources.rs` (samo čitanje) ispisuje izvore taba kao Ingest. Test `the_source_of_a_reader_shared_by_an_address_is_a_valid_source_reference`; provjereno na ovom računalu: LAN pokazuje Jetson.
Popravak 2026-11-22 (Ingest LAN, Jetson: "Source entry not found"): čitač izvora (`qnc-source-reader`, QNC-shared) mapu lista kroz `cap-std`, a `cap-std` dijeljenu mapu mreže (`\192.168.100.28\kartice`) ne zna izlistati (datoteke otvara). Kad `cap-std` mapu ne izlista, lista je običan sustavski poziv nad mapom izvora i provjerenom relativnom putanjom (bez `..`); ostalo čitanje ostaje kroz `cap-std`. Kocke kamera nisu mijenjane: kartica s mreže prolazi isti Select (FX6 profil, MEDIAPRO.XML). Primjeri `qnc-source-reader/examples/list_root.rs` i `qnc-ingest-select/examples/sources.rs` (samo čitanje); provjereno: Jetson se otvara, vide se `kartica` i `XDROOT`. Na Windowsu pristup dijeljenoj mapi treba biti spremljen u upravitelju vjerodajnica (`cmdkey`), jer `net use` vrijedi samo u svojoj sesiji. Na Jetsonu (bez monitora) kartica se montira sama samo za čitanje u `/media/mironik/kartica` (udev pravilo + usluga `qnc-kartica@.service`), Samba je dijeli kao `kartice`.
Izmjena izgleda 2026-11-22 (korisnik: "za Uvoz iz direktorija napravi manji prozor", "sam ću podešavati"; "razmakni tabove izvora"): prozor kartice traya dok se bira direktorij ima svoju veličinu iz datoteke izgleda forme (`contracts/ui/forms/card-ingest.look.json`: `browser_width` 540, `browser_height` 520; podešava se uređivačem izgleda), a kad uvoz krene vraća se na `window_width`/`window_height` (`qnc-form-look` `window_of`). Tabovi izvora (Računalo, LAN, Internet, Baze) u Ingestu imaju razmak 28 (`qnc-dir-browser-panel` `show_with_confirm_spaced`, `BrowserStyle.kind_gap`; ostali preglednici zadržavaju 10). Uživo nije provjereno.
Popravak 2026-11-22 (korisnik, kartica s Jetsona: "postere koji se učitaju za sekundu on nakon tri minute nije učitao"; "materijal mora na laptop": Jetson ne dekodira XAVC 4:2:2 10-bit, pa nema Net načina, kopija ide na laptop): posteri (207 sitnih datoteka) i kopija originala krenuli su istodobno preko istog Wi-Fi-ja (~35 MB/s), pa su posteri čekali iza kopije oko tri minute. Pozadinski radnik sada kopiju originala pokreće tek kad su posteri gotovi (`tools/qnc-ingest-worker` `run.rs`, korak Import čeka `not_done(Posters)`), a posteri kartice kopiraju se po četiri istodobno i svaki se upisuje čim stigne (`qnc-missing-posters` u QNC-shared). Brojke prozora uvoza su ispravne (prozor čita prikvačenu bazu `QNC_WORK_PROJECT`); `qnc-work-progress/examples/progress.rs` prima i id baze. Uživo nije provjereno.
Popravak 2026-11-22 (korisnik: "krivi selekt klipova, nije originalna kocka, uvoziš i klipove i proxy"): na Jetsonu je Odaberi kliknut na korijenu dijeljene mape (`reader-jetson`), a kartica je u podmapi `kartica`; Select je karticu tražio samo u odabranoj mapi i iznad nje, pa je svaku video datoteku (MXF originale i `Sub/*S03.MP4` proxyje) uzeo kao zaseban klip, a postere radio dekodiranjem preko mreže. Sada, kad u odabranoj mapi ni iznad nje nema kartice, Select je traži i u podmapama do dvije razine (najviše 64 mape, bez sistemskih), pa radi prava kocka kamere (`qnc-ingest-select` `scan.rs` `folders_below`). Test `failed_probe_is_durable_and_never_retried` pada i bez ove izmjene (postojeći kvar). Uživo nije provjereno.
Popravak 2026-11-22 (Administracija: "Ingest baza ... nije obrisana: NoOwner" nakon što je baza već obrisana): bazu su istodobno brisali prozor uvoza i Administracija; Administracija je stigla druga i bez zapisa o vlasniku javila sirovu oznaku. Baza koje više nema sada je obrisana bez greške (`qnc-base-admin` `delete`).
Izvedeno 2026-11-22 (korisnik: "i dalje ne zna koja je kamera, koji je serijski broj kartice"): kartica u čitaču na mreži je kartica kao ona u ovom računalu. Čitač montira svaku karticu u mapu nazvanu po serijskom broju njezina volumena (exFAT UUID, `A815-5186`; isti broj koji Windows čita kad je kartica u laptopu), pa `qnc-card-readers` `cards`/`mounted_and_cards` takvu mapu daje kao volumen ovog računala (`qnc://local/source/volume-<serijski>`, korijen je mreža). Koriste ga izvori (`qnc-source-bindings` `load`), popis kartica s kamerom i ranijom bazom (`qnc-attached-cards`), tray (`qnc-card-watch` `with_root`) i preglednik Selecta (mapa na mreži je spojena). Ista kartica je isti izvor gdje god je utaknuta. Na Jetsonu treba montiranje po UUID-u (udev + usluga). Uživo nije provjereno.
Izmjena 2026-11-22 (korisnik: kartica s Jetsona pod Računalo s punim nazivom, pod LAN samo kao mapa): kartica ovog računala u čitaču na mreži prikazuje se pod tabom LAN (`qnc-dir-browser` `BrowserSource::on_tab` u QNC-shared, `qnc-ingest-select` `on_network`), i dalje je isti izvor `volume-<serijski>`. Duga putanja ovog računala (`\?\C:\...`, mapa baza) nije mreža (test `a_share_is_on_the_network_a_long_path_of_this_computer_is_not`).
Izmjena 2026-11-22 (korisnik: LAN popis "ovo je zbunjujuće": "Jetson 0473-9A66 · Sony ILME-FX6V | 04739a66" i "Jetson | jetson"): kartica u čitaču na mreži zove se po čitaču ("Jetson", uz nju kamera i serijski broj), a cijela dijeljena mapa čitača ne nudi se dok u njemu ima kartica i nema svoju oznaku (`qnc-card-readers` `cards`, `qnc-source-bindings` `with_card_readers`; veza izvora ostaje za ono što je s nje uzeto).
Popravak 2026-11-22 (log: tray je prepoznao karticu u Jetsonu i napravio bazu po kameri, a prozor uvoza 20 puta tražio karticu među diskovima i krenuo s 0 klipova): automatski izbor kartice (`qnc-card-autoselect`) karticu koju ne nađe traži naizmjence pod LAN i među diskovima računala, dok je ne nađe ili istekne vrijeme traženja; kartica čitača na mreži je pod LAN.
Izmjena 2026-11-23 (korisnik: "iza dugmeta Dodaj izjavu dugme Dodaj Složenu izjavu, iza nje Ažuriraj, a iza njega Idi na Skript (OFF)"): AI Media Assist (grupa e, raspored `QNC-shared/contracts/ui/ai-editorial.layout.json`, koji forma ugraduje; prvo je greskom ureden `contracts/ui/editorial.layout.json` i vracen) u zaglavlju source timelinea ima redom Dodaj izjavu, Dodaj složenu izjavu (`script_subsegment`), Ažuriraj, Idi na Script (OFF) (`go_to_script`, katalog bez tipke). Forma koja ima gumb Dodaj složenu izjavu piše scenarij projekta i kad ne pokazuje njegov panel (`qnc-panel-focus`: komad scenarija iz gumba), pa MA Script složenu izjavu čita iz baze. Idi na Script otvara MA Script (`qnc-ai-open` `go_to_script`, isti QNC korijen), AI Media Assist ostaje otvoren; greška je crvena traka. Tekstovi `script.add_subsegment`, `script.go_to_script` (QNC-AI i QNC-shared). Uživo nije provjereno.
Popravak 2026-11-23 (korisnik: "opet experiment... traži kako je dogovoreno da se radi složena izjava"): Dodaj složenu izjavu u AI Media Assistu vraćena je na dogovor od 2026-11-19 (vraćen 2026-11-20 sa cijelim popodnevom): svaka izjava može postati složena; gumb (`statement_add_part`) dodaje IN/OUT prikazanog klipa kao novi dio izabrane izjave (`virtual_shot_parts`, QNC-shared b43487c, b0593ed, 5c2d6ea), siv dok izjava nije izabrana; izjava drži samo IN/OUT dijelova, bez teksta; klik na izjavu s više dijelova svira i pokazuje dio po dio kao pokrivalice (`qnc-side-pieces` `statement.rs` kroz `qnc-subsegment-edit`, commit 0ff547c). Današnji pokušaj (scenarij u AI Media Assistu iz gumba `script_subsegment`) uklonjen. Uživo nije provjereno.
Izmjena 2026-11-23 (korisnik: "naredni odabrani tekst u prijepisu dodaje nastavak na tu izjavu, rez se može napraviti SMOOTH, BLIC, CAT"; odluke: svaki nastavak gumbom, vrsta reza odmah pri dodavanju, Blic je bijeli bljesak), korak 1: izabrana izjava + označeni tekst u prijepisu + Dodaj složenu izjavu otvara prozor "Spoj s prethodnim dijelom" s Rez (Cut, predložen), Smooth, Blic i Odustani; dio se upisuje tek na odgovor, s vrstom spoja (`qnc-virtual-shots` `JoinKind`, stupac `virtual_shot_parts.join_kind`, zadano cut; `qnc-short-save` `JOIN`, `JOIN_CANCEL`). Sviranje Smootha i Blica nije izvedeno (koraci 2 i 3): dok nisu, svaki spoj svira kao rez. Uživo nije provjereno.
Popravak 2026-11-23 (korisnik, slika: složena izjava bez filmstripa i vala; "ispred prvog dijela izjave treba biti 1/13 timeline i iza zadnjeg dijela 1/13, kao što je na Izjavama"): izjava s više dijelova otvarala se bez projekta, pa čitač valova i filmstripa nije postavljen; sada `statement.rs` daje projekt previewa (`qnc-source-preview` `project_id`). Ispred prvog i iza zadnjeg dijela je klip u duljini 1/13 prozora (koliko klip ima), bez trake trajanja i rubova, zatamnjen kao izvan IN/OUT izjave (`SubsegmentView.margins`, `qnc-subsegment-panel`); izjava se otvara s playheadom na IN-u prvog dijela. Uživo nije provjereno.

Izmjena 2026-11-23 (korisnik: "Pley izjava i Složenih izjava ide od IN do OUT izjava"): jedno pravilo playa od IN do OUT,
nova `qnc-source-panel` `RangePlay`. Kad je izjava prikazana (prozor 1/13), play koji krene izvan njezina IN..OUT najprije
skoci na IN, a play koji prijede OUT stane na OUT (playhead ostaje na OUT-u, sljedeci Space opet krece od IN-a). Isto vrijedi
za izjavu s vise dijelova: play krece od IN-a prvog dijela i staje na OUT-u zadnjeg, rubovi 1/13 se ne sviraju
(`qnc-side-pieces` `statement.rs`). Obican klip bez izjave radi kao prije (play prije OUT-a staje na OUT-u, s OUT-a svira
dalje). Testovi `qnc-side-pieces`, `qnc-source-timeline`; test `qnc-source-panel`
`undo_puts_only_the_taken_mark_back_where_it_was` pada i bez ove izmjene. Uzivo nije provjereno.

Izmjena 2026-11-23 (korisnik: "klik na traku trajanja selektira odabrani slot, to treba obvezno raditi u zoomu"; "ZUM i kod
Složene izjave treba imati centralno Player kao što je i kod izjave"; log: klik na traku u zoomu slao je `subsegment_select`,
ali je uzeti rub ostajao i nista se nije vidjelo): zoom dijela slozene izjave je kao zoom izjave: IN slika dijela | Player
(slika na playheadu) | OUT slika dijela, ispod IN i OUT traka trajanja slota (`qnc-subsegment-edit` `view::part_zoom`).
Otvara se kad se uzme vlastiti rub dijela i ostaje otvoren (`zoom_part`) dok ga Esc bez uzetog ne zatvori; klik na IN ili OUT
uzima taj rub (nikad roll), klik na Player sprema i pusta uzeti rub i stavlja playhead na IN dijela, klik na traku bira slot
i zoom pokazuje taj slot (uzeti rub drugog dijela ili cijeli rez najprije se spremi). Cijeli rez spoja (roll) i ton spoja i
dalje imaju zoom s dvije slike spoja. Isto u MA Scriptu (ista kocka). Testovi `qnc-subsegment-edit`, `qnc-side-pieces`,
`qnc-script`. Uzivo nije provjereno.

Dopuna 2026-11-23 (korisnik, slika zooma s dvije slike: "i dalje ne vidim monitor-player u sredini kao sto ima Izjava"; log:
desni klik na spoj uzeo je cijeli rez `1:cut`, a zoom cijelog reza imao je samo dvije slike): i zoom cijelog reza spoja ima Player
u sredini: OUT zadnja prethodnog dijela | Player (slika na playheadu) | IN prva sljedeceg (`view::subsegment_zoom`). Klik na
Player sprema rez, rez ostaje uzet, playhead ide na rez. `qnc-trim-overlay` (QNC-shared): Player u sredini nije strana reza
(nema crveni okvir fokusa, linija reza ide izmedju dviju strana). Test zooma spoja prilagoden. Uzivo nije provjereno.

Popravak 2026-11-23 (korisnik: "nema playera"; log: `trim-stills asked [Player @318 ...]` svaku slicicu nova slika dok svira,
dekodiranje ne stize i prozor je crn): Player u sredini zooma slozene izjave dobiva sliku monitora kao zoom izjave
(`qnc-subsegment-edit`: `trim_stills.fill(.., preview.shown_picture())`, prije `None`). Uzivo nije provjereno.

Popravak 2026-11-23 (korisnik: "u zoomu SLOT krivo, analiziraj kakvo treba biti ponasanje slota"; log: s uzetim cijelim rezom
`1:cut` lijevi klik na desnu traku je spremio rez i prebacio zoom na IN | Player | OUT dijela 2, drugi klik vise nista): po
zapisanom pravilu slota (5. i 9. 11.) lijevi klik na traku u zoomu bira slot te strane reza, zoom ostaje na istom rezu, nista se
ne sprema ni ne zatvara; dok uzeto nije pomaknuto izbor ga zamjenjuje vlastitim rubom tog slota (lijeva traka OUT slota prije
reza, desna IN slota poslije), pa strelice produljuju ili skracuju samo taj slot; pomaknuto ceka Enter. Zoom uvijek pokazuje rez
koji se uredjuje: strana prije | Player | strana poslije (zoom jednog dijela IN | Player | OUT iz prethodnog koraka uklonjen).
Klik na Player sprema uzeto, ostaje uzeto, playhead na rez. Desni klik na traku i dalje uzima slot i njegov blizi rub.
Uzivo nije provjereno.
Izmjena 2026-11-23 (korisnik: "kad se lijevim klikne na Player, lijeva slicica prikazuje IN prvog dijela izjave, a desna OUT
izjave; kad se klikne na SLOT uzima se taj slot, lijeva slicica IN odabranog slota, desna OUT; isto za naredni slot; klik na
sliku uzima marker: lijeva IN, desna OUT"): zoom vlastitog ruba slozene izjave (i izjave od vise dijelova u AI Media Assistu)
pokazuje doseg: cijelu izjavu ili jedan slot (`qnc-subsegment-edit` `Scope`, `view::scope_zoom`). Lijevo IN dosega (prva slika
prvog dijela), u sredini Player, desno OUT dosega (zadnja slika zadnjeg dijela); klik na Player je cijela izjava, klik na traku
trajanja je taj slot (i na timelineu dok je zoom otvoren), klik na sliku uzima taj marker (lijeva IN, desna OUT), uzeti je
uokviren. Pomaknuti rub upise se prije promjene dosega. Rub uzet izvan dosega postavlja njegov slot. Rubovi 1/13 izjave nisu
slotovi (`set_margins`; klik na njih ne radi nista). Enter i Esc zatvaraju zoom; Spremi ga ostavlja. Roll cijelog reza i J/L
spoja zadrzavaju zoom spoja. Test `the_zoom_of_a_scope_shows_its_in_the_player_and_its_out_and_a_picture_takes_that_marker`.
Uzivo nije provjereno.
Popravak 2026-11-23 (korisnik: "cilj mi je bio da shvatis razliku izmedju klika na slicicu i klika na traku u boji koja
pokazuje simbolicno trajanje slota"; log: klik na traku pretvarao je uzeti rez u rub tog slota, a klik na traku istog slota
ispustao marker): u zoomu slozene izjave klik na sliku uzima njezin marker (lijeva IN, desna OUT), a klik na traku trajanja
(u zoomu ili na timelineu dok je zoom otvoren) uvijek bira taj slot i nikad ne uzima marker: pomaknuti rub ili rez upise se,
marker se pusta, zoom pokazuje IN i OUT tog slota. Uklonjeno pretvaranje trake u rub slota (pravilo od 5. i 9. 11. za slozenu
izjavu). Ostali slotovi biraju se trakama timelinea, ne dodatnim slikama u zoomu. Uzivo nije provjereno.
Izmjena 2026-11-23 (korisnik: "slozenu izjavu slazes kao pokrivalice, to je greska; slozene izjave su flat, nemaju podlogu
segmenta"): timeline slozene izjave (`qnc-subsegment-panel`) vise ne crta dijelove kao segmente s podlogom ispod traka
(virtualni spanovi uklonjeni, bez zatamnjene podloge): dijelovi su jedan red klipova, svaki sa svojim filmstripom, valom A1
i trakom trajanja svog slota. Dvoklik na A1 vise ne otvara dio kao segment; dvoklik na sliku i dalje povecava dio. Uzivo nije
provjereno.
Izmjena 2026-11-23 (korisnik: "slozena izjava je vise izjava u jednom prikazu"): svaki dio slozene izjave je izjava sa svojim
IN-om i OUT-om. Spoj dvaju dijelova vise nije jedan zajednicki rez: desni klik (i desni klik na sliku zooma) prije spoja uzima
OUT dijela prije, od spoja IN dijela poslije (`qnc-subsegment-edit` `nearest_edge`, nikad `cut`); roll dvaju dijelova vise se
ne uzima misem. J/L tona spoja ostaje. Testovi prilagodjeni. Uzivo nije provjereno.
Izmjena 2026-11-23 (korisnik: "kad se promijeni duljina slota ili mjesto markera to treba biti vidljivo odmah, kao kad se
montira u timelineu"; log: AI Media Assist, IN drugog dijela pomican strelicama, Spremi tri puta): (1) nacrt ruba dijela je
timeline: dijelovi iza pomaknutog ruba pomicu se s njim odmah (`qnc-subsegment-edit`; uklonjeno pravilo od 4. 11. da ostali
dijelovi stoje do Entera). (2) U AI Media Assistu ono sto Enter ili Spremi zadrze ostaje u prikazu izjave
(`qnc-side-pieces` `statement.rs`: blok izjave postaje zadrzani nacrt, rubovi 1/13 ostaju uz prvi IN i zadnji OUT; prije se
vracalo na staro jer se upis bacao). U bazu izjava se jos ne zapisuje. Uzivo nije provjereno.
Popravak 2026-11-23 (korisnik: "klik na player-monitor u sredini zooma treba postaviti playhead na IN prvog klipa bez obzira sto
je prije bilo odabrano"; log: klik je stizao i prebacivao na cijelu izjavu, playhead je ostajao): klik na Player u zoomu slozene
izjave uz cijelu izjavu stavlja playhead na IN prvog dijela (rubovi 1/13 nisu dijelovi). Uzivo nije provjereno.
Popravak 2026-11-23 (korisnik: "otkazuje poslusnost, izgubi se"; log: s uzetim IN-om drugog dijela klik na A1 u zoomu uzeo je
J/L tona spoja, zoom se prebacio na stari zoom spoja, Player je tada samo spremao, a klik na sliku uzimao sliku spoja, pa se iz
zooma nije moglo izaci): vlastiti rub dijela je rub izjave; A1 u zoomu i Ctrl+1 uzimaju zvuk tog dijela, nikad J/L spoja.
J/L spoja uzima se samo uz cijeli rez spoja (koji se misem vise ne uzima). Zoom dosega ostaje. Uzivo nije provjereno.
Izmjena 2026-11-23 (korisnik: "to mozemo imati kao postavku u Postavkama, na taj nacin nista nije hardkodirano"; Lokalno -
Story otvara Ingest QNC-AI-ja): forma koju nacin rada traya otvara nakon ingest Wizarda i na "Otvori aplikaciju" je postavka
racunala `data/tray-forms.json` (`qnc-tray-mode` `Forms`, `Opens`: program iz kataloga aplikacija i argumenti), ne kod; nacin
bez upisa ne otvara nista. Tray i prozor uvoza nisu mijenjani (`Mode::opens` cita postavku). Ovo racunalo: Breaking news AI
Media Assist na tabu All, Story Ingest QNC-AI-ja, Mreza nista. Test. Uzivo nije provjereno.
Izmjena 2026-11-23 (korisnik: "prvo sto trebamo je i postavke pretvoriti u kocke, najprije ono sto trenutno vec koristimo";
"forma postavke se slaze od kocaka"; otkljucani tray i `qnc-project-blocks`): forma Postavke QNC-AI-ja slaze kocke redom koji
pise u `contracts/ui/settings.layout.json` (`blocks`: locations, ai_share, ingest_orchestrator, ai_server, card_readers,
templates; nepoznat naziv je greska). `qnc-machine-settings-view` razdvojen u dvije kocke: `qnc-ai-server-view` (AI server i
LLM) i `qnc-card-readers-view` (citaci kartica); ono sto obje crtaju isto je `qnc-settings-block` (naslov s napomenom, polje,
poruka). `qnc-project-blocks`: `SettingsBlock`, `settings_block`, `templates` (template i Advanced); lokacije vise ne crtaju
ostale dijelove. Redoslijed i izgled isti kao prije. Test. Uzivo nije provjereno. Sljedece: kocka postavki traya s kvacicama
za stavke izbornika (korisnik: "u postavkama trebaju biti checkboxovi za postavljanje onoga sto ce se vidjeti u trayu").
Izvedeno 2026-11-23 (korisnik: "sadrzaj traya nije hardkodiran, i njega cemo kreirati u Postavkama"; "u postavkama trebaju
biti checkboxovi za postavljanje onoga sto ce se vidjeti u trayu"; otkljucan tray): nova kocka `qnc-tray-menu` (+ postavka
racunala `data/tray-menu.json`: skrivene stavke; bez postavke vide se sve, Izlaz uvijek) i nova kocka Postavki
`qnc-tray-settings-view` (kvacica za svaku stavku izbornika traya, upis odmah; za svaki nacin rada forma iz kataloga aplikacija
ili Nijedna i njezini argumenti, upis na Spremi u `data/tray-forms.json`). Blok `tray` u `contracts/ui/settings.layout.json`.
Tray slaze izbornik iz postavke i cita je ponovno svake 2 s (promjena u Postavkama vidi se bez ponovnog pokretanja).
Tekstovi `settings_tray.*`, `tray.status_item` (en, hr). Prozor uvoza i AI Media Assist nisu mijenjani. Test. Uzivo nije
provjereno.
Izvedeno 2026-11-23 (korisnik: "postavke trebamo graditi od kraja, a to su Export postavke; dvije vrste exporta: export file i
export XML; za export file vise templatea, obrazac za kreiranje korisnickih postavki koristeci nase predefinirane postavke;
samostalna kocka koja ima svoj GUI koji se poziva iz forme Postavke"; "na racunalu"; "sve sto je potrebno za postavke exporta
gotovog priloga"; "svaka postavka samostalni json, kako bi se mogle lako prenositi na druga racunala"): nova kocka
`qnc-export-templates` (predlosci racunala, svaki zasebna datoteka `data/export-templates/<id>.qncexport` (JSON): naziv, preset od kojeg je
napravljen, `export.*` vrijednosti; gotovi preseti iz `contracts/export_profiles.json` nikad se ne prepisuju, id koji bi bio
id preseta dobiva `_own`) i polja predloska kao ugovor `contracts/export-template-fields.json` (grupe: slika i spremnik, zvuk,
predaja, export XML; izbor nudi vrijednosti koje gotovi preseti imaju, broj i tekst se upisuju, sklopka). Nova kocka Postavki
s vlastitim prozorom `qnc-export-templates-view`: u formi jedan redak (broj predlozaka, Uredi...), u prozoru popis vlastitih i
gotovih preseta kao polazista, naziv, polja po grupama, Spremi, Obrisi. Blok `export` u `contracts/ui/settings.layout.json`.
Tekstovi `export_template.*` (en, hr). Test. Uzivo nije provjereno. Nije izvedeno: export prozor price jos ne nudi predloske
racunala (cita gotove presete i vlastite iz projekta).
Izvedeno 2026-11-23 (korisnik: "povezi predloske s export prozorom u Storyju, dodaj svugdje nase ekstenzije"): (1) export
predlosci racunala ulaze u Storyjev prozor exporta i u alat exporta kao postavke exporta projekta
(`qnc-export-templates` `with_templates`: dodaje ih u `custom_presets` vrijednosti koja se predaje, projekt se ne pise;
`qnc-segments-keys` `export`, `tools/qnc-export` `--render`); test. (2) Postavke racunala koje su samo QNC-AI-jeve imaju svoje
ekstenzije (JSON unutra): `tray-mode.qnctray`, `tray-menu.qnctray`, `tray-forms.qnctray`, `card-readers.qncreaders`,
`llm.qncllm`, `ingest-orchestrators.qncingest`, export predlosci `.qncexport`; postojece datoteke ovog racunala preimenovane.
Nije izvedeno: postavke koje dijele QNC i QNC-AI kroz zajednicke kocke (`ai-place.json`, `ai-share.json`, `ui-language.json`,
`ingest-bases.json`, `ingest-transport.json`, `mouse-wheel.json`, `voice-input.json`, AI predlosci) - to bi mijenjalo i klasicni
QNC. Uzivo nije provjereno.
Izmjena 2026-11-23 (korisnik: "forma treba imati fiksne dimenzije koje je moguce editirati preko skrivene tocke"; snimka:
prozor export predlozaka sirio se preko lijevog ruba ekrana, polja predaje i XML-a odrezana; predlozak nije spremljen jer
naziv nije upisan): prozor export predlozaka ima fiksnu velicinu i sirinu popisa iz rasporeda Projecta
(`export_templates_window`: width, height, list_width), koje uredjuje editor izgleda Postavki (skrivena tocka; vrijednosti
`export_window_width`, `export_window_height`, `export_list_width` u `contracts/ui/forms/settings.look.json`). Polje naziva ima
napomenu da Spremi trazi naziv. Uzivo nije provjereno.
Izmjena 2026-11-23 (korisnik, snimka: "ne vidi se cijela forma, postavi je na sredinu ekrana; lijevi dio je uzak, ne vidi se
naziv i skrivena tocka nije dostupna; XML isto postavi u lijevi izbornik i linijom ga odvoji od enkode postavki"): prozor
export predlozaka je vlastiti prozor (viewport) na sredini ekrana s vlastitom naslovnom trakom; njegova skrivena tocka otvara
njegov editor izgleda (`contracts/ui/forms/export-templates.look.json`: sirina, visina, sirina izbornika, stupac oznaka,
margina; fiksna velicina). `qnc-title-bar` `with_look_of`, `qnc-form-look` `ask_editor_of`, `editor_of_own` (drugi prozor
programa otvara samo svoj editor). Mjere iz rasporeda Projecta iz prethodnog koraka uklonjene. Export XML je zasebna postavka
racunala (`data/export-xml.qncxml`, `qnc-export-templates` `xml_fields`, `xml`, `save_xml`; ugovor polja `xml`), u lijevom
izborniku ispod crte. Uzivo nije provjereno.
Izmjena 2026-11-23 (korisnik, snimka: "pogledaj poravnanje u postavkama, treba biti stilski isto, ne mozes mijenjati; treba
dodati i sirinu desnog panela i druge mjere"): polja prozora export predlozaka (i naziv) crta ista mreza kao napredne postavke
(`qnc-settings-form` `field_grid`: oznaka iznad polja; grupe kao odjeljci `qnc-settings-rows` `section` s crtom), sklopka
ispod mreze. Izgled prozora (`export-templates.look.json`): sirina i visina prozora, sirina izbornika lijevo, sirina desnog
panela, margina, velicina slova oznaka, najmanja sirina polja, razmak polja, visina polja. Polja sirina, visina, bitrate,
frekvencija i broj kanala su cijeli brojevi. Napomena: ovom izmjenom prepisana je datoteka izgleda koju je korisnik dvaput
spremio editorom (njegove mjere nisu sacuvane). Uzivo nije provjereno.
Izvedeno 2026-11-23 (korisnik: "idemo dalje rjesavati postavke, sljedece neka budu AI postavke isto samostalna forma"; "dodaj
sve sto imamo i sto je planirano: AI predlosci, AI motori, AI server, LLM i AI mapa, oznake kadrova"), korak 1 od 3: nova kocka
`qnc-settings-window` (vlastiti prozor bloka Postavki na sredini ekrana, vlastita traka i editor izgleda, fiksna velicina,
izbornik lijevo, panel desno, mreza polja Postavki; mjere `LOOK_KEYS`); export predlosci je koriste (njihov prozor vise nema
vlastitu kopiju). Nova kocka `qnc-ai-settings-view` (+ izgled `contracts/ui/forms/ai-settings.look.json`): u formi jedan redak,
u prozoru lijevo vlastiti i gotovi AI predlosci (`qnc-ai-features`: naziv, AI mogucnosti s ovisnostima, Spremi kao vlastiti,
Obrisi vlastiti), ispod crte AI motori, AI server i LLM (`qnc-ai-server-view` `panel`, mreza), zajednicka AI mapa
(`qnc-ai-catalog` `set_share`), oznake kadrova. Blokovi `ai_share` i `ai_server` iz forme Postavke zamijenjeni blokom `ai`.
Datoteke su iste kao prije (dijeli ih klasicni QNC). AI motori i oznake kadrova: korak 2 i 3. Uzivo nije provjereno.
Izvedeno 2026-11-23 (AI postavke, korak 2 od 3: AI motori): nova kocka `qnc-ai-catalog-edit` (AI katalog racunala kao JSON,
polja koja ne zna ostaju): koji motor je najbolji, brzi i prijepis dok svira, niti i klipova odjednom svakog motora, model
svakog drugog AI posla (vremena slova, sadrzaj kadra, opis kadra, prijevod, Smooth) iz modela zajednicke AI mape, na cemu
radi opis kadra, slika po kadru. Spremi ne zapisuje promjenu koja bi pokvarila motor koji je radio (iste provjere kao AI
kocke pri pokretanju); sto vec nije radilo ne smeta. U prozoru AI postavki lijevo "AI motori". Usput: savjet zajednicke AI
mape imao je `\` bez escapea, pa se rjecnik QNC-AI-ja nije ucitavao (od koraka 1). Uzivo nije provjereno.
Izvedeno 2026-11-23 (AI postavke, korak 3 od 3: oznake kadrova): nova kocka `qnc-shot-labels-edit` (popis oznaka racunala
kao JSON, polja koja ne zna ostaju): naziv, upit modelu, izjava da/ne, nova oznaka (sifra iz naziva, nikad dvaput), brisanje
(s njezinim pravilima), pragovi `least` i `statement_least`, pravila opisa kadra (oznaka i rijeci). Spremi zapisuje samo popis
koji `qnc_shot_labels::Labels::read` cita. Nova oznaka nema kljuc rjecnika: naziv se pise kako je upisan. U prozoru AI
postavki lijevo "Oznake kadrova". AI postavke imaju sve sto je trazeno (predlosci, motori, server i LLM, AI mapa, oznake).
Uzivo nije provjereno.
Popravak 2026-11-23 (korisnik: "u desnom panelu za Oznake trebas dodati vertikal scroll"): prozor bloka Postavki (`qnc-settings-window`) ima okomiti scroll i u izborniku i u desnom panelu, za svaki prozor takvog bloka (export, AI postavke). Uzivo nije provjereno.
Ispravak 2026-11-23 (korisnik: "ne pragovi, samo OZNAKE trebaju scroll bar"): desni panel prozora bloka vise ne skrola cijeli; u oznakama kadrova skrola samo popis oznaka, pragovi, nova oznaka, pravila i Spremi ostaju na mjestu. Izbornik lijevo skrola kad je visi od prozora. Uzivo nije provjereno.
Ispravak 2026-11-23 (korisnik: "pravila opisa trebaju scroll; desni panel isto poravnanje, isti razmak izmedju redova i isti font; potrazi kocku stila i sve radi po njoj"): oznake kadrova crtaju se kockom stila Postavki (`qnc-settings-form`: naziv iznad polja, mreza prozora): pragovi, svaka oznaka (naziv je naziv polja, polje je upit, uz polje Izjava i X), nova oznaka, pravila (Oznaka i Rijeci opisa, X). Popis oznaka i popis pravila skrolaju svaki zasebno. Uzivo nije provjereno.
Ispravak 2026-11-23 (korisnik: "i dalje ne postujes stil"): oznake kadrova crtaju se retcima Postavki (`qnc_settings_rows::inline_row`: naziv lijevo, polje, stupac gumba), kao glavna forma Postavki; pragovi kao broj s tri decimale (prije 0.15000000596); pravilo je "Pravilo n" s izborom oznake i rijecima u polju. Uzivo nije provjereno.
Uskladivanje GUI-ja 2026-11-23 (korisnik: "analiziraj GUI kompletne aplikacije i uskladi"). Nalaz: prozori blokova Postavki
(export predlosci, AI postavke, AI server) crtali su naziv iznad polja (`qnc_settings_form::field_grid`), a glavna forma
Postavki ima redak s nazivom lijevo, poljem i stupcem gumba (`qnc_settings_rows::inline_row`); nekoliko mjesta koristilo je
gole egui widgete mimo gui-kita. Izvedeno: `qnc-settings-block::rows` (polja kao retci Postavki: tekst, broj, izbor, trajanje
m:ss; upisano ostaje dok se tipka) koriste export predlosci (i prekidaci kao retci), AI predlosci (naziv, svaka AI mogucnost
kao redak s kucicom i sto treba), AI motori, AI mapa; AI server je retcima s tokenom skrivenim; svi Spremi/Obrisi u stupcu
gumba (`trailing_actions`). Gui-kit (`qnc-ui-kit`) umjesto golih widgeta: pretraga baza, polja putanje i predloska u
Postavkama, gumb "_" prozora rada, gumbi pitanja (`qnc-ai-ask`). Ostaje kako jest: napredne postavke projekta (mreza iz
ugovora forme Projekt), gumb X predloska u popisu Projekta, uredivac izgleda (`qnc-form-look`, alat). Debug build obrisan
(disk pun, 18,6 GiB). Tray nije ponovno izgraden (radi). Uzivo nije provjereno.
Ispravak 2026-11-23 (korisnik: "ne moze svaka forma biti razlicitog stila; pronadji kocku stila ostatka QNC-AI i primjeni na sve"): kocka stila ostatka QNC-AI je `qnc_desktop_theme::apply_settings_look` i mjere forme Postavke (`PanelMetrics` iz ugovora). Prozor bloka (`qnc-settings-window`) vise ne uzima velicinu slova ni razmake iz svog izgleda: koristi te iste mjere i izgled teme; vlastiti izgled prozora ima samo velicinu prozora, sirine izbornika i panela i marginu (`label_size`, `min_cell_width`, `gap`, `cell_height` uklonjeni iz izgleda export predlozaka i AI postavki). Uzivo nije provjereno.
Izvedeno 2026-11-23 (korisnik: "gradi kocku, ne krpaj izgled"): kocka stila postavki je `qnc-settings-block` (`Pen`: mjere
forme Postavke iz ugovora, tema i sirina). Kroz nju i ni kroz sto drugo crtaju svi blokovi Postavki i svi njihovi prozori:
naslov bloka i savjet, sekcija, poruka, redak (naziv lijevo, polje, stupac gumba) s kontrolama retka (`Row`: polje, skriveno
polje, izbornik fiksne sirine koji reze dugi tekst, kucica; sve visine kontrole teme, tekst u ravnini s nazivom), cijeli
redak polja/izbornika/kucice, red kucica u stupcu polja, gumbi ispod stupca gumba, popis redaka koji skrola po cijelim
retcima, polja iz specifikacije, polje vise redaka, izbornik prozora (naslov skupine, stavka, crta). Prebaceni: AI server,
citaci kartica, tray, orkestrator Ingesta, AI postavke (predlosci, motori, mapa, oznake kadrova), export predlosci, lokacije
i vlastiti predlozak forme Postavke. Prozor bloka (`qnc-settings-window`) daje tijelu `Pen` za svaku stranu. Blokovi vise
nemaju svojih mjera ni widgeta (uklonjene ovisnosti na `qnc-desktop-theme`, `qnc-settings-rows`, `qnc-ui-kit`). Napredne
postavke projekta i dalje crta svoja mreza iz ugovora forme Projekt. Uzivo nije provjereno.
Izvedeno 2026-11-23 (korisnik: "prebaci i napredne postavke projekta na kocku"): Napredne postavke projekta (`qnc-project-blocks` `project_advanced`) crtaju se kockom stila postavki (`qnc_settings_block::Pen`): sekcije, polja iz ugovora kao retci, izbor tipkovnice, izvozni preset, spremanje vlastitog preseta (redak s poljem i gumbom), jezik sucelja (isti zapis kao dijeljena kocka `qnc-language-choice`, u retku kocke). Mreza polja iz ugovora (`advanced.grid`) se vise ne koristi za crtanje. Izbor aplikacija ostaje prikaz u stupcima (`qnc-ui-kit` option columns). Uzivo nije provjereno.
Ispravak 2026-11-24 (korisnik: "ako Pragovi imaju poravnanje lijevo, zasto sve nema takvo poravnanje"): redak kocke stila (`Pen::row`) crta sam svoje stupce: naziv pocinje gdje i naslov sekcije, poravnat lijevo i okomito u sredini retka (prije centriran i pomaknut za unutarnji razmak `inline_row`); polje i stupac gumba isti; gumbi ispod retka i polje vise redaka u istim stupcima. Vrijedi za sve postavke. Uzivo nije provjereno.
Ispravak 2026-11-24 (korisnik: "naslov grupe lijevo poravnanje, dijelovi te grupe uvlaka jedan tab; i tako sve"): u kocki stila (`Pen`) naslov grupe i naslov skupine izbornika stoje lijevo; sve sto je u grupi (retci, savjeti, poruke, kucice, polje vise redaka, stavke izbornika) je uvuceno jedan tab (`Pen::tab`, dva unutarnja razmaka forme Postavke). Stavke izbornika lijevo poravnate (prije centrirane). Stupci retka (naziv, polje, gumbi) uvijek tocno svoje sirine, pa gumbi ispod retka stoje u stupcu gumba (prije su Dodaj pravilo i Spremi bjezali na polje). Uzivo nije provjereno.
Izvedeno 2026-11-24 (korisnik: "formu postavke podijeli na kocke ovisno o grupama; to smo vec poceli raditi, a ti si se vratio na monolit"), korak 1: AI postavke su kocke po grupama, svaka drzi svoj nacrt, cita i pise svoju datoteku i crta kockom stila: `qnc-ai-templates-view` (AI predlosci, izbornik i panel, 156 redaka), `qnc-ai-engines-view` (AI motori, 251), `qnc-shot-labels-view` (oznake kadrova, 177), `qnc-ai-share-view` (AI mapa, 45), `qnc-ai-server-view` (vec bio). `qnc-ai-settings-view` je samo prozor, izbornik i slaganje grupa (101 redak, prije 589). Uzivo nije provjereno.
Izvedeno 2026-11-24, korak 2: forma Postavke (i Projekt, ista ploca) slaze grupe iz kocki: `qnc-project-texts` (tekstovi formi i ugovor naprednih postavki, podaci), `qnc-locations-view` (lokacije s preglednikom, 68 redaka), `qnc-project-templates-view` (izbor predloska, Novi projekt, akcije predloska, vlastiti predlozak, 126), `qnc-project-advanced-view` (napredne postavke, 202). `qnc-project-blocks` je slaganje ploce, popisa projekata i redoslijeda grupa (952 redaka svi moduli, prije 1518). Uklonjene ovisnosti forme koje vise ne koristi. Kocka stila dobila `Pen::heading` (naslov grupe kako je napisan). Uzivo nije provjereno.
Izvedeno 2026-11-24 (korisnik: "je li postavke monolit" - "da, kreni"): forma Postavke vise nije ploca forme Projekt. Nova kocka `qnc-settings-board`: lijevo izbornik grupa po ugovoru `contracts/ui/settings.layout.json` (Lokacije, Orkestratori Ingesta, AI, Citaci kartica, Tray, Export), desno samo izabrana grupa, svaka grupa svoja kocka, crta kockom stila. Predlozak projekta i napredne postavke izasli su iz Postavki i ostaju samo u formi Projekt (`templates` uklonjen iz ugovora Postavki). Iz `qnc-project-blocks` uklonjeni `show_settings`, `SettingsBlock`, slaganje blokova Postavki i njihove ovisnosti. Program Postavki (`qnc-project-desktop` settings_only) crta ploci Postavki. Uzivo nije provjereno.
Izvedeno 2026-11-24 (korisnik: "postupno iz QNC-Shared trebamo seliti u QNC-AI... sada bi radio samo QNC-AI izvan Shared foldera"; "da, A, kreni"): QNC-AI vise ne uzima kocke iz QNC-shared. Svih 254 zajednicke kocke (sve osim `qnc-editorial-layout`, koju QNC-AI ne koristi: ime je u QNC-AI preusmjereno na `qnc-ai-editorial-layout`) kopirane su iz QNC-shared (stanje 97f1704) u `crates/`, a `Cargo.toml` korijena pokazuje na `crates/...`; nijedna putanja vise ne vodi u `../QNC-shared`. Vanjske ovisnosti workspacea bile su iste. Ugovori koje te kocke ugraduju a QNC-AI ih nije imao kopirani su u `contracts/` (`modules/done-count.module.json`, `ui/ai-editorial.layout.json`, `ui/ai-shell.layout.json`); ostali razliciti ugovori i katalozi ostaju verzija QNC-AI-ja (`catalogs/ai/catalog.json`, `pack.json`, `ui/editorial.layout.json`, `ui/shell.layout.json`, `seed/system_seed.json`; tipke iste po sadrzaju, drugi redoslijed). `done-count.module.json` `state_policy` je `session_local` (brojac u procesu; `process_memory_only` conformance ne dopusta). Testni primjerci Sony XML-a citaju se iz `crates/qnc-sony-metadata`. QNC i QNC-shared nisu dirani. Od sada se kocke QNC-AI-ja mijenjaju samo ovdje. Provjereno: `cargo check --workspace --all-targets`, testovi kocki koje ugraduju ugovore i kataloge (release), conformance isti kao prije selidbe (18 gresaka, sve od prije). Test `qnc-ingest-select` `failed_probe_is_durable_and_never_retried` pada od 18. 11. (pravilo ponovnog probea u zajednickoj kocki), nije od selidbe.
Izvedeno 2026-11-24 (korisnik: "racunalo moze imati vise korisnika s razlicitim preferencijama"; "preset tipki je po korisniku, jezik po korisniku"): postavke po korisniku. Nova kocka `crates/qnc-user-settings` (+ ugovor `contracts/modules/user-settings.module.json`): mapa korisnika `data/users/<korisnik>/` uz postavke racunala; korisnika daje OS (`USERNAME`, inace `USER`), nikad izmisljen. Postavka se cita iz mape korisnika, a kad je korisnik nema iz `data/<datoteka>` racunala (sto je racunalo izabralo prije i dalje vrijedi); pise se samo u mapu korisnika. Po korisniku su: jezik sucelja (`qnc-ui-text` `chosen_language`, `choose_language`, `Texts::for_host`; `ui-language.json`), smjer kotacica (`qnc-edit-step`, `mouse-wheel.json`), mikrofon OFF-a (`qnc-voice-record` `input_setting`, `voice-input.json`) i preset tipki (`qnc-key-preset` `user_preset`, `choose_preset`, `keyboard-preset.json` `{"preset": ...}`; bez izbora korisnika vrijedi preset aktivnog projekta kao dosad). Testovi kocki, conformance isti (18). Izbor u sucelju dolazi s tabom User u Postavkama (sljedeci korak).
Izvedeno 2026-11-24 (korisnik: "postavke su postavke, ne moze biti vise postavki.. mozes uvesti tabove General, User, Project"): forma Postavke ima tabove na vrhu (kocka stila `Pen::tabs`), ispod lijevo grupe izabranog taba, desno izabrana grupa. Ugovor `contracts/ui/settings.layout.json` (schema 2) nabraja tabove i grupe; grupa smije biti samo u svom tabu (`qnc-settings-board` `Group::tab`, test). Opcenito: Lokacije, Orkestratori Ingesta, AI, Citaci kartica, Tray, Export (postavke racunala). Korisnik: nova kocka `qnc-user-settings-view` - jezik sucelja, preset tipki, smjer kotacica, mikrofon za OFF (s popisom ulaza i Osvjezi; `qnc-voice-record` `input_devices`, `chosen_input`, `choose_input`), sve u mapi korisnika. Projekt: predlosci projekta i njihove napredne postavke istim kockama kao forma Projekt (`qnc_project_blocks::show_templates`, bez Novog projekta). Slaganje Postavki je u ploci (`qnc_settings_board::show_settings`); forma `qnc-project-desktop` samo je poziva (144 redaka, prije 145). Napredne postavke i jezik/preset tipki u formi Projekt jos nisu uklonjeni (sljedeci korak). Testovi, conformance isti (18). Uzivo nije provjereno.
Izvedeno 2026-11-24 (isti zahtjev, zadnji korak): forma Projekt ima samo izbor predloska i Novi projekt (i obrazac priloga otvorenog projekta); napredne postavke predloska, AI predloska, akcije predloska i lokacije su samo u Postavkama (lokacije u tabu Opcenito, predlosci u tabu Projekt; `qnc-project-blocks` `templates` dalje od izbora samo kad crta Postavke). Iz naprednih postavki uklonjeni su jezik sucelja i preset tipki (sekcije `ui_language` i `keyboard_preset` iz `contracts/ui/project.layout.json`, `AdvancedDraft` bez `keyboard_*`): oni su korisnikovi, tab Korisnik. Uklonjena kocka `qnc-language-choice` (bez korisnika; QNC ima svoju). Slot `Locations` uklonjen iz `pts_slots`. Novi projekt dobiva postavke izabranog predloska kakve jesu; tko zeli drukcije, u Postavkama pravi vlastiti predlozak. Conformance isti (18), C2 baseline `settings.rs` 7 -> 6. Uzivo nije provjereno.
Izvedeno 2026-11-24 (korisnik: "u tray dodaj Postavke link"): izbornik traya ima stavku Postavke (`qnc-tray-menu` `Item::Settings`, kod `settings`, prije Administracije), koja otvara program Postavki uz tray (`qnc_ai_open::open`, zapis `settings opened` u logu; greska je poruka traya). Od c9d9035 Postavke su bile samo u formi Administracija. Stavka se moze sakriti u Postavkama (Tray) kao ostale. Ponovno izgradeni svi QNC-AI programi (Administracija je bila gradnja od 22. 11.). Log: tray je u 12:17 dvaput otvorio Administraciju, gumba Postavki u njoj nije bilo; program Postavki sam se pokrece i ostaje otvoren.
Popravak 2026-11-24 (korisnik: Postavke iz traya "ne otvara, ili ce otvoriti, ali to je neprihvatljivo sporo"; log: tray nakon pokretanja 64 s ne radi nista, klik na Postavke izvrsi tek tada; 23. 11. 21 s, prije 0 s): tray svake 2 s, u niti izbornika, cita mrezni citac kartica (`\192.168.100.28\kartice`, Jetson), a dok Jetson ne odgovara svako citanje ceka mrezni timeout (izmjereno 7,6 s po citanju; i `qnc_source_bindings::load` ga cita). `qnc-card-readers` `cards`: svaki citac cita svoja nit svake 2 s (`READ_EVERY`), poziv odmah daje zadnje sto je nasao, novi citac dobiva do 300 ms (`FIRST_WAIT`); nikad ne ceka mrezu. Test s nedostupnom adresom (TEST-NET-1) i mapom s karticom. Vrijedi za tray, Ingest i sve koji citaju izvore. Uzivo nije provjereno.
Izvedeno 2026-11-24 (korisnik: "dodaj jos AI tab i Export tab i u njih premjesti sadrzaj tih tabova"): Postavke imaju tabove Opcenito, Korisnik, Projekt, AI, Export. AI i Export vise nisu zasebni prozori otvarani iz Postavki nego tabovi: lijevi izbornik taba je izbornik grupe (`qnc-ai-settings-view` `menu`/`panel`: AI predlosci, motori, server i LLM, AI mapa, oznake kadrova; `qnc-export-templates-view` `menu`/`panel`: predlosci exporta, gotove postavke, Export XML), desno izabrano (`qnc-settings-board` `Group::own_menu`). Opcenito: Lokacije, Orkestratori Ingesta, Citaci kartica, Tray. Uklonjeni kocka `qnc-settings-window` i look datoteke `ai-settings`, `export-templates` (bez korisnika). Conformance 18 -> 16 (uklonjene nevaljane look datoteke). Uzivo nije provjereno.
Izvedeno 2026-11-24 (korisnik: "1, svaki korisnik svoje predloske"): vlastiti predlosci projekta su po korisniku. `qnc-project-store`: predlozak koji korisnik napravi dobiva vlasnika (`project_templates.created_by` = korisnik kojeg daje OS, `qnc-user-settings`; id `tpl_user_<korisnik>_<naziv>_<ms>`); korisnik vidi, bira, koristi za novi projekt i brise samo sistemske predloske i svoje (`visible`), a izabrani predlozak pamti se po korisniku (`selected_template_id:<korisnik>`). Vlastiti predlosci napravljeni prije (bez vlasnika) vide se svima, bez migracije. Pravilo u ugovoru `project-registry.database.json` (`template_owner_rule`). Test `a_user_sees_and_uses_only_their_own_templates_and_the_system_ones`; 44 testa spremista, conformance isti (16). Uzivo nije provjereno.
Izvedeno 2026-11-24 (korisnik: "u projektu AI postavke trebaju imati samo izbor AI templatea, ne prikaz pojedinacnih dijelova... to imamo u AI tabu"): AI predloska projekta (tab Projekt, napredne postavke) je jedan red izbora AI predloska (`qnc-ai-features-view` `show`, kockom stila); pojedinacne AI mogucnosti, spremanje i brisanje AI predloska samo su u tabu AI (`qnc-ai-templates-view`). Prekidac filmstripa (`ai_switches`) ostaje u istoj sekciji. Conformance isti (16). Uzivo nije provjereno.
Izvedeno 2026-11-24 (korisnik o zajednickoj AI mapi: "ako treba tome je mjesto onda u tabu Opcenito jer to je opci podatak"): Zajednicka AI mapa (gdje su AI modeli, `C:\Users\Public\QNC-AI-share`, i za klasicni QNC) je grupa taba Opcenito (`qnc-settings-board` `Group::AiShare`, `qnc-ai-share-view`), vise nije u tabu AI. Mapa kocki `QNC-shared` nije ista mapa (QNC-AI je vise ne koristi). Conformance isti (16). Uzivo nije provjereno.
Izvedeno 2026-11-24 (korisnik: "Lokaciju IngestBase izdvoji iz Lokacija i dodaj u Zajednicku AI mapu koju ces preimenovati Sustav"): tab Opcenito ima grupu Sustav (`qnc-settings-board` `Group::System`): Zajednicka AI mapa i mapa Ingest baza, oba kao red lokacije s putem, Odaberi i preglednikom (`qnc_locations_view::show_targets`). Lokacije su mapa projekata i mapa exporta. Uklonjena kocka `qnc-ai-share-view` (bez korisnika; AI mapu pise sesija kroz `qnc_ai_catalog::set_share`). Conformance isti (16). Uzivo nije provjereno.
Popravak 2026-11-24 (korisnik: "pokrece se pogresni QNC-ingest wizard kad ubacim karticu"; "izvor ... mora uzimati iz postavki koje su odabrane u trayu"; "Jetson mora raditi preko serijskog broja"): log: prozor Ingesta za karticu G: dvaput u sekundi trazi izvor i dobiva "Izvor nije spojen"; Select je odbio cijelu konfiguraciju izvora ("source serial_number missing") jer je mapa mreznog citaca Jetson bez kartice (Jetson ne odgovara) dodana kao izvor bez serijskog broja (od 22. 11., 2e72306). `qnc-source-bindings` `load`: izvori idu po postavci traja (`qnc_tray_mode`): Local - samo diskovi i citaci ovog racunala; Net ili neizabrano - i mrezni citaci i njihove kartice, svaka po serijskom broju volumena (kao do sada). Pokusaj da izvor bez serijskog broja bude dopusten vracen je na zahtjev korisnika. Kopiranje kocki iz QNC-shared (16b4669) nije prepisalo nijednu kocku QNC-AI-ja (nijedna mapa nije postojala u oba repoa). Test `failed_probe_is_durable_and_never_retried` pada od 18. 11. kao i prije. Uzivo nije provjereno.
Popravak 2026-11-24 (korisnik: "nema ni AI prijepis ni ostale"; snimka: prozor Ingesta za karticu bez AI koraka): baza za karticu uzima izabrani predlozak, a izbor predloska po korisniku (6a7a4b8) nije vidio izbor racunala (`selected_template_id`, predlozak s AI-jem), pa je uzet i kao izbor korisnika upisan prvi predlozak (`tpl_breaking_news`, bez AI-ja). `qnc-project-store` `chosen_template_id`: izbor korisnika, inace izbor racunala, inace prvi vidljivi; upisuje se samo izbor korisnika, nikad zamjenski (test). Pogresno upisani izbor korisnika obrisan primjerom `examples/template_choice.rs --forget-mine` (prije: korisnik `tpl_breaking_news`, racunalo `tpl_user_ingest_all_1794487771065`; poslije se koristi izbor racunala). Baza kartice napravljena u 13:57 ostaje bez AI-ja (postavke pokrenutog projekta se ne mijenjaju). Uzivo nije provjereno.
Izvedeno 2026-11-24 (korisnik: "u Skriptu ne treba obrazac za potpise jer potpisi se kreiraju prilikom kreiranja izjava, njih forma treba citati iz baze"; "neka blok zapamti iz koje je izjave nastao"): blok scenarija pamti izjavu poola (`virtual_shots`) iz koje je nastao. `qnc-script-db`: `Block.statement_id`, stupac `script_blocks.statement_id` (starijoj tablici se dodaje, blokovi ostaju bez izjave), u pogledu `public_script_blocks`. `qnc-script`: izjave se citaju uz scenarij u pozadini (`qnc_virtual_shots::list_shorts`; neuspjelo citanje ide u log, scenarij se cita); novi blok klipa (+ IZJAVA, slozena izjava, izjava prijepisa) dobiva izjavu istog klipa s istim IN/OUT, inace onu koju najvise preklapa (`statement_of`); redak bloka dobiva potpis izjave (`BlockRow.caption`) i polja potpisa (`caption_name`, `caption_role`) iz nje, pa ih izvoz scenarija ima (`with_statements`, `CAPTION_FIELDS`). Panel scenarija (MA Audio, MA Script) polja potpisa vise ne uredjuje: blok klipa pokazuje "Potpis: ime, uloga" iz izjave, "jos bez potpisa" ili "nije nastao iz izjave". Profili scenarija nisu mijenjani (polja potpisa ostaju za izvoz). Blokovi napravljeni prije nemaju izjavu. Testovi `qnc-script-db`, `qnc-script`, conformance (16 kao prije). Uzivo nije provjereno.
Popravak 2026-11-24 (korisnik: "zbrljao si slozenu izjavu, analiziraj"; "da, kreni tim redom"; analiza loga i kopije baze: Azuriraj dok je otvorena slozena izjava prepisao je IN/OUT cijele izjave IN/OUT-om dodanog dijela (shot_002 7541-7929 -> 8221-8533), a + IZJAVA u Scenariju uzela je samo jedan raspon i izjavu pogadala po preklapanju), koraci 1 i 2: (1) Izjava izabrana u poolu (AI Media Assist, prozor medija MA Scripta) ide u Scenarij cijela dok je player pokazuje: + IZJAVA pravi blok od njezina IN/OUT i svih dijelova sa spojevima (Smooth: zadano 8 slika, `qnc-subsegment-edit` `SMOOTH_DEFAULT` je javan; Blic zasad rez) i pamti njezin id (`qnc-script` `chosen_statement_block`, `chose_statement`; `qnc-side-pieces` i `qnc-panel-focus` `chose_statement(.., shot_id)` je predaju, izbor klipa je brise). Bez izabrane izjave blok iz IN/OUT-a pamti izjavu samo kad ima tocno isti klip i IN/OUT (`statement_of`; vise nikad po preklapanju). Blok cija je izjava obrisana pise "izjava iz koje je nastao je obrisana". (2) Azuriraj i fino podesavanje izjave na source timelineu vrijede samo za izjavu jednog raspona (`qnc-virtual-short-cards` `chosen_single`); kod slozene izjave Azuriraj je siv (ona se uredjuje na svom timelineu), a dodavanje dijela ostaje. Testovi `qnc-script`, `qnc-virtual-short-cards` (novi), conformance (16 kao prije). Uzivo nije provjereno.
Popravak 2026-11-24 (isti audit, koraci 3 i 4): (3) Klik na rijec prijepisa dok player svira slozenu izjavu slao je sliku klipa na os izjave i player ga je odbijao ("cue is outside execution range"); sada slika klipa ide na mjesto u izjavi gdje se ta rijec cuje (dio prikazanog klipa prvi, `qnc-side-pieces` `axis_frame`, test), a rijec koju izjava ne kaze ne mice playhead. (4) Otvaranje slozene izjave brise IN/OUT source timelinea ispod nje (`qnc-panel-focus` `open_statement(.., timeline)`): ostajali su IN/OUT prethodno otvorene izjave (1954-10194), koje su onda uzimali Dodaj dio i oznaka rijeci u prijepisu; slozena izjava ne povecava source timeline (`shown_marks` kroz `chosen_single`). Conformance 16 kao prije. Uzivo nije provjereno.
Popravak 2026-11-24 (isti audit, korak 5: blok scenarija Mironik 2676 "ne ne mozemo mozemo ..."): prijepis klipa u bazi imao je u vise redaka svaku rijec dvaput, jednom s vremenom Parakeeta (njegov zapis) i jednom s vremenom wav2vec2, a te dvije se vremenski preklapaju (korekcija je jednom ostavila neuparene rijeci, a neuparena gruba rijec ostaje u svakoj sljedecoj korekciji). Dvije iste rijeci u isto vrijeme nisu govor (ponovljena rijec pocinje kad prethodna zavrsi): `qnc-transcript-db` `without_echoes` ih uklanja pri citanju prijepisa (`read_clip`) i prije i poslije korekcije (`qnc-transcribe-run` `corrected_by_fine`); ostaje prva, s grubim zapisom. Test. Ostaje: poneka rijec koju dva modela cuju razlicito ("neminovno neminovano"); tekst bloka 3 vec je spremljen udvostrucen (blok treba dodati ponovno). Pravi uzrok prvog neuparivanja nije nadjen. Uzivo nije provjereno.
Izvedeno 2026-11-24 (korisnik, obrasci za pisanje, pregled urednika i lektora i cue sheet za rezije, PDF-ovi; "kad korisnik ide na skript formu, najprije se otvori overlay obrazac za naziv, novinar, snimatelj... nakon toga obrazac za pisanje"; "novinar na svom laptopu ne mora automatski upisivati ime"; "vise novinara ucestvuje"; "obrazac zaglavlja u jednom stupcu, prikaz u vise"), korak 1, zaglavlje priloga: novi pripremljeni obrazac priloga `catalogs/story-sheets/tv_prilog.json` (emisija, datum, naslov/slug, trajanje TRT, novinar, montazer, snimatelj, izvor/format, napomene za reziju). Nove vrste polja `qnc-field-spec` `Names` (vise imena, red po imenu, cuva se kao tekst odvojen zarezom) i `Date` (prazan je danasnji); ostali obrasci ih upisuju kao tekst. Nova kocka `crates/qnc-story-header` (+ ugovor): zaglavlje iz obrasca priloga projekta (projekt bez obrasca dobiva TV prilog), citanje i spremanje u bazu projekta izvan niti sucelja (`qnc-newsroom-sheet-db`), nacrt dok se tipka; nijedno ime se ne pogadja. MA Script: pri otvaranju projekta preko forme se otvara obrazac zaglavlja u jednom stupcu (Enter na sljedece polje, na zadnjem otvara pisanje, Esc zatvara bez spremanja, x uklanja ime, prazan red dodaje novo); spremljeno zaglavlje stoji na vrhu forme za pisanje u tri stupca, klik na naslov ga otvara ponovno (`qnc-script-writer-panel` `header.rs`, `qnc-script` ga drzi). Akcije kataloga `story_header_*` bez tipki. Nije izvedeno (sljedeci koraci): korisnicki profil s imenima i ekipom (u Postavkama), forma za pisanje u tri stupca, pregled urednika i lektora, cue sheet za reziju. Testovi `qnc-story-header` (2), conformance (16 kao prije). Uzivo nije provjereno.
