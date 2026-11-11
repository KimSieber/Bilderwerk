"""Metadaten (EXIF/IPTC/XMP) für den Bilderkatalog (Work).

Zweck:
    Liest die eingebetteten Metadaten eines Bildes (Aufnahmedatum, Kamera,
    Objektiv, Belichtung, GPS, Urheber, Titel, Stichworte …) und bringt sie
    in ein festes Format mit den Spaltennamen der Tabelle 'bilder'.
    Zusätzlich werden ALLE Metadaten als JSON mitgeliefert.

Ziel:
    Metadaten robust erfassen – Fehler führen nie zum Abbruch des Laufs.
    Fehlt ein Aufnahmedatum, wird es nach Möglichkeit aus dem Dateinamen
    abgeleitet (datum_quelle = 'dateiname').

Werkzeuge:
    Bevorzugt ExifTool (liest JPEG, HEIC, RAW, PNG, TIFF inkl. iPhone-GPS).
    Installation:  brew install exiftool
    Ohne ExifTool wird Pillow genutzt (nur Grunddaten, kein HEIC).
"""
import json
import re
import shutil
import subprocess
from pathlib import Path

EXIFTOOL = shutil.which("exiftool")

# Diese Bloecke sind gross und fuer die Suche wertlos -> nicht speichern
EXIFTOOL_AUSSCHLUSS = ["--MakerNotes:all", "--ICC_Profile:all", "--ThumbnailImage",
                       "--PreviewImage", "--JpgFromRaw"]

LEER = {
    "aufnahme_datum": None, "datum_quelle": None,
    "kamera_hersteller": None, "kamera_modell": None, "objektiv": None,
    "brennweite": None, "brennweite_kb": None, "blende": None,
    "belichtungszeit": None, "iso": None, "blitz": None,
    "gps_lat": None, "gps_lon": None, "gps_hoehe": None,
    "software": None, "urheber": None, "copyright": None,
    "meta_titel": None, "meta_beschreibung": None, "meta_stichworte": None,
    "metadaten_json": None,
}


def _iso_datum(wert) -> str | None:
    """Metadaten-Datum in ISO-Format umwandeln.

    '2015:09:06 14:53:20' → '2015-09-06T14:53:20'. Ungültige Daten
    (z. B. Jahr 0000) ergeben None.

    Parameter:
        wert: Datum aus den Metadaten (Text oder None).

    Rückgabe:
        ISO-Datum als Text oder None.
    """
    if not wert:
        return None
    m = re.match(r"(\d{4})[:\-](\d{2})[:\-](\d{2})[ T](\d{2}):(\d{2}):?(\d{2})?", str(wert))
    if not m or m.group(1) == "0000":
        return None
    j, mo, t, h, mi, s = m.groups()
    return f"{j}-{mo}-{t}T{h}:{mi}:{s or '00'}"


def datum_aus_dateiname(name: str) -> str | None:
    """Aufnahmedatum aus dem Dateinamen ableiten.

    Erkennt z. B. 'Foto 06.09.15, 14 53 20', 'IMG_20190512_143000' und
    '2019-05-12'. Zweistellige Jahre < 70 gelten als 20xx, sonst 19xx.

    Parameter:
        name: Dateiname.

    Rückgabe:
        ISO-Datum als Text oder None.
    """
    m = re.search(r"(\d{2})\.(\d{2})\.(\d{2,4})(?:,?\s*(\d{2})[ .:](\d{2})[ .:](\d{2}))?", name)
    if m:
        t, mo, j, h, mi, s = m.groups()
        j = ("20" + j if int(j) < 70 else "19" + j) if len(j) == 2 else j
        return f"{j}-{mo}-{t}T{h or '00'}:{mi or '00'}:{s or '00'}"
    m = re.search(r"(19\d{2}|20\d{2})[-_]?(\d{2})[-_]?(\d{2})(?:[-_ ]?(\d{2})(\d{2})(\d{2}))?", name)
    if m:
        j, mo, t, h, mi, s = m.groups()
        if 1 <= int(mo) <= 12 and 1 <= int(t) <= 31:
            return f"{j}-{mo}-{t}T{h or '00'}:{mi or '00'}:{s or '00'}"
    return None


def _als_text(wert) -> str | None:
    """Metadatenwert als Text liefern.

    Parameter:
        wert: Beliebiger Wert; Listen werden mit ', ' verbunden.

    Rückgabe:
        Text oder None bei leerem Wert.
    """
    if wert is None or wert == "":
        return None
    if isinstance(wert, list):
        return ", ".join(str(w) for w in wert)
    return str(wert)


def _zahl(wert):
    """Metadatenwert als Zahl liefern.

    Parameter:
        wert: Beliebiger Wert.

    Rückgabe:
        float oder None, wenn nicht umwandelbar.
    """
    try:
        return float(wert)
    except (TypeError, ValueError):
        return None


def _mit_exiftool(pfad: Path) -> dict:
    """Metadaten per ExifTool lesen.

    Große, für die Suche wertlose Blöcke (MakerNotes, ICC-Profil, eingebettete
    Vorschaubilder) werden ausgelassen, dateisystembezogene Felder entfernt.

    Parameter:
        pfad: Bilddatei.

    Rückgabe:
        Dictionary mit den Feldern aus LEER (ohne 'datum_quelle').
    """
    out = subprocess.run(
        [EXIFTOOL, "-json", "-n", "-q", "-q", *EXIFTOOL_AUSSCHLUSS, str(pfad)],
        capture_output=True, text=True, timeout=60,
    ).stdout
    daten = json.loads(out)[0] if out.strip() else {}
    for schluessel in ("SourceFile", "Directory", "FileName", "FilePermissions",
                       "FileAccessDate", "FileInodeChangeDate"):
        daten.pop(schluessel, None)

    stichworte = daten.get("Keywords") or daten.get("Subject")
    datum = (daten.get("DateTimeOriginal") or daten.get("CreateDate")
             or daten.get("CreationDate") or daten.get("DateCreated"))
    return {
        "aufnahme_datum": _iso_datum(datum),
        "kamera_hersteller": _als_text(daten.get("Make")),
        "kamera_modell": _als_text(daten.get("Model")),
        "objektiv": _als_text(daten.get("LensModel") or daten.get("LensID") or daten.get("Lens")),
        "brennweite": _zahl(daten.get("FocalLength")),
        "brennweite_kb": _zahl(daten.get("FocalLengthIn35mmFormat")),
        "blende": _zahl(daten.get("FNumber") or daten.get("Aperture")),
        "belichtungszeit": _zahl(daten.get("ExposureTime")),
        "iso": _zahl(daten.get("ISO")),
        "blitz": _als_text(daten.get("Flash")),
        "gps_lat": _zahl(daten.get("GPSLatitude")),
        "gps_lon": _zahl(daten.get("GPSLongitude")),
        "gps_hoehe": _zahl(daten.get("GPSAltitude")),
        "software": _als_text(daten.get("Software")),
        "urheber": _als_text(daten.get("Artist") or daten.get("Creator") or daten.get("By-line")),
        "copyright": _als_text(daten.get("Copyright") or daten.get("Rights")
                               or daten.get("CopyrightNotice")),
        "meta_titel": _als_text(daten.get("Title") or daten.get("ObjectName")),
        "meta_beschreibung": _als_text(daten.get("ImageDescription") or daten.get("Description")
                                       or daten.get("Caption-Abstract")),
        "meta_stichworte": _als_text(stichworte),
        "metadaten_json": json.dumps(daten, ensure_ascii=False, default=str),
    }


def _mit_pillow(pfad: Path) -> dict:
    """Metadaten per Pillow lesen (Rückfallebene ohne ExifTool).

    Liest nur EXIF-Grunddaten inkl. GPS; kein HEIC, keine IPTC/XMP-Felder.

    Parameter:
        pfad: Bilddatei.

    Rückgabe:
        Dictionary mit den Feldern aus LEER (ohne 'datum_quelle').
    """
    from PIL import Image
    from PIL.ExifTags import TAGS
    with Image.open(pfad) as img:
        exif = img.getexif()
        exif_ifd = exif.get_ifd(0x8769)
        gps = exif.get_ifd(0x8825)
    alles = {TAGS.get(k, str(k)): v for k, v in {**dict(exif), **dict(exif_ifd)}.items()
             if not isinstance(v, bytes)}

    def gps_grad(werte, ref):
        """GPS-Angabe (Grad, Minuten, Sekunden) in Dezimalgrad umrechnen.

        Parameter:
            werte: Tupel (Grad, Minuten, Sekunden) oder None.
            ref:   Himmelsrichtung 'N', 'S', 'E' oder 'W'.

        Rückgabe:
            Dezimalgrad (Süd/West negativ) oder None.
        """
        if not werte:
            return None
        g, m, s = (float(x) for x in werte)
        wert = g + m / 60 + s / 3600
        return -wert if ref in ("S", "W") else wert

    return {
        "aufnahme_datum": _iso_datum(exif_ifd.get(36867) or exif.get(306)),
        "kamera_hersteller": _als_text(exif.get(271)),
        "kamera_modell": _als_text(exif.get(272)),
        "objektiv": _als_text(exif_ifd.get(42036)),
        "brennweite": _zahl(exif_ifd.get(37386)),
        "brennweite_kb": _zahl(exif_ifd.get(41989)),
        "blende": _zahl(exif_ifd.get(33437)),
        "belichtungszeit": _zahl(exif_ifd.get(33434)),
        "iso": _zahl(exif_ifd.get(34855)),
        "blitz": _als_text(exif_ifd.get(37385)),
        "gps_lat": gps_grad(gps.get(2), gps.get(1)),
        "gps_lon": gps_grad(gps.get(4), gps.get(3)),
        "gps_hoehe": _zahl(gps.get(6)),
        "software": _als_text(exif.get(305)),
        "urheber": _als_text(exif.get(315)),
        "copyright": _als_text(exif.get(33432)),
        "meta_titel": None,
        "meta_beschreibung": _als_text(exif.get(270)),
        "meta_stichworte": None,
        "metadaten_json": json.dumps(alles, ensure_ascii=False, default=str),
    }


def lesen(pfad: Path) -> dict:
    """Metadaten eines Bildes lesen (Hauptfunktion des Moduls).

    Nutzt ExifTool, sonst Pillow. Fehler werden verschluckt; ohne Datum in den
    Metadaten wird es aus dem Dateinamen abgeleitet.

    Parameter:
        pfad: Bilddatei.

    Rückgabe:
        Dictionary mit allen Feldern aus LEER; 'datum_quelle' ist 'metadaten',
        'dateiname' oder None.
    """
    ergebnis = dict(LEER)
    try:
        ergebnis.update(_mit_exiftool(pfad) if EXIFTOOL else _mit_pillow(pfad))
    except Exception:
        pass
    if ergebnis["aufnahme_datum"]:
        ergebnis["datum_quelle"] = "metadaten"
    else:
        datum = datum_aus_dateiname(pfad.name)
        if datum:
            ergebnis["aufnahme_datum"], ergebnis["datum_quelle"] = datum, "dateiname"
    return ergebnis
