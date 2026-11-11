"""Bildanalyse für den Bilderkatalog (Work) – gemeinsames Kernmodul.

Zweck:
    Bündelt alles, was zur Analyse EINES Bildes bzw. einer PDF-Seite gehört:
    Bildvorbereitung (Verkleinern/Konvertieren), PDF-Seiten rendern, Bildmaße,
    Farbmessung aus den Pixeln, Aufruf des lokalen KI-Modells (OpenAI-
    kompatibler Server, z. B. LM Studio)
    (inkl. Wiederholung bei ungültigen Antworten) und die Nachbearbeitung der
    Modellantwort.

Ziel:
    Ein einheitliches Ergebnis-Dictionary je Bild, das katalog.py in die
    Datenbank schreibt. Das Modul enthält keine Datenbanklogik.

Genutzt von:
    katalog.py, katalog_gui.py, suche.py, tools/bild_test.py,
    tools/farbe_neu_messen.py

Wichtige Werte:
    API_URL / MODEL / API_SCHLUESSEL – KI-Server, Modellname, optionaler Schlüssel;
                          kommen aus den gemeinsamen Einstellungen (shared/bilderwerk),
                          änderbar per konfigurieren()
    PROMPT_VERSION      – bei jeder Prompt-Änderung hochzählen (steht in der DB)
    STANDARD_DB         – eingestellte Datenbank bzw. Standardvorschlag
    FEHLER_ORDNER       – Ablage ungültiger Antworten (neben der Datenbank)
    SCHEMA / PROMPT     – JSON-Schema und Anweisung an das Modell

KI-Server:
    Jeder OpenAI-kompatible Server mit Bildeingabe (LM Studio, Ollama,
    llama.cpp-Server …). Lehnt der Server einzelne Angaben der Anfrage ab
    (HTTP 400), wird stufenweise mit einer einfacheren Anfrage wiederholt
    (siehe KOMPAT_STUFEN); die funktionierende Stufe gilt für den weiteren Lauf.

Externe Programme:
    sips (macOS), pdftoppm/pdfinfo (Homebrew-Paket poppler);
    Python-Pakete Pillow und numpy (Farbmessung, Rückfallebene ohne sips).
"""
import base64
import json
import re
import shutil
import subprocess
import tempfile
import time
import urllib.error
import sys
import urllib.request
from pathlib import Path

# Gemeinsamer Code (Einstellungen) liegt in <Repo>/shared
_SHARED = Path(__file__).resolve().parent.parent.parent / "shared"
if _SHARED.is_dir() and str(_SHARED) not in sys.path:
    sys.path.insert(0, str(_SHARED))
from bilderwerk import einstellungen  # noqa: E402
from bilderwerk.bild import MAX_KANTE_SEITE, PDFINFO, PDFTOPPM, RAW_ENDUNGEN  # noqa: E402,F401

# Bis v0.1 lagen Einstellungen neben den Programmen – Quelle für die Migration
ALT_EINSTELLUNGEN = Path(__file__).resolve().parent / "gui_einstellungen.json"
_E = einstellungen.laden(ALT_EINSTELLUNGEN)

API_URL = _E["work"]["ki"]["url"]
MODEL = _E["work"]["ki"]["modell"]
API_SCHLUESSEL = _E["work"]["ki"]["schluessel"]
PROMPT_VERSION = 4        # bei Prompt-Aenderungen hochzaehlen
MAX_KANTE = 1024          # Pixel, laengste Seite fuers Modell
KONTEXT_ORDNER = 3        # so viele uebergeordnete Ordner als Hinweis mitgeben
TIMEOUT_S = 900
MAX_TOKENS = 3000
VERSUCHE = 3              # Wiederholungen bei abgeschnittener / ungueltiger Antwort
TIMEOUT_TEST_S = 8        # Verbindungstest (Modellliste)
# Eingestellte Datenbank bzw. Vorschlag im Einstellungsordner
STANDARD_DB = einstellungen.db_pfad(_E)
# Ungueltige Modellantworten landen neben der Datenbank (Katalogisierer setzt den Ordner)
FEHLER_ORDNER = STANDARD_DB.parent / "fehler_antworten"

U = "unklar"
MOTIVE = [
    "Porträt", "Gruppenbild", "Menschen in Aktion", "Landschaft",
    "Stadt/Architektur", "Innenraum", "Tier", "Pflanze/Blume", "Fahrzeug",
    "Essen/Getränk", "Figur/Spielzeug", "Objekt/Stillleben", "Dokument/Text",
    "Sonstiges",
]

SCHEMA = {
    "type": "object",
    "properties": {
        "kurztitel": {"type": "string"},
        "beschreibung": {"type": "string"},
        "bildart": {"type": "string", "enum": [
            "Foto", "Grafik/Illustration", "Gemälde/Zeichnung", "3D-Rendering",
            "Scan/Dokument", "Screenshot", "Fotoalbum/Collage", U]},
        "motiv": {"type": "string"},          # frei, wird im Code zugeordnet
        "anlass": {"type": "string"},
        "innen_aussen": {"type": "string", "enum": ["innen", "außen", U]},
        "tageszeit": {"type": "string", "enum": [
            "Morgen", "Tag", "Abend/Dämmerung", "Nacht", U]},
        "jahreszeit": {"type": "string", "enum": [
            "Frühling", "Sommer", "Herbst", "Winter", U]},
        "wetter": {"type": "string", "enum": [
            "sonnig", "leicht bewölkt", "bedeckt", "Regen", "Schnee", "Nebel",
            "Gewitter", "entfällt", U]},
        "szene": {"type": "string"},
        "stimmung": {"type": "array", "items": {"type": "string"}, "maxItems": 5},
        "personen_anzahl": {"type": "integer"},
        "personen_merkmale": {"type": "array", "items": {"type": "string"}, "maxItems": 20},
        "schlagworte": {"type": "array", "items": {"type": "string"}, "maxItems": 30},
        "sichtbarer_text": {"type": "string"},
        "einzelbilder": {"type": "array", "items": {"type": "string"}, "maxItems": 12},
    },
    "required": ["kurztitel", "beschreibung", "bildart", "motiv", "anlass",
                 "innen_aussen", "tageszeit", "jahreszeit", "wetter", "szene",
                 "stimmung", "personen_anzahl", "personen_merkmale", "schlagworte",
                 "sichtbarer_text", "einzelbilder"],
    "additionalProperties": False,
}

PROMPT = """Du erstellst Katalogeinträge für ein privates Bildarchiv. Die Einträge dienen
später der Suche nach bestimmten Bildern und nach Motiven für Collagen und Vordrucke.
Antworte ausschließlich auf Deutsch.

Hinweis zur Datei: {kontext}
Ordner- und Dateiname können auf Anlass oder Ort hinweisen (z. B. 'Hochzeit',
'Mallorca'), müssen aber nicht stimmen. Maßgeblich ist, was im Bild zu sehen ist.

FELDER
kurztitel: 3 bis 6 Wörter.
beschreibung: ausführlich, 5 bis 10 Sätze. Gehe vom Hauptmotiv zum Hintergrund. Nenne
Personen (Anzahl, ungefähres Alter, Kleidung, Brille, Haltung, Tätigkeit,
Gesichtsausdruck), Tiere (möglichst mit Art), markante Objekte, Pflanzen, Gebäude,
Umgebung, Licht und Farben. Wenn Kleidung oder Technik auf ein Jahrzehnt hindeuten,
nenne es.
bildart: 'Foto' für jede normale Fotografie, auch Handyfotos und eingescannte oder
abfotografierte Papierfotos. '3D-Rendering' für computergenerierte Bilder.
'Scan/Dokument' nur für Schriftstücke. 'Fotoalbum/Collage', wenn mehrere getrennte
Fotos auf einer Seite zu sehen sind (Albumseite, Fotobuch, Collage).
'unklar' nur, wenn wirklich nicht bestimmbar.
motiv: das Hauptmotiv in 1 bis 3 Wörtern, möglichst einer dieser Begriffe:
{motive}.
anlass: z. B. 'Hochzeit', 'Geburtstag', 'Urlaub', 'Weihnachten', 'Ausflug', 'Feier',
'Familientreffen'; 'kein erkennbarer Anlass', wenn keiner erkennbar ist.
innen_aussen: 'innen' oder 'außen'.
tageszeit: 'Morgen' oder 'Abend/Dämmerung' nur bei tiefer Sonne, langen Schatten,
goldenem oder rötlichem Licht; bei normalem Tageslicht 'Tag'; bei Innenaufnahmen
meist 'unklar'.
jahreszeit: nur bei klaren Hinweisen (Laub, Blüte, Schnee, Kleidung), sonst 'unklar'.
wetter: bei Außenaufnahmen: 'sonnig' bei direktem Sonnenlicht, klaren Schatten oder
Sonnenreflexen auf Wasser; 'Schnee' nur, wenn Schnee sichtbar liegt oder fällt;
sonst 'unklar'. Bei Innenaufnahmen immer 'entfällt'.
szene: wenige Worte, z. B. 'Strand', 'Hochzeitsfeier', 'Bergwanderung'.
stimmung: 1 bis 3 Begriffe, z. B. 'fröhlich', 'ruhig', 'festlich', 'melancholisch'.
personen_anzahl: nur echte Menschen zählen. Roboter, Droiden, Spielfiguren, Puppen,
Statuen und Tiere zählen NICHT als Personen; sie gehören in die Schlagworte.
personen_merkmale: sichtbare Merkmale wie 'Brille', 'Sonnenbrille', 'Bart', 'Hut',
'Kind', 'Baby', 'Senior', 'Uniform', 'Brautkleid', 'Anzug'; leer, wenn keine Menschen.
schlagworte: 15 bis 25 konkrete Suchbegriffe im Singular: sichtbare Objekte, Tiere,
Pflanzen, Gebäude, Fahrzeuge, Landschaftsformen, Aktivitäten, Ort. Keine allgemeinen
Wörter wie 'Hintergrund', 'Bild', 'Natur', 'Szene', 'Außen', 'Tag', 'Wand', 'Boden'.
Keine Wiederholung von Tageszeit, Jahreszeit, Wetter oder innen/außen.
sichtbarer_text: gut lesbarer Text im Bild (Schilder, Aufschriften, Wasserzeichen,
Handschrift auf Albumseiten), sonst leer.
einzelbilder: NUR wenn mehrere getrennte Fotos zu sehen sind (Albumseite, Fotobuch,
Collage): je Foto ein Eintrag 'Position: Kurzbeschreibung in einem Satz', z. B.
'oben links: Flugzeug auf dem Rollfeld, daneben ein Bus'. In diesem Fall beschreibt
'beschreibung' die Seite als Ganzes, 'schlagworte' umfassen alle Fotos.
Bei einem einzelnen Foto: leere Liste.

REGELN
Nichts erfinden, nichts raten. Personen nie identifizieren oder benennen.
Keine Vermutungen über Herkunft, Religion oder Gesundheit."""

# Zuordnung freier Motivangaben -> feste Liste. Reihenfolge = Prioritaet
# (spezifische Begriffe vor allgemeinen wie 'Landschaft').
MOTIV_SCHLUESSEL = [
    ("Dokument/Text", ["dokument", "text", "schrift", "brief", "urkunde", "zeitung", "plakat"]),
    ("Figur/Spielzeug", ["figur", "spielzeug", "roboter", "droide", "puppe", "modell", "lego"]),
    ("Tier", ["tier", "vogel", "hund", "katze", "pferd", "wild", "fisch", "insekt",
              "antilope", "oryx", "kuh", "schaf"]),
    ("Essen/Getränk", ["essen", "speise", "getränk", "gericht", "kuchen", "torte", "mahlzeit"]),
    ("Fahrzeug", ["fahrzeug", "auto", "boot", "schiff", "yacht", "flugzeug", "zug",
                  "motorrad", "fahrrad", "wohnmobil", "bus", "lkw"]),
    ("Porträt", ["porträt", "portrait", "bildnis", "selfie"]),
    ("Gruppenbild", ["gruppe", "familie", "klasse", "mannschaft"]),
    ("Menschen in Aktion", ["aktion", "sport", "tanz", "spiel", "feier", "menschen", "person"]),
    ("Pflanze/Blume", ["pflanze", "blume", "blüte", "baum", "garten"]),
    ("Innenraum", ["innenraum", "zimmer", "raum", "küche", "saal"]),
    ("Stadt/Architektur", ["stadt", "architektur", "gebäude", "haus", "kirche", "straße",
                           "brücke", "burg", "schloss", "turm"]),
    ("Landschaft", ["landschaft", "luftaufnahme", "strand", "meer", "berg", "see", "insel",
                    "küste", "wald", "savanne", "feld", "wasser", "himmel", "panorama"]),
    ("Objekt/Stillleben", ["objekt", "stillleben", "gegenstand", "detail"]),
]

FUELLWOERTER = {
    "hintergrund", "vordergrund", "bild", "foto", "fotografie", "aufnahme", "natur",
    "szene", "außen", "aussen", "innen", "tag", "nacht", "licht", "wand", "boden",
    "farbe", "farben", "motiv", "umgebung", "sonstiges",
}


# ------------------------------------------------------------ Bildvorbereitung

def bild_vorbereiten(pfad: Path, tmpdir: str, max_kante: int = MAX_KANTE) -> Path:
    """Bild für das Modell vorbereiten.

    Verkleinert das Bild auf eine maximale Kantenlänge und speichert es als JPEG
    in einem temporären Ordner. Nutzt macOS 'sips' (kann auch HEIC/WebP), ohne
    sips wird Pillow als Rückfallebene verwendet.

    Parameter:
        pfad:      Originalbild.
        tmpdir:    Temporärer Ordner für die Zieldatei.
        max_kante: Längste Kante in Pixel (Standard MAX_KANTE).

    Rückgabe:
        Pfad der erzeugten JPEG-Datei (<tmpdir>/bild.jpg).

    Fehler:
        subprocess.CalledProcessError, wenn sips das Bild nicht lesen kann.
    """
    ziel = Path(tmpdir) / "bild.jpg"
    if shutil.which("sips"):
        subprocess.run(
            ["sips", "-Z", str(max_kante), "-s", "format", "jpeg",
             str(pfad), "--out", str(ziel)],
            check=True, capture_output=True,
        )
    else:
        from PIL import Image
        with Image.open(pfad) as img:
            img = img.convert("RGB")
            img.thumbnail((max_kante, max_kante))
            img.save(ziel, "JPEG", quality=90)
    return ziel


# ------------------------------------------------------------------ PDF-Seiten

# Schluessel und PDF-Seiten: gemeinsamer Code in shared/bilderwerk
from bilderwerk.katalog_db import SEITEN_MARKE, eintrag_schluessel, quelle_aufloesen  # noqa: E402,F401
from bilderwerk.bild import pdf_seitenzahl, pdf_seite_rendern  # noqa: E402,F401


def anzeigebild(schluessel: str, tmpdir: str, max_kante: int) -> Path:
    """Anzeigbare Bilddatei zu einem Datenbank-Schlüssel liefern.

    Bei normalen Bildern das Original, bei PDF-Seiten eine gerenderte Seite.

    Parameter:
        schluessel: Wert aus der Spalte 'pfad'.
        tmpdir:     Temporärer Ordner (nur für PDF-Seiten genutzt).
        max_kante:  Längste Kante für gerenderte PDF-Seiten.

    Rückgabe:
        Pfad einer Bilddatei.
    """
    datei, seite = quelle_aufloesen(schluessel)
    if seite is None:
        return datei
    return pdf_seite_rendern(datei, seite, tmpdir, max_kante)


# ------------------------------------------------------------------ Vorschauen

# Vorschaubilder: gemeinsamer Code in shared/bilderwerk
from bilderwerk.bild import (VORSCHAU_KANTE, VORSCHAU_QUALITAET, VORSCHAU_VERSION,  # noqa: E402,F401
                             nur_online, vorschau_erzeugen)


def bildmasse(pfad: Path) -> dict:
    """Originalgröße und Ausrichtung eines Bildes bestimmen.

    Liest Breite/Höhe per sips, sonst per Pillow. Weichen Breite und Höhe um
    höchstens 5 % voneinander ab, gilt das Bild als quadratisch.

    Parameter:
        pfad: Bilddatei (Original bzw. gerenderte PDF-Seite).

    Rückgabe:
        {'breite': int, 'hoehe': int, 'ausrichtung': 'quer'|'hoch'|'quadratisch'};
        bei Lesefehlern alle Werte None (kein Abbruch).
    """
    b = h = None
    try:
        if shutil.which("sips"):
            out = subprocess.run(["sips", "-g", "pixelWidth", "-g", "pixelHeight", str(pfad)],
                                 check=True, capture_output=True, text=True).stdout
            werte = {z.split(":")[0].strip(): int(z.split(":")[1])
                     for z in out.splitlines()[1:] if ":" in z}
            b, h = werte["pixelWidth"], werte["pixelHeight"]
        else:
            from PIL import Image
            with Image.open(pfad) as img:
                b, h = img.size
    except Exception:
        return {"breite": None, "hoehe": None, "ausrichtung": None}
    if abs(b - h) <= 0.05 * max(b, h):
        ausrichtung = "quadratisch"
    else:
        ausrichtung = "quer" if b > h else "hoch"
    return {"breite": b, "hoehe": h, "ausrichtung": ausrichtung}


def farbgebung_messen(jpeg: Path) -> str:
    """Farbgebung eines Bildes aus den Pixeln bestimmen.

    Wertet Sättigung und Farbton (HSV) auf einer 256x256-Verkleinerung aus.
    Kalibriert an echten Beispielen: abfotografierter SW-Abzug, Sepia-Bild,
    farbige Fotos mit dominanter Farbe (Grün, Blau), Innenaufnahmen bei warmem
    Kunstlicht (dürfen NICHT als Sepia gelten) und ein grauer Wagen mit farbigen
    Details (darf NICHT als Schwarzweiß gelten). Im Zweifel 'farbig'.

    Parameter:
        jpeg: Vorbereitete (verkleinerte) Bilddatei.

    Rückgabe:
        'schwarzweiß' | 'sepia' | 'monochrom getönt' | 'entsättigt' | 'farbig';
        'nicht gemessen', wenn Pillow/numpy fehlen.
    """
    try:
        import numpy as np
        from PIL import Image
    except ImportError:
        return "nicht gemessen"
    with Image.open(jpeg) as img:
        hsv = np.asarray(img.convert("RGB").resize((256, 256)).convert("HSV"),
                         dtype=float) / 255.0
    ton, satt, hell = hsv[..., 0] * 2 * np.pi, hsv[..., 1], hsv[..., 2]
    maske = (hell > 0.08) & (hell < 0.95)          # fast Schwarz/Weiss ignorieren
    if maske.sum() < 500:
        maske = np.ones_like(maske, dtype=bool)
    n = maske.sum()
    s = satt[maske]
    s50, s90, s95 = np.percentile(s, [50, 90, 95])
    # Anteil deutlich farbiger Bildstellen (Scheinwerfer, Innenausstattung, Krawatte ...)
    farbinseln = (maske & (satt > 0.2) & (hell > 0.15)).sum() / n

    # Schwarzweiss: kaum Saettigung UND keine farbigen Stellen.
    # s95 < 0.2 toleriert den Farbstich abfotografierter Papierabzuege.
    if s95 < 0.2 and farbinseln < 0.005:
        return "schwarzweiß"

    # Getoent (Sepia u. a.): praktisch EIN Farbton im ganzen Bild.
    # Warmes Kunstlicht ergibt zwar auch fast nur Orange/Gelb, aber Haut, Holz,
    # Waende usw. streuen staerker und es gibt Ausreisser (Krawatte, Lichtreflexe).
    farbig = maske & (satt > 0.05)
    if farbig.sum() > 500 and s50 > 0.15:
        z = np.exp(1j * ton[farbig]).mean()
        abweichung = np.degrees(np.abs(np.angle(np.exp(1j * (ton - np.angle(z))))))
        streuung95 = np.percentile(abweichung[farbig], 95)
        ausreisser = ((abweichung > 15) & maske & (satt > 0.1)).sum() / n
        if streuung95 < 8 and ausreisser < 0.005:
            mittel_grad = (np.degrees(np.angle(z)) + 360) % 360
            return "sepia" if 15 <= mittel_grad <= 55 else "monochrom getönt"

    if s90 < 0.2 and farbinseln < 0.005:
        return "entsättigt"
    return "farbig"


def kontext_text(pfad: Path, seite: int | None = None, seiten: int | None = None) -> str:
    """Hinweistext zur Datei für den Prompt erzeugen.

    Ordner- und Dateinamen enthalten oft Anlass oder Ort (z. B. 'Hochzeit').
    Mitgegeben werden die letzten KONTEXT_ORDNER Ordner plus Dateiname, bei
    PDF-Seiten zusätzlich die Seitenangabe.

    Parameter:
        pfad:   Bild- bzw. PDF-Datei.
        seite:  Seitennummer bei PDFs, sonst None.
        seiten: Gesamtseitenzahl der PDF (optional).

    Rückgabe:
        Text, z. B. 'Bilder / 2012 Hochzeit / IMG_1150.JPG'.
    """
    teile = list(pfad.resolve().parent.parts[-KONTEXT_ORDNER:]) + [pfad.name]
    text = " / ".join(teile)
    if seite is not None:
        text += f" (Seite {seite} von {seiten or '?'} einer PDF, z. B. digitalisiertes Fotoalbum)"
    return text


# ------------------------------------------------------------------ Modell

def konfigurieren(url: str | None = None, modell: str | None = None,
                  schluessel: str | None = None, fehler_ordner: Path | None = None):
    """KI-Server, Modell, Schlüssel bzw. Fehlerablage zur Laufzeit setzen.

    Nur übergebene Werte werden geändert. Gilt ab dem nächsten Bild.

    Parameter:
        url:           Adresse des Servers (Basis …/v1 oder …/chat/completions).
        modell:        Modellname, wie ihn der Server kennt.
        schluessel:    API-Schlüssel (leer = keiner).
        fehler_ordner: Ordner für ungültige Antworten.
    """
    global API_URL, MODEL, API_SCHLUESSEL, FEHLER_ORDNER, _kompat_stufe
    if url is not None and url.strip() != API_URL:
        API_URL = url.strip()
        _kompat_stufe = 0                     # neuer Server: wieder mit voller Anfrage
    if modell is not None and modell.strip() != MODEL:
        MODEL = modell.strip()
        _kompat_stufe = 0
    if schluessel is not None:
        API_SCHLUESSEL = schluessel.strip()
    if fehler_ordner is not None:
        FEHLER_ORDNER = Path(fehler_ordner)


def endpunkte(url: str) -> tuple[str, str]:
    """Adressen für Chat-Anfrage und Modellliste aus der eingestellten URL bilden.

    Akzeptiert die Basisadresse ('http://host:1234/v1'), die volle Adresse
    ('…/v1/chat/completions') oder nur 'http://host:port' (dann wird '/v1' ergänzt).

    Parameter:
        url: Eingestellte Adresse.

    Rückgabe:
        Tupel (Chat-URL, Modell-URL).
    """
    u = (url or "").strip().rstrip("/")
    if u.endswith("/chat/completions"):
        basis = u[: -len("/chat/completions")]
    elif u.endswith("/models"):
        basis = u[: -len("/models")]
    else:
        basis = u
        pfad = basis.split("://", 1)[-1].partition("/")[2]
        if not pfad:                          # nur Host:Port angegeben
            basis += "/v1"
    return basis + "/chat/completions", basis + "/models"


def _kopfzeilen(schluessel: str | None = None) -> dict:
    """HTTP-Kopfzeilen inkl. optionalem API-Schlüssel.

    Parameter:
        schluessel: Schlüssel; Standard: eingestellter API_SCHLUESSEL.

    Rückgabe:
        Dictionary der Kopfzeilen.
    """
    k = {"Content-Type": "application/json"}
    s = API_SCHLUESSEL if schluessel is None else schluessel
    if s:
        k["Authorization"] = f"Bearer {s}"
    return k


def _http_fehlertext(ex: urllib.error.HTTPError) -> str:
    """Kurze, lesbare Fehlermeldung aus einer HTTP-Fehlerantwort.

    Parameter:
        ex: HTTPError des Servers.

    Rückgabe:
        z. B. 'HTTP 400: model not found'.
    """
    try:
        rumpf = ex.read().decode("utf-8", "replace")
        try:
            d = json.loads(rumpf)
            fehler = d.get("error", d)
            rumpf = fehler.get("message", rumpf) if isinstance(fehler, dict) else str(fehler)
        except ValueError:
            pass
    except Exception:
        rumpf = ""
    return f"HTTP {ex.code}: {rumpf.strip()[:300] or ex.reason}"


def modelle_abfragen(url: str | None = None, schluessel: str | None = None) -> list[str]:
    """Verbindung testen und die Modelle des Servers abfragen (GET …/models).

    Parameter:
        url:        Adresse; Standard: eingestellte API_URL.
        schluessel: API-Schlüssel; Standard: eingestellter.

    Rückgabe:
        Sortierte Liste der Modellnamen (kann leer sein).

    Fehler:
        ServerNichtErreichbar, wenn keine Verbindung zustande kommt;
        RuntimeError mit lesbarer Meldung bei HTTP-Fehlern oder unbekannter Antwort.
    """
    _, modell_url = endpunkte(url or API_URL)
    req = urllib.request.Request(modell_url, headers=_kopfzeilen(schluessel))
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT_TEST_S) as r:
            d = json.load(r)
    except urllib.error.HTTPError as ex:
        raise RuntimeError(_http_fehlertext(ex)) from ex
    except (urllib.error.URLError, OSError) as ex:
        raise ServerNichtErreichbar(str(getattr(ex, "reason", ex))) from ex
    except ValueError as ex:
        raise RuntimeError("Antwort ist kein JSON – ist das ein OpenAI-kompatibler Server?") from ex
    eintraege = d.get("data") if isinstance(d, dict) else None
    if eintraege is None and isinstance(d, dict):
        eintraege = d.get("models")             # einige Server (z. B. Ollama nativ)
    if not isinstance(eintraege, list):
        raise RuntimeError("Unbekanntes Antwortformat der Modellliste")
    namen = {str(m.get("id") or m.get("name") or m.get("model") or "") if isinstance(m, dict)
             else str(m) for m in eintraege}
    return sorted(n for n in namen if n)


class ServerNichtErreichbar(Exception):
    """KI-Server antwortet nicht (Server gestoppt, App beendet, Neustart …).

    Wird von katalog.py abgefangen: Der Lauf wartet und versucht es erneut,
    statt einen Fehler zu buchen.
    """


class AntwortUngueltig(ValueError):
    """Die Modellantwort ist abgeschnitten oder kein gültiges JSON.

    Führt zu einem neuen Versuch (bis VERSUCHE); danach wird das Bild als
    Fehler gebucht.
    """


# Kompatibilitaetsstufen: 0 = volle Anfrage (LM Studio), 1 = nur Standardparameter
# mit JSON-Schema, 2 = nur 'JSON-Objekt' verlangt, 3 = ohne Formatvorgabe.
KOMPAT_STUFEN = 4
_kompat_stufe = 0


def _payload(prompt: str, b64: str, versuch: int, stufe: int) -> dict:
    """Anfrage für eine Kompatibilitätsstufe zusammenstellen.

    Parameter:
        prompt:  Fertig formatierter Prompt-Text.
        b64:     Bild als Base64-kodiertes JPEG.
        versuch: Nummer des Versuchs (1 = erster).
        stufe:   0 … KOMPAT_STUFEN-1 (siehe oben).

    Rückgabe:
        Anfrage als Dictionary.
    """
    payload = {
        "model": MODEL,
        # Ab dem 2. Versuch etwas mehr Variation und Strafe fuer Wiederholungen,
        # damit sich das Modell nicht erneut in einer Endlosschleife verfaengt.
        "temperature": 0.2 if versuch == 1 else 0.5,
        "max_tokens": MAX_TOKENS,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": prompt},
                {"type": "image_url", "image_url": {"url": f"data:image/jpeg;base64,{b64}"}},
            ],
        }],
    }
    if stufe <= 1:
        payload["response_format"] = {
            "type": "json_schema",
            "json_schema": {"name": "bildanalyse", "strict": True, "schema": SCHEMA},
        }
    elif stufe == 2:
        payload["response_format"] = {"type": "json_object"}
    if stufe == 0:
        payload["chat_template_kwargs"] = {"enable_thinking": False}
        if versuch > 1:
            payload["repeat_penalty"] = 1.15
    if versuch > 1:
        payload["frequency_penalty"] = 0.4
    return payload


def _anfrage(prompt: str, b64: str, versuch: int) -> dict:
    """Eine einzelne Anfrage an das Modell senden.

    Baut die OpenAI-kompatible Anfrage mit Prompt, Bild (Base64) und festem
    JSON-Schema. Ab dem 2. Versuch wird mit höherer Temperatur und Strafe für
    Wiederholungen gearbeitet, damit sich das Modell nicht erneut in einer
    Endlosschleife verfängt. Lehnt der Server die Anfrage ab (HTTP 400/422),
    wird mit der nächsten, einfacheren Kompatibilitätsstufe wiederholt.

    Parameter:
        prompt:  Fertig formatierter Prompt-Text.
        b64:     Bild als Base64-kodiertes JPEG.
        versuch: Nummer des Versuchs (1 = erster).

    Rückgabe:
        Rohantwort des Servers als Dictionary.

    Fehler:
        ServerNichtErreichbar, wenn die Verbindung abgelehnt wird;
        RuntimeError mit Servermeldung, wenn auch die einfachste Stufe abgelehnt
        wird (z. B. Modell unbekannt) oder der Schlüssel fehlt/falsch ist;
        andere urllib-Fehler (z. B. Timeout) werden weitergereicht.
    """
    global _kompat_stufe
    if not MODEL:
        raise RuntimeError("Kein KI-Modell eingestellt (Work → KI-Server …)")
    chat_url, _ = endpunkte(API_URL)
    while True:
        req = urllib.request.Request(
            chat_url, data=json.dumps(_payload(prompt, b64, versuch, _kompat_stufe)).encode(),
            headers=_kopfzeilen())
        try:
            with urllib.request.urlopen(req, timeout=TIMEOUT_S) as r:
                return json.load(r)
        except urllib.error.HTTPError as ex:
            if ex.code in (400, 422) and _kompat_stufe < KOMPAT_STUFEN - 1:
                _kompat_stufe += 1
                continue
            if ex.code in (400, 422):
                _kompat_stufe = 0          # lag nicht am Format – beim nächsten Mal wieder voll
            raise RuntimeError(_http_fehlertext(ex)) from ex
        except urllib.error.URLError as ex:
            grund = getattr(ex, "reason", ex)
            if isinstance(grund, ConnectionRefusedError) or "refused" in str(grund).lower():
                raise ServerNichtErreichbar(str(grund)) from ex
            raise


def _rohantwort_sichern(pfad: Path, versuch: int, antwort: dict):
    """Fehlerhafte Modellantwort zur späteren Analyse speichern.

    Schreibt die Rohantwort als JSON nach FEHLER_ORDNER (neben der Datenbank)
    (<Dateiname>_versuch<n>.json). Fehler beim Speichern werden ignoriert.

    Parameter:
        pfad:    Analysierte Datei (für den Dateinamen).
        versuch: Nummer des Versuchs.
        antwort: Rohantwort des Servers.

    Rückgabe:
        Keine.
    """
    try:
        FEHLER_ORDNER.mkdir(parents=True, exist_ok=True)
        ziel = FEHLER_ORDNER / f"{pfad.stem}_versuch{versuch}.json"
        ziel.write_text(json.dumps(antwort, indent=2, ensure_ascii=False), encoding="utf-8")
    except Exception:
        pass


def modell_anfragen(pfad: Path, jpeg: Path, kontext: str | None = None) -> tuple[dict, int, int]:
    """Bild vom Modell analysieren lassen – mit Wiederholungen.

    Formatiert den Prompt, sendet das Bild und extrahiert das JSON. Ist die
    Antwort ungültig, wird sie gesichert und bis zu VERSUCHE-mal neu angefragt.

    Parameter:
        pfad:    Originaldatei (für Kontext und Fehlerablage).
        jpeg:    Vorbereitete Bilddatei, die an das Modell geht.
        kontext: Hinweistext; Standard: kontext_text(pfad).

    Rückgabe:
        Tupel (Ergebnis-Dictionary, Ausgabe-Tokens, benötigte Versuche).

    Fehler:
        AntwortUngueltig nach dem letzten Versuch; ServerNichtErreichbar.
    """
    b64 = base64.b64encode(jpeg.read_bytes()).decode()
    prompt = PROMPT.format(kontext=kontext or kontext_text(pfad),
                           motive=", ".join(f"'{m}'" for m in MOTIVE))
    letzter_fehler = None
    for versuch in range(1, VERSUCHE + 1):
        antwort = _anfrage(prompt, b64, versuch)
        tokens = antwort.get("usage", {}).get("completion_tokens", 0)
        try:
            return json_extrahieren(antwort), tokens, versuch
        except AntwortUngueltig as ex:
            _rohantwort_sichern(pfad, versuch, antwort)
            letzter_fehler = ex
    raise AntwortUngueltig(f"nach {VERSUCHE} Versuchen: {letzter_fehler}")


def json_extrahieren(antwort: dict) -> dict:
    """JSON-Ergebnis aus der Modellantwort herauslösen.

    Berücksichtigt Antworten in 'content' oder 'reasoning_content', entfernt
    einen <think>-Block und nimmt den Text von der ersten '{' bis zur letzten '}'.

    Parameter:
        antwort: Rohantwort des Servers.

    Rückgabe:
        Geparstes Ergebnis als Dictionary.

    Fehler:
        AntwortUngueltig, wenn kein JSON gefunden wird (z. B. Token-Limit
        erreicht) oder das JSON fehlerhaft ist.
    """
    wahl = antwort["choices"][0]
    nachricht = wahl["message"]
    text = (nachricht.get("content") or nachricht.get("reasoning_content") or "").strip()
    if "</think>" in text:
        text = text.split("</think>", 1)[1].strip()
    start, ende = text.find("{"), text.rfind("}")
    tokens = antwort.get("usage", {}).get("completion_tokens", "?")
    if start < 0 or ende < start:
        grund = ("abgeschnitten (Token-Limit erreicht)" if wahl.get("finish_reason") == "length"
                 else "unvollständig")
        raise AntwortUngueltig(f"Antwort {grund}, {tokens} Tokens, "
                               f"Ende: …{text[-150:]!r}")
    try:
        return json.loads(text[start:ende + 1])
    except json.JSONDecodeError as ex:
        raise AntwortUngueltig(f"Ungültiges JSON ({ex.msg} bei Zeichen {ex.pos}), "
                               f"{tokens} Tokens") from ex


# ---------------------------------------------------------- Nachbearbeitung

def motiv_zuordnen(roh: str) -> str:
    """Freie Motivangabe des Modells auf die feste Motivliste abbilden.

    Erst exakter Treffer in MOTIVE, dann Stichwortsuche über MOTIV_SCHLUESSEL
    (Reihenfolge = Priorität, spezifische vor allgemeinen Begriffen).

    Parameter:
        roh: Motivangabe des Modells.

    Rückgabe:
        Ein Eintrag aus MOTIVE; 'Sonstiges', wenn nichts passt.
    """
    t = (roh or "").strip().lower()
    for motiv in MOTIVE:
        if t == motiv.lower():
            return motiv
    for motiv, schluessel in MOTIV_SCHLUESSEL:
        if any(s in t for s in schluessel):
            return motiv
    return "Sonstiges"


def liste_bereinigen(werte, entferne=frozenset()) -> list[str]:
    """Liste von Begriffen bereinigen.

    Entfernt leere Einträge, Dubletten (Groß-/Kleinschreibung egal) und alle
    Begriffe aus 'entferne'. Die Reihenfolge bleibt erhalten.

    Parameter:
        werte:    Liste (oder None) von Begriffen.
        entferne: Kleingeschriebene Begriffe, die entfallen sollen.

    Rückgabe:
        Bereinigte Liste von Texten.
    """
    gesehen, ergebnis = set(), []
    for w in werte or []:
        w = str(w).strip()
        k = w.lower()
        if not w or k in gesehen or k in entferne:
            continue
        gesehen.add(k)
        ergebnis.append(w)
    return ergebnis


def felder_ergaenzen(e: dict) -> dict:
    """Antwort an das SCHEMA anpassen – für Modelle, die es nicht streng einhalten.

    - fehlende Felder: leerer Text, leere Liste, 'unklar' bzw. None
    - Listen als Text ('a, b') werden zu Listen, Texte als Liste zu Text
    - Werte mit fester Auswahl (enum): Groß-/Kleinschreibung angleichen,
      Unbekanntes → 'unklar'
    - Personenanzahl als Text ('2') → Zahl

    Parameter:
        e: Ergebnis-Dictionary des Modells (wird direkt verändert).

    Rückgabe:
        Dasselbe Dictionary.
    """
    for feld, art in SCHEMA["properties"].items():
        wert = e.get(feld)
        if art["type"] == "array":
            if isinstance(wert, str):
                wert = [w.strip() for w in re.split(r"[,;\n]", wert) if w.strip()]
            elif not isinstance(wert, list):
                wert = []
            e[feld] = wert
        elif art["type"] == "integer":
            if isinstance(wert, str) and wert.strip().isdigit():
                wert = int(wert.strip())
            e[feld] = wert if isinstance(wert, int) and not isinstance(wert, bool) else None
        else:
            if isinstance(wert, list):
                wert = ", ".join(str(w) for w in wert)
            wert = "" if wert is None else str(wert).strip()
            if "enum" in art:
                treffer = {w.lower(): w for w in art["enum"]}
                wert = treffer.get(wert.lower(), U)
            e[feld] = wert
    return e


def nachbearbeiten(e: dict) -> dict:
    """Modellantwort plausibilisieren und vereinheitlichen.

    - Felder an das SCHEMA anpassen (felder_ergaenzen)
    - Motiv auf die feste Liste abbilden (Original bleibt in 'motiv_roh')
    - Wetter: innen → 'entfällt'; 'entfällt' bei außen → 'unklar'
    - Schlagworte ohne Füllwörter und ohne Wiederholung der Kategorien
    - Listen bereinigen, ungültige Personenanzahl → None

    Parameter:
        e: Ergebnis-Dictionary des Modells (wird direkt verändert).

    Rückgabe:
        Dasselbe, bereinigte Dictionary.
    """
    felder_ergaenzen(e)
    e["motiv_roh"] = e.get("motiv", "")
    e["motiv"] = motiv_zuordnen(e["motiv_roh"])
    if e.get("innen_aussen") == "innen":
        e["wetter"] = "entfällt"
    elif e.get("wetter") == "entfällt":
        e["wetter"] = U
    kategorien = {str(e.get(k, "")).lower() for k in
                  ("tageszeit", "jahreszeit", "wetter", "innen_aussen")}
    e["schlagworte"] = liste_bereinigen(e.get("schlagworte"), FUELLWOERTER | kategorien)
    e["personen_merkmale"] = liste_bereinigen(e.get("personen_merkmale"))
    e["stimmung"] = liste_bereinigen(e.get("stimmung"))
    e["einzelbilder"] = liste_bereinigen(e.get("einzelbilder"))
    if not isinstance(e.get("personen_anzahl"), int):
        e["personen_anzahl"] = None
    return e


# ------------------------------------------------------------- Hauptfunktion

def analysieren(pfad: Path, seite: int | None = None, seiten: int | None = None) -> dict:
    """Komplette Analyse eines Bildes oder einer PDF-Seite (Hauptfunktion).

    Ablauf: Bild vorbereiten bzw. PDF-Seite rendern → Farbgebung messen →
    Modell anfragen → Bildmaße bestimmen → nachbearbeiten.

    Parameter:
        pfad:   Bild- oder PDF-Datei.
        seite:  Seitennummer bei PDFs, sonst None.
        seiten: Gesamtseitenzahl der PDF (nur für den Kontexthinweis).

    Rückgabe:
        Ergebnis-Dictionary mit den KI-Feldern (siehe SCHEMA) plus 'motiv_roh',
        'farbgebung', 'breite', 'hoehe', 'ausrichtung', 'tokens', 'versuche'
        und 'dauer_s'.

    Fehler:
        ServerNichtErreichbar, AntwortUngueltig, Fehler beim Bildlesen.
    """
    start = time.time()
    with tempfile.TemporaryDirectory() as tmp:
        if seite is None:
            quelle = pfad
            jpeg = bild_vorbereiten(pfad, tmp)
        else:
            quelle = pdf_seite_rendern(pfad, seite, tmp, MAX_KANTE_SEITE)
            jpeg = quelle                     # bereits passend verkleinert
        farbe = farbgebung_messen(jpeg)
        ergebnis, tokens, versuche = modell_anfragen(
            pfad, jpeg, kontext_text(pfad, seite, seiten))
        masse = bildmasse(quelle)
    ergebnis = nachbearbeiten(ergebnis)
    ergebnis["farbgebung"] = farbe
    ergebnis.update(masse)
    ergebnis["tokens"] = tokens
    ergebnis["versuche"] = versuche
    ergebnis["dauer_s"] = round(time.time() - start, 1)
    return ergebnis
