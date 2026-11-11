#!/usr/bin/env python3
"""Vorschaubilder für den Bestand nachholen – ohne KI-Analyse (Work-Tool).

Zweck:
    Erzeugt für alle Einträge mit Status 'ok'/'duplikat', deren Hash noch
    keine Vorschau in Tabelle 'vorschauen' hat, ein Vorschaubild (640 px)
    nach den Vorgaben in bildanalyse.vorschau_erzeugen(). Ältere Datenbanken
    werden dabei auf Schema 4 erweitert (nie gelöscht).

Ziel:
    Die ca. 24.000 bereits erfassten Bilder mit Vorschauen versorgen, ohne
    LM Studio und ohne den langen KI-Lauf. (Der normale Katalog-Lauf ergänzt
    fehlende Vorschauen ebenfalls – dieses Werkzeug ist der schnelle Weg
    ohne Modell.)

Eigenschaften:
    - Fortsetzbar: Abbruch (Ctrl+C) und Neustart setzen fort; Commit alle 200.
    - Dropbox „nur online“: Platzhalter werden übersprungen und gezählt,
      nicht geladen (außer mit --auch-online).
    - Fortschritt mit Restzeit, Abschlussbericht, Sicherung am Ende.

Aufruf:
    python3.14 vorschau_nachholen.py [--db DATEI] [--backup DATEI] [--limit N]
                                     [--nur-ordner PFAD] [--neu] [--auch-online]
"""
import argparse
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # Kernmodule in apps/work/

import bildanalyse
import katalog

COMMIT_ALLE = 200


def offene_eintraege(db, neu: bool, nur_ordner: str | None) -> list[tuple[str, str]]:
    """Einträge ohne (aktuelle) Vorschau ermitteln – je Hash nur einmal.

    Parameter:
        db:         Offene Datenbankverbindung.
        neu:        True = auch Vorschauen mit älterer Verfahrensversion neu erzeugen.
        nur_ordner: Nur Einträge, deren Pfad mit diesem Ordner beginnt (oder None).

    Rückgabe:
        Liste (hash, pfad), sortiert nach Pfad (PDF-Seiten liegen beieinander).
    """
    bedingung = ("v.hash IS NULL OR v.version < ?" if neu else "v.hash IS NULL")
    werte = [bildanalyse.VORSCHAU_VERSION] if neu else []
    zeilen = db.execute(
        f"SELECT b.hash, MIN(b.pfad) FROM bilder b LEFT JOIN vorschauen v ON v.hash = b.hash "
        f"WHERE b.status IN ('ok', 'duplikat') AND b.hash IS NOT NULL AND ({bedingung}) "
        f"GROUP BY b.hash ORDER BY MIN(b.pfad)", werte).fetchall()
    if nur_ordner:
        praefix = str(Path(nur_ordner).expanduser().resolve()).rstrip("/") + "/"
        zeilen = [z for z in zeilen if z[1].startswith(praefix)]
    return zeilen


def main():
    """Argumente lesen, fehlende Vorschauen erzeugen, Bericht ausgeben, sichern.

    Rückgabe:
        Keine.

    Fehler:
        SystemExit, wenn Pillow fehlt oder die Datenbank nicht existiert.
    """
    ap = argparse.ArgumentParser(description="Vorschaubilder nachholen (ohne KI)")
    ap.add_argument("--db", type=Path, default=bildanalyse.STANDARD_DB)
    ap.add_argument("--backup", type=Path, default=None, help="Sicherungskopie am Ende")
    ap.add_argument("--limit", type=int, default=0, help="höchstens N Vorschauen erzeugen")
    ap.add_argument("--nur-ordner", help="nur Bilder unterhalb dieses Ordners")
    ap.add_argument("--neu", action="store_true",
                    help="Vorschauen älterer Verfahrensversion neu erzeugen")
    ap.add_argument("--auch-online", action="store_true",
                    help="Dropbox-Platzhalter („nur online“) bewusst herunterladen")
    args = ap.parse_args()

    try:
        import PIL  # noqa: F401
    except ImportError:
        raise SystemExit("Pillow fehlt. Bitte zuerst: brew install pillow")
    db_pfad = args.db.expanduser()
    if not db_pfad.is_file():
        raise SystemExit(f"Datenbank nicht gefunden: {db_pfad}")
    db = katalog.db_oeffnen(db_pfad)            # erweitert bei Bedarf auf Schema 4
    protokoll = katalog.Protokoll(db_pfad.with_name(db_pfad.stem + "_protokoll.txt"))

    eintraege = offene_eintraege(db, args.neu, args.nur_ordner)
    if args.limit:
        eintraege = eintraege[:args.limit]
    print(f"{len(eintraege)} Vorschauen fehlen.")
    protokoll.schreiben(f"=== VORSCHAU-NACHHOLEN START | offen: {len(eintraege)}")

    erzeugt, online, fehlt, fehler = 0, 0, 0, []
    start = time.time()
    try:
        for nr, (hash_wert, pfad_text) in enumerate(eintraege, 1):
            datei, seite = bildanalyse.quelle_aufloesen(pfad_text)
            if not datei.exists():
                fehlt += 1
                continue
            if not args.auch_online and bildanalyse.nur_online(datei):
                online += 1
                continue
            try:
                katalog.vorschau_speichern(db, hash_wert,
                                           bildanalyse.vorschau_erzeugen(datei, seite))
                erzeugt += 1
            except Exception as ex:
                fehler.append((pfad_text, f"{type(ex).__name__}: {ex}"))
                protokoll.schreiben(f"VORSCHAU| {pfad_text} | {fehler[-1][1]}"[:400])
            if erzeugt and erzeugt % COMMIT_ALLE == 0:
                db.commit()
            if nr % 50 == 0 or nr == len(eintraege):
                dauer = time.time() - start
                rest = dauer / nr * (len(eintraege) - nr)
                print(f"[{nr}/{len(eintraege)}] erzeugt: {erzeugt} | nur online: {online} | "
                      f"Fehler: {len(fehler)} | Rest ca. {rest / 60:.0f} min", flush=True)
    except KeyboardInterrupt:
        print("\nAbgebrochen – der nächste Start setzt fort.")
    finally:
        db.commit()
        bericht = (f"erzeugt: {erzeugt} | übersprungen (nur online): {online} | "
                   f"Datei fehlt: {fehlt} | fehlerhaft: {len(fehler)}")
        protokoll.schreiben(f"=== VORSCHAU-NACHHOLEN ENDE | {bericht}")
        print("\n" + bericht)
        for pfad_text, grund in fehler:
            print(f"  FEHLER {pfad_text}\n         {grund}")
        if args.backup:
            katalog.backup_schreiben(db, args.backup.expanduser())
            print(f"Sicherung geschrieben: {args.backup.expanduser()}")
        db.close()
    if erzeugt > 1000:
        print("Tipp: Die DB ist deutlich gewachsen – einmalig 'VACUUM' kann Platz sparen.")


if __name__ == "__main__":
    main()
