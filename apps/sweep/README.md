# Bilderwerk Sweep

Dubletten im Bildkatalog sichten und einzeln löschen – Python + tkinter.

**Gegen die LIVE-DB starten** (parallel zu Work möglich), nie gegen die Sicherung.

Start: Doppelklick auf `Bilderwerk Sweep starten.command` – oder:

```bash
cd apps/sweep
python3 sweep.py                       # Datenbank aus den gemeinsamen Einstellungen
python3 sweep.py ~/Pfad/Bildkatalog.db # einmalig eine andere (wird nicht gespeichert)
```

- **Datenbank:** dieselbe wie in Work (gemeinsame Einstellungen,
  `~/Library/Application Support/Bilderwerk/einstellungen.json`). Über **Datenbank …** wechselbar;
  die Wahl wird gespeichert und gilt damit auch für Work. Ohne gültige DB fragt Sweep beim Start.
- **Unabhängig von Work:** gemeinsamer Code (Schlüssel, Vorschau, Entfernen aus der DB, Einstellungen)
  liegt in `../../shared/bilderwerk`. Python wie Work (Pillow, Tk).
- Dublette = identischer SHA-256 (Status `duplikat` von Work) → Bilder sind bytegleich, nur Name/Ablage unterscheiden sich.
- Löschen: Bestätigung → Datei löschen → `katalog_db.eintraege_entfernen()` (nächste Kopie wird Original, Vorschau bleibt, solange noch genutzt) → Zeile in `sweep_protokoll.csv` neben der DB.
- PDFs: je Datei eine Gruppe, **ohne** Lösch-Button (nur Hinweis, im Finder prüfen).
- Vorschau aus der DB; fehlt sie, lokal erzeugt – nicht bei Dropbox-Platzhaltern („nur online“).
- Tests: `python3 -m unittest discover -s tests` (legt die Test-DB mit Work an).
