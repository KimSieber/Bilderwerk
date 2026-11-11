"""Gemeinsame Bildfunktionen von Work und Sweep: PDF-Seiten und Vorschaubilder.

Zweck:
    PDF-Seiten rendern (poppler), Dropbox-Platzhalter („nur online“) erkennen und
    Vorschaubilder nach festen Vorgaben erzeugen – von Work beim Katalogisieren,
    von Sweep, wenn in der DB noch keine Vorschau liegt.

Externe Programme:
    sips (macOS; HEIC, RAW), pdftoppm/pdfinfo (Homebrew-Paket poppler);
    Python-Paket Pillow (pillow-heif optional).
"""
from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

MAX_KANTE_SEITE = 1280    # PDF-Seiten (oft mehrere Fotos) etwas groesser
PDFTOPPM = shutil.which("pdftoppm")   # aus Homebrew-Paket "poppler"
PDFINFO = shutil.which("pdfinfo")
# RAW-Formate: nur ueber sips lesen (Pillow liefert hier oft nur das kleine Vorschaubild)
RAW_ENDUNGEN = {".cr2", ".cr3", ".nef", ".nrw", ".arw", ".dng", ".orf", ".raf", ".rw2",
                ".pef", ".srw"}


def pdf_seitenzahl(pdf: Path) -> int:
    """Seitenzahl einer PDF-Datei ermitteln.

    Liest die Zeile 'Pages:' aus der Ausgabe von pdfinfo (poppler).

    Parameter:
        pdf: Pfad der PDF-Datei.

    Rückgabe:
        Anzahl der Seiten.

    Fehler:
        RuntimeError, wenn pdfinfo fehlt oder die Seitenzahl nicht lesbar ist
        (PDF beschädigt oder verschlüsselt).
    """
    if not PDFINFO:
        raise RuntimeError("pdfinfo fehlt (brew install poppler)")
    out = subprocess.run([PDFINFO, str(pdf)], capture_output=True, text=True,
                         timeout=120).stdout
    for zeile in out.splitlines():
        if zeile.startswith("Pages:"):
            return int(zeile.split(":", 1)[1])
    raise RuntimeError("Seitenzahl nicht lesbar (PDF beschädigt oder verschlüsselt?)")


def pdf_seite_rendern(pdf: Path, seite: int, tmpdir: str, max_kante: int,
                      format_: str = "jpeg") -> Path:
    """Eine einzelne PDF-Seite als Bild rendern.

    Nutzt pdftoppm (poppler); die längste Kante wird auf max_kante skaliert.

    Parameter:
        pdf:       Pfad der PDF-Datei.
        seite:     Seitennummer (ab 1).
        tmpdir:    Temporärer Ordner für die Zieldatei.
        max_kante: Längste Kante in Pixel.
        format_:   'jpeg' (Standard) oder z. B. 'png' (Vorschau in der GUI).

    Rückgabe:
        Pfad der erzeugten Bilddatei.

    Fehler:
        RuntimeError, wenn pdftoppm fehlt oder die Seite nicht renderbar ist.
    """
    if not PDFTOPPM:
        raise RuntimeError("pdftoppm fehlt (brew install poppler)")
    ziel = Path(tmpdir) / f"seite_{seite}"
    r = subprocess.run([PDFTOPPM, "-f", str(seite), "-l", str(seite), "-scale-to",
                        str(max_kante), f"-{format_}", "-singlefile", str(pdf), str(ziel)],
                       capture_output=True, text=True, timeout=300)
    datei = ziel.with_suffix(".jpg" if format_ == "jpeg" else f".{format_}")
    if not datei.exists():
        raise RuntimeError(f"PDF-Seite {seite} nicht renderbar: {r.stderr.strip()[:150]}")
    return datei


VORSCHAU_KANTE = 640      # laengste Kante der Vorschau in Pixel (nie vergroessern)
VORSCHAU_QUALITAET = 80   # JPEG-Qualitaet
VORSCHAU_VERSION = 1      # Verfahrensversion; hochzaehlen erlaubt gezieltes Neuerzeugen
_SF_DATALESS = 0x40000000  # macOS: Datei liegt nur in der Cloud (Dropbox "nur online")


def nur_online(pfad: Path) -> bool:
    """Prüfen, ob eine Datei nur als Cloud-Platzhalter vorliegt (Dropbox „nur online“).

    Erkennung ohne Herunterladen: macOS-Flag SF_DATALESS, ersatzweise
    Datei mit Größe > 0, aber ohne belegte Blöcke, oder Größe 0.

    Parameter:
        pfad: Zu prüfende Datei.

    Rückgabe:
        True, wenn der Inhalt nicht lokal vorliegt; False sonst (auch bei Fehlern).
    """
    try:
        st = pfad.stat()
    except OSError:
        return False
    if getattr(st, "st_flags", 0) & _SF_DATALESS:
        return True
    return st.st_size == 0 or (getattr(st, "st_blocks", 1) == 0 and st.st_size > 0)


def _bild_oeffnen_fuer_vorschau(pfad: Path, tmpdir: str):
    """Bild mit Pillow öffnen – notfalls über eine sips-Kopie (z. B. HEIC ohne pillow-heif).

    Parameter:
        pfad:   Bilddatei.
        tmpdir: Temporärer Ordner für die sips-Kopie.

    Rückgabe:
        Geöffnetes PIL.Image (vom Aufrufer zu schließen).

    Fehler:
        Pillow-/sips-Fehler, wenn das Bild auf keinem Weg lesbar ist.
    """
    from PIL import Image
    try:
        import pillow_heif                     # optional: HEIC direkt mit Pillow
        pillow_heif.register_heif_opener()
    except ImportError:
        pass
    try:
        if pfad.suffix.lower() in RAW_ENDUNGEN:
            raise ValueError("RAW-Format – Umwandlung über sips")
        return Image.open(pfad)
    except Exception:
        if not shutil.which("sips"):
            raise
        # sips kann HEIC/HEIF; JPEG behaelt die EXIF-Drehangabe fuer exif_transpose
        ziel = Path(tmpdir) / "vorschau_quelle.jpg"
        subprocess.run(["sips", "-Z", str(VORSCHAU_KANTE * 2), "-s", "format", "jpeg",
                        "-s", "formatOptions", "95", str(pfad), "--out", str(ziel)],
                       check=True, capture_output=True, timeout=120)
        return Image.open(ziel)


def vorschau_erzeugen(pfad: Path, seite: int | None = None) -> tuple[bytes, int, int]:
    """Vorschaubild (JPEG, längste Kante ≤ 640 px) zu einem Bild bzw. einer PDF-Seite erzeugen.

    Vorgaben laut Auftrag „Vorschaubilder in der Datenbank“: Quelle ist das
    Original bzw. die gerenderte PDF-Seite (1280 px); EXIF-Drehung anwenden;
    nach RGB wandeln, Transparenz auf Weiß; LANCZOS; nie vergrößern; JPEG
    Qualität 80, optimiert, nicht progressiv, ohne Metadaten.

    Parameter:
        pfad:  Bild- oder PDF-Datei.
        seite: Seitennummer bei PDFs, sonst None.

    Rückgabe:
        Tupel (JPEG-Bytes, Breite, Höhe) der Vorschau.

    Fehler:
        Jeder Lese-/Renderfehler wird weitergereicht – der Aufrufer entscheidet
        (im Katalog-Lauf: protokollieren, Eintrag trotzdem speichern).
    """
    import io
    from PIL import Image, ImageOps
    with tempfile.TemporaryDirectory() as tmp:
        quelle = (pdf_seite_rendern(pfad, seite, tmp, MAX_KANTE_SEITE) if seite is not None
                  else pfad)
        img = _bild_oeffnen_fuer_vorschau(quelle, tmp)
        try:
            if img.format == "JPEG":
                img.draft("RGB", (VORSCHAU_KANTE * 2, VORSCHAU_KANTE * 2))  # schneller laden
            img = ImageOps.exif_transpose(img)
            if img.mode in ("RGBA", "LA", "PA") or (img.mode == "P" and "transparency" in img.info):
                img = img.convert("RGBA")
                weiss = Image.new("RGB", img.size, (255, 255, 255))
                weiss.paste(img, mask=img.getchannel("A"))
                img = weiss
            elif img.mode != "RGB":
                img = img.convert("RGB")
            img.thumbnail((VORSCHAU_KANTE, VORSCHAU_KANTE), Image.LANCZOS)  # vergroessert nie
            puffer = io.BytesIO()
            img.save(puffer, "JPEG", quality=VORSCHAU_QUALITAET, optimize=True,
                     progressive=False)
            return puffer.getvalue(), img.width, img.height
        finally:
            img.close()
