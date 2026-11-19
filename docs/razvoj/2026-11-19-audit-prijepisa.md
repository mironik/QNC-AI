# Audit prijepisa i pozadine (2026-11-19)

Korisnik: "ajde napravi audit.. potpuno si se pogubio". Stanje koda na `libav-decoder`
(QNC-AI 8aabbb9). Ništa nije mijenjano u sklopu audita.

## 1. Tko danas piše prijepis (6 putova)

| # | Put | Proces | Motor | Što prepisuje | Kako sprema |
|---|-----|--------|-------|---------------|-------------|
| 1 | Izjave uz val (`Statements`, danas) | radnik | Parakeet | kanal 1 iz memorije vala; probe 15 s, ≥30 % riječi = izjava, onda cijeli klip | prijepis klipa (grubi) + red: Done ili Failed "nema izjave" |
| 2 | Red prijepisa radnika (`queue_imported` + `QueueRunner`) | radnik | Parakeet | klipovi o kojima val nije odlučio, tek kad valovi završe; čita disk | dijelovi u tijeku + prijepis raspona |
| 3 | Red prijepisa forme (`QueueRunner` u `qnc-transcript`) | forma | Parakeet (Transcribe, Fast) ili wav2vec2 (Ponovo HR, Best) | ono što korisnik zatraži | isto kao 2 |
| 4 | Fini prolaz (`fine_pass`, vraćen danas) | forma, kroz isti red | wav2vec2 | svaki dio koji je čuo samo grubi motor, otvoreni klip prvi | prijepis raspona (fini, sada ispravljen u Parakeetov tekst) |
| 5 | Prijepis uživo (`qnc-stream-transcript`) | forma | Parakeet (`live`) | komadi od 5 s ispred playheada, samo gdje nema prijepisa | svaki komad kao grubi prijepis |
| 6 | Riječi zooma (`qnc-zoom-words`) | forma | Parakeet | nekoliko sekundi oko reza | grubi prijepis |

Plus izvan prijepisa klipova: snimke OFF-a (`qnc-take-heard`, `qnc-session-readings`, wav2vec2).

## 2. Nađene greške

**A. Red prijepisa ima jedan redak po klipu** (`transcript_queue.clip_id PRIMARY KEY`, upis
`ON CONFLICT(clip_id)`), a radnik od 17. 11. dugi klip stavlja u red u dijelovima od 60 s. Svaki
dio prepiše prethodni, ostaje samo zadnji: **dugi klip kroz red dobije prijepis samo zadnje
minute.** Isto tako zahtjev wav2vec2 (Best) za klip prepiše njegov redak Parakeeta (Fast) i
obrnuto, i oznaku "nema izjave".

**B. Jedan red, dva vlasnika.** Red vrti onaj proces koji uzme najam `transcribe_lease` (radnik
ili forma). Radnikov red tako može vrtjeti wav2vec2 zahtjeve forme i obrnuto; kad forma ugasi,
njezini zahtjevi čekaju radnika. Stanje "prijepis traje" za dirigenta i AI kadrove čita se iz
najma, a najam drži i fini prolaz.

**C. Fini prolaz i prijepis uživo vrte se jedan oko drugog.** Prijepis uživo upiše grubi komad od
5 s → fini prolaz ga vidi kao "čuo samo grubi motor" → wav2vec2 ga prepisuje (zadržava klip) →
i tako za svaki komad koji korisnik pusti. Fini prolaz uz to troši procesor koji treba AI kadrovima.

**D. Panel prijepisa pokazuje i retke "nema izjave".** Red u panelu pokazuje sve ne-Best retke,
pa ~70 % klipova kartice stoji kao "neuspjelo: Na prvom kanalu nema izjave".

**E. Previše redoslijeda.** Kopiranje i valovi: najnoviji dan, najdulji klip. Izjave uz val:
otvoreni klip, inače redom valova. Red prijepisa: kliknuti klipovi, inače najkraći. Fini prolaz:
otvoreni klip. AI kadrovi: otvoreni klip, inače redom kataloga. Pet pravila za istu stvar.

**F. Prag izjave (30 % riječi u najglasnijih 15 s) nije provjeren.** Klip od 201 s proglašen je
"nije izjava" (0,04); klip od 6 s odlučen iz samih 6 s.

**G. Ostaci današnjih pokušaja.** `qnc-transcribe-nemotron` i model Nemotron (katalog motora,
`pack.json`) ne koriste se. sherpa-onnx je zamijenjen s 2.0.0 na 2.0.2 (Parakeet radi).
Klasični QNC se ne kompajlira (polja `fine`, `placed` u zajedničkim kockama).

**H. Zadržavanje klipa (hold)** radi samo dok je klip "tražen" (prikazan); fini prolaz za
neprikazani klip upisuje odmah, a za prikazani se vidi tek kad se klip ponovno otvori.

## 3. Prijedlog (čeka odluku)

1. **Jedan vlasnik prijepisa klipova: radnik pozadine.** Redoslijed jedan: otvoreni klip, pa
   najnoviji dan, pa najdulji klip (isti kao kopiranje i val).
   - izjave uz val (put 1) ostaju glavni put;
   - red radnika (put 2) samo za klipove koje val nije pročitao;
   - fini prolaz (put 4) premjestiti u radnika, **tek kad sve ostalo završi** (AI kadrovi,
     filmstrip), i samo za izjave; nikad za komade prijepisa uživo.
2. **Forma ne vrti red.** Forma samo pokazuje bazu; prijepis uživo (put 5) i riječi zooma (put 6)
   ostaju jer trebaju odmah. Dugmad Transcribe / Ponovo (HR) samo upišu zahtjev u bazu.
3. **Red prijepisa: jedan redak po dijelu** (ključ klip + raspon + motor), ili dugi klip u redu
   kao jedan zapis koji red sam dijeli — popravlja grešku A.
4. **Panel prijepisa ne pokazuje retke "nema izjave"** (oni su odluka, ne greška).
5. Ukloniti Nemotron (program, motor u katalogu, `pack.json`) ili ga ostaviti za kasnije.
6. Prag izjave provjeriti na kartici prije nego postane konačan.
