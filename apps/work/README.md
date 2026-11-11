# Work (Bildkatalog)

Python-Analyseprogramm, das die Bilder per lokalem KI-Modell (OpenAI-kompatibler Server, z. B.
LM Studio, Ollama, llama.cpp) erkennt und die SQLite-Bilddatenbank erzeugt bzw. aktualisiert, die
Bilderwerk View (`../view`) durchsucht.

**Version:** A1.1 · **Dokumentation:** [`docs/Work Dokumentation.md`](docs/Work%20Dokumentation.md)

## Schnellstart

```bash
brew install python-tk@3.14 pillow numpy exiftool poppler
```

Start der Oberfläche: Doppelklick auf `Bilderwerk Work starten.command`
(oder `cd apps/work && python3 katalog_gui.py`). Vorher den KI-Server mit einem Vision-Modell
starten; Adresse und Modell unter **KI-Server → Einstellen …**.

Einstellungen: `~/Library/Application Support/Bilderwerk/einstellungen.json` (gemeinsam mit Sweep).
Tests: `python3 -m unittest discover -s tests`

Die Datenbank gehört nicht ins Repo (`*.db`, `Katalog-DB/` sind in `.gitignore`).
Bei Schemaänderungen `SCHEMA_VERSION` in `katalog.py` erhöhen und die View-Tests prüfen.
