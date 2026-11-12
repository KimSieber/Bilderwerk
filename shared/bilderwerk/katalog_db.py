"""Gemeinsamer Datenbankzugriff von Work und Sweep (Bildkatalog-DB).

Zweck:
    Was beide Module über die Katalog-Datenbank wissen müssen: Schemaversion,
    Aufbau des Schlüssels in der Spalte 'pfad' (Datei bzw. PDF-Seite) und das
    konsistente Entfernen von Einträgen (Original-Nachfolge bei Duplikaten).
    Das Anlegen/Erweitern des Schemas bleibt in Work (katalog.py).
"""
from __future__ import annotations

from pathlib import Path

SCHEMA_VERSION = 4        # 4 = Tabelle vorschauen (Vorschaubilder)

SEITEN_MARKE = "#seite="


def eintrag_schluessel(datei: Path, seite: int | None) -> str:
    """Eindeutigen Datenbank-Schlüssel (Spalte 'pfad') bilden.

    Bilddateien werden über ihren absoluten Pfad identifiziert, PDF-Seiten über
    '<Pfad>#seite=<n>'.

    Parameter:
        datei: Absoluter Pfad der Datei.
        seite: Seitennummer bei PDFs, sonst None.

    Rückgabe:
        Schlüssel als Text, z. B. '/…/Album.pdf#seite=3'.
    """
    return str(datei) if seite is None else f"{datei}{SEITEN_MARKE}{seite}"


def quelle_aufloesen(schluessel: str) -> tuple[Path, int | None]:
    """Datenbank-Schlüssel in Datei und Seite zerlegen.

    Gegenstück zu eintrag_schluessel().

    Parameter:
        schluessel: Wert aus der Spalte 'pfad'.

    Rückgabe:
        Tupel (Dateipfad, Seitennummer oder None).
    """
    if SEITEN_MARKE in schluessel:
        datei, seite = schluessel.rsplit(SEITEN_MARKE, 1)
        return Path(datei), int(seite)
    return Path(schluessel), None


def eintraege_entfernen(db, ids: list[int]) -> int:
    """Einträge zu nicht mehr vorhandenen Dateien vollständig entfernen.

    Löscht die Zeilen aus 'bilder', 'begriffe' und 'suche'. War ein gelöschter
    Eintrag das Original von Duplikaten, wird das erste verbleibende Duplikat
    zum neuen Original (Status 'ok'), die übrigen zeigen auf dieses – so geht
    keine KI-Analyse verloren. Vorschauen werden nur gelöscht, wenn kein
    verbleibender Eintrag denselben Hash nutzt. Kein Commit.

    Parameter:
        db:  Offene Datenbankverbindung.
        ids: bilder.id der zu entfernenden Einträge.

    Rückgabe:
        Anzahl entfernter Einträge.
    """
    weg = set(ids)
    for bild_id in ids:
        nachfolger = [i for (i,) in db.execute(
            "SELECT id FROM bilder WHERE duplikat_von = ? ORDER BY id", (bild_id,))
            if i not in weg]
        if nachfolger:
            neu = nachfolger[0]
            db.execute("UPDATE bilder SET status = 'ok', duplikat_von = NULL WHERE id = ?", (neu,))
            db.execute("UPDATE bilder SET duplikat_von = ? WHERE duplikat_von = ?", (neu, bild_id))
    hashes = set()
    for bild_id in ids:
        z = db.execute("SELECT hash FROM bilder WHERE id = ?", (bild_id,)).fetchone()
        if z and z[0]:
            hashes.add(z[0])
        db.execute("DELETE FROM begriffe WHERE bild_id = ?", (bild_id,))
        db.execute("DELETE FROM suche WHERE rowid = ?", (bild_id,))
        db.execute("DELETE FROM bilder WHERE id = ?", (bild_id,))
    for h in hashes:
        if not db.execute("SELECT 1 FROM bilder WHERE hash = ? LIMIT 1", (h,)).fetchone():
            db.execute("DELETE FROM vorschauen WHERE hash = ?", (h,))
    return len(ids)
