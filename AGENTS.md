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
proces, neovisno o aktivnom projektu. Nije izvedeno (sljedeci korak).
