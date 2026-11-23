# Audit: slot u Storyju (QNC) i u složenoj izjavi (QNC-AI), 2026-11-23

Pregledan kod: QNC-shared `qnc-program-segments` (`nav.rs`, `slot_move.rs`, `slot_trim.rs`,
`commands.rs`), `qnc-program-view` (`story_zoom_clicks`), QNC-AI `qnc-subsegment-edit`
(`lib.rs`, `pointer.rs`, `view.rs`). Uživo nije provjereno.

## Što je slot (korisnik 2026-11-23)

Slot je prostor između dva markera, bez obzira koji su to markeri (IN, OUT ili M). Svaki takav
dio je slot.

- **Izjava:** IN/OUT = jedan klip = jedan slot.
- **Složena izjava:** slot po dijelu (dva dijela = dva slota, više dijelova = više slotova).
- **Story:** slot između dva M markera na Wrap retku; pokrivalica je u slotu.

(Prva verzija ovog audita slot je vezala samo uz M markere Storyja; to je bilo krivo.)

## Tri radnje slota u Storyju

| Radnja | Što radi | Kod |
|---|---|---|
| Izbor slota | slot postaje izabran, ništa se ne miče | `take_slot` (SelectSlot) |
| Slot cijeli (Ctrl+S) | strelice miču slot s oba M markera, duljina i pokrivalica ostaju, susjedni slot raste ili se smanjuje; Enter zapisuje `MoveSlot` | `slot_move.rs` |
| Rub slota (Ctrl+I / Ctrl+O) | slot dulji ili kraći na jednom rubu, ostali zadržavaju duljinu, markeri iza se pomiču; Enter zapisuje `TrimSlot` | `slot_trim.rs` |

## Miš u Storyju

| Gesta | Timeline (Wrap redak) | Zoom |
|---|---|---|
| lijevi klik na traku | izbor slota | **slot cijeli** (`zoom_take_slot` = izbor + Ctrl+S) |
| desni klik na traku | slot cijeli (`select_current_marker_slot`, Ctrl+S) | slot cijeli (isto) |
| desni klik na sliku | najbliži M marker | najbliži M marker |

Zoom pomaknutog slota pokazuje četiri slike: zadnju prije slota, prvu i zadnju slota, prvu poslije.

## Miš u složenoj izjavi (sada)

| Gesta | Timeline | Zoom |
|---|---|---|
| lijevi klik na traku | izbor dijela (`subsegment_select`) | izbor dijela; danas dodano: nepomaknuti rez postaje vlastiti rub tog dijela |
| desni klik na traku | **rub dijela** bliži kliku (TAKE_EDGE `n:in/out:slot`), odluka korisnika 5. 11. | isto |
| Ctrl+S | dio cijeli (`slide_key`) | nema zooma za dio cijeli |
| desni klik na sliku | rez na spoju (roll) ili rub dijela | isto |

## Razlike (nalazi)

1. **Lijevi klik na traku u zoomu:** Story uzima slot cijeli, složena izjava samo bira dio.
2. **Desni klik na traku:** Story uzima slot cijeli (Ctrl+S), složena izjava uzima rub dijela.
   To je odluka korisnika od 5. 11. za složenu izjavu; Story je ostao drukčiji.
3. **Dio uzet cijeli (Ctrl+S) nema zoom** u složenoj izjavi; Story za pomaknuti slot ima zoom
   s četiri slike.
4. **Moje izmjene danas su odstupile od Storyja:** zoom jednog dijela (IN | Player | OUT) je
   uveden i uklonjen; lijevi klik na traku sada pretvara nepomaknuti rez u rub dijela, a u
   Storyju taj klik uzima slot cijeli.
5. **Player u sredini zooma** (danas) Story nema; ima ga samo zoom izjave (20.–22. 11.).

## Prijedlog (čeka odluku)

Složena izjava radi isto kao Story, jedna kocka pravila:
- lijevi klik na traku na timelineu bira dio; u zoomu uzima dio cijeli (kao `zoom_take_slot`);
- dio uzet cijeli ima zoom kao pomaknuti slot u Storyju (četiri slike, Player u sredini);
- desni klik na traku: ili kao Story (dio cijeli), ili ostaje rub dijela (odluka 5. 11.).
- Danas dodano "nepomaknuti rez postaje rub dijela" ukloniti.

---

# Audit u QNC-AI: slot složene izjave u AI Media Assistu (2026-11-23)

Put od klika do zapisa: `qnc-side-pieces/statement.rs` (`StatementPlay`) → `qnc-subsegment-edit`
(`pointer.rs` → `trim_key` / `slide_key` / `sound_key` / `join_key`) → upisi (`take_writes`).
Isti `qnc-subsegment-edit` koristi MA Script (`qnc-script`), koji upise piše u blok scenarija.

## Što slot radi danas (kod)

| Gesta | Što pošalje | Što napravi |
|---|---|---|
| lijevi klik na traku (timeline) | `subsegment_select` | izabere dio; playhead na IN dijela |
| lijevi klik na traku (zoom) | `subsegment_select` | izabere dio; nepomaknuti rez postaje vlastiti rub tog dijela (95184ee; isto pravilo bilo je 19. 11. u 66efaf1, vraćeno 20. 11.) |
| desni klik na traku | `subsegment_take_edge n:in/out:slot` | uzme bliži rub dijela (produljivanje / skraćivanje) |
| desni klik na sliku ili blizu spoja | `subsegment_take_edge n:cut` | uzme cijeli rez spoja (roll: OUT prije i IN poslije zajedno) |
| Ctrl+S | `slide_key` | dio cijeli se miče, susjedi rastu / smanjuju se; nema zooma |
| strelice / kotačić | korak uzetog | nacrt se odmah čuje |
| Enter / Spremi / klik na Player | `write_trim`, `slide` …, `write_join` | upis se preda vlasniku |

## Nalazi

1. **Ništa se ne sprema.** `statement.rs` baca sve upise (`let _ = self.edit.take_writes()`),
   jer su upisi operacije baze scenarija (`qnc_script_db::SetPiece`, `SetJoin`, `together`), a
   izjava je u `virtual_shot_parts`. Nakon Entera dio se vrati na spremljeni IN/OUT. To je
   glavni razlog zašto se slot u zoomu „ponaša krivo“.
2. **`qnc-virtual-shots` nema operaciju za izmjenu dijela**: postoji samo `AddPart` (i
   `TrimShort` za izjavu s jednim rasponom). Nema promjene IN/OUT dijela, spoja, pauze,
   brisanja ni redoslijeda.
3. **Rubovi 1/13 su pravi dijelovi za logiku slota.** `open` ih dodaje kao dijelove 0 i
   zadnji; panel im ne crta traku ni rubove, ali logika ih vidi. Zato je prvi pravi dio
   indeks 1, a Ctrl+S i roll mogu micati rub 1/13 kao susjeda.
4. **Vrsta spoja (Cut / Smooth / Blic) iz `virtual_shot_parts` ne dolazi do uređivanja**; svi
   spojevi se uređuju i sviraju kao rez.
5. **Dio uzet cijeli (Ctrl+S) nema zoom**; zoom postoji samo dok je uzet rub ili rez.
6. **Danas promijenjeno više puta** (93f93e3, d2178ca, a5fcb06, 95184ee): Player u sredini
   zooma rez-a ostaje, zoom jednog dijela uklonjen, lijevi klik na traku u zoomu vraćen na
   pravilo od 19. 11.

## Što treba da slot radi u AI Media Assistu

1. Izmjena dijela spremljena u izjavu: nove operacije `qnc-virtual-shots` (IN/OUT dijela,
   spoj, ukloni dio), a `statement.rs` upise prevodi u njih umjesto da ih baca.
2. Rubovi 1/13 nisu dijelovi za uređivanje (samo prikaz).
3. Pravila miša i zooma ostaju ista kao u MA Scriptu (ista kocka).
