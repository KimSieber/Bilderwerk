"""Tests der gemeinsamen Einstellungen (shared/bilderwerk/einstellungen.py).

Aufruf (aus dem Repo-Wurzelordner):  python3 -m unittest discover -s shared/tests
"""
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from bilderwerk import einstellungen  # noqa: E402


class EinstellungenTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.datei = Path(self.tmp.name) / "conf" / "einstellungen.json"
        self._alt_env = os.environ.get("BILDERWERK_EINSTELLUNGEN")
        os.environ["BILDERWERK_EINSTELLUNGEN"] = str(self.datei)

    def tearDown(self):
        if self._alt_env is None:
            os.environ.pop("BILDERWERK_EINSTELLUNGEN", None)
        else:
            os.environ["BILDERWERK_EINSTELLUNGEN"] = self._alt_env
        self.tmp.cleanup()

    def test_ohne_datei_standardwerte(self):
        e = einstellungen.laden()
        self.assertEqual(e["work"]["ki"]["url"], einstellungen.KI_STANDARD_URL)
        self.assertEqual(e["work"]["ki"]["modell"], "")
        self.assertEqual(einstellungen.db_pfad(e), self.datei.parent / "Bildkatalog.db")
        self.assertFalse(self.datei.exists())

    def test_migration_aus_alter_datei(self):
        alt = Path(self.tmp.name) / "gui_einstellungen.json"
        alt.write_text(json.dumps({"ordner": "/bilder", "db": "/x/Bildkatalog.db",
                                   "backup": "/s.db", "backup_alle": 500, "wach": False}))
        e = einstellungen.laden(alt)
        self.assertEqual(e["db"], "/x/Bildkatalog.db")
        self.assertEqual(e["work"]["ordner"], "/bilder")
        self.assertEqual(e["work"]["backup_alle"], 500)
        self.assertFalse(e["work"]["wach"])
        self.assertEqual(e["work"]["ki"]["modell"], "qwen/qwen3.5-9b")
        self.assertEqual(e["work"]["ki"]["url"], "http://127.0.0.1:1234/v1")
        self.assertTrue(self.datei.exists(), "neue Datei wird sofort geschrieben")
        self.assertTrue(alt.exists(), "alte Datei bleibt liegen")
        # zweiter Start: neue Datei gilt, alte wird ignoriert
        alt.write_text(json.dumps({"db": "/anders.db"}))
        self.assertEqual(einstellungen.laden(alt)["db"], "/x/Bildkatalog.db")

    def test_speichern_behaelt_fremde_abschnitte(self):
        e = einstellungen.laden()
        e["sweep"] = {"fenster": "800x600"}
        e["zukunft"] = 1
        einstellungen.speichern(e)
        e2 = einstellungen.laden()
        e2["work"]["ordner"] = "/neu"
        einstellungen.speichern(e2)
        e3 = einstellungen.laden()
        self.assertEqual(e3["sweep"], {"fenster": "800x600"})
        self.assertEqual(e3["zukunft"], 1)
        self.assertEqual(e3["work"]["ordner"], "/neu")
        self.assertEqual(e3["version"], einstellungen.VERSION)

    def test_fehlende_werte_ergaenzt(self):
        self.datei.parent.mkdir(parents=True)
        self.datei.write_text(json.dumps({"db": "~/k.db", "work": {"ki": {"modell": "m"}}}))
        e = einstellungen.laden()
        self.assertEqual(e["work"]["ki"]["modell"], "m")
        self.assertEqual(e["work"]["ki"]["url"], einstellungen.KI_STANDARD_URL)
        self.assertEqual(e["work"]["backup_alle"], 1000)
        self.assertEqual(einstellungen.db_pfad(e), Path("~/k.db").expanduser())

    def test_alte_volle_adresse_wird_gekuerzt(self):
        self.datei.parent.mkdir(parents=True)
        self.datei.write_text(json.dumps(
            {"work": {"ki": {"url": "http://h:1234/v1/chat/completions/"}}}))
        self.assertEqual(einstellungen.laden()["work"]["ki"]["url"], "http://h:1234/v1")

    def test_defekte_datei_ergibt_standard(self):
        self.datei.parent.mkdir(parents=True)
        self.datei.write_text("{kaputt")
        self.assertEqual(einstellungen.laden()["db"], "")


if __name__ == "__main__":
    unittest.main()
