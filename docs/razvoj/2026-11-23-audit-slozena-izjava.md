# Audit: gumbi izjave i složena izjava u AI Media Assistu (2026-11-23)

Stanje nakon vraćanja dogovora od 19. 11. (QNC-AI 0ff547c i današnji commit, QNC-shared b43487c,
b0593ed, 5c2d6ea). Pregledan kod; uživo nije provjereno.

## Gumbi u zaglavlju source timelinea (grupa e)

Raspored: `QNC-shared/contracts/ui/ai-editorial.layout.json` (ugrađen u program).

| Gumb | Radnja | Aktivan kad | Što radi | Gdje piše |
|---|---|---|---|---|
| Dodaj izjavu | `save_virtual_shot` | klip izabran | hrvatski ponovni prijepis dijela, prozor za potpis, nova izjava (IN/OUT, TC, potpis), poster sa slike IN-a | `virtual_shots` (`qnc-short-save`) |
| Dodaj složenu izjavu | `statement_add_part` | izabrana izjava i prikazani IN/OUT koji nije njezin | hrvatski ponovni prijepis dijela, IN/OUT prikazanog klipa dodan kao sljedeći dio izabrane izjave; poster ostaje od prvog dijela | `virtual_shot_parts` (`qnc-short-save::add_part`) |
| Ažuriraj | `statement_update` | izabrana izjava, njezin klip prikazan, IN/OUT drukčiji od spremljenih | IN/OUT izjave postaju prikazani; slike IN/OUT se rade iznova | `virtual_shots` (`trim_short_now`) |
| Idi na Script (OFF) | `go_to_script` | uvijek | otvara MA Script na istom projektu; AI Media Assist ostaje | ništa |

Klik na izjavu: jedan dio otvara klip s IN/OUT; više dijelova otvara izjavu dio po dio na mjestu
source timelinea (`qnc-side-pieces` `statement.rs` → `qnc-subsegment-edit`), svira se prvi dio pa
odmah drugi, bez onoga između.

## Nalazi

1. **Dodaj složenu izjavu i Ažuriraj aktivni su u isto vrijeme.** Kad je izjava izabrana i IN/OUT se
   promijeni na njezinu klipu, oba gumba su aktivna: Dodaj složenu izjavu dodaje dio, Ažuriraj
   prepisuje izjavu tim IN/OUT. Nema oznake što će koji napraviti.
2. **Dio iz drugog klipa ne može se dodati.** Klik na drugi klip u poolu briše izbor izjave
   (`PreviewClip` → `chosen_shot_id = None`), pa je gumb siv. Dodaje se samo iz klipa izjave.
3. **Uređivanje dijelova se ne sprema.** Na timelineu izjave s više dijelova rade rubovi, pauza,
   spoj, J/L, Smooth (alat `qnc-subsegment-edit`), ali `statement.rs` njihove upise baca
   (`take_writes` se odbacuje). Nakon ponovnog otvaranja sve je kao prije.
4. **Dio se ne može ukloniti ni premjestiti, nema Undo.** `qnc-virtual-shots` ima samo `AddPart`.
5. **Ažuriraj na izjavi s više dijelova mijenja samo prvi dio** (njezin vlastiti IN/OUT).
6. **Dijelovi nisu označeni u prijepisu.** Označavanje dijelova u tekstu (`show_statement_parts`,
   19. 11.) nije vraćeno; klik na izjavu označava samo prvi dio.
7. **Dva modela složene izjave.** AI Media Assist: `virtual_shot_parts` (samo klip i IN/OUT).
   MA Script i MA Audio: blok scenarija `script_blocks.pieces` (IN/OUT, pauza, rez/pretapanje, J/L
   slike i tona, A1, krivulja, Smooth, i kopija teksta). Ista stvar je zapisana na dva načina.
8. **Izjava s više dijelova ne ide u priču.** MA Script je vidi u tabu Izjave i svira je dio po dio
   (isti kod), ali + IZJAVA i Send to S-Story uzimaju blok scenarija, ne dijelove izjave.
9. **Idi na Script otvara novi prozor svaki put**, i kad je MA Script već otvoren.

## Smooth (kako je radio)

- Postoji samo za složenu izjavu scenarija (MA Script, MA Audio). Uz pauzu između dva dijela
  gumb Smooth (`subsegment_join_smooth`) uključuje spoj; zadano 8 slika, -/+ po 2 (2–24).
- Zapis: `script_blocks.pieces[n].smooth_frames` (traženo) i `smooth_clip` (napravljeni klip).
- Pozadina `qnc-smooth-joins`: kad player miruje, dvije slike iz originala oko reza, međuslike
  programom `rife-ncnn-vulkan` (AI katalog `smooth`, mogućnost projekta `smooth_cut`), kratki MXF u
  mapi glasa projekta, klip u katalogu.
- Program (`qnc-program-input`) međuslike stavlja preko slike oko reza kao pokrivalicu bez tona:
  pola prije, pola poslije reza; s pauzom i kroz pauzu (broj slika = zadane + pauza).
- U QNC-AI je radio (player log 17. 11.: `smooth-joins ... made`).
- **Za izjavu iz AI Media Assista Smootha nema:** `virtual_shot_parts` nema polja spoja, a upisi
  uređivanja se bacaju (nalaz 3).

## Prijedlog (čeka odluku)

Jedan zapis složene izjave: dijelovi izjave (`virtual_shot_parts`) dobivaju polja spoja koja ima
scenarij (pauza, rez/pretapanje, J/L, A1, Smooth); `statement.rs` upise `qnc-subsegment-edit`
piše u izjavu umjesto da ih baca; MA Script i priča uzimaju izjavu s dijelovima. Tada Smooth,
uklanjanje i premještanje dijela rade jednako u AI Media Assistu i MA Scriptu.
