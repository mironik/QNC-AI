# QNC-AI - zakon repoa

QNC-AI je samostalna aplikacija: AI wizard koji novinara vodi od kartice do exporta
(`docs/01-wizard-plan.md`). Gradi se na kockama QNC-a (`C:\Users\miron\Projects\QNC`,
`https://github.com/mironik/QNC`).

## 1. Odnos prema QNC-u

- Kocke QNC-a koriste se kao git ovisnost zakljucana na oznaku QNC-a (`Cargo.toml`,
  `[workspace.dependencies]`). Oznaka se mijenja svjesno, jednim commitom, uz zapis ovdje.
- Kocku koju wizard treba promijeniti QNC-AI preuzima k sebi (`crates/`, isti naziv) i mijenja
  je tamo; `[patch."https://github.com/mironik/QNC.git"]` u `Cargo.toml` usmjerava sve ovisnosti
  na nju. Svako preuzimanje zapisuje se u odjeljku 4: sto, s koje oznake, zasto.
- Zamrzavanja iz `AGENTS.md` QNC-a ne vrijede za QNC-AI. Nacela vrijede:
  - baza projekta je jedina istina; stanje wizarda je u bazi projekta, ne u memoriji prozora;
  - forma je samo raspored; sve sto radi je u kockama s jednom odgovornoscu;
  - svaki natpis ima sifru u rjecniku (`contracts/text`), engleski je osnovni jezik;
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
- Izolirani korijen za probu: `cargo run -p qnc-wizard-application --example iso_root -- <QNC> <novi>`
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
