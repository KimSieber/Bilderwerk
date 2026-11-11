#!/usr/bin/env python3
"""Farbgebung aller Bilder einer Datenbank neu messen – ohne KI-Analyse (Work-Tool).

Zweck:
    Misst für alle Einträge mit Status 'ok'/'duplikat' die Farbgebung neu
    (bildanalyse.farbgebung_messen) und aktualisiert bei Abweichung die
    Spalte 'farbgebung' sowie den Suchindex.

Ziel:
    Nach einer Änderung der Schwellwerte in der Farbmessung den Bestand
    nachziehen, ohne den langen KI-Lauf zu wiederholen.

Aufruf:  python3 farbe_neu_messen.py            (Standard-Datenbank)
         python3 farbe_neu_messen.py --db andere.db
"""
import argparse
import json
import sqlite3
import tempfile
from pathlib import Path

import sys
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # Kernmodule in apps/work/

import bildanalyse
import katalog


def main():
    """Alle Einträge durchgehen, Farbgebung neu messen, Änderungen speichern.

    Fehlende Bilddateien werden gemeldet und übersprungen. Ausgabe: jede
    Änderung (alt → neu) und eine Zusammenfassung.

    Rückgabe:
        Keine.

    Fehler:
        SystemExit, wenn Pillow/numpy oder die Datenbank fehlen.
    """
    ap = argparse.ArgumentParser(description="Farbgebung neu messen")
    ap.add_argument("--db", type=Path, default=bildanalyse.STANDARD_DB)
    args = ap.parse_args()

    try:
        import numpy, PIL  # noqa: F401
    except ImportError:
        raise SystemExit("Pillow/numpy fehlen. Bitte zuerst: python3 -m pip install pillow numpy")

    if not args.db.expanduser().is_file():
        raise SystemExit(f"Datenbank nicht gefunden: {args.db.expanduser()}")
    db = katalog.db_oeffnen(args.db.expanduser())
    db.row_factory = sqlite3.Row
    zeilen = db.execute("SELECT * FROM bilder WHERE status IN ('ok', 'duplikat')").fetchall()
    geaendert = 0
    for z in zeilen:
        pfad, seite = bildanalyse.quelle_aufloesen(z["pfad"])
        if not pfad.exists():
            print(f"fehlt: {pfad}")
            continue
        with tempfile.TemporaryDirectory() as tmp:
            if seite is None:
                jpeg = bildanalyse.bild_vorbereiten(pfad, tmp)
            else:
                jpeg = bildanalyse.pdf_seite_rendern(pfad, seite, tmp,
                                                    bildanalyse.MAX_KANTE_SEITE)
            neu = bildanalyse.farbgebung_messen(jpeg)
        if neu == z["farbgebung"]:
            continue
        print(f"{z['relativer_pfad']}: {z['farbgebung']} -> {neu}")
        db.execute("UPDATE bilder SET farbgebung = ? WHERE id = ?", (neu, z["id"]))
        daten = dict(z)
        daten["farbgebung"] = neu
        for feld in katalog.LISTEN_FELDER:
            daten[feld] = json.loads(daten[feld]) if daten[feld] else []
        katalog.suchindex_aktualisieren(db, z["id"], daten)
        geaendert += 1
    db.commit()
    print(f"\n{len(zeilen)} Bilder geprüft, {geaendert} geändert.")


if __name__ == "__main__":
    main()
