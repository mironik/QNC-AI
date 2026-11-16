# Audit tray-ingesta i prijedlog (2026-11-16)

Pitanje korisnika: je li smjer dobar; po čemu se postojeće rješenje razlikuje od profesionalnih
ingest rješenja; što preurediti da ingest sa svim analizama radi jednako dobro na laptopu i kao
serverski ingest u TV kući.

## 1. Kako radi danas

1. Tray vidi karticu i otvara zaseban prozor uvoza (`qnc-ai-card-ingest`) po kartici.
2. Za svaku karticu napravi se **baza**. Tehnički je to **QNC projekt**: redak u registru projekata,
   vlastita mapa i `project.db`, ime s prefiksom "Ingest". Baza se u registru prebacuje u aktivni
   projekt kad se otvori.
3. Prozor uvoza pokreće QNC Select (čita XML kamere, probe samo kad treba), a zatim pozadinski
   proces (`qnc-ingest-worker`). Proces redom radi:
   - uvoz (kopija u `original/` pod imenom `clip-<id>_<ime>`);
   - dovršetak zapisa;
   - postere;
   - filmstrip i val;
   - analizu kadrova, sadržaj kadra (SigLIP) i opise (Florence-2);
   - prijepis;
   - opis baze.
4. Projekt priče nastaje kasnije, na Uvezi u Ingestu. Od 15. 11. projekt QNC Selectom pročita
   karticu baze (`QNCCARD.JSON`), a medij povezuje (Link).
5. AI modeli mogu raditi na AI serveru (TLS); baze i projekti su na disku računala.

Veličina: tray, prozor uvoza, `qnc-card-ingest`, `qnc-ingest-base` i radnik imaju oko 3.700 redaka.
Na to se oslanja velik dio QNC-shared kocki.

## 2. Što je dobro i treba ostati

- **Čitanje kartice po kameri:** katalog kamera, profili, XML umjesto probea. To je razina
  profesionalnih alata (Sony Catalyst, XDCAM Browser) i bolja od generičkih.
- **Jedan probe po mediju**, zapis klipa konačan i provjeren (fps, trajanje, TC).
- **Obrada kao lanac samostalnih poslova** (filmstrip, val, kadrovi, sadržaj, opisi, prijepis) koji
  popuštaju playeru i mogu se zaustaviti po funkciji.
- **AI izdvojen** u programe dodatke s ugovorom i AI serverom (TLS, token): to je već "servisna"
  arhitektura.
- **Baza kao arhiva**: naziv, mjesto, opis, analiza. Pretraga po svemu tome.

## 3. Po čemu se razlikuje od profesionalnih ingest rješenja

Profesionalni ingest u TV kućama (MAM/ingest sustavi velikih proizvođača, ingest stanice za kartice,
DIT alati za offload) ima zajednički obrazac. Ovdje je ono u čemu se QNC-AI danas razlikuje:

| Profesionalno | QNC-AI danas | Posljedica |
|---|---|---|
| **Provjerena kopija (offload):** svaka kopija se provjerava kontrolnim zbrojem (xxHash ili MD5), uz izvješće (ASC MHL). Kartica se smije brisati tek nakon provjere. | Kopija bez kontrolnog zbroja. | Nema dokaza da je kopija ispravna; ne smije se reći "kartica je sigurna". |
| **Asset ≠ projekt.** Postoji jedna **biblioteka** (katalog) materijala. Klip je asset s trajnim id-em, a projekti ga referenciraju. | Svaka kartica je "projekt" u registru projekata. Baze i priče žive u istom registru, a postoji i "aktivni projekt". | Miješanje uloga je uzrok većine grešaka zadnjih dana: forme rade u bazi, Ingest mijenja aktivni projekt, pitanje koji je projekt "pravi". |
| **Ingest je servis s redom poslova (job queue):** prioriteti, ponovni pokušaj, nastavak nakon prekida. UI je samo klijent koji šalje posao i prati ga. | Logika je u prozoru (prozor po kartici) i u pozadinskom procesu po projektu. Stanje je raspršeno: najmovi, signali, privremeni JSON, prozori. | Na serveru nema prozora; na laptopu zatvoren prozor ili restart prekida kontrolu nad poslom. |
| **Isti servis na laptopu i serveru**, razlika je samo gdje radi. | Tray i prozor su desktop aplikacija; mrežno je riješen samo AI. Baze se preko LAN-a ne čitaju (`qnc-content-read` to odbija). | Serverski ingest danas nije moguć bez preslagivanja. |
| **Izvorni nazivi i identitet:** UMID, izvorno ime, kartica i kamera ostaju. | Datoteke u bazi su `clip-<id>_Mironik_3683.MXF`. | Klip u projektu ima "ružno" ime; izvan QNC-a nije jasno što je što. |
| **Proxy za pregledavanje** (house proxy) za brzo gledanje na mreži, original ostaje na pohrani. | Proxy samo ako ga kamera snimi; ako ne, "optimizirana kopija" se radi uz uvoz. | Na mreži se gleda original (težak). |
| **Obogaćivanje (AI) po assetu**, kao poseban posao koji se može ponoviti ili nadograditi novim modelom. | AI je po bazi (`ai.db` uz bazu), vezan uz projektni najam i proces. | Teško je ponovno analizirati arhivu ili analizirati više baza paralelno na serveru. |
| **Korisnici, prava, zapis tko je što radio.** | Jedan korisnik računala. | Za TV kuću nužno, ali može kasnije. |

## 4. Procjena

Smjer **nije pogrešan u sadržaju**: čitanje kartica, zapisi, analize i AI su vrijedni i iznad
prosjeka. Pogrešan je **organizacijski kalup**: kartica je "nategnuta" na QNC projekt, a obrada je
vezana uz prozore i aktivni projekt. Zato svaka nova želja (više projekata iz baze, linkanje,
otvaranje Ingesta bez zapisa, server) traži zaobilaznice. Bez promjene kalupa zakrpe će se
nastavljati.

## 5. Prijedlog: Ingest kao servis i biblioteka materijala

### 5.1 Tri dijela, isti kod na laptopu i serveru

1. **Biblioteka (Media Library)**: jedna baza materijala instalacije (laptop ili server), odvojena od
   projekata.
   - **Kartica (volume):** što je s kartice uvezeno, kamera, serijski broj, vrijeme, naziv, mjesto,
     opis i analiza. Ovo je današnja "baza", ali kao zapis, ne kao projekt.
   - **Asset (klip):** trajni id, izvorno ime, UMID, zapis klipa i mjesta na pohrani (original,
     proxy, posteri, filmstrip, val).
   - **Rezultati analiza po assetu:** kadrovi, sadržaj, opisi, prijepis, s verzijom modela.
   - Medij u biblioteci ima **raspored kartice** (npr. `Library/<kartica>/XDROOT/...`) s izvornim
     imenima, plus `QNCCARD.JSON`. Svaka kartica je tako i dalje samostalna, prenosiva i čitljiva
     QNC Selectom.
2. **Ingest servis (headless)**: program bez prozora s **redom poslova u biblioteci**:
   - offload s kontrolnim zbrojem i MHL izvješćem;
   - zapis klipa (Select, kamere, probe);
   - posteri, filmstrip, val, proxy (po postavci);
   - analize i prijepis;
   - opis kartice.

   Poslovi imaju stanje, prioritet, ponovni pokušaj i nastavak. Ono što danas radi
   `qnc-ingest-worker` postaje izvršitelj poslova. Na laptopu servis pokreće tray; na serveru radi
   kao Windows servis ili Linux daemon. Ugovor je isti kao AI server: JSON preko TLS-a s tokenom.
3. **Klijenti (UI)**: tray, prozor uvoza (samo praćenje poslova), Ingest forma, Projekti, pretraga.
   - Klijent servisu šalje **posao** ("uvezi karticu X", "analiziraj kartice Y"), čita napredak i
     pretražuje biblioteku.
   - Na laptopu servis je lokalan, a na serveru isti klijenti rade preko mreže.

### 5.2 Projekti

- Projekt priče je i dalje QNC projekt sa svojom bazom (`project.db`).
- U projekt ulaze **reference na assete** biblioteke: Select kartice iz biblioteke, kao QNC Link
  danas. Ništa se ne kopira, a sve što se radi u projektu ide u projekt.
- Ingest forma u projektu bira iz biblioteke: kartica, pretraga, AI pokrivalice preko svih kartica.
- Nema više "aktivne baze" ni prebacivanja aktivnog projekta radi uvoza.

### 5.3 Što ostaje, što se mijenja

| Ostaje (kocke) | Mijenja se |
|---|---|
| Katalog kamera, profili, Select, probe, zapisi klipa | "Baza = projekt" postaje "kartica u biblioteci". |
| Poster, filmstrip, val i AI radnici | Pokretanje kroz red poslova u biblioteci, ne kroz najam projekta. |
| AI programi dodaci, AI server, TLS | Isti obrazac servisa i za ingest. |
| `QNCCARD.JSON`, opis i mjesto kartice | Uz raspored kartice i izvorna imena; dodaje se MHL. |
| QNC projekti, Story, AI MA | Čitaju assete iz biblioteke (Link), ne bazu kartice kao projekt. |

### 5.4 Redoslijed (svaki korak upotrebljiv sam za sebe)

1. **Provjerena kopija:** kontrolni zbroj (xxHash64) i MHL; kopija u rasporedu kartice s izvornim
   imenima. Mala izmjena, a odmah daje sigurnost.
2. **Biblioteka:** jedna baza materijala instalacije (kartice, asseti, analize). Postojeće baze se
   ne prenose (razvoj, bez migracije).
3. **Red poslova i ingest servis:** `qnc-ingest-worker` postaje izvršitelj poslova iz biblioteke.
   Prozor uvoza samo prati. Nastavak nakon prekida i restarta.
4. **Klijent-servis ugovor** (JSON i TLS kao AI server). Na laptopu lokalno, na serveru isto preko
   mreže.
5. **Projekti iz biblioteke:** Ingest forma bira kartice i assete biblioteke; Select i Link kao danas.
6. **Proxy po postavci** (house proxy) za rad preko mreže.
7. Kasnije: korisnici i prava, zapis tko je što radio, arhivske razine pohrane.

## 6. Što treba odlučiti

1. **Biblioteka:** jedna po instalaciji (laptop: lokalna; server: jedna za kuću), ili jedna po
   disku ili mjestu pohrane?
2. **Medij u biblioteci:** uvijek kopija u rasporedu kartice (sigurno, arhiva), ili i "samo
   referenca" za breaking news?
3. **Kontrolni zbroj:** xxHash64 (brz, standard DIT alata) ili MD5 (sporiji, traže ga neke kuće)?
4. **House proxy:** raditi uvijek, po postavci, ili samo na serveru?
5. Može li se krenuti s **korakom 1 (provjerena kopija)**, dok se o koracima 2-4 odlučuje?
