"""Gemeinsame Einstellungen von Bilderwerk Work und Sweep.

Zweck:
    Eine Einstellungsdatei im Benutzerordner statt Dateien im Programmordner.
    Work und Sweep teilen sich den Datenbankpfad; jedes Modul hat einen eigenen
    Abschnitt. View ist eine eigenständige Anwendung und hat eigene Einstellungen.

Ort:
    macOS:  ~/Library/Application Support/Bilderwerk/einstellungen.json
    sonst:  ~/.config/bilderwerk/einstellungen.json
    Abweichend per Umgebungsvariable BILDERWERK_EINSTELLUNGEN (Pfad der Datei).

Aufbau (Version 1):
    {
      "version": 1,
      "db": "/Pfad/Bildkatalog.db",            – gemeinsam für Work und Sweep
      "work":  {"ordner", "backup", "backup_alle", "wach",
                "ki": {"url", "modell", "schluessel"}},
      "sweep": {}
    }

Migration:
    Fehlt die Datei, werden beim ersten Laden die Werte aus der bisherigen
    apps/work/gui_einstellungen.json übernommen (die alte Datei bleibt liegen).
"""
from __future__ import annotations

import copy
import json
import os
import sys
import tempfile
from pathlib import Path

VERSION = 1
DATEINAME = "einstellungen.json"
DB_NAME = "Bildkatalog.db"

# Standard für den KI-Server: LM Studio auf demselben Rechner
KI_STANDARD_URL = "http://127.0.0.1:1234/v1"

# Werte vor v0.2 (fest im Code) – nur für die Migration
_ALT_KI_URL = KI_STANDARD_URL           # war …/v1/chat/completions, gespeichert wird die Basis
_ALT_KI_MODELL = "qwen/qwen3.5-9b"

STANDARD = {
    "version": VERSION,
    "db": "",
    "work": {
        "ordner": "",
        "backup": "",
        "backup_alle": 1000,
        "wach": True,
        "ki": {"url": KI_STANDARD_URL, "modell": "", "schluessel": ""},
    },
    "sweep": {},
}


def ordner() -> Path:
    """Ordner der Einstellungsdatei (und der Standard-Datenbank).

    Rückgabe:
        Pfad des Ordners (wird nicht angelegt).
    """
    if os.environ.get("BILDERWERK_EINSTELLUNGEN"):
        return Path(os.environ["BILDERWERK_EINSTELLUNGEN"]).expanduser().parent
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Application Support" / "Bilderwerk"
    basis = os.environ.get("XDG_CONFIG_HOME") or str(Path.home() / ".config")
    return Path(basis) / "bilderwerk"


def datei() -> Path:
    """Pfad der Einstellungsdatei.

    Rückgabe:
        Pfad (die Datei muss nicht existieren).
    """
    if os.environ.get("BILDERWERK_EINSTELLUNGEN"):
        return Path(os.environ["BILDERWERK_EINSTELLUNGEN"]).expanduser()
    return ordner() / DATEINAME


def standard_db() -> Path:
    """Vorschlag für die Datenbank, wenn keine eingestellt ist.

    Rückgabe:
        <Einstellungsordner>/Bildkatalog.db
    """
    return ordner() / DB_NAME


def ki_url_normalisieren(url: str) -> str:
    """KI-Adresse auf die Basis kürzen ('…/v1/chat/completions' → '…/v1').

    Der Pfad zur Chat- bzw. Modellliste wird beim Aufruf ergänzt
    (bildanalyse.endpunkte); gespeichert und angezeigt wird nur die Basis.

    Parameter:
        url: Eingetragene Adresse.

    Rückgabe:
        Adresse ohne abschließendes '/', '/chat/completions' bzw. '/models'.
    """
    u = (url or "").strip().rstrip("/")
    for endung in ("/chat/completions", "/models"):
        if u.endswith(endung):
            u = u[: -len(endung)]
    return u


def _normalisieren(werte: dict) -> dict:
    """Gelesene bzw. zu speichernde Werte vereinheitlichen (derzeit: KI-Adresse).

    Parameter:
        werte: Einstellungen (werden direkt verändert).

    Rückgabe:
        Dieselben Einstellungen.
    """
    ki = werte.get("work", {}).get("ki")
    if isinstance(ki, dict) and isinstance(ki.get("url"), str):
        ki["url"] = ki_url_normalisieren(ki["url"])
    return werte


def _mischen(basis: dict, werte: dict) -> dict:
    """Werte rekursiv über die Standardwerte legen (unbekannte Schlüssel bleiben erhalten).

    Parameter:
        basis: Standardwerte (wird nicht verändert).
        werte: Gelesene Werte.

    Rückgabe:
        Neues, zusammengeführtes Dictionary.
    """
    ergebnis = copy.deepcopy(basis)
    for k, v in (werte or {}).items():
        if isinstance(v, dict) and isinstance(ergebnis.get(k), dict):
            ergebnis[k] = _mischen(ergebnis[k], v)
        else:
            ergebnis[k] = v
    return ergebnis


def aus_alt_uebernehmen(alt: dict) -> dict:
    """Einstellungen im Format vor v0.2 (gui_einstellungen.json) umsetzen.

    Damals waren KI-Server und Modell fest im Code; sie werden als Werte
    übernommen, damit sich am bisherigen Verhalten nichts ändert.

    Parameter:
        alt: Inhalt der alten gui_einstellungen.json.

    Rückgabe:
        Einstellungen im neuen Format (vollständig, mit Standardwerten).
    """
    neu = copy.deepcopy(STANDARD)
    neu["db"] = alt.get("db", "") or ""
    for k in ("ordner", "backup", "backup_alle", "wach"):
        if k in alt:
            neu["work"][k] = alt[k]
    neu["work"]["ki"] = {"url": _ALT_KI_URL, "modell": _ALT_KI_MODELL, "schluessel": ""}
    return neu


def laden(alt_datei: Path | None = None) -> dict:
    """Einstellungen lesen – fehlende Werte mit Standardwerten ergänzt.

    Gibt es noch keine Datei, aber eine alte gui_einstellungen.json, werden
    deren Werte übernommen und sofort in die neue Datei geschrieben.

    Parameter:
        alt_datei: Pfad der alten gui_einstellungen.json (nur für die Migration).

    Rückgabe:
        Einstellungen als Dictionary. Eine defekte Datei ergibt die Standardwerte
        (sie wird nicht überschrieben, bis gespeichert wird).
    """
    pfad = datei()
    if pfad.exists():
        try:
            return _normalisieren(_mischen(STANDARD, json.loads(pfad.read_text(encoding="utf-8"))))
        except (OSError, ValueError):
            return copy.deepcopy(STANDARD)
    if alt_datei and Path(alt_datei).is_file():
        try:
            neu = aus_alt_uebernehmen(json.loads(Path(alt_datei).read_text(encoding="utf-8")))
        except (OSError, ValueError):
            return copy.deepcopy(STANDARD)
        try:
            speichern(neu)
        except OSError:
            pass
        return neu
    return copy.deepcopy(STANDARD)


def speichern(werte: dict) -> None:
    """Einstellungen schreiben (atomar über eine Hilfsdatei).

    Parameter:
        werte: Vollständige Einstellungen (wie von laden()).

    Fehler:
        OSError, wenn der Ordner nicht beschreibbar ist.
    """
    pfad = datei()
    pfad.parent.mkdir(parents=True, exist_ok=True)
    werte = _normalisieren(copy.deepcopy(dict(werte, version=VERSION)))
    fd, tmp = tempfile.mkstemp(dir=pfad.parent, prefix=".einstellungen-", suffix=".json")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            json.dump(werte, f, indent=2, ensure_ascii=False)
        os.replace(tmp, pfad)
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)


def db_pfad(werte: dict) -> Path:
    """Eingestellte Datenbank, sonst der Standardvorschlag.

    Parameter:
        werte: Einstellungen.

    Rückgabe:
        Pfad der Datenbank (muss nicht existieren).
    """
    text = (werte.get("db") or "").strip()
    return Path(text).expanduser() if text else standard_db()
