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
