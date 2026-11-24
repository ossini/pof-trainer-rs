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
- **Liste** mit Nummernleiste, Sprung zu jeder Frage.
- **Pfeiltasten** ← / → am Desktop: vorherige / nächste Frage in Liste, Fehler, Markiert,
  Ultra schwer und Test; → drückt in den Blöcken „Weiter“ bzw. „Nächster Block“.
- **Fehler**, **Markiert**, **Markiert lernen** (in Runden), **Ultra schwer**.
- **Test** (Prüfung simulieren): 12 oder 16 zufällige Fragen, ohne Zeitlimit, bestanden ab
  75 % (9/12 bzw. 12/16). Auflösung erst nach dem Abgeben, Antworten bis dahin änderbar.
  Danach Auswertung pro Frage mit KI-Erklärung, „Falsche markieren“ und Verlauf der letzten
  Tests. Ein laufender Test bleibt beim Schließen der App erhalten. Falsche/unbeantwortete
  Fragen zählen als Fehlversuch (für Überraschungsfragen), der Richtig/Falsch-Stand der
  anderen Modi bleibt unberührt.
- **Suche** (zum Nachschlagen): Suchfeld durchsucht Fragetext, Antworten und CAA-Nummer,
  Groß-/Kleinschreibung egal. Treffer erscheinen sofort mit allen Antworten, die richtige
  grün, der Suchbegriff markiert. Ändert den Lernstand nicht.
- Ähnliche Fragen: Unterschiede markiert, in Tests ausgeblendet.
- KI-Erklärung, Merkhilfe und Verwechslungsgefahr nach der Antwort über das Glühbirnen-Icon oben an der Frage (öffnet ein Fenster, kein Scrollen).
- **Stand übertragen** zwischen Geräten, ohne Server: Teilen (Link + Code), Code kopieren
  und auf dem anderen Gerät einfügen, oder als JSON-Datei (gleiches Format wie die Web-App).
  Vor dem Übernehmen zeigt die App beide Stände nebeneinander.
- App-Log zum Exportieren, Zurücksetzen, dunkles und helles Design.

### Stand zwischen Geräten übertragen

Startseite → **Stand übertragen**:

- **Senden:** „Teilen …“ (z. B. per AirDrop/Nachricht an dich selbst) oder „Kopieren“.
  Der Code beginnt mit `POF1-` und ist ca. 4.000 Zeichen lang (komprimiert).
- **Empfangen:** Link öffnen *oder* „Einfügen aus Zwischenablage“ / Code ins Feld /
  Datei wählen. Danach „Übernehmen“. Der Stand auf dem Gerät wird ersetzt.
- iPhone: Safari und die App auf dem Home-Bildschirm haben getrennten Speicher. Für die
  Home-Bildschirm-App den Code **in der App** einfügen, nicht den Link in Safari öffnen.

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

**GitHub Pages (automatisch):** Jeder Push auf `main` baut die Web-App und veröffentlicht
sie unter `https://ossini.github.io/pof-trainer-rs/` (`.github/workflows/pages.yml`).
Einmalig in den Repo-Einstellungen unter *Settings → Pages → Source* **GitHub Actions** wählen.

Liegt die Seite anderswo in einem Unterordner, vorher in `Dioxus.toml` unter `[web.app]`
`base_path = "<ordner>"` eintragen (der Workflow macht das für Pages selbst).

### Offline (Flugmodus)

- **Native App:** Alles ist in der App enthalten, sie läuft immer offline.
- **Web-App:** Beim ersten Öffnen mit Internet lädt der Service Worker *alle* Dateien
  (Liste beim Build erzeugt) auf das iPhone. Danach kommt alles aus dem Cache, auch im
  Flugmodus. Ein WLAN mit Login-Seite (Flugzeug, Hotel) kann die App nicht überschreiben.
  Unter **Einstellungen → Offline** steht, ob alles da ist („Offline bereit“).
  Am besten vor dem Flug einmal mit Internet öffnen: So kommen auch Updates aufs Gerät.
- **Updates:** Eine neue Version wird in den ersten Sekunden nach dem Start automatisch
  übernommen. Später erscheint oben „Neue Version – tippen zum Laden“.

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
| `src/exam.rs` | Test-Modus: Auswahl, Prüfung, Auswertung |
| `src/transfer.rs` | Stand übertragen: Code, Link, Datei, Bestätigung |
| `src/util.rs` | Zeit, Zufall, Speicher (localStorage / Datei), App-Log |
| `assets/style.css` | Design |
| `data/` | PoF-Fragen + Startstand (aus der Web-App erzeugt) |
| `pwa/` | Icons, Manifest, Service Worker für die Web-Variante |
