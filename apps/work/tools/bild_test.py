#!/usr/bin/env python3
"""Bildanalyse an einzelnen Bildern testen – ohne Datenbank (Work-Tool).

Zweck:
    Analysiert die angegebenen Bilder mit dem Modell in LM Studio und gibt
    Ergebnis und Metadaten als JSON aus, zum Schluss die Durchschnittsdauer.

Ziel:
    Prompt-, Modell- oder Schwellwert-Änderungen schnell an Testbildern
    prüfen, ohne eine Katalog-Datenbank zu verändern.

Voraussetzung:
    LM Studio läuft, Modell geladen, Server gestartet.

Aufruf:  python3 bild_test.py bild1.jpg bild2.jpg ...
         python3 bild_test.py ../testdaten/bilder/set1/*
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # Kernmodule in apps/work/

import bildanalyse
import metadaten

if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit("Aufruf: python3 bild_test.py bild1.jpg [bild2.jpg ...]")
    dauern = []
    for arg in sys.argv[1:]:
        bild = Path(arg).expanduser()
        if not bild.is_file():
            continue
        print(f"\n===== {bild.name} =====")
        try:
            ergebnis = bildanalyse.analysieren(bild)
        except Exception as ex:
            print(f"FEHLER: {type(ex).__name__}: {ex}")
            continue
        meta = metadaten.lesen(bild)
        meta.pop("metadaten_json", None)
        dauern.append(ergebnis["dauer_s"])
        print(json.dumps(ergebnis, indent=2, ensure_ascii=False))
        print("--- Metadaten ---")
        print(json.dumps({k: v for k, v in meta.items() if v is not None},
                         indent=2, ensure_ascii=False))
    if dauern:
        print(f"\nØ {sum(dauern) / len(dauern):.1f} s über {len(dauern)} Bilder")
