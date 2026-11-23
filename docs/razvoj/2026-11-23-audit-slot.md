# Audit: slot u Storyju (QNC) i u složenoj izjavi (QNC-AI), 2026-11-23

Pregledan kod: QNC-shared `qnc-program-segments` (`nav.rs`, `slot_move.rs`, `slot_trim.rs`,
`commands.rs`), `qnc-program-view` (`story_zoom_clicks`), QNC-AI `qnc-subsegment-edit`
(`lib.rs`, `pointer.rs`, `view.rs`). Uživo nije provjereno.

## Što je slot

- **Story (QNC):** razmak između dva M markera na Wrap retku. Pokrivalica je u slotu.
- **Složena izjava (QNC-AI):** svaki dio izjave je slot (od 4. 11.: "sve funkcije slota iz
  pokrivalica").

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
