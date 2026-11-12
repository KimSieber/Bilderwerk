"""Bilderwerk Sweep – Datenzugriff (ohne Oberfläche, testbar).

Liest Dublettengruppen aus der Live-Katalog-DB von Work, liefert Vorschaubilder
und löscht einzelne Dateien samt Katalog-Einträgen. Für das Entfernen aus der DB
wird das gemeinsame katalog_db.eintraege_entfernen() genutzt (Original-Nachfolge,
begriffe/suche/vorschauen bleiben konsistent). Keine Abhängigkeit vom Work-Ordner:
gemeinsamer Code und Einstellungen liegen in shared/bilderwerk.

Dublette = gleicher SHA-256 der Datei (so wie Work 'duplikat' vergibt).
PDFs werden je Datei zusammengefasst (Hash '<dateihash>#<seite>' → '<dateihash>')
und sind nur Hinweis – Löschen ist für PDFs nicht vorgesehen.
"""
from __future__ import annotations

import csv
import os
import sqlite3
import sys
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

# Gemeinsamer Code von Work und Sweep liegt in <Repo>/shared
_SHARED = Path(__file__).resolve().parent.parent.parent / "shared"
if _SHARED.is_dir() and str(_SHARED) not in sys.path:
    sys.path.insert(0, str(_SHARED))
from bilderwerk import einstellungen                                  # noqa: E402
from bilderwerk.bild import nur_online, vorschau_erzeugen             # noqa: E402
from bilderwerk.katalog_db import (SCHEMA_VERSION, eintraege_entfernen,  # noqa: E402
                                   quelle_aufloesen)

# Bis v0.1 stand der DB-Pfad in den Work-Einstellungen – Quelle für die Migration
ALT_EINSTELLUNGEN = Path(__file__).resolve().parent.parent / "work" / "gui_einstellungen.json"

VORSCHAU_PX = 640
PROTOKOLL_NAME = "sweep_protokoll.csv"
PROTOKOLL_SPALTEN = ["zeitpunkt", "geloescht", "hash", "groesse_bytes", "breite", "hoehe",
                     "verbleibend"]


@dataclass
class Datei:
    """Eine Datei einer Dublettengruppe (bei PDFs: alle Seiten-Einträge zusammen)."""
    pfad: str                 # Dateipfad (ohne Seitenmarke)
    ids: list[int]            # bilder.id aller Einträge dieser Datei
    vorschau_hash: str        # Schlüssel in 'vorschauen' (PDF: Seite 1)
    breite: int | None
    hoehe: int | None
    groesse: int | None
    ist_pdf: bool
    seiten: int | None = None

    @property
    def name(self) -> str:
        return Path(self.pfad).name

    @property
    def ordner(self) -> str:
        return str(Path(self.pfad).parent)


@dataclass
class Gruppe:
    """Alle Dateien mit identischem Inhalt."""
    hash: str
    dateien: list[Datei] = field(default_factory=list)

    @property
    def ist_pdf(self) -> bool:
        return any(d.ist_pdf for d in self.dateien)


def standard_db() -> Path | None:
    """Datenbank aus den gemeinsamen Einstellungen (dieselbe wie in Work).

    Rückgabe:
        Pfad, wenn die Datei existiert, sonst None (dann fragt die Oberfläche).
    """
    p = einstellungen.db_pfad(einstellungen.laden(ALT_EINSTELLUNGEN))
    return p if p.exists() else None


def db_merken(pfad: Path) -> None:
    """Gewählte Datenbank in den gemeinsamen Einstellungen speichern (gilt auch für Work).

    Parameter:
        pfad: Datenbankdatei.

    Fehler:
        OSError, wenn die Einstellungen nicht geschrieben werden können.
    """
    e = einstellungen.laden(ALT_EINSTELLUNGEN)
    e["db"] = str(pfad)
    einstellungen.speichern(e)


def verbinden(pfad: Path) -> sqlite3.Connection:
    """Live-DB öffnen (ohne Schemaänderung). Wartet bis 30 s auf Sperren von Work."""
    db = sqlite3.connect(pfad, timeout=30, check_same_thread=False)
    version = db.execute("PRAGMA user_version").fetchone()[0]
    if version != SCHEMA_VERSION:
        db.close()
        raise ValueError(f"Keine passende Katalog-Datenbank (Schema-Version {version}, "
                         f"erwartet {SCHEMA_VERSION}): {pfad}")
    return db


def gruppen_laden(db: sqlite3.Connection) -> list[Gruppe]:
    """Alle Dublettengruppen (≥ 2 verschiedene Dateien) laden, größte Dateien zuerst."""
    zeilen = db.execute("""
        SELECT id, pfad, hash, breite, hoehe, groesse, seite, seiten_gesamt
        FROM bilder
        WHERE status IN ('ok', 'duplikat') AND hash IS NOT NULL AND hash <> ''
        ORDER BY id""").fetchall()
    gruppen: dict[str, dict[str, Datei]] = {}
    for bild_id, pfad, h, breite, hoehe, groesse, seite, seiten in zeilen:
        ist_pdf = seite is not None
        datei_hash = h.split("#", 1)[0] if ist_pdf else h
        datei_pfad = str(quelle_aufloesen(pfad)[0]) if ist_pdf else pfad
        dateien = gruppen.setdefault(datei_hash, {})
        d = dateien.get(datei_pfad)
        if d is None:
            dateien[datei_pfad] = Datei(datei_pfad, [bild_id], h, breite, hoehe, groesse,
                                        ist_pdf, seiten)
        else:
            d.ids.append(bild_id)
            if ist_pdf and seite == 1:
                d.vorschau_hash, d.breite, d.hoehe = h, breite, hoehe
    ergebnis = [Gruppe(h, list(d.values())) for h, d in gruppen.items() if len(d) >= 2]
    ergebnis.sort(key=lambda g: (g.ist_pdf, -(g.dateien[0].groesse or 0)))
    return ergebnis


def vorschau(db: sqlite3.Connection, datei: Datei) -> tuple[bytes | None, str]:
    """Vorschau-JPEG (≤ 640 px) liefern.

    Rückgabe: (JPEG-Bytes oder None, Hinweistext bei fehlender Vorschau).
    Erst aus der Tabelle 'vorschauen'; sonst lokal erzeugt – aber nie bei
    Dropbox-Platzhaltern („nur online“), damit kein Download angestoßen wird.
    """
    z = db.execute("SELECT jpeg FROM vorschauen WHERE hash = ?", (datei.vorschau_hash,)).fetchone()
    if z:
        return z[0], ""
    p = Path(datei.pfad)
    if not p.exists():
        return None, "Datei nicht vorhanden"
    if nur_online(p):
        return None, "Nur online (Dropbox) – keine Vorschau"
    try:
        jpeg, _, _ = vorschau_erzeugen(p, 1 if datei.ist_pdf else None)
        return jpeg, ""
    except Exception as ex:  # Anzeige darf nie abbrechen
        return None, f"Keine Vorschau: {type(ex).__name__}"


def protokoll_pfad(db_pfad: Path) -> Path:
    return db_pfad.with_name(PROTOKOLL_NAME)


def loeschen(db: sqlite3.Connection, gruppe: Gruppe, datei: Datei, protokoll: Path) -> None:
    """Datei vom Laufwerk löschen, dann ihre Einträge aus der DB entfernen, dann protokollieren.

    Schlägt das Löschen der Datei fehl, bleibt die DB unverändert (Ausnahme wird
    weitergereicht). Fehlt die Datei bereits, werden nur die Einträge entfernt.
    """
    if datei.ist_pdf:
        raise ValueError("PDFs werden in Bilderwerk Sweep nicht gelöscht.")
    try:
        os.remove(datei.pfad)
    except FileNotFoundError:
        pass
    try:
        eintraege_entfernen(db, datei.ids)
        db.commit()
    except Exception:
        db.rollback()
        raise
    gruppe.dateien.remove(datei)
    neu = not protokoll.exists()
    with protokoll.open("a", newline="", encoding="utf-8") as f:
        w = csv.writer(f, delimiter=";")
        if neu:
            w.writerow(PROTOKOLL_SPALTEN)
        w.writerow([datetime.now().isoformat(timespec="seconds"), datei.pfad, gruppe.hash,
                    datei.groesse, datei.breite, datei.hoehe,
                    " | ".join(d.pfad for d in gruppe.dateien)])


def groesse_text(n: int | None) -> str:
    if not n:
        return "–"
    for einheit in ("B", "KB", "MB", "GB"):
        if n < 1024 or einheit == "GB":
            return f"{n:.0f} {einheit}" if einheit == "B" else f"{n:.1f} {einheit}"
        n /= 1024
    return ""
