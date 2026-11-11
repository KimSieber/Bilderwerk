# Bilderwerk Sweep (PS)

Dubletten im Bildkatalog sichten und einzeln löschen – Python + tkinter, einmaliges Werkzeug.

**Nur auf dem Mac mini gegen die LIVE-DB starten** (parallel zum Work möglich), nie gegen die Sicherung.

```bash
cd sweep
python3 sweep.py                       # DB aus apps/work/gui_einstellungen.json
python3 sweep.py ~/Pfad/Bildkatalog.db # oder explizit
```

Gleiches Python wie Work (Pillow, python-tk). Nutzt `../work/katalog.py`.

- Dublette = identischer SHA-256 (Status `duplikat` von Work) → Bilder sind bytegleich, nur Name/Ablage unterscheiden sich.
- Löschen: Bestätigung → Datei löschen → `katalog.eintraege_entfernen()` (nächste Kopie wird Original, Vorschau bleibt, solange noch genutzt) → Zeile in `sweep_protokoll.csv` neben der DB.
- PDFs: je Datei eine Gruppe, **ohne** Lösch-Button (nur Hinweis, im Finder prüfen).
- Vorschau aus der DB; fehlt sie, lokal erzeugt – nicht bei Dropbox-Platzhaltern („nur online“).
