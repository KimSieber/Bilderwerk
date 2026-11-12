"""Tests für den Datenzugriff von Sweep (daten.py) – ohne Oberfläche.

Aufruf (in apps/sweep):  python3 -m unittest discover -s tests
Die Test-Datenbank wird mit Work (katalog.db_oeffnen) angelegt.
"""
import csv
import os
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

_TMP = tempfile.TemporaryDirectory()
os.environ["BILDERWERK_EINSTELLUNGEN"] = str(Path(_TMP.name) / "conf" / "einstellungen.json")
HIER = Path(__file__).resolve().parent
sys.path.insert(0, str(HIER.parent))
sys.path.insert(0, str(HIER.parent.parent / "work"))
import daten    # noqa: E402
import katalog  # noqa: E402  (nur zum Anlegen der Test-DB)


class DatenTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        t = Path(self.tmp.name)
        self.db_pfad = t / "Bildkatalog.db"
        db = katalog.db_oeffnen(self.db_pfad)
        self.a, self.b, self.c = t / "a.jpg", t / "b.jpg", t / "c.jpg"
        for p in (self.a, self.b, self.c):
            p.write_bytes(b"x")
        zeilen = [(1, str(self.a), "h1", "ok", None), (2, str(self.b), "h1", "duplikat", 1),
                  (3, str(self.c), "h2", "ok", None),
                  (4, f"{t}/album.pdf#seite=1", "p1#1", "ok", None),
                  (5, f"{t}/kopie.pdf#seite=1", "p1#1", "duplikat", 4)]
        for i, pfad, h, status, von in zeilen:
            seite = 1 if "#seite=" in pfad else None
            db.execute("INSERT INTO bilder (id, pfad, hash, status, duplikat_von, groesse, seite) "
                       "VALUES (?, ?, ?, ?, ?, 100, ?)", (i, pfad, h, status, von, seite))
        db.execute("INSERT INTO vorschauen (hash, jpeg, breite, hoehe, version, erzeugt) "
                   "VALUES ('h1', x'FFD8', 1, 1, 1, '2026-01-01')")
        db.commit()
        db.close()
        self.db = daten.verbinden(self.db_pfad)

    def tearDown(self):
        self.db.close()
        self.tmp.cleanup()

    def test_gruppen(self):
        gruppen = daten.gruppen_laden(self.db)
        self.assertEqual(len(gruppen), 2)                  # Bilder h1, PDF p1; h2 ist allein
        bild = [g for g in gruppen if not g.ist_pdf][0]
        self.assertEqual({d.pfad for d in bild.dateien}, {str(self.a), str(self.b)})
        self.assertTrue(gruppen[-1].ist_pdf)               # PDFs am Ende
        self.assertEqual(daten.vorschau(self.db, bild.dateien[0])[0], b"\xff\xd8")

    def test_loeschen_original_nachfolge_und_protokoll(self):
        g = [g for g in daten.gruppen_laden(self.db) if not g.ist_pdf][0]
        original = [d for d in g.dateien if d.pfad == str(self.a)][0]
        protokoll = daten.protokoll_pfad(self.db_pfad)
        daten.loeschen(self.db, g, original, protokoll)
        self.assertFalse(self.a.exists())
        self.assertEqual(self.db.execute("SELECT status, duplikat_von FROM bilder WHERE id = 2")
                         .fetchone(), ("ok", None))
        self.assertIsNone(self.db.execute("SELECT 1 FROM bilder WHERE id = 1").fetchone())
        self.assertIsNotNone(self.db.execute("SELECT 1 FROM vorschauen WHERE hash='h1'").fetchone())
        with protokoll.open(encoding="utf-8") as f:
            zeilen = list(csv.reader(f, delimiter=";"))
        self.assertEqual(zeilen[0], daten.PROTOKOLL_SPALTEN)
        self.assertEqual(zeilen[1][1], str(self.a))

    def test_pdf_nicht_loeschbar(self):
        g = [g for g in daten.gruppen_laden(self.db) if g.ist_pdf][0]
        with self.assertRaises(ValueError):
            daten.loeschen(self.db, g, g.dateien[0], daten.protokoll_pfad(self.db_pfad))

    def test_fremde_datei_abgelehnt(self):
        fremd = Path(self.tmp.name) / "fremd.db"
        sqlite3.connect(fremd).close()
        with self.assertRaises(ValueError):
            daten.verbinden(fremd)

    def test_db_merken_und_standard(self):
        daten.db_merken(self.db_pfad)
        self.assertEqual(daten.standard_db(), self.db_pfad)
        daten.db_merken(Path(self.tmp.name) / "gibt_es_nicht.db")
        self.assertIsNone(daten.standard_db())


if __name__ == "__main__":
    unittest.main()
