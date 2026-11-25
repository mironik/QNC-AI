# Cjeloviti audit Storyja (i S-Storyja), 2026-11-25

Korisnik: "nisi napravio dubinski audit Storija ... tek tada ćeš shvatiti markere"; "za pokrivalicu
postoji Overwrite". Izvor: v5 pravilo `QNC_v5/.cursor/rules/qnc-story-segment-timeline.mdc`,
QNC docs/93 i docs/94 (procesi i markeri iz v5 koda), QNC-shared `qnc-program-db`,
`qnc-program-segments`, `qnc-program-view`, `qnc-program-commands`, `qnc-program-playlist`,
`qnc-program-input`, `qnc-sync-cover`, `qnc-wrap-session`, `qnc-segment-panel`, raspored grupe `o`
(`contracts/ui/editorial.layout.json`), katalog tipki (QNC preset, `storyboard`), karta miša
(`segments`). Samo čitanje, kod nije mijenjan.

## 1. Što je Story

Story je **program priloga vijesti**: jedna kontinuirana os, sastavljena od segmenata Ton i Off, koja
se u Wrap pogledu prelama u retke kao tekst. Retci su samo prikaz dijela iste osi. Svrha: grafički
alat koji gradi zapis za export (XML/EDL, MXF, Express); istina je baza (`story_*`), player i export
čitaju isti zapis kroz ravnu playlistu.

## 2. Slojevi programa (ravna playlista)

| Sloj | Što je | Kako nastaje | Kako se mijenja |
|---|---|---|---|
| **Osnova V1/A1 = govor priče** | Ton: slika i zvuk izjave; Off: samo glas (slika crna dok je ne pokrije pokrivalica). Segmenti jedan za drugim, bez rupa; trajanje programa je njihov zbroj | Talking Head (Shift+T) / Voice over (Shift+V) iz IN/OUT izvora, na kraj programa; Send to Story iz S-Storyja | Edit (dvoklik, Ctrl+Shift+I/O: novi IN/OUT istog klipa), Replace (novi izvor, vrsta ostaje), Slip (Ctrl+Shift+S), pomak gore/dolje, Ukloni (iskljuci, `active = 0`, ripple, vraća se Vrati), Ctrl+Delete (zauvijek) |
| **Lead-in / lead-out** | crno/tišina (Off) ili slika klipa (Ton) prije prvog i iza zadnjeg segmenta, iz postavki projekta | automatski (`sync_leads`) | ne uređuje se; zalijepljen za prvi i zadnji segment |
| **Pokrivalice V2/A2 = slika preko govora** | kadar B-rolla u slotu; A1 teče ispod, pokrivalica donosi sliku i ambijent na A2; segment se ne mijenja | Cover slot (B, Shift+B, dugme B-roll): IN/OUT izvora u **prazni** slot; Sync/B-roll (Y): izvor svira uz govor, IN u letu, O stane, Enter zapiše | **Overwrite** (Shift+O): drugi izvor u slot koji već ima pokrivalicu; kraći izvor dijeli slot novim M-om; zvuk A2 (kanal, razina, J/L), krivulja razine |
| **M markeri = rezovi slike** | M je OUT kadra prije i IN kadra poslije; slot M–M je mjesto jednog kadra | M / srednja tipka (na playheadu), Shift+M (i nastavak), automatski na kraju kraće pokrivalice | Ctrl+M / desni klik: cijeli rez (roll pokrivalica); u zoomu samo slika ili samo ton A2 (J/L); Delete: briše M |
| **Spoj dva segmenta = rez govora** | OUT segmenta prije i IN segmenta poslije; **nije M** (v5: granica segmenata nije M marker) | nastaje sam između segmenata | roll (`RollJoin`), J/L slike (`SetJoinPicture`), J/L tona A1 (`SetJoinSound`), A2 Tona u Off (`SetJoinA2`) |

Rezovi slike (M) ne prate rezove govora (spojevi): jedan kadar može teći preko spoja dva Offa, a u
jedan Off mogu stati dva kadra.

## 3. Slot (M–M)

Slot je mjesto kadra u montaži pokrivalica. Izbor: klik na traku, Ctrl+S, prvi prazni (Alt+strelice,
Shift+S u v5). Pomak cijelog slota (Ctrl+S, strelice): oba M-a se miču, kadar putuje. Rub slota
(Ctrl+I/O): slot dulji ili kraći, ostali zadržavaju duljinu (do praznog slota). Pokrivalica u slotu:
Cover slot u prazni, Overwrite zamjena. **Delete na slotu danas briše njegovu pokrivalicu**; slot i
oba M-a ostaju. Brisanje M-a briše pokrivalice oba njegova slota.

## 4. Pogledi, zvuk, izvoz

- **Source / Wrap**: dva pogleda, dva playheada; Wrap svira program, Source klip iz poola. Zoom reza
  (slike, val, riječi), zoom slota (dvoklik, Ctrl+E), povijest zooma (Ctrl+H), povijest projekta
  (Ctrl+Shift+H).
- **Zvuk**: kanal, razina i iskljucenost A1 segmenta i A2 pokrivalice; J/L na M-u (A2) i na spoju
  (A1); krivulje razine (Ctrl+Shift+1/2); normalizacija EBU (N); pomoćnik tona AI (Shift+N,
  Ctrl+Shift+N zadrži).
- **Undo/Redo** iz dnevnika projekta; **undo izabranog** (Ctrl+Backspace).
- **Izvoz**: Export HI-res (XML, Express iz prerendera, Original i postavke iz originala), prerender
  u pozadini.
- **Pool**: All, SOT (izjave), B-roll (kadrovi pokrivalica), AI B-roll (prijedlozi po sadržaju),
  Segment (popis segmenata s Edit, Vrati, Ukloni, Zamijeni, gore/dolje).

## 5. Model (korisnik 2026-11-25, konačno)

- **A1 je kontinuirani govor priče i zaključan je**: grade ga segmenti (Off, Ton), ne pokriva se i
  ne reže.
- **Markeri su sloj pokrivalica**: na A1 se ne odnose. Segment čini Off, a na njega se postavljaju
  pokrivalice. M je OUT kadra prije i IN kadra poslije; slot je kadar.
- Oba sloja su spojena u **flat program nepromjenjive duljine** sa zaključanim A1: nijedna radnja
  markera ni slota ne mijenja duljinu programa.
- Posljedica: rez (C) u Storyju već postoji, to je M. Brisanje slota djeluje samo u sloju
  pokrivalica (OUT prije i IN poslije spoje se u jedan M), nikad ne reže A1. Pokrivalica se mijenja
  Overwriteom.
- Odbačeno: izrezivanje programa s pomakom (ripple) i spojevi segmenata kao M markeri.

## 6. S-Story

Ista baza i isti kod priče (verzija `story-draft-n.db`), od danas s lead-inom i lead-outom.
`clip_slots.rs` (odluka 1. 11.) stavlja M na svaki spoj klipova i briše svaki drugi M.
