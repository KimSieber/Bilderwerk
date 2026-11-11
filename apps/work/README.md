# Work (Bildkatalog)

Python-Analyseprogramm, das die Bilder per lokalem KI-Modell (LM Studio) erkennt und die
SQLite-Bilddatenbank erzeugt bzw. aktualisiert, die Bilderwerk View (`../app`) durchsucht.

**Version:** A1.1 · **Dokumentation:** [`docs/Work Dokumentation.md`](docs/Work%20Dokumentation.md)

## Schnellstart

```bash
brew install python-tk@3.14 pillow numpy exiftool poppler
```

Start der Oberfläche: Doppelklick auf `Bilderwerk Work starten.command`
(oder `cd work && /opt/homebrew/bin/python3.14 katalog_gui.py`).
Vorher LM Studio mit dem Modell starten.

Die Datenbank gehört nicht ins Repo (`*.db`, `Katalog-DB/` sind in `.gitignore`).
Schemaänderungen bitte in `../shared/schema.sql` nachziehen – das ist der Vertrag zur App.
