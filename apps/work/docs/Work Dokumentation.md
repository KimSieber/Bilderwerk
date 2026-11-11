# Work (Bildkatalog) – Dokumentation

**Version:** A1.1 · **Stand:** November 2026
**Teil von:** Bilderwerk View (View) – Work erzeugt die Datenbank, die App (`../app`) durchsucht sie.

> Versionierung: **A**x.y = Work, **v**x.y.z = App. A1.0 ist im produktiven Einsatz; A1.1 = Aufräumen (a), Dokumentation (b), Erweiterungen (c).

---

## 1. Überblick

Der Work katalogisiert ein privates Bildarchiv **vollständig lokal**:

1. Bilder (und jede Seite digitalisierter Fotobücher als PDF) werden rekursiv eingesammelt.
2. Ein lokales KI-Modell in **LM Studio** beschreibt jedes Bild (Kurztitel, Beschreibung, Motiv, Anlass, Wetter, Schlagworte, Personenmerkmale …).
3. **Metadaten** (EXIF/IPTC/XMP), **Bildmaße** und **Farbgebung** (aus den Pixeln gemessen) kommen dazu.
4. Alles landet in einer **SQLite-Datenbank** mit Begriffstabelle und Volltextindex.

Die App Bilderwerk View liest diese Datenbank **nur lesend**. Das Schema ist der Vertrag zwischen beiden (`../shared/schema.sql`).

```
Bildordner ──► katalog.py / katalog_gui.py ──► Bildkatalog.db ──► Bilderwerk View (apps/view/)
                    │
                    ├─ bildanalyse.py ──► LM Studio (localhost:1234)
                    └─ metadaten.py   ──► ExifTool / Pillow
```

---

## 2. Ordner- und Dateistruktur

```
apps/work/
├── bildanalyse.py                  Kernmodul: Bildvorbereitung, PDF, Farbmessung, Modellaufruf, Nachbearbeitung
├── metadaten.py                    Metadaten per ExifTool (Rückfall Pillow), Datum aus Dateiname
├── katalog.py                      Batch-Lauf + Datenbank (Schema, Speichern, Suchindex, Sicherung) + CLI
├── katalog_gui.py                  Tkinter-Oberfläche für den Lauf
├── suche.py                        Suche/Statistik/Fehlerliste per Kommandozeile (Testwerkzeug)
├── Bilderwerk Work starten.command     Doppelklick-Start der Oberfläche (macOS)
├── requirements.txt                Python-Abhängigkeiten (Pillow, numpy) + Hinweise auf externe Tools
├── gui_einstellungen.example.json  Vorlage für die GUI-Einstellungen
├── gui_einstellungen.json          lokale GUI-Einstellungen            [gitignore]
├── README.md                       Kurzbeschreibung
├── tools/
│   ├── bild_test.py                Einzelbilder analysieren, ohne Datenbank
│   ├── farbe_neu_messen.py         Farbgebung im Bestand neu messen (ohne KI)
│   ├── vorschau_nachholen.py       Vorschaubilder für den Bestand nachholen (ohne KI)
│   └── db_struktur.py              Schema einer SQLite-Datei anzeigen
├── docs/
│   ├── Work Dokumentation.md   dieses Dokument
│   ├── Auftrag Bildkatalog – …md   Änderungsauftrag aus dem Gesamtprojekt (Vorschaubilder)
│   └── intern/                     interne Unterlagen                  [gitignore]
├── testdaten/                      Testbilder                          [gitignore]
│   └── bilder/
│       ├── archiv/                 gemischte Testbilder (20)
│       └── set1/                   kleines Set (3), u. a. Sepia
└── Katalog-DB/                     Standard-Datenbankordner            [gitignore]
    ├── Bildkatalog.db (+ -wal/-shm) Test-Datenbank
    ├── Bildkatalog_protokoll.txt   Laufprotokoll
    └── fehler_antworten/           Rohantworten des Modells bei Fehlern (zur Analyse)
```

**Nicht im Repo:** Datenbanken (`*.db`), Testdaten, `Katalog-DB/`, `gui_einstellungen.json`, `docs/intern/`.

> **Hinweis:** Der Ordnername `Katalog-DB` ist in `bildanalyse.py` fest verdrahtet (`KATALOG_ORDNER`) und muss neben den Programmen liegen. Er dient als Standard für DB, Protokoll und Fehlerablage. Die **Produktiv-DB** liegt außerhalb des Repos (Pfad steht in `gui_einstellungen.json`).

---

## 3. Die Dateien im Einzelnen

| Datei | Aufgabe | Wichtigste Funktionen |
|---|---|---|
| `bildanalyse.py` | Analyse **eines** Bildes/einer PDF-Seite. Definiert Modell, Prompt, JSON-Schema, Standardpfade. Keine DB-Logik. | `analysieren()`, `farbgebung_messen()`, `modell_anfragen()`, `nachbearbeiten()`, `quelle_aufloesen()` |
| `metadaten.py` | Metadaten lesen und normieren; Fehler führen nie zum Abbruch. | `lesen()`, `datum_aus_dateiname()` |
| `katalog.py` | Ordner durchlaufen, Duplikate erkennen, speichern, Suchindex, Sicherung, Protokoll. Klasse `Katalogisierer` wird auch von der GUI genutzt. | `Katalogisierer.ausfuehren()`, `db_oeffnen()`, `speichern()`, `suchindex_aktualisieren()` |
| `katalog_gui.py` | Bedienoberfläche: Start/Pause/Stopp, Fortschritt, Restzeit, Vorschau, Meldungen. Lauf in eigenem Thread. | `KatalogGUI`, `main()` |
| `suche.py` | Ergebnisse prüfen: Volltext + Filter, Detailanzeige, Statistik, Fehlerliste. | `suchen()`, `statistik()` |
| `tools/bild_test.py` | Prompt/Modell an Einzelbildern testen. | – |
| `tools/farbe_neu_messen.py` | Nach Schwellwert-Änderung den Bestand nachziehen. | `main()` |
| `tools/vorschau_nachholen.py` | Fehlende Vorschauen ohne LM Studio erzeugen; fortsetzbar, überspringt „nur online“. | `main()` |
| `tools/db_struktur.py` | Schema einer DB-Datei ausgeben. | `analyze_db()` |

Jede Datei hat einen Kopf-Docstring (Zweck, Ziel, Aufruf), jede Funktion einen Docstring mit Titel, Erläuterung, Parametern und Rückgabe. Anzeigen z. B. mit `python3.14 -m pydoc bildanalyse`.

---

## 4. Systemvoraussetzungen

| Komponente | Details |
|---|---|
| macOS | nutzt `sips` (Verkleinern, HEIC) und `caffeinate` (Ruhezustand verhindern) |
| Python | **Homebrew-Python 3.14**: `/opt/homebrew/bin/python3.14` |
| Tk für die GUI | `brew install python-tk@3.14` |
| Python-Pakete | Pillow, numpy (siehe `requirements.txt`) |
| ExifTool | `brew install exiftool` – sonst nur Basis-Metadaten, kein HEIC |
| poppler | `brew install poppler` – nur für PDF-Fotobücher (`pdftoppm`, `pdfinfo`) |
| LM Studio | lokaler Server `http://127.0.0.1:1234`, Modell `qwen/qwen3.5-9b` (mit Bildverständnis) |

---

## 5. Starten

### Vorbereitung (bei jedem Analyselauf)
1. **LM Studio** öffnen, Modell `qwen/qwen3.5-9b` laden, Server starten (Developer → Status *Running*).
2. Mac am Netzteil, Stromsparmodus aus.

### Oberfläche
Doppelklick auf **`Bilderwerk Work starten.command`** – oder im Terminal:
```bash
cd work
/opt/homebrew/bin/python3.14 katalog_gui.py
```
Dann Bildordner wählen, Datenbank öffnen bzw. neu anlegen, optional Sicherungsziel, **▶ Start**.

### Kommandozeile
```bash
cd work
caffeinate -i python3.14 katalog.py "<Bildordner>" --db "<Datei.db>" --backup "<Sicherung.db>"
# Test mit den Testbildern in die Standard-Test-DB (Katalog-DB/Bildkatalog.db):
python3.14 katalog.py testdaten/bilder
```

### Suche und Werkzeuge
```bash
python3.14 suche.py brille --farbe schwarzweiß
python3.14 suche.py --statistik
python3.14 suche.py --fehler
python3.14 suche.py --zeige 12
python3.14 tools/bild_test.py testdaten/bilder/set1/*
python3.14 tools/farbe_neu_messen.py --db "<Datei.db>"
python3.14 tools/db_struktur.py "<Datei.db>"
python3.14 tools/vorschau_nachholen.py --db "<Datei.db>" --backup "<Sicherung.db>"
```

### Wiederholungslauf über das ganze Archiv
Einfach denselben Ordner erneut starten (GUI oder `katalog.py`). Der Lauf

1. **analysiert neue und geänderte Bilder** (Pfad + Größe + Änderungszeit),
2. **ergänzt fehlende Vorschaubilder** bei bereits erfassten Bildern (ohne KI; „nur online“-Dateien werden übersprungen und gezählt),
3. **entfernt Einträge gelöschter Dateien** – nur nach einem *vollständigen* Lauf (nicht bei Stopp/Limit) und nur unterhalb des gewählten Ordners.

Verschobene/umbenannte Dateien werden über den Hash als Duplikat erkannt und behalten ihre KI-Analyse; wird danach das alte Original entfernt, rückt das Duplikat zum Original auf. **Schutz:** Fehlen mehr als 25 % der Einträge (z. B. Laufwerk nicht eingebunden), wird nichts entfernt, nur gewarnt (`--entfernen-erzwingen` übergeht das).

Dauer: unveränderte Bilder kosten nur einen Dateizugriff – bei 81.000 Bildern einige Minuten fürs Durchsuchen, dazu ~40–90 s je *neuem* Bild.

---

## 6. Worauf achten

- **Richtiges Python:** immer Homebrew-`python3.14`. Ein anderes Python (Conda-`base`, venv) hat kein Pillow/numpy bzw. Tk → Farbgebung wird nicht gemessen (die GUI zeigt das verwendete Python an und warnt). Nach `brew upgrade` prüfen, ob `python-tk@3.14` mitaktualisiert wurde.
- **Ausführungsrecht:** `Bilderwerk Work starten.command` braucht das Ausführungsrecht (`chmod +x`), sonst meldet macOS „keine Zugriffsrechte“.
- **Sicherung:** Standard alle 1.000 Änderungen; der Wert ist während des Laufs in der GUI änderbar. Am Ende jedes Laufs (auch bei Stopp/Abbruch) wird **immer** gesichert.
- **Datenbank nicht in Dropbox:** lokal ablegen, Sicherung per `--backup`/GUI nach Dropbox. Die Sicherung wird atomar ersetzt (Dropbox sieht nie eine halbe Datei).
- **Dropbox „nur online“:** Bilder und Programme müssen lokal vorliegen. Online-only-Dateien erscheinen als 0 Byte bzw. sind nicht lesbar → Ordner „Offline verfügbar machen“.
- **Fortsetzen:** Abbruch jederzeit (Stopp/Ctrl+C). Der nächste Lauf überspringt Erledigtes (Pfad + Größe + Änderungszeit). Einträge mit Status `fehler` werden erneut versucht.
- **LM Studio weg?** Der Lauf wartet (alle 60 s neuer Versuch) statt Fehler zu buchen.
- **Prompt geändert?** `PROMPT_VERSION` in `bildanalyse.py` hochzählen – sie wird je Eintrag gespeichert.
- **Schema geändert?** `SCHEMA_VERSION` in `katalog.py` und `../shared/schema.sql` anpassen, App-Tests prüfen.
- **Gleichzeitiger Zugriff:** Dank WAL kann die App lesen, während Work schreibt – die App öffnet nur lesend (`mode=ro`).

---

## 7. Verarbeitungsablauf je Bild

1. **Finden:** Endungen `.jpg .jpeg .png .heic .heif .tif .tiff .webp .pdf`; versteckte Dateien/Ordner und Dateien < 30 KB werden ignoriert; jede PDF-Seite ist ein eigener Eintrag.
2. **Schon erledigt?** → überspringen.
3. **Hash (SHA-256):** gleicher Inhalt schon analysiert → Eintrag als **Duplikat** mit übernommenen KI-Feldern.
4. **Metadaten** lesen (Datum notfalls aus dem Dateinamen).
5. **Bild vorbereiten:** 1024 px (PDF-Seiten 1280 px), JPEG.
6. **Farbgebung messen** (schwarzweiß, sepia, monochrom getönt, entsättigt, farbig).
7. **Modellaufruf** mit festem JSON-Schema; Ordner-/Dateiname als Hinweis; bis zu 3 Versuche.
8. **Nachbearbeitung:** Motiv auf feste Liste, Wetter-Plausibilität, Füllwörter/Dubletten entfernen.
9. **Speichern** + Begriffs- und Volltextindex; Sicherung alle N Einträge.

---

## 8. Datenbank (Kurzfassung)

SQLite 3, WAL, `PRAGMA user_version = 4`. Ältere Versionen werden beim Öffnen automatisch erweitert (2→3 PDF-Spalten, 3→4 Tabelle `vorschauen`) – vorhandene Daten bleiben unverändert, eine DB wird nie gelöscht. Unbekannte (neuere) Versionen werden abgelehnt.

| Tabelle | Inhalt |
|---|---|
| `bilder` | ein Datensatz je Bild bzw. PDF-Seite: Ablage, Bildeigenschaften, Metadaten (+ alle als JSON), KI-Felder, Verwaltung (`status` = ok/duplikat/fehler, Modell, Prompt-Version, Dauer) |
| `begriffe` | Einzelbegriffe (kleingeschrieben) für Filter: `schlagwort`, `merkmal`, `stimmung`, `stichwort` |
| `vorschauen` | Vorschaubilder: `hash` (= `bilder.hash`, Duplikate teilen sich eine), `jpeg` (≤ 640 px, Qualität 80, ohne Metadaten, EXIF-Drehung angewendet), `breite`, `hoehe`, `version`, `erzeugt` |
| `suche` | FTS5-Volltext (Trigramm, findet Wortteile; Begriffe ≥ 3 Zeichen): `ablage`, `ki_text`, `meta_text`; `rowid` = `bilder.id` |

Schlüssel `bilder.pfad`: absoluter Pfad, bei PDF-Seiten `…/Album.pdf#seite=3` (`bildanalyse.quelle_aufloesen()`).

Vollständige Spaltenbeschreibung, Abfragebeispiele und Regeln für lesende Programme: `../app/docs/Ausgangsprojekt Bildkatalog Dokumentation.md` und `../shared/schema.sql`.

---

## 9. Bekannte Grenzen

- Kategorien (Tageszeit, Jahreszeit, Anlass) sind beim 9B-Modell eher vorsichtig (`unklar`); der Ordnername im Volltext hilft oft mehr.
- Personen werden beschrieben, **nie identifiziert**.
- Aufnahmedatum kann falsch sein (Kamera-Uhr nicht gestellt).
- Handschrift auf Albumseiten wird nicht immer korrekt gelesen.
- Pfade sind absolute macOS-Pfade – verschiebt sich das Archiv, stimmen sie nicht mehr.

---

## 10. Offene Punkte

- DB-Ordner und Einstellungsdatei konfigurierbar machen (statt fest neben den Programmen)
