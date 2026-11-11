"""Tests für KI-Endpunkt und Antwort-Robustheit in bildanalyse.py (ohne echtes Modell).

Aufruf (in apps/work):  python3 -m unittest discover -s tests
"""
import json
import os
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

_TMP = tempfile.TemporaryDirectory()
os.environ["BILDERWERK_EINSTELLUNGEN"] = str(Path(_TMP.name) / "einstellungen.json")
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import bildanalyse  # noqa: E402


class AttrappenServer(BaseHTTPRequestHandler):
    """Kleiner OpenAI-kompatibler Server, der json_schema/Zusatzparameter ablehnt."""
    anfragen: list = []
    ablehnen_400 = False
    modelle = {"object": "list", "data": [{"id": "b-modell"}, {"id": "a-modell"}]}

    def log_message(self, *a):
        pass

    def _senden(self, code, daten):
        rumpf = json.dumps(daten).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(rumpf)))
        self.end_headers()
        self.wfile.write(rumpf)

    def do_GET(self):
        if self.path == "/v1/models":
            self._senden(200, self.modelle)
        else:
            self._senden(404, {"error": {"message": "nicht gefunden"}})

    def do_POST(self):
        p = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        type(self).anfragen.append((self.path, p, self.headers.get("Authorization")))
        if self.ablehnen_400:
            return self._senden(400, {"error": {"message": "model not loaded"}})
        if p.get("model") != "a-modell":
            return self._senden(404, {"error": {"message": "model not found"}})
        if "chat_template_kwargs" in p or \
                p.get("response_format", {}).get("type") == "json_schema":
            return self._senden(400, {"error": {"message": "unsupported parameter"}})
        inhalt = '```json\n{"kurztitel": "Hund am Strand", "bildart": "foto", ' \
                 '"wetter": "Sonne", "schlagworte": "Hund, Strand, hintergrund", ' \
                 '"personen_anzahl": "0", "innen_aussen": "außen"}\n```'
        self._senden(200, {"choices": [{"message": {"content": inhalt},
                                        "finish_reason": "stop"}],
                           "usage": {"completion_tokens": 42}})


class EndpunkteTest(unittest.TestCase):
    def test_varianten(self):
        erwartet = ("http://h:1/v1/chat/completions", "http://h:1/v1/models")
        for url in ("http://h:1/v1", "http://h:1/v1/", "http://h:1/v1/chat/completions",
                    "http://h:1", "http://h:1/v1/models"):
            self.assertEqual(bildanalyse.endpunkte(url), erwartet, url)
        self.assertEqual(bildanalyse.endpunkte("https://x.de/api/v2")[0],
                         "https://x.de/api/v2/chat/completions")


class FelderTest(unittest.TestCase):
    def test_felder_ergaenzen(self):
        e = bildanalyse.felder_ergaenzen({"bildart": "foto", "wetter": "Sonne",
                                          "schlagworte": "a, b; c", "personen_anzahl": "3",
                                          "stimmung": "ruhig", "beschreibung": ["x", "y"]})
        self.assertEqual(e["bildart"], "Foto")
        self.assertEqual(e["wetter"], "unklar")
        self.assertEqual(e["schlagworte"], ["a", "b", "c"])
        self.assertEqual(e["stimmung"], ["ruhig"])
        self.assertEqual(e["personen_anzahl"], 3)
        self.assertEqual(e["beschreibung"], "x, y")
        self.assertEqual(e["kurztitel"], "")
        self.assertEqual(e["einzelbilder"], [])
        self.assertEqual(e["tageszeit"], "unklar")

    def test_nachbearbeiten_vollstaendige_antwort_unveraendert(self):
        e = bildanalyse.nachbearbeiten({"kurztitel": "T", "bildart": "Foto", "motiv": "Hund",
                                        "innen_aussen": "innen", "wetter": "sonnig",
                                        "personen_anzahl": 2, "schlagworte": ["Hund", "Bild"]})
        self.assertEqual(e["motiv"], "Tier")
        self.assertEqual(e["wetter"], "entfällt")
        self.assertEqual(e["schlagworte"], ["Hund"])
        self.assertEqual(e["personen_anzahl"], 2)


class ServerTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = HTTPServer(("127.0.0.1", 0), AttrappenServer)
        cls.url = f"http://127.0.0.1:{cls.server.server_address[1]}/v1"
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()

    def setUp(self):
        AttrappenServer.anfragen = []
        bildanalyse.konfigurieren(url="http://neu.invalid", modell="")   # Stufe zurücksetzen
        bildanalyse.konfigurieren(url=self.url, modell="a-modell", schluessel="geheim")

    def test_modelle_abfragen(self):
        self.assertEqual(bildanalyse.modelle_abfragen(self.url, ""), ["a-modell", "b-modell"])

    def test_nicht_erreichbar(self):
        with self.assertRaises(bildanalyse.ServerNichtErreichbar):
            bildanalyse.modelle_abfragen("http://127.0.0.1:9/v1", "")

    def test_kompatibilitaetsstufen_und_schluessel(self):
        antwort = bildanalyse._anfrage("prompt", "AAAA", 1)
        e = bildanalyse.nachbearbeiten(bildanalyse.json_extrahieren(antwort))
        self.assertEqual(e["kurztitel"], "Hund am Strand")
        self.assertEqual(e["bildart"], "Foto")
        self.assertEqual(e["wetter"], "unklar")
        self.assertEqual(e["schlagworte"], ["Hund", "Strand"])
        self.assertEqual(e["personen_anzahl"], 0)
        self.assertEqual(len(AttrappenServer.anfragen), 3)        # Stufe 0, 1 abgelehnt, 2 ok
        self.assertEqual(bildanalyse._kompat_stufe, 2)
        self.assertEqual(AttrappenServer.anfragen[-1][0], "/v1/chat/completions")
        self.assertEqual(AttrappenServer.anfragen[-1][2], "Bearer geheim")
        # Folgeanfrage startet direkt mit der funktionierenden Stufe
        bildanalyse._anfrage("prompt", "AAAA", 2)
        self.assertEqual(len(AttrappenServer.anfragen), 4)

    def test_modell_unbekannt_lesbarer_fehler(self):
        bildanalyse.konfigurieren(modell="gibt-es-nicht")
        with self.assertRaises(RuntimeError) as ctx:
            bildanalyse._anfrage("prompt", "AAAA", 1)
        self.assertIn("model not found", str(ctx.exception))

    def test_alle_stufen_abgelehnt_setzt_zurueck(self):
        bildanalyse.konfigurieren(modell="b-modell")
        AttrappenServer.ablehnen_400 = True
        try:
            with self.assertRaises(RuntimeError):
                bildanalyse._anfrage("prompt", "AAAA", 1)
        finally:
            AttrappenServer.ablehnen_400 = False
        self.assertEqual(len(AttrappenServer.anfragen), bildanalyse.KOMPAT_STUFEN)
        self.assertEqual(bildanalyse._kompat_stufe, 0)

    def test_ohne_modell(self):
        bildanalyse.konfigurieren(modell="")
        with self.assertRaises(RuntimeError):
            bildanalyse._anfrage("prompt", "AAAA", 1)


if __name__ == "__main__":
    unittest.main()
