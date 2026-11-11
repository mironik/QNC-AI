# QNC-AI: plan AI wizarda

Datum: 2026-11-11. Odluke korisnika:
- Opcija B: zaseban repo `QNC-AI`, samostalna aplikacija na kockama QNC-a; kocke se smiju mijenjati ovdje.
- Wizard radi i na laptopu: ne trazi nista vise od QNC-a.
- **Wizard vodi korisnika, korisnik ne bira sto ce raditi** (2026-11-11: "ne bi radio ovakvu vrstu izbornika").
  Put pocinje dogadajem: ubacena kartica. Svaki sljedeci korak otvara se sam kad je prethodni gotov;
  korisnik potvrduje ili ide korak natrag.
- **Wizard nema svoju formu** (2026-11-11: "ne zelim formu vizarda, vizard moze biti sastavni dio
  traya"): wizard je program u trayu koji se ne vidi; otvara formu koraka (Ingest za karticu,
  obrazac priloga, AI Media Assist, MA Script, Story) i javlja se obavijestima. Gdje je prica vidi se
  u izborniku ikone (klik na ikonu, bilo kojom tipkom) i u njezinu opisu.
- Pokrivalice idu po ponudi, korisnik bira; postavljaju se markerima ili Sync/B-rollom.
  Automatsko pokrivanje mozda kasnije, nije prioritet.
- AI server nije obavezan; moze biti samostalni lokalni server na laptopu bez QNC-a.
- wav2vec2 ParlaSpeech se koristi bez obzira na nenavedenu licencu.

## 1. Tok

1. **Program u trayu** (`qnc-ai-tray`) radi na racunalu za montazu: lagan, bez AI modela i bez
   prozora, jedan primjerak, pokrece se s prijavom korisnika. Svake dvije sekunde gleda spojene
   diskove (`qnc-source-volumes`).
2. **Ubacena kartica**: novi disk se prepoznaje katalogom kamera (`qnc-camera-detector`). Kartica
   kamere koju QNC zna pokrece wizard s tom karticom. Nepoznata kartica ili obican disk: samo
   obavijest, nista se ne pokrece. Disk spojen prije pokretanja programa ne pokrece nista.
3. **Uvoz s kartice** (Ingest forma, promijenjen redoslijed): pregled kartice samo za citanje, bez
   baze i bez probea (klipovi, trajanja i posteri iz zapisa kartice). Korisnik bira kadrove u mrezi
   kartica kao u Ingestu.
4. **Novi projekt** kao prozor preko forme: obrazac priloga (`qnc-newsroom-sheet`), ime predlozeno,
   AI template zadan ("Vijesti sve").
5. **Kreiraj**: projekt postaje aktivan, Select kartice zapisuje izabrane klipove, Uvezi ih kopira
   ili poveze po postavkama projekta, pozadina (zapisi, valovi, filmstrip, AI kadrova, prijepis
   klipova s govorom) krece sama.
6. **Izjave**: wizard sam prelazi na njih kad su prijepisi gotovi i kaze sto jos ceka.
7. **Pisanje** OFF-ova i opaski "kadrovi koje zelim" (MA Script).
8. **Snimanje** (Snimi prilog, STT provjera citanja), **rez OFF-ova** po izgovorenim rijecima.
9. **Prica** iz scenarija, **pokrivalice** po ponudi (korisnik postavlja markerima ili Sync/B-rollom).
10. **Pregled** (player, HDMI, NORM, AI ton), **potvrda i export**.

Kartica ubacena dok je prilog otvoren: wizard pita "dodaj u ovaj prilog" ili "novi prilog".

## 2. Arhitektura

- QNC-AI je samostalan: kod kocki QNC-a koje treba kopiran je u `crates/` ovog repoa (zapis u
  `AGENTS.md`); u radu ne koristi nista iz QNC-a (svoja `data/`, `catalogs/`, `contracts/`).
- Kod je odvojen od GUI-ja: kocke bez sucelja rade, forma samo crta i salje radnje.
- Stanje wizarda je u bazi projekta QNC-AI-a (`qnc-wizard-db`, tablica `wizard_steps`); wizard
  nastavlja gdje je stao. Prije projekta ono sto je procitano s kartice je privremeni JSON
  (`qnc-card-read`, `data/cards/<serijski>.json`).
- Tray: neutralna kocka `qnc-tray` (ikona, izbornik, obavijest) s adapterom po OS-u; prvi je Windows.
- Prepoznavanje kartice: kocka `qnc-card-watch` (novi disk, katalog kamera), bez stanja izvan memorije
  procesa i bez pisanja.
- Tijek price: kocka `qnc-wizard-flow` (na kojem je koraku, bez crtanja); pita je tray program.
  Forme koraka su forme QNC-a ili njihove izmjene ovdje; tray ih otvara, wizard ih ne crta.
- Projekt koji wizard napravi je obican QNC projekt.

## 3. Plan izvedbe

**W0** (gotovo): kostur, kocke wizarda, stanje u bazi projekta. Izbornik koraka izbacen u W1.

**W1. Tray i kartica** (gotovo): `qnc-tray` (Windows), `qnc-card-watch`, `qnc-wizard-flow`, program
`qnc-ai-tray` (jedan primjerak, prepoznavanje kartice, obavijest, priča u izborniku ikone). Prozor
wizarda iz W0 izbacen.

**W2. Pregled kartice i izbor kadrova**: tray kad prepozna karticu procita njezin sadrzaj u
privremeni JSON (gotovo, `qnc-card-read`); modificirana Ingest forma pokazuje te klipove i bira
kadrove; tray je otvara kad kartica stigne i na "Nastavi".

**W3. Projekt i uvoz**: obrazac priloga preko forme, Kreiraj pokrece Select izabranih i Uvezi.

**W4. Izjave**, **W5. Pisanje, snimanje, rez OFF-ova**, **W6. Prica i pokrivalice**,
**W7. Pregled i export**.

**W8. Samostalnost i paket**: vlastita mapa wizarda (ugovori, katalozi, postavke), wav2vec2 u AI
paketu, samostalni lokalni AI server, automatsko pokretanja tray programa s prijavom (uz dozvolu
korisnika), instalacija na cisto racunalo, cijeli put na stvarnoj kartici.

## 4. Rizici

- Prepoznavanje novog diska ide provjerom svake dvije sekunde, ne obavijescu sustava; kasnjenje do 2 s.
- Kartica koja se budi sporo: katalog je pregledava tek kad je disk citljiv. Opazeno 2026-11-11:
  citac FX6 kartice (D:) postane citljiv oko 9 s nakon pokretanja traya, pa ga tray vidi kao tek
  ubacenu karticu.
- Preuzete kocke se odvajaju od QNC-a: popravke treba svjesno prenositi.
- Licenca wav2vec2: odluka korisnika.
