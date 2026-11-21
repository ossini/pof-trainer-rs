# PoF Trainer (Rust)

PPL(A) **Principles of Flight**: 364 Fragen mit KI-Erklärungen, Merkhilfen und
Hervorhebung ähnlicher Fragen. Geschrieben in Rust mit [Dioxus 0.6](https://dioxuslabs.com).
Der Fortschritt bleibt **nur auf dem Gerät** (kein Server). Sichern und übertragen geht
per Export/Import in den Einstellungen.

## Funktionen (wie in der Web-App)

- **10er-Blöcke**: Fehler kommen ans Blockende, Gelerntes kommt im nächsten Block als Test.
  Nur ein bestandener Test zählt als richtig.
- **Überraschungsfragen**: Oft falsch beantwortete Fragen (lila) kommen mit wachsendem
  Abstand wieder (30 min → 2 h → 8 h → 1 Tag → 2 Tage …).
- **Liste** mit Nummernleiste, Sprung zu jeder Frage, Pfeiltasten am Desktop.
- **Fehler**, **Markiert**, **Markiert lernen** (in Runden), **Ultra schwer**.
- Ähnliche Fragen: Unterschiede markiert, in Tests ausgeblendet.
- Erklärung, Merkhilfe und Verwechslungsgefahr nach jeder Antwort.
- Export/Import im selben JSON-Format wie die Web-App, App-Log zum Exportieren,
  Zurücksetzen, Nachtflug- und Tag-Design.

Beim ersten Start wird der PoF-Stand aus `data/seed-progress.json` übernommen
(Schnappschuss von `pilot-prep-hub/src/data/current.json`). Aktualisieren:

```bash
python3 scripts/import-from-web.py ../pilot-prep-hub
```

Danach neu bauen. Auf dem Gerät gilt immer der dort gespeicherte Stand, der Seed
wird nur genutzt, wenn noch nichts gespeichert ist. Alternativ in der alten App
exportieren und hier importieren.

> ⚠️ Ein Export **aus dieser App** enthält nur Principles of Flight. Wenn du ihn in die
> alte Web-App importierst, ersetzt er dort *alle* Fächer.

## Auf dem Mac bauen

Einmalig:

```bash
xcode-select --install                     # bzw. Xcode aus dem App Store
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown aarch64-apple-ios aarch64-apple-ios-sim
cargo install dioxus-cli --version 0.6.3 --locked
```

Tests (Logik, Zeitstempel, Daten):

```bash
cargo test
```

### Variante A: iOS-App (nativ)

```bash
open -a Simulator
dx serve --platform ios                   # läuft im iPhone-Simulator
dx bundle --platform ios --release        # .app-Bundle unter target/dx/…
```

Auf einem echten iPhone muss die App mit deiner Apple-ID signiert werden
(Xcode → Settings → Accounts; mit kostenlosem Account läuft die Signatur nach 7 Tagen ab).
Der Fortschritt liegt dann in `Documents/pof-trainer-progress.json` im App-Container.

### Variante B: Web-App für den Home-Bildschirm (ohne Apple-Account)

```bash
./build-web.sh            # erzeugt dist/
./build-web.sh serve      # zum Ausprobieren auf http://localhost:8090
```

Den Ordner `dist/` auf einen beliebigen HTTPS-Host legen (z. B. GitHub Pages),
auf dem iPhone in Safari öffnen → Teilen → **Zum Home-Bildschirm**. Danach startet sie
wie eine App und läuft offline. Gespeichert wird in `localStorage` auf dem iPhone.

Liegt die Seite in einem Unterordner (z. B. `https://<name>.github.io/pof-trainer/`),
vorher in `Dioxus.toml` unter `[web.app]` `base_path = "pof-trainer"` eintragen.

### Desktop (zum schnellen Testen)

```bash
dx serve --platform desktop
```

## Aufbau

| Datei | Inhalt |
| --- | --- |
| `src/model.rs` | Fragen (eingebettet), Fortschritts-Format, Antwort-/Markier-Logik |
| `src/logic.rs` | Blöcke, Tests, Wiederholungen, Überraschungsfragen |
| `src/card.rs` | Fragekarte mit Antworten und Erklärung |
| `src/main.rs` | Startseite, Modi, Liste, Block-Ansicht, Einstellungen |
| `src/util.rs` | Zeit, Zufall, Speicher (localStorage / Datei), App-Log |
| `assets/style.css` | Design |
| `data/` | PoF-Fragen + Startstand (aus der Web-App erzeugt) |
| `pwa/` | Icons, Manifest, Service Worker für die Web-Variante |
