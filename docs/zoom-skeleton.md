# Kostur zooma (dogovor s korisnikom, 2026-11-25)

Zoom je uvećani dio timelinea (pravilo 0.12): isti alati kao timeline, drukčiji samo prikaz. Prvo se
gradi kostur grafičkih dijelova, zatim ponašanje.

```
┌─[A] ZAGLAVLJE (hvat za pomicanje prozora) ················································· [✕]─┐
│ [B] PLAYERI                                                                                    │
│ ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐                                  │
│ │ lijevi (IN)     │  │ srednji         │  │ desni (OUT)     │                                  │
│ │                 │  │ player timelinea│  │                 │                                  │
│ └─────────────────┘  └─────────────────┘  └─────────────────┘                                  │
│  [C] trake slota u boji ispod playera, samo gdje postoji susjedni klip                         │
│  nazivi: IN - prva · Player · OUT - zadnja                                                     │
│ [D] TRANSPORT: Play/Pause · Odustani · Spremi · Zatvori (Esc)                                  │
│ [E] A1: oznaka | razina | val | slova                                                          │
│ [F] RIJEČI A1                                                                                  │
│ [G] A2: oznaka | razina | val                                                                  │
│     crveni rez i bijeli playhead prolaze kroz E–G                                              │
├─[H] STATUSNA TRAKA: poruka zooma (lijevo) ··········· Potraži: [riječ u izjavi] n od m (desno)─┤
└────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Dijelovi
- **[A] Zaglavlje**: gornja traka prozora; za nju se prozor hvata i pomiče. Gumb ✕ = Zatvori (Esc).
- **[B] Playeri**: tri prozora, ne slike.
  - Srednji je player cijelog timelinea: slika na mjestu playheada.
  - Odabir lijeve ili desne slike (IN ili OUT) ili trake slota: playhead skače na taj IN/OUT; srednji
    prozor zapamti tu sliku i drži staro mjesto (usporedba); odabrani prozor postaje player u kojem
    se strelicama ili kotačićem kontrolira položaj IN/OUT.
  - Spremi ili Enter zapisuje novi IN/OUT, otpušta Ctrl+ i srednji prozor se vraća na običan player
    cijelog timelinea.
- **[C] Trake slota**: traka u boji po duljini ispod playera, samo gdje na timelineu postoji susjedni
  klip (Story, S-Story, složena izjava; ne na samom klipu source timelinea).
  - Klik na **sredinu** trake: pomak cijelog slota (oba ruba, duljina ostaje).
  - Klik na **kraj** trake: produljenje ili skraćenje tim rubom.
  - Odabrani slot: player postaje prozor tog slota, srednji drži staro mjesto (kao IN/OUT).
- **Dvostruko stanje videa**: klik na sliku mijenja IN/OUT (rez slike); klik na traku slota mijenja
  slot.
- **[D] Transport**: Play/Pause (novo) · Odustani (Backspace) · Spremi (Enter) · Zatvori (Esc).
  - Space ili Play/Pause pušta od mjesta gdje playhead stoji (izmjena 2026-11-25); playhead unutar
    odabranog dijela klipa staje na njegovu OUT-u. Na početak dijela vodi samo klik na player u sredini.
- **[E] A1**, **[F] riječi A1** (odmah ispod A1), **[G] A2**: oznaka, razina, val, slova; crveni rez i
  bijeli playhead prolaze kroz sve tri.
- **[H] Statusna traka**: lijevo poruka zooma; desno tražilica teksta u izjavi kad je otvorena izjava
  s prijepisom (n od m).

## Pravila koja vrijede
- Svaka radnja ima prečac; gumb šalje istu radnju kao tipka (Spremi = Enter, Odustani = Backspace,
  Zatvori = Esc, Play/Pause = Space).
- Jedno uzeto u isto vrijeme.
- Vlasnik zooma (source timeline, Story, S-Story, složena izjava) daje samo os i ono što je uzeto;
  zoom crta i šalje radnje na panel vlasnika.
