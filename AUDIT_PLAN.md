# AUDIT_PLAN — QNC-AI (stanje grane `claude/hungry-tesla-3f9797`, commit 5c93145, 2026-11-28)

## Upute izvođaču (vrijede za svaku stavku)
- Prije promjene ponašanja vrijedi pravilo repoa: najprije pravilo u `AGENTS.md` §3a, pa kod. Stavke ispod ne mijenjaju
  pravila ponašanja osim gdje je izričito rečeno (tada pitati korisnika).
- Jedna stavka = jedan commit; testovi dotaknutih kocki u **release** profilu (debug ne stane na disk);
  `conformance` mora ostati na 16 grešaka (od prije).
- Testovi i probe nikad ne pišu u stvarni projekt; za probu koristiti izolirani korijen (`iso_root`, AGENTS §3).
- Logika ostaje u kockama bez sučelja; forma samo crta.
- Poznati padajući testovi od prije (ne kvariti dodatno, ne "popravljati" bez odluke): `qnc-ingest-select`
  `failed_probe_is_durable_and_never_retried`, `qnc-source-panel` `undo_puts_only_the_taken_mark_back_where_it_was`,
  dva testa `qnc-ingest-import-worker` (vidi stavku R-14).
- Linije su orijentacijske (stanje 5c93145); tražiti po imenu funkcije.

## Sažetak prioriteta
| # | Prioritet | Područje | Naziv |
|---|---|---|---|
| S-1 | KRITIČNO | SQLite | Upis i commit dnevnika u dvije transakcije, a Undo/Restore čitaju dnevnik |
| S-2 | KRITIČNO | SQLite | `copy_tables` prebacuje `project.db` iz WAL-a u PERSIST |
| R-1 | KRITIČNO | Regresija | Kreiranje projekta iz baze nije atomarno, napola napravljena priča se ne popravlja |
| C-1 | VISOKO | Konkurentnost | `qnc-trim-stills`: globalni lock drži se tijekom otvaranja dekodera |
| C-2 | VISOKO | Konkurentnost | Ingest worker: lock dirigenta drži se preko DB čitanja i popisa procesa |
| C-3 | VISOKO | Konkurentnost/memorija | Red prijepisa: DB pod lockom i neograničen red PCM-a |
| S-3 | VISOKO | SQLite | `ensure_schema` dnevnika radi DROP/CREATE VIEW pri svakom pridruživanju modula |
| S-4 | VISOKO | SQLite | `row_history` skenira cijeli dnevnik kroz `json_extract` |
| S-5 | VISOKO | SQLite | `ProjectStore::open` piše u registar pri svakom otvaranju, poziva se svake sekunde |
| R-2 | VISOKO | Regresija | Nacrt složene izjave ostaje nakon neuspjelog/pretečenog upisa |
| R-3 | VISOKO | Regresija | Strukturne izmjene složene izjave u prikazu izjave se tiho gube |
| C-4 | SREDNJE | Konkurentnost | `Player::drop` radi `join` na UI dretvi (do >12 s) |
| C-5 | SREDNJE | Performanse | Puna kopija svake slike monitora na UI dretvi |
| C-6 | SREDNJE | Konkurentnost | Shared-memory frame map može dati poderanu sliku |
| C-7 | SREDNJE | Konkurentnost | Tray glavna petlja radi disk/DB/mrežu između događaja traya |
| C-8 | SREDNJE | Konkurentnost | `qnc-playback-activity`: DB pod lockom + `.expect` (trovanje ruši ingest) |
| S-6 | SREDNJE | SQLite | `stacks` ponavlja cijeli dnevnik modula pri svakom čitanju priče |
| S-7 | SREDNJE | SQLite | Restore projekta nije atoman preko modula |
| S-8 | SREDNJE | SQLite | Nedostaju indeksi (`ClaimNext`, `source_uri`, dnevnik) |
| S-9 | SREDNJE | SQLite | Uhvaćeni retci neuspjelog upisa ulaze u sljedeći commit |
| S-10 | SREDNJE | SQLite | Nacrti priče i `ai.db` nastaju u rollback modu, nikad WAL |
| R-4 | SREDNJE | Regresija | `ShortSave` gubi ishode istodobnih spremanja |
| R-5 | SREDNJE | Regresija | Optimistička kartica "saving" nikad se ne pokaže; Update briše prikazanu izjavu |
| R-6 | SREDNJE | Regresija | Povijest zooma: vraćanje po `id 0` bira krivi redak |
| R-7 | SREDNJE | Regresija | Povijest zooma: stanje prelazi s klipa na klip |
| R-8 | SREDNJE | Regresija | Dva puta kopiranja izjava baze u priču s različitim pravilima |
| R-9 | SREDNJE | Regresija | Space u zoomu svira preko OUT-a |
| R-10 | SREDNJE | Regresija | Povijest zooma prikazuje UTC vrijeme |
| R-11 | SREDNJE | Regresija | Filter jeke prijepisa zadržava krivu kopiju riječi |
| R-12..14 | NISKO | razno | Ostalo (vidi kraj) |

---

## 1. KRITIČNO

### [Prioritet: KRITIČNO] S-1 Upis podataka i commit dnevnika su dvije odvojene transakcije, a Undo/Restore sada ovise o dnevniku
- **Lokacija:** `crates/qnc-db-broker/src/database.rs` — `ProjectDb::execute` (≈204-222), `fn commit` (≈246-265);
  modul piše svoju transakciju npr. `crates/qnc-program-db/src/store.rs` (≈107-120, `BEGIN IMMEDIATE … COMMIT`).
- **Opis problema:** Modul svoj upis potvrdi (`COMMIT`), tek zatim broker radi `qnc_change_capture::take` i otvara
  **drugi** `BEGIN IMMEDIATE` za `qnc_project_journal::append`. Pad procesa između, `SQLITE_BUSY` nakon 5 s ili pun disk
  ostavljaju izmjenu bez commita; greška se samo logira (`project-journal not kept`). AGENTS (25. 11., korak 1) je to
  prihvatio jer "ništa još ne čita dnevnik" — to više ne vrijedi: `qnc-program-db/src/undo.rs` i
  `qnc-script-db/src/undo.rs` (`stacks`), `qnc-program-db/src/object_undo.rs` i `qnc-clip-marks/src/store.rs`
  (`row_history`), `qnc-db-broker/src/owned.rs` (Restore preko `after`). Posljedice: izgubljen `undo` commit ostavlja
  korak na Undo stogu → sljedeći Undo pokušava ponovno i dobiva `journal.changed_since`; izgubljen commit izmjene je
  nevidljiv Undo-u, povijesti i Restoreu.
- **Zaštićena logika (NE DIRATI):** `qnc_change_capture` TEMP okidači i njihova tablica; autorizator `write_only_own_tables`
  (dnevnik samo dodaje, shema zatvorena); razlika "odabir nije commit" (grana s `take` i odbacivanjem, linija ≈218-219);
  `journal_step` / `refers` semantika Undo/Redo; `leased()` (lease i upis u jednoj transakciji).
- **Točan plan popravka:**
  1. U `qnc-db-broker` uvesti vanjsku transakciju koju otvara broker za journaled pisanje: prije `module.execute`
     (kad je `ReadWrite`, `factory.journaled()` neprazan i `factory.is_write`) izvršiti `BEGIN IMMEDIATE` na
     `module.connection()`.
  2. Module koji sami rade `BEGIN IMMEDIATE`/`COMMIT` prilagoditi da unutar već otvorene transakcije koriste
     `SAVEPOINT`/`RELEASE` (provjera `conn.is_autocommit()`; isti obrazac koji `leased()` već koristi). Najmanja izmjena:
     pomoćna funkcija u `qnc-write-lease` (`begin_write`) koja bira `BEGIN IMMEDIATE` ili `SAVEPOINT` prema `is_autocommit()`.
  3. Nakon uspješnog `execute`: `take` + `append` na istoj vezi, zatim jedan `COMMIT`. Na grešku `execute` ili `append`:
     `ROLLBACK` cijele transakcije i vrati grešku korisniku (ne samo log). Grana "odabir nije commit" ostaje: `take` i
     odbaci, pa `COMMIT`.
  4. `fn commit` više ne otvara vlastiti `BEGIN IMMEDIATE`.
  5. Ažurirati komentar iznad `Kept`/`commit` i zapis u AGENTS §5 (da je commit sada u istoj transakciji).
- **Uvjet provjere:** novi test u `qnc-db-broker`: pisanje kroz pravi put gdje `append` namjerno padne (npr. zatvorena
  tablica dnevnika) → podatak **nije** zapisan. Postojeći testovi `qnc-db-broker`, `qnc-program-db` (67), `qnc-script-db`
  (26), `qnc-project-journal`, `qnc-journal-revert`, `qnc-project-history`, `qnc-clip-marks` prolaze (release). Ručno: u
  izoliranom korijenu Undo/Redo priče i scenarija, povijest zooma, Ctrl+Shift+H povratak.

### [Prioritet: KRITIČNO] S-2 `copy_tables` tiho prebacuje bazu projekta iz WAL-a u PERSIST
- **Lokacija:** `crates/qnc-ingest-base/src/lib.rs` — `fn copy_tables` (≈473-519), linija `PRAGMA main.journal_mode = PERSIST`.
- **Opis problema:** `project.db` nastaje u WAL-u (`qnc-project-store` `configure_connection`). `journal_mode = PERSIST`
  na WAL bazi trajno je prebacuje u rollback način ako u tom trenutku nema drugih veza; greška se baca (`let _`). Broker
  (`database.rs` `connect`) i content-store postavljaju PERSIST samo kad baza **nije** WAL, pa se WAL nikad ne vraća. U
  rollback načinu svaki čitač (a ima ih svake 1-2 s) blokira upise. Uz to transakcija je DEFERRED i prvo čita → u WAL-u
  moguć `SQLITE_BUSY_SNAPSHOT` koji busy handler ne ponavlja. Funkcija zaobilazi brokera, autorizator i dnevnik.
- **Zaštićena logika (NE DIRATI):** namjera commita eb9c620 — mapa projekta ne smije brisati journal datoteku, zato PERSIST
  za bazu koja **nije** WAL; `foreign_keys = OFF` za kopiju; "samo retci, tablice su vlasnikove (`prepare_story`)"
  (5c93145); uklanjanje `\\?\` prefiksa.
- **Točan plan popravka:**
  1. Pročitati `PRAGMA main.journal_mode`; postaviti PERSIST samo ako rezultat nije `wal` (isto pravilo kao broker).
  2. `conn.transaction()` zamijeniti s `conn.transaction_with_behavior(TransactionBehavior::Immediate)`.
  3. Graditi URI ispravno: obraditi i `\\?\UNC\` (→ `//server/share/...`) te percent-enkodirati `%`, `#`, `?` u putanji
     (vidi i R-1).
  4. Zapisati u log ako `journal_mode` ili ATTACH ne uspije (ne `let _` bez traga).
- **Uvjet provjere:** test u `qnc-ingest-base`: napraviti WAL bazu, pozvati `copy_tables`, provjeriti da je
  `journal_mode` i dalje `wal`; test s putanjom koja sadrži `#`. Postojeći primjeri `copy_into_story` i `settle` rade
  na izoliranom korijenu.

### [Prioritet: KRITIČNO] R-1 "Kreiraj projekt" iz baze nije atoman; napola napravljena priča se ne popravlja
- **Lokacija:** `crates/qnc-story-header/src/lib.rs` — `keep_story` (≈309-333), `keep` (≈247, `self.kept = values`);
  `crates/qnc-ingest-base/src/lib.rs` — `take_for_story` (≈338-350), `copy_base_into_story`;
  `crates/qnc-project-store/src/lib.rs` `create_in` (postavlja `active_project_id`).
- **Opis problema:** Redoslijed: imenovanje baze → kreiranje projekta i aktiviranje → kopija baze → spremanje zaglavlja.
  Ako kopija padne (ATTACH, BUSY, putanja s `#`/`%`), prazna priča je aktivna; ponovni pokušaj vidi `active_is_base == false`
  i radi samo `qnc_newsroom_sheet_db::save` → priča zauvijek bez klipova i izjava, bez poruke. `self.kept` se postavlja
  prije uspjeha upisa, pa Script pokazuje zaglavlje koje nije spremljeno.
- **Zaštićena logika (NE DIRATI):** pravilo "projekt nastaje tek u Scriptu, naziv priloga postaje naziv projekta"
  (AGENTS 24. 11.); zaštita baze (`is_ingest_base`, nema S-Storyja uz bazu); `take_for_story` zapis `story_ingest_bases`.
- **Točan plan popravka:**
  1. U priču upisati oznaku "kopija baze dovršena" (redak u postojećoj tablici `story_ingest_bases` ili novi stupac
     `copied_at`, kroz vlasnika tablice) tek nakon uspjele `copy_base_into_story`.
  2. U `keep_story`: ako je aktivni projekt priča koja ima bazu **bez** te oznake, ponovno pozvati
     `copy_base_into_story` (idempotentno: `INSERT OR IGNORE`) prije spremanja zaglavlja.
  3. Neuspjeh kopije vratiti kao poruku korisniku (`header.*` šifra u rječniku), ne samo log.
  4. `self.kept = values` postaviti tek kad stigne uspješan ishod upisa (nit već vraća rezultat).
- **Uvjet provjere:** test u `qnc-story-header`/`qnc-ingest-base` s nedostupnom bazom (nepostojeća putanja): prvi pokušaj
  javlja grešku, nakon vraćanja baze drugi pokušaj kopira klipove. Postojeći testovi `qnc-story-header` (2), `qnc-ingest-base`.

---

## 2. VISOKO

### [Prioritet: VISOKO] C-1 `qnc-trim-stills`: procesni lock drži se tijekom otvaranja dekodera; zaglavljene dretve se gomilaju
- **Lokacija:** `crates/qnc-trim-stills/src/open_run.rs` `decode()` (≈61-93); `worker.rs` (≈23-44, scoped dretva po slici);
  `lib.rs` (≈338-343 `STUCK` 8 s, ≈174 zamjena na promjenu projekta).
- **Opis problema:** statički `decoders.lock()` drži se dok `open_decoder` čita katalog i otvara datoteku (kartica/mreža).
  Sve slike čekaju isti lock; `cancel` se provjerava samo između paketa, čekanje na lock se ne može otkazati. Posao stariji
  od 8 s se otkaže ali dretva ostaje blokirana; svaki novi posao se također zaglavi → rastući broj blokiranih dretvi.
  Zamjena na promjenu projekta ne postavlja `cancel`.
- **Zaštićena logika (NE DIRATI):** odluka 22. 11. "zoom za živu sliku ne dekodira unaprijed" (`ahead`); punjenje
  srednjeg playera slikom monitora (`fill(.., shown_picture())`); `hand_player` pravilo.
- **Točan plan popravka:**
  1. U `decode()`: pod globalnim lockom samo pogledati/umetnuti `Arc<Mutex<Option<Decoder>>>` slot za `place`; otpustiti
     globalni lock; otvaranje dekodera raditi pod lockom **tog slota**.
  2. Prije čekanja na slot provjeriti `cancel`; koristiti `try_lock` u petlji s kratkim spavanjem i provjerom `cancel`
     (ili odustati ako je slot zauzet i posao otkazan).
  3. U `lib.rs` pri zamjeni (`fill` na promjeni projekta) postaviti `cancel` starog posla.
- **Uvjet provjere:** postojeći testovi `qnc-trim-stills`, `qnc-subsegment-edit`, `qnc-program-segments`. Novi test: dva
  mjesta, jedno s lažnim dekoderom koji blokira → drugo dobije sliku. Ručno: zoom izjave na klipu s kartice.

### [Prioritet: VISOKO] C-2 Ingest worker: lock dirigenta drži se preko DB čitanja i popisa procesa
- **Lokacija:** `tools/qnc-ingest-worker/src/run.rs` (watch dretva u `run()`, ≈233-296; `places` ≈132);
  `main.rs` `run_artifacts` (≈216-219, 274-283, petlja 20 ms).
- **Opis problema:** `conductor.lock()` drži se dok se radi `qnc_playback_activity::is_fresh` (DB), `held()`→`level()`
  (DB), `programs.used()` (popis procesa svakih 10 s) i log. Koraci (val, filmstrip, AI) uzimaju isti lock do ~50×/s,
  pa stoje dok watch dretva radi spor posao. Nema deadlocka (redoslijed uvijek dirigent → level state).
- **Zaštićena logika (NE DIRATI):** Breaking news klip-po-klip pravilo (`breaking_held`, 22. 11.); kopija čeka postere
  (`not_done(Posters)`); `stopped_work` zastavice; redoslijed prioriteta (prijepis iznad slika); `watching` cijeli rad.
- **Točan plan popravka:**
  1. U watch dretvi najprije bez locka izračunati sve ulaze: `hearing`, rezultat `held(step)` za svaki korak, `busy`,
     `used` → u lokalnu strukturu.
  2. Tek tada uzeti `conductor.lock()` i samo pozvati `conductor.work(...)` s gotovim vrijednostima; otpustiti odmah.
  3. Logiranje izvan locka.
- **Uvjet provjere:** release gradnja radnika; ručno u izoliranom korijenu uvoz kartice — isti redoslijed rada u logu,
  vrijeme do prvog filmstripa ne raste. Nema novih testova obveznih; ako postoji test dirigenta, mora proći.

### [Prioritet: VISOKO] C-3 Red prijepisa: DB čitanje pod lockom i neograničen red cijelog zvuka klipova
- **Lokacija:** `tools/qnc-ingest-worker/src/transcribe.rs` — `Statements::sink` (≈247-249), `busy` (≈257),
  `Waiting::next` (≈212, 277-284), `clips.0: Vec<(String, Vec<f32>)>`.
- **Opis problema:** `qnc_playback_activity::read` (DB) se radi pod `self.clips` lockom; callback vala (`sink`) i `busy()`
  (svakih 20 ms) čekaju. Red drži cijeli prvi kanal svakog klipa na 16 kHz bez granice (~230 MB po satu); valovi su brži
  od prijepisa → memorija raste s veličinom kartice.
- **Zaštićena logika (NE DIRATI):** samo VAD govor ide prijepisu (AGENTS §3a); prednost klipa koji player pokazuje
  (`PLAYBACK_CLIP`); prijepis prvog kanala (CH1).
- **Točan plan popravka:**
  1. U `next`: pročitati `shown` (DB) **prije** uzimanja locka; pod lockom samo izabrati i `remove`.
  2. Ograničiti red: zadržati najviše N klipova s PCM-om (npr. konstanta `MOST_WAITING = 8`); preko toga spremiti samo
     id klipa i PCM ponovno dobiti kad dođe na red (put koji već postoji za klip bez vala), ili blokirati `sink` s
     `Condvar` dok se ne oslobodi mjesto. Izabrati varijantu koja ne mijenja redoslijed prijepisa.
- **Uvjet provjere:** release gradnja; ručno uvoz kartice s >50 klipova — memorija radnika ograničena, prijepis istim
  redom (log). Testovi `qnc-transcribe-queue`/`qnc-transcribe-run` prolaze.

### [Prioritet: VISOKO] S-3 `ensure_schema` dnevnika mijenja shemu pri svakom pridruživanju modula
- **Lokacija:** `crates/qnc-project-journal/src/lib.rs` `ensure_schema` (≈94-131: `DROP VIEW IF EXISTS` + `CREATE VIEW`,
  ad-hoc `ALTER TABLE ADD COLUMN`); poziv `qnc-db-broker/src/database.rs` `start_journal` (≈228-232);
  `transport.rs` `IDLE_RELEASE = 2s` (≈27, 268-284).
- **Opis problema:** svako pridruživanje (nakon 2 s mirovanja) radi DDL u autocommitu: diže schema cookie (svi pripremljeni
  upiti u svim procesima se ponovno pripremaju), traži write lock, može pasti na BUSY i srušiti korisnikovu izmjenu.
- **Zaštićena logika (NE DIRATI):** sadržaj pogleda `public_project_journal`; stupci `undoable`, `refers` (dodaju se
  starim dnevnicima); tablice u bazi u koju ide izmjena (projektna ili nacrt priče).
- **Točan plan popravka:**
  1. `CREATE TABLE/INDEX IF NOT EXISTS` ostaje; `DROP VIEW` + `CREATE VIEW` zamijeniti s `CREATE VIEW IF NOT EXISTS`, a
     pogled ponovno stvarati samo kad se razlikuje (usporediti `sql` iz `sqlite_master` s očekivanim tekstom).
  2. Provjeru stupaca (`pragma_table_info`) raditi prije ALTER-a i ALTER izvoditi samo kad stupca nema (provjeriti da je tako;
     ako jest, samo izbaciti bezuvjetni DDL).
  3. Sve DDL koje se ipak izvede staviti u `BEGIN IMMEDIATE … COMMIT`.
  4. U brokeru zapamtiti (po datoteci, u procesu) da je shema već osigurana i ne zvati `ensure_schema` ponovno.
- **Uvjet provjere:** test: dva uzastopna `ensure_schema` na istoj bazi ne mijenjaju `PRAGMA schema_version`. Testovi
  `qnc-project-journal` (3), `qnc-db-broker`, `qnc-program-db` prolaze.

### [Prioritet: VISOKO] S-4 `row_history` skenira cijeli dnevnik kroz `json_extract`
- **Lokacija:** `crates/qnc-project-journal/src/lib.rs` `row_history` (≈274-310); pozivi `qnc-clip-marks/src/store.rs`
  (≈65), `qnc-program-db/src/object_undo.rs` (≈84, `limit = usize::MAX>>1`).
- **Opis problema:** `project_journal_changes` ima samo PK `(commit_id, seq)`; upit parsira JSON svakog retka projekta.
  Trošak raste bez granice (vidi S-11 rast dnevnika).
- **Zaštićena logika (NE DIRATI):** semantika "najstariji prvi, najnovijih 200"; dnevnik samo dodaje (autorizator).
- **Točan plan popravka:**
  1. Dodati stupac `row_key TEXT` u `project_journal_changes` (dodaje se starim dnevnicima kao S-3), puniti ga u `append`
     iz ključa tablice (isti ključ koji `qnc-journal-revert` koristi).
  2. Indeks `CREATE INDEX IF NOT EXISTS ... ON project_journal_changes(tbl, row_key, commit_id)`.
  3. `row_history`: `WHERE tbl = ?1 AND row_key = ?2`; za stare retke s `row_key IS NULL` zadržati postojeći `json_extract`
     kao rezervu (`OR (row_key IS NULL AND json_extract(...))`).
  4. Autorizator (`owned.rs`) mora dopustiti novi stupac pri dodavanju (provjeriti).
- **Uvjet provjere:** postojeći testovi `qnc-project-journal` (`row_history`), `qnc-clip-marks` (2), `qnc-program-db`
  (undo selected); `EXPLAIN QUERY PLAN` pokazuje korištenje indeksa (test ili primjer).

### [Prioritet: VISOKO] S-5 `ProjectStore::open` piše u registar pri svakom otvaranju, a poziva se svake sekunde
- **Lokacija:** `crates/qnc-project-store/src/lib.rs` `ProjectStore::open` (≈169-189), `ensure_templates_seeded`
  (≈1691-1764), `lock_registered_project_dirs`; pozivatelji: `qnc-card-ingest/src/lib.rs` (≈215-250, petlja `READ = 1s`),
  `qnc-ingest-base` `closing` (≈83-93), `not_taken`; `qnc-work-progress/src/lib.rs` `progress` (≈136-200).
- **Opis problema:** svako otvaranje: validacija ugovora, nova veza + pragme, ~15 `CREATE IF NOT EXISTS`, upsert svih
  sistemskih predložaka s `updated_at = now` (uvijek upis, autocommit po retku), ACL poziv po projektu. Dok je prozor uvoza
  otvoren: ~8-15 otvaranja i nekoliko upisnih transakcija u sekundi; natječu se s pravim upisima.
- **Zaštićena logika (NE DIRATI):** odluka 22. 11. (audit 12.3): sistemski predlošci se namjerno ponovno upisuju iz seeda;
  zaštita mapa (`icacls_has`, `set_project_hidden`); vlasnik projekta i predlošci po korisniku.
- **Točan plan popravka:**
  1. Uvesti lagani `ProjectStore::open_read` (ili `RegistryReader`) koji samo otvara registar read-only s `busy_timeout`,
     bez seeda, sanitizacije i zaključavanja mapa; koristiti ga u `closing`, `not_taken`, `not_taken_of`, `active_is_base`,
     `story_bases`, i u `qnc-work-progress`.
  2. U `ensure_templates_seeded` upsert raditi u jednoj transakciji i samo kad se sadržaj razlikuje
     (`ON CONFLICT DO UPDATE ... WHERE excluded.<json> IS NOT <json>`), da `updated_at` ne mijenja bez razloga.
  3. Ne mijenjati put `create_project`, `delete_project`, `create_ingest_base`.
- **Uvjet provjere:** testovi `qnc-project-store` (46), `qnc-ingest-base`, `qnc-card-ingest`, `qnc-work-progress`. Ručno:
  prozor uvoza otvoren 60 s — `updated_at` sistemskih predložaka ne mijenja se (primjer `template_choice` ili SQL upit
  na kopiji registra).

### [Prioritet: VISOKO] R-2 Nacrt složene izjave ostaje nakon neuspjelog ili pretečenog upisa
- **Lokacija:** `crates/qnc-side-pieces/src/statement.rs` `keep` (≈159-194), `poll` (≈111-120).
- **Opis problema:** `self.block = Some(drafted)` prije upisa; na `Err` samo `qnc_notice::raise`, prikaz ostaje na
  dijelovima kojih baza nema, ništa se ponovno ne čita. Dva brza Entera pokreću dvije dretve `set_parts_now`; prvi
  rezultat se baca, stariji upis može pasti zadnji.
- **Zaštićena logika (NE DIRATI):** "ono što Enter ili Spremi zadrže ostaje u prikazu" (23. 11.); rubovi 1/13 uz prvi IN i
  zadnji OUT; play od IN do OUT (`RangePlay`); `axis_frame`.
- **Točan plan popravka:**
  1. Čuvati prethodni `block` (`self.block_before`) prije postavljanja nacrta.
  2. U `poll`, na `Err`: vratiti `block_before` i označiti ponovno čitanje izjave iz baze.
  3. U `keep`: ako je upis u tijeku, ne pokretati drugi; zapamtiti zadnji nacrt kao `queued` i pokrenuti ga kad prvi završi
     (zadnji pobjeđuje, nikad dva istodobno).
- **Uvjet provjere:** test u `qnc-side-pieces` s lažnim pisačem koji vraća grešku → prikaz se vrati; dva `keep` zaredom →
  jedan upis u tijeku, drugi slijedi, konačno stanje = zadnji nacrt. Postojeći testovi `qnc-side-pieces`.

### [Prioritet: VISOKO] R-3 Strukturne izmjene složene izjave u prikazu izjave se tiho gube
- **Lokacija:** `crates/qnc-side-pieces/src/statement.rs` `act` (≈146-153); `crates/qnc-subsegment-edit/src/lib.rs`
  `write` / `take_writes` (≈1350), `drafted()`.
- **Opis problema:** `StatementPlay` baca closure iz `take_writes()` i sprema `drafted` uhvaćen prije tipke; `drafted()`
  pokriva samo trim, join, slide, slip, sound. `PART_REMOVE`, `PART_MOVE`, `JOIN_GAP/CUT/SMOOTH`, `PART_CHANNEL/LEVEL/OFF`
  vraćaju `true` a ništa se ne zapiše.
- **Zaštićena logika (NE DIRATI):** pravila složene izjave (AGENTS §3a "Izjave": pauza, J/L i A1 dijela izjava ne čuva);
  zoom dosega (`Scope`), `nearest_edge` nikad `cut`.
- **Točan plan popravka:**
  1. Najprije provjeriti (i zapisati u izvještaj) koje od tih radnji su dostupne u prikazu izjave AI Media Assista.
  2. Za dostupne koje izjava čuva (uklanjanje, premještanje dijela, vrsta spoja): `drafted` uzeti **nakon** obrade tipke,
     a `drafted()` proširiti da odražava redoslijed/broj dijelova i vrstu spoja.
  3. Za radnje koje izjava ne čuva (kanal, razina, OFF, pauza): u `StatementPlay` ih ne prosljeđivati (vratiti `false`),
     da tipka ne izgleda prihvaćena. Ako je potrebna odluka koje radnje dopustiti — pitati korisnika.
- **Uvjet provjere:** test: `PART_REMOVE` na izjavi od 3 dijela → baza ima 2 dijela. Testovi `qnc-subsegment-edit`,
  `qnc-side-pieces`, `qnc-script`.

---

## 3. SREDNJE

### [Prioritet: SREDNJE] C-4 `Player::drop` radi `join` na UI dretvi
- **Lokacija:** `crates/qnc-player-client/src/lib.rs` (≈491-497 `Drop`), `connection.rs` (≈590 `open_session` do 10 s,
  ≈466-492, ≈774-778 `session_ended` 2 s + kill), `FramePump::stop` (≈366-377).
- **Opis problema:** drop playera na UI dretvi može čekati radnika u `load`, `open_session` (10 s) ili gašenju veze (2 s) →
  smrzavanje >12 s. `.lock().unwrap()` posvuda: panika s uzetim `view` truje lock i sljedeći `view()` ruši UI.
- **Zaštićena logika (NE DIRATI):** redoslijed lockova `view` → `load`; `publish` otpušta `view` prije `notify`.
- **Točan plan popravka:** u `Drop` poslati stop i premjestiti `JoinHandle` u odvojenu "reaper" dretvu (detach) umjesto
  `join` na pozivatelju; `.lock().unwrap()` → `.lock().unwrap_or_else(|e| e.into_inner())` za `view`.
- **Uvjet provjere:** testovi `qnc-player-client`; ručno: zatvaranje forme dok player učitava klip s mreže ne smrzava prozor.

### [Prioritet: SREDNJE] C-5 Puna kopija svake slike monitora na UI dretvi
- **Lokacija:** `crates/qnc-ui-kit/src/raster.rs` `paint_stream_frame` (≈42-47), `write_rgba_texture` (≈329-336);
  `crates/qnc-monitor/src/lib.rs` (≈71-78); `crates/qnc-trim-stills/src/lib.rs` `show_live` (≈146-151, `to_vec()`).
- **Opis problema:** `MonitorFrame.rgba` je već `Arc<[u8]>`, ali se predaje kao `&[u8]` pa `Arc::from` kopira (do 8 MB po
  slici, ~400 MB/s na 1080p50). `show_live` kopira još jednom po slici i dobiva novi `content_key` → nova tekstura po
  slici; izbacivanje iz cachea skenira 1024 unosa.
- **Zaštićena logika (NE DIRATI):** `press_feedback` i tema; ključ `held` po `picture` sekvenci.
- **Točan plan popravka:** dodati varijantu `paint_stream_frame_arc(.., Arc<[u8]>)` i koristiti je iz `qnc-monitor`; u
  `RgbaImage` dopustiti `Arc<[u8]>` (ili `show_live` predati `Arc` klon) i ponovno koristiti isti `content_key` za
  živo mjesto (jedna tekstura koja se ažurira).
- **Uvjet provjere:** testovi `qnc-ui-kit`, `qnc-monitor`, `qnc-trim-stills`; ručno: playback 1080p50 — CPU UI procesa niži,
  slika ista.

### [Prioritet: SREDNJE] C-6 Shared-memory frame map može dati poderanu sliku
- **Lokacija:** `crates/qnc-player-frame-transport/src/lib.rs` pisač (≈142-154), čitač (≈287-310).
- **Opis problema:** pisač nulira `begin`, kopira, postavlja `begin` običnim kopijama bez atomika/ograda; čitač provjeri
  `begin == end == generation` samo prije `to_vec()`. Ako pisač obiđe 8 slotova tijekom kopije, slika je poderana.
- **Zaštićena logika (NE DIRATI):** raspored slotova (8), format zaglavlja koji čita program output host.
- **Točan plan popravka:** seqlock: pisač `begin` piše s `Release` (`AtomicU64` preko pokazivača u mapi), payload, `end`
  s `Release`; čitač čita `end` (`Acquire`), kopira, ponovno čita `begin` (`Acquire`) i odbacuje kopiju ako se razlikuje.
- **Uvjet provjere:** test s pisačem u petlji i čitačem koji provjerava uzorak svakog bajta; testovi transporta i
  `qnc-program-output-host` gradnja.

### [Prioritet: SREDNJE] C-7 Tray glavna petlja radi disk, DB i mrežu na dretvi događaja traya
- **Lokacija:** `apps/qnc-ai-tray/src/main.rs` glavna petlja (≈166-401: `menu_items` ≈170, `catalog` ≈289,
  `watch.look` ≈302, `known_card` ≈332, `try_event` ≈233); `crates/qnc-card-readers/src/lib.rs` (≈179-182 `FIRST_WAIT`).
- **Opis problema:** spora kartica ili mrežni čitač smrzavaju izbornik i dijaloge traya (popravak 24. 11. riješio je samo
  čitače, ne `what_is` nove kartice ni `known_card`).
- **Zaštićena logika (NE DIRATI):** AutoPlay guard odluka po kartici; jedan prozor uvoza po bazi; `StartAsk` pravilo;
  brisanje baze i Izlaz koji čeka brisanje.
- **Točan plan popravka:** premjestiti `watch.look` + `what_is` + `known_card` u pozadinsku dretvu koja šalje gotove događaje
  kanalom (`sync_channel`); glavna petlja samo prima i reagira. `menu_items` čitati samo kad se promijeni vrijeme izmjene
  datoteke.
- **Uvjet provjere:** release gradnja traya; ručno s nedostupnim Jetsonom: klik na ikonu traya otvara izbornik < 200 ms;
  ubacivanje kartice i dalje pokreće prozor uvoza.

### [Prioritet: SREDNJE] C-8 `qnc-playback-activity`: DB pod lockom i `.expect` koji ruši cijeli ingest
- **Lokacija:** `crates/qnc-playback-activity/src/lib.rs` (≈65-86, `playback_pause` ≈130-141);
  `tools/qnc-ingest-worker/src/run.rs` (≈366 `stuck.lock().expect`, ≈412, 424 `failure.lock().expect`).
- **Opis problema:** svi koraci čekaju jedno DB čitanje svakih 500 ms; panika u upitu truje lock, a `.expect` na sljedećem
  pozivu unutar `thread::scope` ruši cijeli rad radnika.
- **Zaštićena logika (NE DIRATI):** `playback_plays` (kopija čeka samo dok player svira, 22. 11.).
- **Točan plan popravka:** pod lockom samo provjeriti je li vrijednost svježa; ako nije, otpustiti lock, pročitati DB,
  ponovno uzeti lock i upisati. Sve `.expect` na tim lockovima → `unwrap_or_else(|e| e.into_inner())`.
- **Uvjet provjere:** test `the_copy_waits_while_a_player_plays_not_while_the_user_edits` i ostali `qnc-playback-activity`.

### [Prioritet: SREDNJE] S-6 `stacks` ponavlja cijeli dnevnik modula pri svakom čitanju priče
- **Lokacija:** `crates/qnc-project-journal/src/lib.rs` `stacks` (≈207-233); poziv
  `crates/qnc-program-db/src/store/reads.rs` (≈198, `ReadStorySelection` u svakom `ReadProgramSnapshot`).
- **Opis problema:** O(cijela povijest) nakon svake izmjene priče; raste s projektom.
- **Zaštićena logika (NE DIRATI):** pravila stogova (korak ide na Undo i prazni Redo, `undo`/`redo` preko `refers`).
- **Točan plan popravka:** memoizirati u modulu: zapamtiti `(zadnji_id, stacks)`; pri sljedećem pozivu čitati samo commitove
  `id > zadnji_id` i primijeniti ih inkrementalno (ista funkcija prijelaza). Restore ili tuđi upis s manjim id-om ne postoji
  (dnevnik samo dodaje), pa je inkrement siguran.
- **Uvjet provjere:** test: inkrementalni rezultat == puni rezultat nakon niza korak/undo/redo/snimka. Testovi
  `qnc-project-journal`, `qnc-program-db`, `qnc-script-db`.

### [Prioritet: SREDNJE] S-7 Restore projekta nije atoman preko modula
- **Lokacija:** `crates/qnc-project-restore/src/lib.rs` (≈30-48); `crates/qnc-db-broker/src/owned.rs` (≈236-265).
- **Opis problema:** proba (`check`) pa vraćanje, svaki modul u svojoj transakciji/vezi; izmjena između prolaza ili pad
  modula N ostavlja module 1..N-1 vraćene.
- **Zaštićena logika (NE DIRATI):** samo vlasnik smije vratiti; povratak je i sam trenutak (grana se ne briše).
- **Točan plan popravka:** svi moduli iste datoteke dijele vezu brokera — vraćanje izvesti u jednoj `BEGIN IMMEDIATE`
  transakciji na jednoj vezi (ATTACH nije potreban jer su u istoj datoteci); `check` i vraćanje u istoj transakciji.
  Ako to arhitektura brokera ne dopušta bez većeg zahvata, barem: na grešku modula N izvesti povratak već vraćenih
  modula (njihovi `restore` commitovi preko `revert`) i javiti grešku.
- **Uvjet provjere:** test u `qnc-project-history`: modul 2 namjerno padne → modul 1 nije promijenjen. Postojeća 2 testa prolaze.

### [Prioritet: SREDNJE] S-8 Nedostaju indeksi na vrućim upitima
- **Lokacija:** `crates/qnc-content-store/src/schema.sql` (nema sekundarnih indeksa na `clips`), `database.rs` `ClaimNext`
  (≈395-425), `Inventory` (≈249); `crates/qnc-content-read/src/lib.rs` (≈396-450); `crates/qnc-artifact-db/src/store.rs`
  `read_made` (≈77, `json_extract(frames_json,'$.status')`).
- **Opis problema:** `ClaimNext` radi puni sken + sortiranje unutar `BEGIN IMMEDIATE` po svakom klipu; `read_made` sken
  svake sekunde.
- **Zaštićena logika (NE DIRATI):** redoslijed uvoza po vremenu snimanja (13. 11.), klip bez vremena iza, po nazivu.
- **Točan plan popravka:** dodati `CREATE INDEX IF NOT EXISTS clips_queue ON clips(import_status) WHERE import_status IN
  ('queued','processing')` i `clips_source ON clips(source_uri, clip_id)` (kroz postojeći mehanizam sheme content-storea);
  za artefakte stupac/indeks statusa ili generirani stupac `status` s indeksom. ORDER BY se ne mijenja.
- **Uvjet provjere:** test `the_import_takes_the_clips_in_the_order_they_were_recorded` prolazi; `EXPLAIN QUERY PLAN` koristi indeks.

### [Prioritet: SREDNJE] S-9 Uhvaćeni retci neuspjelog upisa ulaze u sljedeći commit
- **Lokacija:** `crates/qnc-db-broker/src/database.rs` (≈204-209, rani povratak `?`).
- **Opis problema:** ako journaled modul napravi autocommit izjave pa padne, TEMP retci ostaju i pridruže se sljedećoj
  uspješnoj izmjeni pod krivom radnjom; Undo bi ih vratio zajedno.
- **Zaštićena logika (NE DIRATI):** isto kao S-1.
- **Točan plan popravka:** na grani greške pozvati `qnc_change_capture::take(module.connection())` i odbaciti. (Ako se
  izvede S-1, ROLLBACK vanjske transakcije ovo pokriva; svejedno dodati `take` radi TEMP tablice izvan transakcije.)
- **Uvjet provjere:** test: neuspjeli upis pa uspješan → commit sadrži samo retke drugog.

### [Prioritet: SREDNJE] S-10 Nacrti priče i `ai.db` nastaju u rollback modu
- **Lokacija:** `crates/qnc-db-broker/src/database.rs` `with_companion` (≈126-141), `connect` (≈294-320).
- **Opis problema:** `story-draft-N.db` i `ai.db` uvijek rade s čitačima koji blokiraju pisače (AI radnici pišu `ai.db`
  dok ga forme čitaju). Nedosljedan `synchronous` (broker FULL, project-store NORMAL na istoj datoteci).
- **Zaštićena logika (NE DIRATI):** PERSIST za bazu u zaštićenoj mapi kad nije WAL (journal se ne smije brisati);
  `keep_wal_file`.
- **Točan plan popravka:** pri **stvaranju** pratioca (`CREATE`) postaviti `journal_mode=WAL` (WAL datoteke se ne brišu uz
  `PERSIST_WAL`, pa je kompatibilno sa zaštitom mape — provjeriti na zaštićenoj mapi). Postojeće datoteke ne prebacivati.
- **Uvjet provjere:** test `an_ingest_base_is_known_by_its_own_name_and_gets_no_draft_story` i ostali `qnc-db-broker`; novi
  nacrt priče ima `journal_mode = wal`.

### [Prioritet: SREDNJE] R-4 `ShortSave` gubi ishode istodobnih spremanja
- **Lokacija:** `crates/qnc-short-save/src/lib.rs` `write` (≈220-237), `tune` (≈122-127), `self.written` (≈129, 170),
  overwrite (≈313), `add_part` (≈146).
- **Opis problema:** novi upis prepisuje `pending` → ishod prvog (i spremanje slika IN/OUT) se gubi; fine-tune dok je upis
  u tijeku se tiho baca; `written` postavljen prije upisa i ne briše se na grešku → sljedeći pokušaj na istom rezu
  preskočen kao `already`.
- **Zaštićena logika (NE DIRATI):** pitanje spoja (Cut/Smooth/Blic, `JOIN`, `JOIN_CANCEL`); `can_compound` pravilo (27. 11.).
- **Točan plan popravka:** red upisa (`VecDeque`) umjesto jednog `pending`; obrađivati jedan po jedan; `written` postaviti
  tek na uspjeh, na grešku obrisati; odbijene radnje dok je upis u tijeku stavljati u red, ne bacati.
- **Uvjet provjere:** testovi `qnc-short-save`; novi: dva `write` zaredom → dva ishoda u `short_saved`.

### [Prioritet: SREDNJE] R-5 Optimistička kartica "saving" se ne pokazuje; Update briše prikazanu izjavu
- **Lokacija:** `crates/qnc-editorial-application/src/lib.rs` (≈387 `saving()` samo kad `question().is_none()`,
  ≈401 `drop_saving`, ≈404 `clear_source_marks`, ≈407 `if !saved.part { shown_shot_id = None }`).
- **Opis problema:** `start()` uvijek postavi pitanje ili potpis, pa uvjet nikad nije ispunjen — instant kartica (15. 11.)
  nestala. Svaki trim/Update/Overwrite vraća `part: false` → prikazana izjava nestaje i IN/OUT se brišu nakon svakog
  finog podešavanja.
- **Zaštićena logika (NE DIRATI):** pravila 27. 11.: Dodaj izjavu izabire ali ne prikazuje; nakon spremanja IN/OUT se briše;
  zasivljeni korišteni rasponi (`used_ranges`).
- **Točan plan popravka:** u `Saved` dodati polje vrste (`New`, `Part`, `Trim`); `shown_shot_id = None` i
  `clear_source_marks` samo za `New`/`Part`. `saving()` pozvati kad je pitanje/potpis zatvoren i upis krenuo (tamo gdje se
  stvarno šalje upis), ne pri `start()`. **Pitati korisnika** želi li instant karticu natrag (ponašanje se mijenja).
- **Uvjet provjere:** test `qnc-editorial-application`: Update prikazane izjave → `shown_shot_id` ostaje; Dodaj izjavu →
  marke obrisane. `qnc-virtual-short-cards` testovi.

### [Prioritet: SREDNJE] R-6 Povijest zooma: vraćanje po `id` bira krivi redak dok upis nije pročitan
- **Lokacija:** `crates/qnc-clip-marks/src/lib.rs` (≈135-151, 172: nezapisani unosi s `id: 0`);
  `crates/qnc-source-timeline/src/zoom.rs` (≈211, 219 `find(|e| Some(e.id) == restore)`).
- **Opis problema:** prvi Keep doda dva unosa s `id 0`; u ~1 s prije ponovnog čitanja Enter na najnovijem retku vrati
  najstariji `id 0` redak.
- **Zaštićena logika (NE DIRATI):** "prvi Spremi klipa najprije zapiše gdje su bili"; povratak je upis kao svaki drugi.
- **Točan plan popravka:** vraćati po indeksu u prikazanom popisu (ili po `(id, made_at, in, out)`), ne samo po `id`;
  ili lokalnim unosima davati negativne privremene id-ove (-1, -2...).
- **Uvjet provjere:** test u `qnc-source-timeline`: Keep pa odmah restore najnovijeg → vraćene marke najnovijeg.

### [Prioritet: SREDNJE] R-7 Povijest zooma: `held_from`/`last_taken` prelaze s klipa na klip
- **Lokacija:** `crates/qnc-source-timeline/src/zoom.rs` (≈88-96, 172-189 `keep_history`).
- **Opis problema:** promjena klipa dok je oznaka uzeta ne resetira stanje; na klipu B se zapiše "from" s markama klipa A.
  Isto nakon `ZoomUse::Yield`.
- **Zaštićena logika (NE DIRATI):** zoom ostaje otvoren nakon Entera do Esc/Zatvori/drugog klipa (`close_zoom`).
- **Točan plan popravka:** pamtiti klip uz `held_from`; ako se klip promijeni ili `used != On`, obrisati `held_from` i
  `last_taken`.
- **Uvjet provjere:** test: uzmi IN na A, promijeni na B, uzmi i pusti → zapis za B ima "from" od B.

### [Prioritet: SREDNJE] R-8 Dva puta kopiranja izjava baze u priču s različitim pravilima
- **Lokacija:** `crates/qnc-ingest-base/src/lib.rs` `copy_base_records` (≈461-519, `INSERT OR IGNORE`, id-ovi baze);
  `crates/qnc-base-statements/src/lib.rs` `copy` (≈41-72, po nazivu klipa, novi id, dedup `(clip,in,out)`).
- **Opis problema:** put 2 (radnik nakon svakog uvoza) vraća izjave koje je korisnik obrisao ili skratio u priči; dva klipa
  istog naziva (dvije kamere) sudaraju se u mapi naziva; djelomičan `add_part_now` ostavlja izjavu trajno okrnjenu; dijelovi
  baze mogu se zakačiti na izjavu priče s istim `shot_id`.
- **Zaštićena logika (NE DIRATI):** odluka 24. 11. "izjave smiju ostati u bazi i kopiraju se u projekt"; 5c93145 (vlasnici
  rade shemu, kopija samo retke).
- **Točan plan popravka:** **Pitati korisnika** koji put ostaje kanonski. Preporuka: zapisati u priči koje su izjave baze već
  kopirane (id izjave baze → id u priči) i nikad ih ponovno ne kopirati; ključ klipa = original (putanja/izvor), ne naziv;
  izjavu i njezine dijelove upisati u jednoj transakciji.
- **Uvjet provjere:** test u `qnc-base-statements`: obrisana izjava u priči nakon drugog uvoza ostaje obrisana; dva klipa
  istog naziva dobivaju svoje izjave.

### [Prioritet: SREDNJE] R-9 Space u zoomu svira preko OUT-a
- **Lokacija:** `crates/qnc-playhead-moves/src/lib.rs` `play_part` (≈139-170); `crates/qnc-panel-focus/src/route.rs`
  `zoom_play_part` (≈604-615); `zoom_axis.rs` (≈149).
- **Opis problema:** play koji krene s `to-1` nikad ne zadovolji `last + 1 < to` → ne staje; prethodni play završava upravo
  `cue(to-1)`. `zoom_play_part` ne provjerava zoom source timelinea.
- **Zaštićena logika (NE DIRATI):** pravilo 25. 11.: Space svira od mjesta playheada, nikad skok na početak; na početak vodi
  samo klik na srednji player. `RangePlay` izjave.
- **Točan plan popravka:** uvjet zaustavljanja `frame >= to - 1` (zadnja slika dijela) i kad je start == `to-1`, play izlazi
  iz dijela (obični play) — ili stati odmah; odlučiti prema docs [D] (playhead na OUT-u: obični play dalje, kao source
  timeline `stop_on_out`). Ne dodavati drugo pravilo za source zoom: on već ima `stop_on_out` — samo provjeriti da se
  ne preklapaju.
- **Uvjet provjere:** test `qnc-playhead-moves`: start na `to-1` → definirano ponašanje; postojeći testovi prolaze.

### [Prioritet: SREDNJE] R-10 Povijest zooma prikazuje UTC vrijeme
- **Lokacija:** `crates/qnc-source-timeline/src/zoom.rs` (≈244 `made_at_utc.get(11..16)`).
- **Opis problema:** u Hrvatskoj sat/dva iza. `qnc-project-journal` `moments` već daje lokalno vrijeme.
- **Zaštićena logika (NE DIRATI):** format retka "IN tc · OUT tc".
- **Točan plan popravka:** koristiti istu pretvorbu u lokalno vrijeme kao `moments` (izdvojiti u javnu funkciju ako treba).
- **Uvjet provjere:** test pretvorbe; ručno: vrijeme u popisu = sat na računalu.

### [Prioritet: SREDNJE] R-11 Filter jeke prijepisa zadržava krivu kopiju riječi
- **Lokacija:** `crates/qnc-transcript-db/src/correct.rs` `corrected` (≈40-56), `without_echoes` (≈189-206).
- **Opis problema:** neporavnata fina riječ umetne se po svom `from`, može stajati prije grube; `without_echoes` zadrži prvu
  (finu, bez interpunkcije i velikog slova) — suprotno komentaru. Jeka preko granice retka ostaje.
- **Zaštićena logika (NE DIRATI):** pravilo 22. 11.: Parakeetova interpunkcija i veliko slovo ostaju, wav2vec2 prepisuje
  riječ; testovi `the_fine_words_correct_the_rough_ones_and_keep_their_sentences`,
  `a_fine_word_a_little_off_its_rough_one_overwrites_it_and_is_not_there_twice`.
- **Točan plan popravka:** u `without_echoes` između dvije jeke zadržati onu s grubim zapisom (interpunkcija/veliko slovo),
  ne prvu; provjeru proširiti na zadnju riječ prethodnog retka.
- **Uvjet provjere:** novi test s finom riječi ispred grube → ostaje gruba s točkom. Postojeći testovi prolaze.

---

## 4. NISKO (kratko)
- **R-12 Zastarjeli indeks dijela u zoomu složene izjave** — `qnc-subsegment-edit/src/lib.rs` `forget_parts_gone`
  (≈305-334), `let_go` (≈1398): brisati `zoom_part` kad dio nestane ili se premjesti.
- **R-13 Rubovi izjave** — `qnc-side-pieces/src/statement.rs:170`: `.min(frames)` primijeniti nakon `.max(last_out+1)`;
  `qnc-subsegment-edit/src/pointer.rs` `nearest_edge` (≈96-108) ne smije birati rub 1/13.
- **R-14 Pravilo "Dodaj dio" na dva mjesta** — `qnc-editorial-application/src/view.rs:58` i `lib.rs:391`: istu provjeru
  (korisnik postavio IN/OUT) staviti u kocku (`qnc-short-save` `can_compound`) i zvati je s oba mjesta; `REMOVE_SHORT`
  brisati odabir tek nakon uspjelog brisanja.
- **C-9 Petlje s aktivnim čekanjem** — `qnc-player-client/src/lib.rs:331` (1 ms), `tools/qnc-player-runner/src/main.rs:227`
  (1 ms), `control.rs:138` (2 ms accept), `qnc-audio-output/src/lib.rs:192` (1 ms). Ne dirati sat playera bez mjerenja;
  accept petlju prebaciti na blokirajući accept s timeoutom.
- **C-10 Dretva po TCP vezi i 300 s read timeout** — `tools/qnc-player-runner/src/control.rs` (≈116-142): ograničiti broj
  dretvi, timeout na 10 s, provjera `stop`.
- **C-11 Dretve čitača kartica nikad ne staju** — `qnc-card-readers/src/lib.rs` (≈174-178): zastavica za kraj kad čitač
  nestane iz postavke.
- **C-12 ffmpeg stderr pipe** — `qnc-ffmpeg-decode/src/lib.rs` `run_filmstrip_command` (≈515-548): stderr čitati u zasebnoj
  dretvi tijekom rada.
- **C-13 Dretva po zahtjevu** — `qnc-program-waveform` (≈282-302), `qnc-program-filmstrip` (≈193-213): jedna dugotrajna
  dretva s redom.
- **C-14 Prozor uvoza čita datoteku pri svakom crtanju** — `apps/qnc-ai-card-ingest/src/main.rs:225` (`qnc_tray_mode::read`
  u `update`) i `qnc-card-ingest/src/lib.rs:333` (`not_taken` u `poll` na UI dretvi): premjestiti u pozadinsku dretvu.
- **S-11 Dnevnik raste bez granice i čuva cijele retke dvaput** — `qnc-change-capture/src/lib.rs` (≈44-53),
  `script_blocks` JSON stupci. Odluka korisnika potrebna (dnevnik je "kvazi git", ne smije se brisati). Prijedlog:
  pohranjivati samo promijenjene stupce u `after_row`; bez brisanja povijesti.
- **S-12 Migracije bez verzije** — `qnc-content-store/src/database.rs` `ensure_*_column` (≈946-1000), `qnc-script-db`
  `has_column` po podnizu `sql.contains(name)` (≈662), `qnc-ingest-base` `info_writer_in` i ostali `CREATE TABLE IF NOT
  EXISTS` pisači izvan brokera (≈67-79, 166-179, 528-546): `has_column` preko `pragma_table_info`; ALTER u `BEGIN
  IMMEDIATE` i tolerirati "duplicate column"; `Connection::open` bez `CREATE` flaga gdje baza mora postojati.
- **S-13 WAL datoteka se nikad ne skraćuje** — nakon velikog uvoza `PRAGMA wal_checkpoint(TRUNCATE)` kad radnik završi.
- **Poznati padajući testovi (odluka korisnika):** `failed_probe_is_durable_and_never_retried` — `qnc-record-probe/src/lib.rs`
  (≈228-238) od 18. 11. tretira `Error::Failed` kao `Interrupted` (test zastario ili pokvarene datoteke se probaju pri
  svakom Selectu); `undo_puts_only_the_taken_mark_back_where_it_was` — `qnc-source-panel/src/lib.rs` (≈103-112, 174)
  `back_to_origin` vraća IN i OUT po pravilu 20. 11. (test zastario); `qnc-ingest-import-worker` — pokrenuti
  `cargo test --release -p qnc-ingest-import-worker` i utvrditi koja dva (vjerojatno
  `without_a_lease_and_without_its_executable_launching_says_so` ovisi o okolini i tekstu rječnika, a
  `the_copy_waits_while_a_player_plays_not_while_the_user_edits` o fiksnom spavanju od 600 ms).

## Što je pregledano i izgleda ispravno (ne dirati bez razloga)
- Audio callback (`qnc-audio-output`): samo atomici, bez lockova, alokacija i IO.
- Redoslijed lockova u `qnc-player-client` (`view` → `load`), `qnc-program-output-host`.
- `qnc-panel-focus`, `qnc-form-host`: bez sinkronizacijskih primitiva i blokirajućih poziva.
- Pisanja su dosljedno `BEGIN IMMEDIATE` (`qnc_write_lease::begin_write`, `attach`, `ClaimNext`, lease).
- Read-only veze nikad ne stvaraju shemu; TEMP okidači ne cure (nema poola).
- `busy_timeout` 5 s gotovo posvuda (iznimke: `qnc-ingest-base` `active_is_base`, `story_bases`, `closing` — dodati).
- Autorizator drži dnevnik append-only i shemu zatvorenom.
- Upis priče je atoman (izmjena, provjera pokrivanja, revizija) i snapshot se čita u jednoj transakciji.

## Preporučeni redoslijed izvedbe
1. S-1 (+ S-9) → 2. S-2 → 3. R-1 → 4. S-3 → 5. S-4 → 6. C-1 → 7. C-2, C-3, C-8 (radnik zajedno) → 8. S-5 →
9. R-2, R-3 → 10. ostale SREDNJE → 11. NISKO.
Stavke s oznakom **pitati korisnika**: R-3 (koje radnje), R-5 (instant kartica), R-8 (kanonski put), S-11 (dnevnik).
