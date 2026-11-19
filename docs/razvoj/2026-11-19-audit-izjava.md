# Audit izjava i složenih izjava u AI Media Assistu (2026-11-19)

Stanje grane `libav-decoder` nakon commita od 22:40. Pregledano: kod (QNC-AI, QNC-shared), baza
aktivnog projekta (samo čitanje) i player log.

## Put svake radnje danas

| Radnja | Kamo ide | Gdje se sprema |
|---|---|---|
| Dodaj izjavu | `qnc-panel-focus` (hrvatski ponovni prijepis) -> `qnc-editorial-application` -> `qnc-short-save` (pitanje za potpis crta **source timeline**) | `virtual_shots` (IN/OUT, ime, funkcija), poster sa slike na IN-u kao datoteka u projektu |
| Složena izjava | `qnc-panel-focus` (ponovni prijepis) -> `qnc-script` (pitanje za potpis crta **domaćin forme**) | blok scenarija `script_blocks` (dijelovi: IN/OUT **i kopija teksta**, polja potpisa) |
| Klik na izjavu | `PreviewShort` -> klip s IN/OUT | — |
| Klik na složenu izjavu | `qnc-script` `script_show_compound` -> klip prvog dijela | — |
| X na izjavi | `qnc-virtual-shots` brisanje | — |
| X na složenoj izjavi | `qnc-script` uklanjanje bloka | — |
| Poster izjave | slika s IN-a, datoteka u projektu (`qnc-shot-stills`) | |
| Poster složene izjave | slika s IN-a prvog dijela, samo u memoriji (`qnc-compound-posters`) | |

## Nalazi

1. **Složena izjava se nastavlja zauvijek (greška).** `qnc-script` dodaje novi dio u „izabrani
   blok”. U AI Media Assistu taj izbor se ne vidi i ne može se maknuti, pa nakon prve složene
   izjave svaki klik „Složena izjava” dodaje dio u nju, i za drugi klip i drugu osobu. Novi
   blok nastaje tek nakon ponovnog otvaranja forme.
2. **Dva puta za prozor s pitanjem.** Pitanje obične izjave crta source timeline, pitanje
   složene domaćin forme.
3. **Pitanje za potpis napisano dvaput** (`qnc-short-save` i `qnc-script`), svako sa svojim
   traženjem zadnjeg imena za klip.
4. **Potpis na dva mjesta:** obična izjava u `virtual_shots`, složena u polja bloka scenarija.
5. **Poster na dva načina:** obična izjava datoteka u projektu, složena dekodira se u memoriju
   pri svakom otvaranju forme.
6. **Složena izjava u bazi nosi i kopiju teksta** (`script_blocks.text`, `pieces_json` tekst
   dijela). Kopija se ne mijenja kad hrvatski prijepis ispravi riječi, pa scenarij i izvoz
   mogu pokazivati stari tekst. Prema pravilu korisnika dovoljni su IN/OUT, a tekst se čita iz
   izvornog prijepisa.
7. **Kopija rasporeda forme:** `QNC-AI/contracts/ui/editorial.layout.json` čita samo provjera
   pravila; Wizard gradi `QNC-shared/contracts/ui/ai-editorial.layout.json`. Danas je izmjena
   prve kopije izgledala kao da ne radi.
8. **Ponovni prijepis i na Odustani:** „Dodaj izjavu” i „Složena izjava” šalju dio na hrvatski
   prijepis prije pitanja, pa se prijepis radi i kad korisnik odustane.
9. **Složena izjava od više klipova:** klik označi samo dijelove prvog klipa.
10. **Scenarij u AI Media Assistu** radi u pozadini i pri prvom čitanju upisuje profil
    scenarija u projekt.
11. **Provjera pravila (conformance) u QNC-AI** ne prolazi već od ranije (13 grešaka, forme veće
    od dopuštenog); ne služi kao zaštita.

## Popravljeno danas (provjeriti uživo)

- Dijelovi složene izjave označeni su samo dok je ona izabrana (klip 3882).
- Izabrana složena izjava ima X i crveni okvir.
