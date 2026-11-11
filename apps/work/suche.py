#!/usr/bin/env python3
"""Suche im Bilderkatalog per Kommandozeile (Work, Testwerkzeug).

Zweck:
    Durchsucht eine Katalog-Datenbank: Volltext über alle Texte
    (Trigramm, auch Wortteile) kombiniert mit Filtern (Farbe, Motiv,
    Wetter, Merkmale, Kamera, Zeitraum …). Außerdem Detailanzeige eines
    Bildes, Statistik und Fehlerliste.

Ziel:
    Schnelle Prüfung der Analyseergebnisse ohne die App Bilderwerk View.
    Öffnet die Datenbank nur zum Lesen von Daten (schreibt nichts).

Beispiele:
  python3 suche.py brille                       # Volltext (auch Wortteile)
  python3 suche.py baum landschaft --farbe farbig --ausrichtung quer
  python3 suche.py hochzeit --merkmal brautkleid
  python3 suche.py --motiv Tier --wetter sonnig
  python3 suche.py --kamera nikon --von 2015 --bis 2016-06
  python3 suche.py --zeige 12                   # alle Daten zu Bild 12
  python3 suche.py --statistik                  # Überblick über den Katalog
  python3 suche.py --fehler                     # fehlerhafte Bilder mit Grund
"""
import argparse
import bildanalyse
import json
import sqlite3
from pathlib import Path


def fts_ausdruck(begriffe: list[str]) -> str:
    """FTS5-Suchausdruck bilden: jeder Begriff muss vorkommen (UND).

    Begriffe werden in Anführungszeichen gesetzt (sicher gegen Sonderzeichen).
    Der Trigramm-Index braucht Begriffe mit mindestens 3 Zeichen.

    Parameter:
        begriffe: Suchbegriffe.

    Rückgabe:
        Ausdruck für 'suche MATCH ?', z. B. '"baum" AND "see"'.
    """
    return " AND ".join('"' + b.replace('"', '""') + '"' for b in begriffe)


def suchen(db, args):
    """Suche ausführen und Treffer ausgeben.

    Kombiniert Volltext (Begriffe ab 3 Zeichen), LIKE-Suche für kürzere
    Begriffe und alle gesetzten Filter mit UND. Berücksichtigt nur Einträge
    mit Status 'ok' oder 'duplikat'.

    Parameter:
        db:   Offene Datenbankverbindung.
        args: Ergebnis von argparse (Begriffe und Filter).

    Rückgabe:
        Keine (Ausgabe auf der Konsole).
    """
    bedingungen, werte = ["b.status IN ('ok', 'duplikat')"], []
    lange = [b for b in args.begriffe if len(b) >= 3]
    kurze = [b for b in args.begriffe if len(b) < 3]
    if lange:
        bedingungen.append("b.id IN (SELECT rowid FROM suche WHERE suche MATCH ?)")
        werte.append(fts_ausdruck(lange))
    for b in kurze:  # sehr kurze Begriffe: einfache Textsuche
        bedingungen.append("(b.beschreibung LIKE ? OR b.schlagworte LIKE ?)")
        werte += [f"%{b}%"] * 2

    for spalte, wert in (("farbgebung", args.farbe), ("motiv", args.motiv),
                         ("bildart", args.bildart), ("wetter", args.wetter),
                         ("jahreszeit", args.jahreszeit), ("tageszeit", args.tageszeit),
                         ("innen_aussen", args.ort), ("ausrichtung", args.ausrichtung)):
        if wert:
            bedingungen.append(f"lower(b.{spalte}) = lower(?)")
            werte.append(wert)
    for m in args.merkmal or []:
        bedingungen.append("b.id IN (SELECT bild_id FROM begriffe "
                           "WHERE typ = 'merkmal' AND wert LIKE ?)")
        werte.append(f"%{m.lower()}%")
    if args.personen_min is not None:
        bedingungen.append("b.personen_anzahl >= ?")
        werte.append(args.personen_min)
    if args.kamera:
        bedingungen.append("(b.kamera_hersteller LIKE ? OR b.kamera_modell LIKE ?)")
        werte += [f"%{args.kamera}%"] * 2
    if args.von:
        bedingungen.append("b.aufnahme_datum >= ?")
        werte.append(args.von)
    if args.bis:
        bedingungen.append("b.aufnahme_datum < ?")
        werte.append(args.bis + "\uffff")   # '2016-06' schliesst den ganzen Juni ein
    if args.mit_gps:
        bedingungen.append("b.gps_lat IS NOT NULL")

    sql = (f"SELECT b.id, b.relativer_pfad, b.kurztitel, b.motiv, b.farbgebung, "
           f"b.aufnahme_datum FROM bilder b WHERE {' AND '.join(bedingungen)} "
           f"ORDER BY b.aufnahme_datum, b.relativer_pfad LIMIT ?")
    zeilen = db.execute(sql, werte + [args.limit]).fetchall()
    for i, pfad, titel, motiv, farbe, datum in zeilen:
        print(f"#{i:<6} {(datum or '')[:10]:10}  {motiv or '':18} {farbe or '':12} "
              f"{titel or ''}\n        {pfad}")
    print(f"\n{len(zeilen)} Treffer" + (" (Limit erreicht)" if len(zeilen) == args.limit else ""))


def zeigen(db, bild_id: int):
    """Alle gefüllten Felder eines Bildes ausgeben.

    Parameter:
        db:      Offene Datenbankverbindung.
        bild_id: id in Tabelle 'bilder'.

    Rückgabe:
        Keine (Ausgabe auf der Konsole).
    """
    db.row_factory = sqlite3.Row
    z = db.execute("SELECT * FROM bilder WHERE id = ?", (bild_id,)).fetchone()
    if not z:
        print("Nicht gefunden.")
        return
    for k in z.keys():
        if k == "metadaten_json" or z[k] in (None, ""):
            continue
        print(f"{k:18} {z[k]}")
    if z["metadaten_json"]:
        alle = json.loads(z["metadaten_json"])
        print(f"\n(+ {len(alle)} weitere Metadatenfelder in 'metadaten_json')")


def statistik(db):
    """Überblick über den Katalog ausgeben.

    Status-Verteilung, häufigste Werte je Kategorie, häufigste Schlagworte,
    Anteil mit GPS/Kameradaten und durchschnittliche Analysezeit.

    Parameter:
        db: Offene Datenbankverbindung.

    Rückgabe:
        Keine (Ausgabe auf der Konsole).
    """
    gesamt = db.execute("SELECT status, COUNT(*) FROM bilder GROUP BY status").fetchall()
    print("Status:", ", ".join(f"{s}: {n}" for s, n in gesamt))
    for spalte in ("motiv", "farbgebung", "bildart", "wetter", "anlass"):
        top = db.execute(f"SELECT {spalte}, COUNT(*) FROM bilder WHERE status = 'ok' "
                         f"GROUP BY {spalte} ORDER BY 2 DESC LIMIT 8").fetchall()
        print(f"\n{spalte}: " + ", ".join(f"{w} ({n})" for w, n in top))
    top = db.execute("SELECT wert, COUNT(*) FROM begriffe WHERE typ = 'schlagwort' "
                     "GROUP BY wert ORDER BY 2 DESC LIMIT 25").fetchall()
    print("\nHäufigste Schlagworte: " + ", ".join(f"{w} ({n})" for w, n in top))
    z = db.execute("SELECT COUNT(*), SUM(gps_lat IS NOT NULL), SUM(kamera_modell IS NOT NULL), "
                   "ROUND(AVG(dauer_s), 1) FROM bilder WHERE status = 'ok'").fetchone()
    print(f"\nMit GPS: {z[1]} | mit Kameradaten: {z[2]} | Ø Analysezeit: {z[3]} s")


def fehler_liste(db):
    """Alle Einträge mit Status 'fehler' samt Fehlertext ausgeben.

    Parameter:
        db: Offene Datenbankverbindung.

    Rückgabe:
        Keine (Ausgabe auf der Konsole).
    """
    zeilen = db.execute("SELECT id, relativer_pfad, fehler, erfasst FROM bilder "
                        "WHERE status = 'fehler' ORDER BY relativer_pfad").fetchall()
    for i, pfad, fehler, erfasst in zeilen:
        print(f"#{i:<6} {(erfasst or '')[:16]}  {pfad}\n        {fehler}")
    print(f"\n{len(zeilen)} fehlerhafte Bilder (werden beim nächsten Lauf erneut versucht)")


def main():
    """Kommandozeilen-Einstieg: Argumente lesen und Aktion ausführen.

    Rückgabe:
        Keine.
    """
    ap = argparse.ArgumentParser(description="Suche im Bilderkatalog")
    ap.add_argument("begriffe", nargs="*", help="Suchbegriffe (alle müssen vorkommen)")
    ap.add_argument("--db", type=Path, default=bildanalyse.STANDARD_DB)
    ap.add_argument("--farbe", help="farbig | schwarzweiß | sepia | monochrom getönt | entsättigt")
    ap.add_argument("--motiv")
    ap.add_argument("--bildart")
    ap.add_argument("--wetter")
    ap.add_argument("--jahreszeit")
    ap.add_argument("--tageszeit")
    ap.add_argument("--ort", help="innen | außen")
    ap.add_argument("--ausrichtung", help="quer | hoch | quadratisch")
    ap.add_argument("--merkmal", action="append", help="Personenmerkmal, mehrfach möglich")
    ap.add_argument("--personen-min", type=int)
    ap.add_argument("--kamera")
    ap.add_argument("--von", help="Datum ab, z. B. 2015 oder 2015-06-01")
    ap.add_argument("--bis", help="Datum bis einschließlich, z. B. 2016 oder 2016-06")
    ap.add_argument("--mit-gps", action="store_true")
    ap.add_argument("--limit", type=int, default=50)
    ap.add_argument("--zeige", type=int, help="alle Daten zu einer Bild-ID anzeigen")
    ap.add_argument("--statistik", action="store_true")
    ap.add_argument("--fehler", action="store_true", help="fehlerhafte Bilder auflisten")
    args = ap.parse_args()

    db = sqlite3.connect(args.db.expanduser())
    if args.zeige:
        zeigen(db, args.zeige)
    elif args.fehler:
        fehler_liste(db)
    elif args.statistik:
        statistik(db)
    else:
        suchen(db, args)


if __name__ == "__main__":
    main()
