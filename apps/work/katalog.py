#!/usr/bin/env python3
"""Bilderkatalog – Batch-Lauf (Work).

Zweck:
    Durchläuft einen Bildordner rekursiv, analysiert jedes Bild bzw. jede
    PDF-Seite mit dem lokalen Modell (über bildanalyse.py), liest die
    Metadaten (metadaten.py) und speichert alles in einer SQLite-Datenbank
    mit Begriffstabelle und Volltextindex (FTS5, Trigramm).

Ziel:
    Eine durchsuchbare Katalog-Datenbank, die Bilderwerk View (../view)
    nur lesend nutzt. Das Schema hier ist der Vertrag zur App
    (Abgleich mit ../shared/schema.sql).

Eigenschaften:
    - Wiederanlauffähig für regelmäßige Läufe über das ganze Archiv:
      erledigte Einträge (Pfad + Größe + Änderungszeit) werden übersprungen,
      neue/geänderte analysiert, fehlende Vorschaubilder ergänzt und – nach
      einem vollständigen Lauf – Einträge entfernter Dateien gelöscht.
      Abbruch jederzeit mit Ctrl+C bzw. Stopp in der GUI.
    - Vorschaubilder (640 px) in Tabelle 'vorschauen' (Schema 4); ältere
      Datenbanken werden automatisch erweitert, nie gelöscht.
    - Duplikate (gleicher SHA-256) übernehmen das KI-Ergebnis des Originals.
    - Ist LM Studio nicht erreichbar, wartet der Lauf und versucht es erneut.
    - Optionale Sicherungskopie alle N Bilder (Dropbox-sicher, atomar).

Aufruf:
  python3 katalog.py ORDNER [--db DATEI] [--backup DATEI] [--protokoll DATEI]
                            [--limit N] [--backup-alle N] [--entfernen-erzwingen]

Beispiel:
  caffeinate -i python3 katalog.py "/Volumes/Fotos/Familie" \\
      --backup ~/Dropbox/Bildkatalog_Sicherung.db
  (Standard-Datenbank: Katalog-DB/Bildkatalog.db neben den Programmen)

Die Klasse Katalogisierer wird auch von katalog_gui.py genutzt.
"""
import argparse
import itertools
import hashlib
import json
import os
import sqlite3
import threading
import time
from datetime import datetime
from pathlib import Path
from typing import NamedTuple

import bildanalyse
import metadaten

SCHEMA_VERSION = 4        # 4 = Tabelle vorschauen (Vorschaubilder)
MIN_BYTES = 30_000       # kleinere Dateien (Thumbnails, Icons) ignorieren
ENDUNGEN = {".jpg", ".jpeg", ".png", ".heic", ".heif", ".tif", ".tiff", ".webp"}
PDF_ENDUNG = ".pdf"      # jede PDF-Seite wird ein eigener Eintrag
WARTEZEIT_SERVER_S = 60  # Wartezeit, wenn LM Studio nicht antwortet
BACKUP_ALLE = 1000       # Standard: Sicherungskopie nach je N neuen/geaenderten Eintraegen
ENTFERNEN_MAX_ANTEIL = 0.25  # Schutz: mehr fehlende Dateien gelten als Ablage-Problem

SQL_SCHEMA = """
CREATE TABLE IF NOT EXISTS bilder (
    id                INTEGER PRIMARY KEY,
    -- Datei und Ablage
    pfad              TEXT UNIQUE NOT NULL,   -- absoluter Pfad
    archiv_wurzel     TEXT,                   -- beim Lauf angegebener Ordner
    relativer_pfad    TEXT,                   -- Pfad relativ zur Wurzel
    ordner            TEXT,                   -- relativer Ordner
    dateiname         TEXT,
    endung            TEXT,
    hash              TEXT,
    groesse           INTEGER,
    mtime             REAL,
    seite             INTEGER,                -- nur bei PDF: Seitennummer
    seiten_gesamt     INTEGER,                -- nur bei PDF: Seitenzahl
    -- Bildeigenschaften (gemessen)
    breite            INTEGER,
    hoehe             INTEGER,
    ausrichtung       TEXT,
    farbgebung        TEXT,
    -- Metadaten (EXIF/IPTC/XMP)
    aufnahme_datum    TEXT,
    datum_quelle      TEXT,                   -- metadaten | dateiname
    kamera_hersteller TEXT,
    kamera_modell     TEXT,
    objektiv          TEXT,
    brennweite        REAL,
    brennweite_kb     REAL,
    blende            REAL,
    belichtungszeit   REAL,
    iso               REAL,
    blitz             TEXT,
    gps_lat           REAL,
    gps_lon           REAL,
    gps_hoehe         REAL,
    ort               TEXT,                   -- spaeter: Ortsname aus GPS
    software          TEXT,
    urheber           TEXT,
    copyright         TEXT,
    meta_titel        TEXT,
    meta_beschreibung TEXT,
    meta_stichworte   TEXT,
    metadaten_json    TEXT,                   -- ALLE Metadaten als JSON
    -- KI-Analyse
    kurztitel         TEXT,
    beschreibung      TEXT,
    bildart           TEXT,
    motiv             TEXT,
    motiv_roh         TEXT,
    anlass            TEXT,
    innen_aussen      TEXT,
    tageszeit         TEXT,
    jahreszeit        TEXT,
    wetter            TEXT,
    szene             TEXT,
    stimmung          TEXT,                   -- JSON-Liste
    personen_anzahl   INTEGER,
    personen_merkmale TEXT,                   -- JSON-Liste
    schlagworte       TEXT,                   -- JSON-Liste
    sichtbarer_text   TEXT,
    einzelbilder      TEXT,                   -- JSON-Liste (Albumseiten mit mehreren Fotos)
    -- Verwaltung
    status            TEXT,                   -- ok | duplikat | fehler
    fehler            TEXT,
    duplikat_von      INTEGER,
    modell            TEXT,
    prompt_version    INTEGER,
    tokens            INTEGER,
    dauer_s           REAL,
    erfasst           TEXT
);
CREATE INDEX IF NOT EXISTS idx_bilder_hash   ON bilder(hash);
CREATE INDEX IF NOT EXISTS idx_bilder_ordner ON bilder(ordner);
CREATE INDEX IF NOT EXISTS idx_bilder_datum  ON bilder(aufnahme_datum);

-- Einzelbegriffe fuer gezielte Filter (typ: schlagwort | merkmal | stimmung | stichwort)
CREATE TABLE IF NOT EXISTS begriffe (
    bild_id INTEGER NOT NULL,
    typ     TEXT NOT NULL,
    wert    TEXT NOT NULL,
    PRIMARY KEY (bild_id, typ, wert)
);
CREATE INDEX IF NOT EXISTS idx_begriffe_wert ON begriffe(wert);

-- Volltextsuche (Trigramm: findet auch Wortteile, z. B. 'landschaft' in 'Berglandschaft')
CREATE VIRTUAL TABLE IF NOT EXISTS suche USING fts5(
    ablage, ki_text, meta_text, tokenize = 'trigram'
);

-- Vorschaubilder (Schema 4). Schluessel = bilder.hash, Duplikate teilen sich eine Vorschau.
CREATE TABLE IF NOT EXISTS vorschauen (
    hash     TEXT PRIMARY KEY,   -- wie bilder.hash (PDF-Seite: '<hash>#<seite>')
    jpeg     BLOB NOT NULL,      -- JPEG, laengste Kante <= 640 px
    breite   INTEGER NOT NULL,   -- Masse der Vorschau (nach EXIF-Drehung)
    hoehe    INTEGER NOT NULL,
    version  INTEGER NOT NULL,   -- Verfahrensversion (bildanalyse.VORSCHAU_VERSION)
    erzeugt  TEXT NOT NULL       -- ISO-Zeitpunkt
) WITHOUT ROWID;
"""

KI_FELDER = ["kurztitel", "beschreibung", "bildart", "motiv", "motiv_roh", "anlass",
             "innen_aussen", "tageszeit", "jahreszeit", "wetter", "szene", "stimmung",
             "personen_anzahl", "personen_merkmale", "schlagworte", "sichtbarer_text",
             "einzelbilder", "farbgebung", "breite", "hoehe", "ausrichtung", "tokens",
             "dauer_s"]
LISTEN_FELDER = ("stimmung", "personen_merkmale", "schlagworte", "einzelbilder")


class Einheit(NamedTuple):
    """Ein zu katalogisierendes Element: eine Bilddatei oder eine PDF-Seite.

    Felder:
        datei:  Absoluter Pfad der Datei.
        seite:  Seitennummer bei PDFs, sonst None.
        seiten: Gesamtseitenzahl der PDF, sonst None.
    """
    datei: Path
    seite: int | None = None
    seiten: int | None = None

    @property
    def schluessel(self) -> str:
        """Eindeutiger Datenbank-Schlüssel (Spalte 'pfad').

        Rückgabe:
            Pfad bzw. '<Pfad>#seite=<n>' (siehe bildanalyse.eintrag_schluessel).
        """
        return bildanalyse.eintrag_schluessel(self.datei, self.seite)

    @property
    def name(self) -> str:
        """Anzeigename für Meldungen.

        Rückgabe:
            Dateiname, bei PDFs ergänzt um '[Seite n/m]'.
        """
        return (self.datei.name if self.seite is None
                else f"{self.datei.name} [Seite {self.seite}/{self.seiten}]")


# ------------------------------------------------------------------ Datenbank

def db_oeffnen(pfad: Path) -> sqlite3.Connection:
    """Katalog-Datenbank öffnen bzw. anlegen und Schema sicherstellen.

    Schaltet WAL ein (robuster bei Abbrüchen, gleichzeitiges Lesen möglich)
    und bringt ältere Datenbanken schrittweise auf die aktuelle Version –
    ohne vorhandene Daten zu löschen oder zu verändern:
    Version 2 → 3: Spalten für PDF-Seiten werden ergänzt.
    Version 3 → 4: Tabelle 'vorschauen' wird angelegt.
    Fehlende Tabellen/Indizes werden angelegt.

    Parameter:
        pfad: Pfad der Datenbankdatei (Ordner wird bei Bedarf angelegt).

    Rückgabe:
        Offene sqlite3-Verbindung (check_same_thread=False für die GUI).

    Fehler:
        SystemExit bei unbekannter (z. B. neuerer) Schema-Version – die Datei
        bleibt dann unangetastet.
    """
    pfad.parent.mkdir(parents=True, exist_ok=True)
    db = sqlite3.connect(pfad, check_same_thread=False)
    db.execute("PRAGMA journal_mode = WAL")      # robuster bei Abbruechen
    version = db.execute("PRAGMA user_version").fetchone()[0]
    alt = db.execute("SELECT 1 FROM sqlite_master WHERE name = 'bilder'").fetchone()
    if alt and version == 2:
        # Erweiterung fuer PDF-Seiten - vorhandene Eintraege bleiben unveraendert
        for spalte in ("seite INTEGER", "seiten_gesamt INTEGER", "einzelbilder TEXT"):
            db.execute(f"ALTER TABLE bilder ADD COLUMN {spalte}")
        db.commit()
    elif alt and version not in (3, SCHEMA_VERSION):
        raise SystemExit(
            f"Die Datenbank {pfad} hat ein unbekanntes Format (Version {version}).\n"
            f"Bitte mit --db eine andere Datei angeben.")
    db.executescript(SQL_SCHEMA)
    db.execute(f"PRAGMA user_version = {SCHEMA_VERSION}")
    return db


def speichern(db, daten: dict) -> int:
    """Datensatz in Tabelle 'bilder' einfügen oder aktualisieren (Upsert).

    Konflikt über 'pfad': vorhandene Einträge werden überschrieben. Listen
    werden als JSON-Text gespeichert. Kein Commit – das macht der Aufrufer.

    Parameter:
        db:    Offene Datenbankverbindung.
        daten: Spaltenname → Wert.

    Rückgabe:
        id des Datensatzes.
    """
    spalten = list(daten)
    sql = (f"INSERT INTO bilder ({', '.join(spalten)}) "
           f"VALUES ({', '.join('?' * len(spalten))}) "
           f"ON CONFLICT(pfad) DO UPDATE SET "
           + ", ".join(f"{s}=excluded.{s}" for s in spalten if s != "pfad")
           + " RETURNING id")
    werte = [json.dumps(daten[s], ensure_ascii=False) if isinstance(daten[s], list)
             else daten[s] for s in spalten]
    return db.execute(sql, werte).fetchone()[0]


def suchindex_aktualisieren(db, bild_id: int, d: dict):
    """Begriffstabelle und Volltextindex für ein Bild neu schreiben.

    Tabelle 'begriffe': Schlagworte, Personenmerkmale, Stimmung und Stichworte
    aus den Metadaten, jeweils kleingeschrieben.
    Volltext 'suche' (rowid = bilder.id): Spalten ablage, ki_text, meta_text.

    Parameter:
        db:      Offene Datenbankverbindung.
        bild_id: id des Bildes in 'bilder'.
        d:       Datensatz (Listenfelder als Python-Listen).

    Rückgabe:
        Keine.
    """
    db.execute("DELETE FROM begriffe WHERE bild_id = ?", (bild_id,))
    zeilen = []
    for typ, feld in (("schlagwort", "schlagworte"), ("merkmal", "personen_merkmale"),
                      ("stimmung", "stimmung")):
        zeilen += [(bild_id, typ, w.lower()) for w in d.get(feld) or []]
    for w in (d.get("meta_stichworte") or "").split(","):
        if w.strip():
            zeilen.append((bild_id, "stichwort", w.strip().lower()))
    db.executemany("INSERT OR IGNORE INTO begriffe VALUES (?, ?, ?)", zeilen)

    def text(*werte):
        """Werte zu einem Suchtext verbinden.

        Listen werden aufgelöst, leere Werte übersprungen.

        Parameter:
            *werte: Texte, Zahlen, Listen oder None.

        Rückgabe:
            Text mit ' | ' als Trenner.
        """
        teile = []
        for w in werte:
            if isinstance(w, list):
                teile.extend(str(x) for x in w)
            elif w not in (None, ""):
                teile.append(str(w))
        return " | ".join(teile)

    ablage = text(d.get("relativer_pfad", "").replace("/", " / "), d.get("dateiname"))
    ki = text(*(d.get(f) for f in ("kurztitel", "beschreibung", "bildart", "motiv", "motiv_roh",
                                   "anlass", "szene", "stimmung", "personen_merkmale",
                                   "schlagworte", "sichtbarer_text", "einzelbilder",
                                   "farbgebung",
                                   "jahreszeit", "wetter", "tageszeit")))
    meta = text(*(d.get(f) for f in ("kamera_hersteller", "kamera_modell", "objektiv",
                                     "urheber", "copyright", "meta_titel",
                                     "meta_beschreibung", "meta_stichworte", "ort",
                                     "aufnahme_datum")))
    db.execute("DELETE FROM suche WHERE rowid = ?", (bild_id,))
    db.execute("INSERT INTO suche (rowid, ablage, ki_text, meta_text) VALUES (?, ?, ?, ?)",
               (bild_id, ablage, ki, meta))


def backup_schreiben(db, ziel: Path):
    """Konsistente Sicherungskopie der Datenbank schreiben.

    Nutzt die SQLite-Backup-API in eine temporäre Datei und ersetzt das Ziel
    danach atomar – so sieht Dropbox nie eine halbfertige Datei.

    Parameter:
        db:   Offene Datenbankverbindung (Quelle).
        ziel: Pfad der Sicherungsdatei.

    Rückgabe:
        Keine.
    """
    ziel.parent.mkdir(parents=True, exist_ok=True)
    tmp = ziel.with_name("." + ziel.name + ".tmp")
    kopie = sqlite3.connect(tmp)
    with kopie:
        db.backup(kopie)
    kopie.close()
    os.replace(tmp, ziel)


def vorschau_speichern(db, hash_wert: str, vorschau: tuple[bytes, int, int]):
    """Vorschaubild in Tabelle 'vorschauen' schreiben (ersetzt eine vorhandene).

    Kein Commit – läuft in derselben Transaktion wie der bilder-Eintrag.

    Parameter:
        db:        Offene Datenbankverbindung.
        hash_wert: Schlüssel (= bilder.hash).
        vorschau:  Tupel (JPEG-Bytes, Breite, Höhe) aus bildanalyse.vorschau_erzeugen().

    Rückgabe:
        Keine.
    """
    jpeg, breite, hoehe = vorschau
    db.execute("INSERT OR REPLACE INTO vorschauen (hash, jpeg, breite, hoehe, version, erzeugt) "
               "VALUES (?, ?, ?, ?, ?, ?)",
               (hash_wert, jpeg, breite, hoehe, bildanalyse.VORSCHAU_VERSION,
                datetime.now().isoformat(timespec="seconds")))


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


# -------------------------------------------------------------------- Dateien

def bilder_finden(wurzel: Path, melden=print,
                  pdf_bekannt: dict | None = None) -> list[Einheit]:
    """Alle zu katalogisierenden Bilder und PDF-Seiten eines Ordners finden.

    Rekursiv; berücksichtigt ENDUNGEN und PDFs, ignoriert versteckte Dateien und
    Ordner (z. B. '._xyz', '.Trash') sowie Dateien unter MIN_BYTES. Jede
    PDF-Seite wird eine eigene Einheit (nur wenn poppler installiert ist).

    Bei unveränderten PDFs (gleiche Größe und Änderungszeit wie in der
    Datenbank) wird die Seitenzahl aus 'pdf_bekannt' übernommen – die PDF
    muss dann nicht geöffnet (bei Dropbox „nur online“: nicht geladen) werden.

    Parameter:
        wurzel:      Bildordner.
        melden:      Funktion für Warnungen (Standard: print).
        pdf_bekannt: {Dateipfad: (Größe, Änderungszeit, Seitenzahl)} oder None.

    Rückgabe:
        Liste von Einheit, sortiert nach Ordner, Dateiname und Seite.
    """
    dateien = []
    for p in wurzel.rglob("*"):
        if any(t.startswith(".") for t in p.relative_to(wurzel).parts):
            continue
        try:
            endung = p.suffix.lower()
            if ((endung in ENDUNGEN or endung == PDF_ENDUNG)
                    and p.is_file() and p.stat().st_size >= MIN_BYTES):
                dateien.append(p)
        except OSError:
            continue
    dateien.sort(key=lambda p: (str(p.parent).lower(), p.name.lower()))

    einheiten = []
    pdf_hinweis = False
    for p in dateien:
        if p.suffix.lower() != PDF_ENDUNG:
            einheiten.append(Einheit(p))
            continue
        if not (bildanalyse.PDFTOPPM and bildanalyse.PDFINFO):
            if not pdf_hinweis:
                melden("WARNUNG | PDF-Dateien werden übersprungen - poppler fehlt "
                       "(brew install poppler)")
                pdf_hinweis = True
            continue
        try:
            st = p.stat()
            bekannt = (pdf_bekannt or {}).get(str(p))
            if bekannt and bekannt[0] == st.st_size and bekannt[1] == st.st_mtime and bekannt[2]:
                n = bekannt[2]
            else:
                n = bildanalyse.pdf_seitenzahl(p)
        except Exception as ex:
            melden(f"WARNUNG | PDF übersprungen: {p.name} ({ex})")
            continue
        einheiten.extend(Einheit(p, s, n) for s in range(1, n + 1))
    return einheiten


def datei_hash(pfad: Path) -> str:
    """SHA-256-Prüfsumme einer Datei berechnen (blockweise, 1 MB).

    Dient der Duplikaterkennung.

    Parameter:
        pfad: Datei.

    Rückgabe:
        Hexadezimaler Hash als Text.
    """
    h = hashlib.sha256()
    with open(pfad, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


# ------------------------------------------------------------------ Protokoll

class Protokoll:
    """Einfaches Laufprotokoll als Textdatei (eine Zeile je Ereignis)."""
    def __init__(self, pfad: Path):
        """Protokoll anlegen.

        Parameter:
            pfad: Protokolldatei (Ordner wird bei Bedarf angelegt).
        """
        self.pfad = pfad
        pfad.parent.mkdir(parents=True, exist_ok=True)

    def schreiben(self, text: str):
        """Zeile mit Zeitstempel an das Protokoll anhängen.

        Parameter:
            text: Inhalt der Zeile.

        Rückgabe:
            Keine.
        """
        zeit = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        with open(self.pfad, "a", encoding="utf-8") as f:
            f.write(f"{zeit} | {text}\n")


# ----------------------------------------------------------------- Verarbeiten

# Ergebnis von bild_verarbeiten -> Zaehler in Katalogisierer.stand
ERGEBNIS_ZAEHLER = {"neu": "neu", "duplikat": "duplikate", "fehler": "fehler",
                    "uebersprungen": "uebersprungen", "vorschau": "vorschauen",
                    "nur_online": "nur_online", "vorschau_fehler": "vorschau_fehler"}


class Katalogisierer:
    """Kern des Katalogisierungslaufs.

    Wird vom Kommandozeilen-Aufruf und von der Oberfläche genutzt.
    Rückmeldungen laufen über 'melden', die Steuerung über die Events 'stopp'
    und 'pause' (threading.Event). Der aktuelle Stand steht in 'stand'
    (Zähler, aktueller Ordner/Datei, letztes Ergebnis) und wird von der GUI
    regelmäßig ausgelesen.
    """

    def __init__(self, wurzel: Path, db_pfad: Path, protokoll_pfad: Path,
                 backup_pfad: Path | None = None, backup_alle: int = BACKUP_ALLE,
                 limit: int = 0, melden=print, entfernen_erzwingen: bool = False):
        """Lauf vorbereiten: Datenbank öffnen, Protokoll anlegen, Stand initialisieren.

        Parameter:
            wurzel:         Bildordner.
            db_pfad:        Katalog-Datenbank.
            protokoll_pfad: Protokolldatei.
            backup_pfad:    Ziel der Sicherungskopie oder None.
            backup_alle:    Sicherung nach je N neuen Einträgen.
            limit:          Höchstzahl neu zu analysierender Bilder (0 = unbegrenzt).
            melden:         Funktion für Statusmeldungen (Standard: print).
            entfernen_erzwingen: Einträge fehlender Dateien auch dann entfernen,
                            wenn mehr als ENTFERNEN_MAX_ANTEIL fehlen.

        Hinweis: 'backup_alle' ist ein normales Attribut und darf während des
        Laufs geändert werden (die GUI tut das); es gilt ab dem nächsten Bild.

        Fehler:
            SystemExit, wenn Sicherung und Datenbank dieselbe Datei sind.
        """
        self.wurzel = wurzel.expanduser().resolve()
        self.db = db_oeffnen(db_pfad.expanduser())
        self.protokoll = Protokoll(protokoll_pfad.expanduser())
        self.backup_pfad = backup_pfad.expanduser() if backup_pfad else None
        if self.backup_pfad and self.backup_pfad.resolve() == db_pfad.expanduser().resolve():
            raise SystemExit("Sicherung und Datenbank dürfen nicht dieselbe Datei sein.")
        self.backup_alle = backup_alle
        self.limit = limit
        self.melden = melden
        self.entfernen_erzwingen = entfernen_erzwingen
        self._bestand: dict[str, tuple] = {}      # pfad -> (groesse, mtime, hash, status, id)
        self._vorschau_hashes: set[str] = set()
        self._datei_cache: dict[str, tuple[str, dict]] = {}
        self.stopp = threading.Event()
        self.pause = threading.Event()
        self.stand = {"gesamt": 0, "offen": 0, "position": 0, "neu": 0, "duplikate": 0,
                      "start": time.time(), "server_wartet": False,
                      "fehler": 0, "uebersprungen": 0, "summe_dauer": 0.0,
                      "vorschauen": 0, "vorschau_fehler": 0, "nur_online": 0,
                      "entfernt": 0,
                      "ordner": "", "datei": "", "letztes_ergebnis": None}

    def _protokollieren(self, text: str):
        """Text ins Protokoll schreiben und zusätzlich melden.

        Parameter:
            text: Meldung.

        Rückgabe:
            Keine.
        """
        self.protokoll.schreiben(text)
        self.melden(text)

    def _bestand_laden(self):
        """Bestand der Datenbank einmalig in den Speicher laden.

        Statt je Datei eine Abfrage zu stellen, wird für den Wiederholungslauf
        über das ganze Archiv einmal der Bestand gelesen: alle Einträge
        (Pfad, Größe, Änderungszeit, Hash, Status, id) und alle vorhandenen
        Vorschau-Schlüssel.

        Rückgabe:
            Keine (füllt self._bestand und self._vorschau_hashes).
        """
        self._bestand = {pfad: (groesse, mtime, h, status, i) for pfad, groesse, mtime, h, status, i
                         in self.db.execute("SELECT pfad, groesse, mtime, hash, status, id "
                                            "FROM bilder")}
        self._vorschau_hashes = {h for (h,) in self.db.execute("SELECT hash FROM vorschauen")}

    def _pdf_bekannt(self) -> dict:
        """Seitenzahlen unveränderter PDFs aus dem Bestand (für bilder_finden).

        Rückgabe:
            {Dateipfad: (Größe, Änderungszeit, Seitenzahl)}.
        """
        bekannt = {}
        for pfad, groesse, mtime, seiten in self.db.execute(
                "SELECT pfad, groesse, mtime, seiten_gesamt FROM bilder "
                "WHERE seite = 1 AND seiten_gesamt IS NOT NULL"):
            datei, _ = bildanalyse.quelle_aufloesen(pfad)
            bekannt[str(datei)] = (groesse, mtime, seiten)
        return bekannt

    def _offene_zaehlen(self, einheiten: list[Einheit]) -> int:
        """Zählen, wie viele Einheiten noch nicht erledigt sind (für die Restzeit).

        Erledigt = Pfad, Größe und Änderungszeit stehen mit Status 'ok' oder
        'duplikat' in der Datenbank.

        Parameter:
            einheiten: Alle gefundenen Einheiten.

        Rückgabe:
            Anzahl offener Einheiten.
        """
        offen = 0
        for e in einheiten:
            try:
                st = e.datei.stat()
            except OSError:
                continue
            if not self._erledigt(e, st):
                offen += 1
        return offen

    # -- einzelnes Bild ------------------------------------------------------
    def _erledigt(self, einheit: Einheit, st) -> bool:
        """Prüfen, ob eine Einheit unverändert bereits katalogisiert ist.

        Parameter:
            einheit: Zu prüfende Einheit.
            st:      os.stat_result der Datei (Größe, Änderungszeit).

        Rückgabe:
            True, wenn mit Status 'ok' oder 'duplikat' vorhanden.
        """
        z = self._bestand.get(einheit.schluessel)
        return (bool(z) and z[0] == st.st_size and z[1] == st.st_mtime
                and z[3] in ("ok", "duplikat"))

    def _vorschau_ergaenzen(self, einheit: Einheit) -> str:
        """Fehlende Vorschau zu einem bereits erledigten Eintrag nachholen.

        Dateien, die nur als Cloud-Platzhalter vorliegen, werden nicht geladen,
        sondern übersprungen und gezählt (Nachholen später, wenn lokal).

        Parameter:
            einheit: Bereits katalogisierte Einheit.

        Rückgabe:
            'uebersprungen' (Vorschau vorhanden), 'vorschau' (ergänzt),
            'nur_online' oder 'vorschau_fehler'.
        """
        h = self._bestand[einheit.schluessel][2]
        if not h or h in self._vorschau_hashes:
            return "uebersprungen"
        if bildanalyse.nur_online(einheit.datei):
            return "nur_online"
        try:
            vorschau_speichern(self.db, h, bildanalyse.vorschau_erzeugen(einheit.datei,
                                                                         einheit.seite))
            self.db.commit()
            self._vorschau_hashes.add(h)
            return "vorschau"
        except Exception as ex:
            self._protokollieren(f"VORSCHAU| {einheit.datei.name} | "
                                 f"{type(ex).__name__}: {ex}"[:300])
            return "vorschau_fehler"

    def _vorschau_mit(self, hash_wert: str, einheit: Einheit, name: str):
        """Vorschau für einen neuen Eintrag erzeugen und (ohne Commit) speichern.

        Ist der Hash schon vorhanden (Duplikat), passiert nichts. Fehler werden
        protokolliert und verhindern den Katalog-Eintrag nicht.

        Parameter:
            hash_wert: Schlüssel (= bilder.hash).
            einheit:   Zu verarbeitende Einheit.
            name:      Anzeigename für das Protokoll.

        Rückgabe:
            Keine.
        """
        if hash_wert in self._vorschau_hashes:
            return
        try:
            vorschau_speichern(self.db, hash_wert,
                               bildanalyse.vorschau_erzeugen(einheit.datei, einheit.seite))
            self._vorschau_hashes.add(hash_wert)
        except Exception as ex:
            self.stand["vorschau_fehler"] += 1
            self._protokollieren(f"VORSCHAU| {name} | {type(ex).__name__}: {ex}"[:300])

    def _hash_und_meta(self, datei: Path) -> tuple[str, dict]:
        """Hash und Metadaten einer Datei bestimmen – je Datei nur einmal.

        Zwischengespeichert für die zuletzt gelesene Datei; wichtig bei großen PDFs
        mit vielen Seiten.

        Parameter:
            datei: Datei.

        Rückgabe:
            Tupel (SHA-256 als Text, Metadaten-Dictionary).
        """
        schluessel = str(datei)
        if schluessel not in self._datei_cache:
            self._datei_cache = {schluessel: (datei_hash(datei), metadaten.lesen(datei))}
        return self._datei_cache[schluessel]

    def _basisdaten(self, einheit: Einheit, st) -> dict:
        """Grunddatensatz einer Einheit ohne KI-Felder zusammenstellen.

        Enthält Ablage (Pfade, Ordner, Name), Hash (bei PDF-Seiten '<hash>#<seite>'),
        Größe, Änderungszeit, Modell, Prompt-Version, Erfassungszeit und Metadaten.

        Parameter:
            einheit: Einheit.
            st:      os.stat_result der Datei.

        Rückgabe:
            Dictionary mit Spaltennamen der Tabelle 'bilder'.
        """
        pfad = einheit.datei
        rel = pfad.relative_to(self.wurzel)
        datei_hash_wert, meta = self._hash_und_meta(pfad)
        if einheit.seite is not None:
            rel_text = f"{rel} [Seite {einheit.seite}/{einheit.seiten}]"
            hash_wert = f"{datei_hash_wert}#{einheit.seite}"
        else:
            rel_text, hash_wert = str(rel), datei_hash_wert
        d = {
            "pfad": einheit.schluessel, "archiv_wurzel": str(self.wurzel),
            "relativer_pfad": rel_text,
            "ordner": str(rel.parent) if str(rel.parent) != "." else "",
            "dateiname": pfad.name, "endung": pfad.suffix.lower(),
            "seite": einheit.seite, "seiten_gesamt": einheit.seiten,
            "hash": hash_wert, "groesse": st.st_size, "mtime": st.st_mtime,
            "modell": bildanalyse.MODEL, "prompt_version": bildanalyse.PROMPT_VERSION,
            "erfasst": datetime.now().isoformat(timespec="seconds"),
            "fehler": None, "duplikat_von": None,
        }
        d.update(meta)
        return d

    def _analysieren_mit_wiederholung(self, einheit: Einheit) -> dict:
        """Einheit analysieren und warten, solange LM Studio nicht erreichbar ist.

        Statt einen Fehler zu buchen, wird alle WARTEZEIT_SERVER_S Sekunden erneut
        versucht; stand['server_wartet'] zeigt den Zustand in der GUI an.

        Parameter:
            einheit: Zu analysierende Einheit.

        Rückgabe:
            Ergebnis-Dictionary von bildanalyse.analysieren().

        Fehler:
            KeyboardInterrupt, wenn während des Wartens gestoppt wird.
        """
        while True:
            try:
                ergebnis = bildanalyse.analysieren(einheit.datei, einheit.seite, einheit.seiten)
                self.stand["server_wartet"] = False
                return ergebnis
            except bildanalyse.ServerNichtErreichbar:
                self.stand["server_wartet"] = True
                self.melden(f"LM Studio nicht erreichbar - neuer Versuch in "
                            f"{WARTEZEIT_SERVER_S} s (Server gestartet?)")
                if self.stopp.wait(WARTEZEIT_SERVER_S):
                    raise KeyboardInterrupt

    def bild_verarbeiten(self, einheit: Einheit) -> str:
        """Eine Einheit vollständig verarbeiten und speichern.

        Ablauf: schon erledigt? → nur fehlende Vorschau ergänzen. Gleicher Hash
        bereits analysiert? → als Duplikat mit übernommenen KI-Feldern speichern.
        Sonst analysieren und speichern; Fehler werden mit Status 'fehler'
        gebucht und protokolliert. Vorschau und Eintrag werden in derselben
        Transaktion geschrieben; ein Vorschau-Fehler verhindert den Eintrag nicht.

        Parameter:
            einheit: Zu verarbeitende Einheit.

        Rückgabe:
            'neu' | 'duplikat' | 'fehler' | 'uebersprungen' | 'vorschau' |
            'nur_online' | 'vorschau_fehler'.

        Fehler:
            KeyboardInterrupt wird durchgereicht (Abbruch).
        """
        st = einheit.datei.stat()
        if self._erledigt(einheit, st):
            return self._vorschau_ergaenzen(einheit)
        d = self._basisdaten(einheit, st)

        original = self.db.execute(
            f"SELECT id, {', '.join(KI_FELDER)} FROM bilder "
            f"WHERE hash = ? AND status = 'ok' AND pfad <> ?", (d["hash"], d["pfad"])).fetchone()
        if original:
            # Duplikat: KI-Ergebnis vom Original uebernehmen, eigene Ablage behalten
            for feld, wert in zip(KI_FELDER, original[1:]):
                d[feld] = json.loads(wert) if feld in LISTEN_FELDER and wert else wert
            d.update(status="duplikat", duplikat_von=original[0], dauer_s=0)
            bild_id = speichern(self.db, d)
            suchindex_aktualisieren(self.db, bild_id, d)
            self._vorschau_mit(d["hash"], einheit, d["relativer_pfad"])
            self.db.commit()
            return "duplikat"

        try:
            e = self._analysieren_mit_wiederholung(einheit)
            if e.get("versuche", 1) > 1:
                self._protokollieren(f"WIEDERHOLT | {d['relativer_pfad']} | "
                                     f"erfolgreich im {e['versuche']}. Versuch")
            d.update({k: e.get(k) for k in KI_FELDER})
            d["status"] = "ok"
            bild_id = speichern(self.db, d)
            suchindex_aktualisieren(self.db, bild_id, d)
            self._vorschau_mit(d["hash"], einheit, d["relativer_pfad"])
            self.stand["summe_dauer"] += e["dauer_s"]
            self.stand["letztes_ergebnis"] = d
            ergebnis = "neu"
        except KeyboardInterrupt:
            raise
        except Exception as ex:
            d.update(status="fehler", fehler=f"{type(ex).__name__}: {ex}"[:500])
            speichern(self.db, d)
            self._protokollieren(f"FEHLER  | {d['relativer_pfad']} | {d['fehler'][:200]}")
            ergebnis = "fehler"
        self.db.commit()
        return ergebnis

    # -- ganzer Lauf ---------------------------------------------------------
    def ausfuehren(self):
        """Den gesamten Lauf ausführen.

        Wiederanlauffähig und für Wiederholungsläufe über das ganze Archiv
        gedacht: Bestand einmal laden, alle Einheiten finden, ordnerweise
        verarbeiten (neue/geänderte analysieren, erledigte nur um fehlende
        Vorschauen ergänzen), Pause/Stopp/Limit beachten, je Ordner eine
        Protokollzeile, Sicherung nach je 'backup_alle' Änderungen. Nach einem
        vollständigen Lauf werden Einträge entfernter Dateien gelöscht.
        Am Ende (auch bei Abbruch) wird committet, IMMER gesichert, eine
        Zusammenfassung protokolliert und die Datenbank geschlossen.

        Rückgabe:
            Keine.
        """
        s = self.stand
        try:
            import numpy, PIL  # noqa: F401  (fuer die Farbmessung noetig)
        except ImportError:
            import sys
            self._protokollieren(f"WARNUNG | Pillow/numpy fehlen in diesem Python - Farbgebung "
                                 f"wird nicht gemessen. Python: {sys.executable}")
        self._bestand_laden()
        self.melden(f"Suche Bilder in {self.wurzel} ...")
        dateien = bilder_finden(self.wurzel, melden=self._protokollieren,
                                pdf_bekannt=self._pdf_bekannt())
        s["gesamt"] = len(dateien)
        s["offen"] = self._offene_zaehlen(dateien)
        s["start"] = time.time()
        pdf_seiten = sum(1 for e in dateien if e.seite is not None)
        self.melden(f"{len(dateien)} Bilder gefunden"
                    + (f" (davon {pdf_seiten} PDF-Seiten)" if pdf_seiten else "")
                    + f", noch offen: {s['offen']}.")
        self._protokollieren(f"=== START | Wurzel: {self.wurzel} | Bilder: {len(dateien)} | "
                             f"offen: {s['offen']} | in der DB: {len(self._bestand)}")

        seit_backup = 0
        abbruch_grund = None
        vollstaendig = False
        try:
            for ordner, gruppe in itertools.groupby(dateien, key=lambda e: e.datei.parent):
                gruppe = list(gruppe)
                rel_ordner = (str(ordner.relative_to(self.wurzel)) if ordner != self.wurzel
                              else "(Hauptordner)")
                s["ordner"] = rel_ordner
                zaehler = dict.fromkeys(ERGEBNIS_ZAEHLER, 0)
                beendet = True

                for einheit in gruppe:
                    while self.pause.is_set() and not self.stopp.is_set():
                        s["pausiert"] = True
                        time.sleep(0.5)
                    s["pausiert"] = False
                    if self.stopp.is_set():
                        abbruch_grund = "gestoppt"
                        raise KeyboardInterrupt
                    if self.limit and s["neu"] >= self.limit:
                        abbruch_grund = f"Limit {self.limit} erreicht"
                        beendet = False
                        break

                    s["position"] += 1
                    s["datei"] = einheit.name
                    try:
                        ergebnis = self.bild_verarbeiten(einheit)
                    except FileNotFoundError:          # waehrend des Laufs geloescht
                        ergebnis = "uebersprungen"
                    zaehler[ergebnis] += 1
                    s[ERGEBNIS_ZAEHLER[ergebnis]] += 1
                    if ergebnis in ("neu", "duplikat", "fehler"):
                        self._fortschritt(einheit, ergebnis)
                    if ergebnis in ("neu", "duplikat", "vorschau"):
                        seit_backup += 1
                        # backup_alle bei jedem Bild neu lesen: in der GUI live aenderbar
                        if self.backup_pfad and seit_backup >= max(1, int(self.backup_alle)):
                            backup_schreiben(self.db, self.backup_pfad)
                            seit_backup = 0

                zeile = (f"{'FERTIG ' if beendet else 'TEILWEISE'} | {rel_ordner} | "
                         f"Bilder: {len(gruppe)} | neu: {zaehler['neu']} | "
                         f"Duplikate: {zaehler['duplikat']} | Fehler: {zaehler['fehler']} | "
                         f"Vorschau ergänzt: {zaehler['vorschau']} | "
                         f"nur online: {zaehler['nur_online']} | "
                         f"schon erledigt: {zaehler['uebersprungen']}")
                if any(v for k, v in zaehler.items() if k != "uebersprungen"):
                    self._protokollieren(zeile)
                if not beendet:
                    break
            else:
                vollstaendig = True
            if vollstaendig:
                self._fehlende_entfernen(dateien)
        except KeyboardInterrupt:
            abbruch_grund = abbruch_grund or "abgebrochen (Ctrl+C)"
            self._protokollieren(f"ABBRUCH | {s['ordner']} | bei Datei: {s['datei']}")
        finally:
            self.db.commit()
            # Abschluss-Sicherung IMMER (auch bei Stopp/Abbruch und wenn das
            # Intervall nicht erreicht wurde) - Fehler duerfen das Schliessen nicht verhindern
            if self.backup_pfad:
                try:
                    backup_schreiben(self.db, self.backup_pfad)
                    self._protokollieren(f"SICHERUNG | {self.backup_pfad}")
                except Exception as ex:
                    self._protokollieren(f"FEHLER  | Sicherung fehlgeschlagen: {ex}")
            zusammenfassung = (f"neu: {s['neu']} | Duplikate: {s['duplikate']} | "
                               f"Fehler: {s['fehler']} | Vorschauen ergänzt: {s['vorschauen']} | "
                               f"Vorschau-Fehler: {s['vorschau_fehler']} | "
                               f"nur online (ohne Vorschau): {s['nur_online']} | "
                               f"entfernt: {s['entfernt']} | "
                               f"schon erledigt: {s['uebersprungen']}")
            self.protokoll.schreiben(f"=== ENDE ({abbruch_grund or 'vollständig'}) | "
                                     + zusammenfassung)
            self.melden(f"{abbruch_grund or 'Fertig'} - {zusammenfassung}")
            self.db.close()

    def _fehlende_entfernen(self, dateien: list[Einheit]):
        """Einträge entfernen, deren Datei im durchsuchten Ordner nicht mehr existiert.

        Läuft nur am Ende eines vollständigen Laufs (nicht bei Stopp/Limit) –
        so werden verschobene Dateien vorher als Duplikat erkannt und behalten
        ihre KI-Analyse. Betrachtet werden nur Einträge unterhalb des
        durchsuchten Ordners. Schutz gegen nicht eingebundene Laufwerke oder
        Dropbox-Probleme: Wurden keine Dateien gefunden oder fehlen mehr als
        ENTFERNEN_MAX_ANTEIL der Einträge, wird nichts entfernt (außer mit
        'entfernen_erzwingen'). Danach werden verwaiste Vorschauen gelöscht.

        Parameter:
            dateien: Alle in diesem Lauf gefundenen Einheiten.

        Rückgabe:
            Keine (Ergebnis in stand['entfernt'] und im Protokoll).
        """
        gefunden = {e.schluessel for e in dateien}
        praefix = str(self.wurzel).rstrip("/") + "/"
        unter_wurzel = {pfad: z[4] for pfad, z in self._bestand.items()
                        if pfad.startswith(praefix)}
        fehlend = [i for pfad, i in unter_wurzel.items() if pfad not in gefunden]
        if not fehlend:
            return
        anteil = len(fehlend) / len(unter_wurzel)
        if not self.entfernen_erzwingen and (not dateien or anteil > ENTFERNEN_MAX_ANTEIL):
            self._protokollieren(
                f"WARNUNG | {len(fehlend)} von {len(unter_wurzel)} Einträgen ({anteil:.0%}) "
                f"haben keine Datei mehr - NICHT entfernt (Schutz). Ordner/Laufwerk prüfen; "
                f"bewusst entfernen mit --entfernen-erzwingen.")
            return
        self.melden(f"Entferne {len(fehlend)} Einträge, deren Datei nicht mehr existiert ...")
        for pfad, i in unter_wurzel.items():
            if pfad not in gefunden:
                self.protokoll.schreiben(f"ENTFERNT| {pfad}")
        self.stand["entfernt"] = eintraege_entfernen(self.db, fehlend)
        self.db.execute("DELETE FROM vorschauen WHERE hash NOT IN "
                        "(SELECT hash FROM bilder WHERE hash IS NOT NULL)")
        self.db.commit()
        self._protokollieren(f"ENTFERNT| {self.stand['entfernt']} Einträge ohne Datei entfernt")

    def noch_offen(self) -> int:
        """Anzahl der im laufenden Lauf noch offenen Einheiten.

        Rückgabe:
            Offene Einheiten (mindestens 0).
        """
        s = self.stand
        return max(0, s["offen"] - s["neu"] - s["duplikate"] - s["fehler"])

    def _fortschritt(self, einheit: Einheit, ergebnis: str):
        """Fortschrittsmeldung zur gerade verarbeiteten Einheit ausgeben.

        Enthält Position, Ergebnis (Kurztitel, Dauer, Durchschnitt) und die
        geschätzte Restzeit.

        Parameter:
            einheit:  Verarbeitete Einheit.
            ergebnis: 'neu' | 'duplikat' | 'fehler'.

        Rückgabe:
            Keine.
        """
        s = self.stand
        schnitt = s["summe_dauer"] / s["neu"] if s["neu"] else 0
        rest_h = schnitt * self.noch_offen() / 3600
        info = ""
        if ergebnis == "neu" and s["letztes_ergebnis"]:
            e = s["letztes_ergebnis"]
            info = f"{e.get('kurztitel')} ({e.get('dauer_s', 0):.0f}s | Ø {schnitt:.0f}s)"
        elif ergebnis == "duplikat":
            info = "Duplikat"
        else:
            info = "FEHLER (siehe Protokoll)"
        ort = einheit.datei.parent.relative_to(self.wurzel) / einheit.name
        self.melden(f"[{s['position']}/{s['gesamt']}] {ort}: {info}"
                    f" | Rest ca. {rest_h:.1f} h")


# ----------------------------------------------------------------------- Main

def main():
    """Kommandozeilen-Einstieg: Argumente lesen, Hinweise ausgeben, Lauf starten.

    Warnt, wenn die Datenbank in Dropbox liegt oder ExifTool fehlt. Das
    Protokoll liegt standardmäßig neben der Datenbank (<Name>_protokoll.txt).

    Rückgabe:
        Keine.
    """
    ap = argparse.ArgumentParser(description="Bilderkatalog mit lokalem KI-Modell")
    ap.add_argument("ordner", type=Path)
    ap.add_argument("--db", type=Path, default=bildanalyse.STANDARD_DB)
    ap.add_argument("--protokoll", type=Path, default=None,
                    help="Protokolldatei (Standard: neben der Datenbank)")
    ap.add_argument("--backup", type=Path, default=None,
                    help="Sicherungskopie, z. B. im Dropbox-Ordner")
    ap.add_argument("--backup-alle", type=int, default=BACKUP_ALLE,
                    help=f"Sicherung nach je N neuen Bildern (Standard {BACKUP_ALLE})")
    ap.add_argument("--entfernen-erzwingen", action="store_true",
                    help="Einträge fehlender Dateien auch bei großem Anteil entfernen")
    ap.add_argument("--limit", type=int, default=0, help="max. Anzahl neu zu analysierender Bilder")
    args = ap.parse_args()

    db_pfad = args.db.expanduser()
    if "dropbox" in str(db_pfad).lower():
        print("WARNUNG: Die Datenbank liegt in Dropbox. Empfehlung: lokal ablegen und "
              "mit --backup in Dropbox sichern.\n")
    if not metadaten.EXIFTOOL:
        print("Hinweis: ExifTool nicht gefunden - nur Basis-Metadaten, kein HEIC. "
              "Installation: brew install exiftool\n")
    protokoll = args.protokoll or db_pfad.with_name(db_pfad.stem + "_protokoll.txt")

    Katalogisierer(args.ordner, db_pfad, protokoll, args.backup,
                   args.backup_alle, args.limit,
                   entfernen_erzwingen=args.entfernen_erzwingen).ausfuehren()


if __name__ == "__main__":
    main()
