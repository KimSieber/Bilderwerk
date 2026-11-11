# Bilderwerk View

Suche über den KI-Bildkatalog (SQLite, erzeugt vom Projekt „Bildkatalog“).
Tauri 2 + React + Rust. Die Datenbank wird **nur lesend** geöffnet.
Bundle-ID: `de.sieber-bw.bilderwerk.view`.

## Entwicklung

```bash
cd apps/view
npm install
npm run tauri dev       # App im Entwicklungsmodus
npm run bauen           # Installationsdatei, siehe „Installationsdatei bauen“
npm run installieren    # bauen und nach /Programme kopieren (ersetzt vorhandene Fassung)
```

Teil von Bilderwerk (siehe README im Repo-Stamm).
Voraussetzungen: Node.js, Rust (rustup), Xcode Command Line Tools.
Vorschaubilder nutzen nur macOS-Bordmittel (`sips`, PDFKit) – keine zusätzlichen Werkzeuge nötig.

### Ordner in Dropbox

Lokale Werte: `view.local.example.json` nach `view.local.json` kopieren und anpassen (nicht im Repo):
`standardDb` (DB-Vorschlag), `dropboxAppKey` (Dropbox-App), `anbieter` (Speicheranbieter je Pfad). Nach Änderungen neu bauen.

`node_modules/` und `src-tauri/target/` von der Dropbox-Synchronisierung ausschließen:

```bash
xattr -w com.dropbox.ignored 1 apps/view/node_modules
xattr -w com.dropbox.ignored 1 apps/view/src-tauri/target
```

## Aufbau

| Pfad | Inhalt |
|---|---|
| `src-tauri/src/search.rs` | Suchkern: Filter → SQL (FTS5-Trigram + Bedingungen) |
| `src-tauri/src/facetten.rs` | Facetten mit Anzahl (Motiv, Bildart, Farbgebung, Ausrichtung, Jahreszeit, Wetter, Merkmale, Schlagworte) |
| `src-tauri/src/db.rs` | DB öffnen (`mode=ro&immutable=1`), DB-Info |
| `src-tauri/src/settings.rs` | Einstellungen pro Rechner (DB-Pfad) |
| `src-tauri/src/thumbs.rs` | Vorschaubilder (sips, PDF über `pdf.rs`/PDFKit), Nur-online-Erkennung |
| `src-tauri/src/speicher/` | Speicheranbieter `lokal` · `file-provider` · `dropbox-klassisch` (Dropbox-API), Zuordnung nach Pfad |
| `src-tauri/src/vorschau_db.rs` | Vorschauen aus der Katalog-DB (Tabelle `vorschauen`, falls vorhanden) |
| `src-tauri/src/cache.rs` | Vorschau-Cache: Größe, Leeren, Obergrenze (Standard 2 GB) |
| `src-tauri/src/details.rs` · `gross.rs` · `aktionen.rs` · `teilen.rs` | Detailbereich (alle DB-Angaben), große Fassung für die Großansicht, Öffnen/Finder/Sichern, macOS-Teilen |
| `src-tauri/src/beobachter.rs` | erkennt, wenn Dropbox die DB-Sicherung ersetzt → neu laden |
| `src-tauri/src/lib.rs` | Tauri-Befehle (`start`, `db_setzen`, `db_neu_laden`, `suchen`, `facetten`, `vorschau`, `details`, `vorschau_gross`, Aktionen, Dropbox, Cache), App-Menü „Katalog“ |
| `docs/Technologie- und Architekturkonzept.md` | Technologie- und Architekturkonzept |
| `docs/Testdokumentation.md` | Testfallkatalog, manuelle Prüfungen je Version |
| `src-tauri/tests/` | Rust-Integrationstests, Test-DB (`gemeinsam/`), Testbilder (`daten/`) |
| `tests/vertrag/` | JSON-Antworten aus Rust, die die Frontend-Tests als Mock nutzen |
| `src/` | React-Oberfläche |

Einstellungen: `~/Library/Application Support/de.sieber-bw.bilderwerk.view/einstellungen.json`

## Installationsdatei bauen

```bash
cd apps/view
npm test          # vorher: alles grün
npm run bauen     # = tauri build → .app und .dmg
```

Ergebnis:
- `apps/view/src-tauri/target/release/bundle/macos/Bilderwerk View.app`
- `apps/view/src-tauri/target/release/bundle/dmg/Bilderwerk View_<Version>_aarch64.dmg`

Hinweise:
- **Werte aus `view.local.json` werden eingebaut** (Vorschlag für die DB, Speicheranbieter, Dropbox-App-Schlüssel).
  Die gewählte DB merkt sich jeder Mac selbst (`einstellungen.json`). Die Pfade in `anbieter` müssen auf dem Ziel-Mac
  stimmen. Bei gleichem Benutzer und gleichem Dropbox-Ort ist das der Fall.
- **Signatur:** festes „Apple Development“-Zertifikat (`bundle.macOS.signingIdentity`), keine Notarisierung.
  Auf den eigenen Macs reicht das. Blockiert macOS den ersten Start, im Finder Rechtsklick → „Öffnen“.
- Das `.dmg`-Fenster gestaltet `tauri build` per AppleScript: Beim ersten Bau fragt macOS evtl., ob das Terminal
  den Finder steuern darf → erlauben.
- Bau nur auf Apple Silicon getestet (MacBook Air M3, Mac mini M1).
- Vor einer Weitergabe an andere (zurückgestellt): Dropbox-App „Produktion beantragen“, Notarisierung.

## Tests

```bash
cd apps/view
npm test               # alles: cargo test → vitest run → tsc, am Ende Testsumme
npm run test:rust      # nur Rust
npm run test:frontend  # nur Frontend
npm run test:watch     # Frontend beim Speichern
```

## Suche

- **Freitext:** alle Wörter müssen vorkommen (Dateiname, Ordner, KI-Beschreibung, Schlagworte, Metadaten).
  `"rotes auto"` sucht die Phrase, `-winter` schließt aus. Wortteile werden gefunden (`boot` → `Boote`).
- **Kriterien:** Dateiname, Ordner, Datum von/bis (`2015`, `2015-06`, `2015-06-09`), Dateityp, Duplikate.
- **Facetten** (unter „Kriterien“): Motiv, Bildart, Farbgebung, Ausrichtung, Jahreszeit, Wetter, Merkmale,
  Schlagworte – je Wert mit Anzahl. Innerhalb einer Facette ODER, zwischen Facetten UND. Facetten funktionieren
  auch ohne Suchtext (Stöbern im ganzen Katalog).
- Bilder mit unplausiblem Datum (z. B. `2056-55-10`) oder ohne Datum stehen am Ende.

## Stufen

| Stufe | Version | Inhalt | Status |
|---|---|---|---|
| 1 | v0.0.1 | Grundgerüst, Einstellungen, Suche mit Freitext und Basisfiltern, leere Startfläche | ✅ |
| 2 | v0.0.2 | Vorschau-Cache, Raster und Liste (virtualisiert), Kachelgröße, Suche leeren | ✅ |
| 2b | v0.0.3 | Testinfrastruktur, Anbieter-Schnittstelle (lokal · File Provider · Dropbox-API), Vorschauen aus der Katalog-DB, PDFKit, Cache-Grenze und „Cache leeren“ | ✅ |
| – | – | Repo-Umbau: App nach `apps/view/`, Work aufgenommen, feste Code-Signatur (keine eigene Version, daher kein v0.0.4) | ✅ |
| 3 | v0.0.5 | Facetten mit Anzahl (Motiv, Bildart, Farbgebung, Ausrichtung, Jahreszeit, Wetter, Merkmale, Schlagworte) als Chips · ganzes Bild in Kachel und Liste · HEIC über die Dropbox-Vorschau-API | ✅ |
| 4 | v0.0.6 | Detailbereich rechts (640-px-Vorschau, alle Angaben aus der DB, Pfeiltasten) · Großansicht über das ganze Fenster (2048 px auf Wunsch), Breite per Schieber · Öffnen, Im Finder, Sichern unter …, Teilen | ✅ |
| 5 | v0.0.7 | DB neu laden, wenn Dropbox die Sicherung ersetzt (automatisch, ⌘R, DB-Menü) · Tastenkürzel (⌘O, ⌥⌘R, ⇧⌘S, Esc) · Programmname „Bilderwerk View“ · Bauanleitung | ⏳ Prüfung |
| 6 | | Feinschliff, Build | |
| 7 | | Duplikat-Bereinigung (Duplikat ↔ Original, Datei + DB-Eintrag entfernen) | |
