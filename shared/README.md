# shared – Gemeinsames von Work und Sweep

- `bilderwerk/` – Python-Paket, das Work und Sweep gemeinsam nutzen
  - `einstellungen.py` – gemeinsame Einstellungsdatei im Benutzerordner
    (`~/Library/Application Support/Bilderwerk/einstellungen.json`): Datenbankpfad
    für beide, je ein Abschnitt `work` und `sweep`. Übernimmt beim ersten Start die
    alte `apps/work/gui_einstellungen.json`.
  - `katalog_db.py` – Schemaversion, Schlüssel der Spalte `pfad` (Datei/PDF-Seite),
    Einträge konsistent entfernen
  - `bild.py` – PDF-Seiten rendern, „nur online“ erkennen, Vorschaubilder erzeugen, RAW-Endungen
- `python_finden.sh` – sucht für die `.command`-Dateien ein passendes Python (ab 3.10 mit Tk)
- `tests/` – Tests: `python3 -m unittest discover -s shared/tests` (aus dem Repo-Wurzelordner)

View ist eine eigenständige Anwendung und hat eigene Einstellungen.

Datenbankschema: Bei jeder Schemaänderung `SCHEMA_VERSION` in `apps/work/katalog.py`
erhöhen und die App-Tests (`apps/view/src-tauri/tests/`) prüfen.
