# Bildkatalog – Projektdokumentation

**Stand:** November 2026 · **Zweck dieses Dokuments:** Übergabe an ein Folgeprojekt, das die Bilddatenbank *nutzt* (Suche, Auswahl, Weiterverarbeitung).

---

## 1. Worum es geht

Ein privates Bildarchiv (ca. 81.000 Dateien, dazu digitalisierte Fotobücher als PDF) wird **vollständig lokal** von einem KI-Modell analysiert. Pro Bild entsteht ein Datensatz in einer **SQLite-Datenbank** mit:

- **Ablage:** Pfad, Ordner, Dateiname (Ordnernamen tragen oft Anlass/Ort/Datum, z. B. `2012-06-09 Hochzeit …`)
- **Metadaten:** Aufnahmedatum, Kamera, Objektiv, Belichtung, GPS, Urheber, vorhandene Stichworte – plus *alle* Metadaten als JSON
- **Gemessene Eigenschaften:** Bildgröße, Ausrichtung, Farbgebung (aus den Pixeln berechnet, nicht vom Modell geschätzt)
- **KI-Analyse:** ausführliche Beschreibung, Kurztitel, Kategorien (Motiv, Anlass, Wetter, Tageszeit …), Schlagworte, Personenmerkmale, sichtbarer Text, bei Albumseiten eine Liste der Einzelfotos

Ziel ist die Suche nach bestimmten Bildern und nach Motiven für Collagen und Vordrucke (z. B. „Menschen mit Brille“, „Landschaft mit Baum“, „Schwarzweiß, hochformatig“).

Die Analyse läuft **einmalig** und abschnittsweise nach Prioritätsklassen (Ordner für Ordner); neue Bilder werden später nachgetragen. Laufzeit spielt keine Rolle (ca. 40–90 s pro Bild).

---

## 2. Systemumgebung

| Komponente | Details |
|---|---|
| Rechner | MacBook Air M3, 24 GB (Entwicklung/Tests); geplant: Mac mini M1, 16 GB für den Dauerlauf |
| KI-Laufzeit | **LM Studio** (klassische App, nicht „Bionic“), lokaler Server `http://127.0.0.1:1234/v1` (OpenAI-kompatibel), nur auf localhost |
| Modell | `qwen/qwen3.5-9b`, MLX, 4 bit (≈ 6 GB), mit Bildverständnis; Denkmodus aus |
| Python | **Homebrew-Python** `/opt/homebrew/bin/python3` (keine venv) |
| Python-Pakete | `pillow`, `numpy` (per `brew install pillow numpy`) |
| Homebrew-Tools | `exiftool` (Metadaten), `poppler` (PDF-Seiten: `pdftoppm`, `pdfinfo`) |
| macOS-Bordmittel | `sips` (Verkleinern, HEIC/WebP-Konvertierung, Vorschau), `caffeinate` (Ruhezustand verhindern) |

### Pfade

| Was | Pfad |
|---|---|
| Programme | `/Users/kim/KI-Laufzeitumgebungen/Bildkatalg erstellen/` |
| Datenbank | `…/Bildkatalg erstellen/Katalog-DB/Bildkatalog.db` |
| Protokoll | `…/Katalog-DB/Bildkatalog_protokoll.txt` |
| Fehlerhafte Modellantworten | `…/Katalog-DB/fehler_antworten/` |
| GUI-Einstellungen | `…/Bildkatalg erstellen/gui_einstellungen.json` |
| Startdatei (Doppelklick) | `…/Bildkatalg erstellen/Bilderwerk Work starten.command` |
| Bilder (Beispiel) | `/Users/kim/Dropbox/Archiv/Bilder/…` |

> **Wichtig:** Programme immer mit `/opt/homebrew/bin/python3` starten. Ein anderes Python (z. B. eine aktivierte `.venv`) hat kein Pillow/numpy → Farbgebung wird dann nicht gemessen (die GUI zeigt das verwendete Python und warnt).

---

## 3. Programmdateien

| Datei | Aufgabe |
|---|---|
| `bildanalyse.py` | Kernmodul: Prompt, JSON-Schema, Modellaufruf mit Wiederholung, Farbmessung, Bildmaße, PDF-Seiten rendern, Nachbearbeitung. Definiert Standardpfade. |
| `metadaten.py` | Metadaten per ExifTool (Rückfall Pillow); Datum notfalls aus dem Dateinamen |
| `katalog.py` | Batch-Lauf über einen Ordner (rekursiv), Datenbank, Protokoll, Sicherung. Auch per Kommandozeile nutzbar. |
| `katalog_gui.py` | Tkinter-Oberfläche: Start/Pause/Stopp, Fortschritt, Restzeit, Vorschau, Meldungen |
| `suche.py` | Suche per Kommandozeile (Volltext + Filter), Statistik, Fehlerliste, Detailanzeige |
| `farbe_neu_messen.py` | Farbgebung aller Einträge neu messen (ohne KI-Lauf) |
| `bild_test.py` | Einzelbilder testen, ohne Datenbank |

---

## 4. Verarbeitungsablauf (pro Bild)

1. **Dateien finden:** rekursiv, Endungen `.jpg .jpeg .png .heic .heif .tif .tiff .webp .pdf`; versteckte Dateien und Dateien < 30 KB werden ignoriert. Jede **PDF-Seite** wird ein eigener Eintrag.
2. **Schon erledigt?** Übersprungen, wenn Pfad + Größe + Änderungszeit bereits mit Status `ok`/`duplikat` existieren → Abbruch/Fortsetzen jederzeit möglich.
3. **Hash (SHA-256)** → ist der Inhalt schon analysiert, wird der Eintrag als **Duplikat** angelegt (KI-Felder werden übernommen, eigene Ablage bleibt).
4. **Metadaten** lesen.
5. **Bild vorbereiten:** auf 1024 px (PDF-Seiten 1280 px) verkleinert, als JPEG.
6. **Farbgebung messen** (Sättigung/Farbton-Statistik, kalibriert an echten Problemfällen).
7. **Modellaufruf** mit festem JSON-Schema; Ordner- und Dateiname werden als Hinweis mitgegeben. Bis zu 3 Versuche bei abgeschnittener/ungültiger Antwort.
8. **Nachbearbeitung:** Motiv auf feste Liste abbilden, Wetter-Plausibilität, Füllwörter und Dubletten aus Listen entfernen.
9. **Speichern** + Begriffs- und Volltextindex aktualisieren; alle 50 Bilder Sicherungskopie (optional).

---

## 5. Datenbank

**Format:** SQLite 3, Journal-Modus **WAL**, `PRAGMA user_version = 3` (Schema-Version). Ältere Version 2 wird automatisch um die PDF-Spalten erweitert.

### 5.1 Tabelle `bilder` (ein Datensatz je Bild bzw. PDF-Seite)

**Datei & Ablage**

| Spalte | Typ | Inhalt |
|---|---|---|
| `id` | INTEGER PK | |
| `pfad` | TEXT UNIQUE | **Schlüssel.** Absoluter Pfad; bei PDF-Seiten `…/Album.pdf#seite=3` |
| `archiv_wurzel` | TEXT | beim Lauf gewählter Ordner |
| `relativer_pfad` | TEXT | relativ zur Wurzel; bei PDF `…/Album.pdf [Seite 3/40]` |
| `ordner` | TEXT | relativer Ordner (leer = Hauptordner) |
| `dateiname`, `endung` | TEXT | |
| `hash` | TEXT | SHA-256 der Datei; bei PDF-Seiten `<hash>#<seite>` |
| `groesse`, `mtime` | INTEGER, REAL | Dateigröße, Änderungszeit (zur Erkennung geänderter Dateien) |
| `seite`, `seiten_gesamt` | INTEGER | nur bei PDF, sonst NULL |

**Gemessene Bildeigenschaften**

| Spalte | Werte |
|---|---|
| `breite`, `hoehe` | Pixel (bei PDF: der gerenderten Seite) |
| `ausrichtung` | `quer` · `hoch` · `quadratisch` |
| `farbgebung` | `farbig` · `schwarzweiß` · `sepia` · `monochrom getönt` · `entsättigt` · (`nicht gemessen`, falls Pillow fehlte) |

**Metadaten** (aus EXIF/IPTC/XMP)

`aufnahme_datum` (ISO `YYYY-MM-DDTHH:MM:SS`), `datum_quelle` (`metadaten` | `dateiname`), `kamera_hersteller`, `kamera_modell`, `objektiv`, `brennweite`, `brennweite_kb`, `blende`, `belichtungszeit`, `iso`, `blitz`, `gps_lat`, `gps_lon`, `gps_hoehe`, `software`, `urheber`, `copyright`, `meta_titel`, `meta_beschreibung`, `meta_stichworte` (kommagetrennt), `metadaten_json` (**alle** Felder als JSON), `ort` (vorgesehen, noch leer).

**KI-Analyse**

| Spalte | Typ / Werte |
|---|---|
| `kurztitel` | 3–6 Wörter |
| `beschreibung` | ausführlich, 5–10 Sätze |
| `bildart` | `Foto` · `Grafik/Illustration` · `Gemälde/Zeichnung` · `3D-Rendering` · `Scan/Dokument` · `Screenshot` · `Fotoalbum/Collage` · `unklar` |
| `motiv` | `Porträt` · `Gruppenbild` · `Menschen in Aktion` · `Landschaft` · `Stadt/Architektur` · `Innenraum` · `Tier` · `Pflanze/Blume` · `Fahrzeug` · `Essen/Getränk` · `Figur/Spielzeug` · `Objekt/Stillleben` · `Dokument/Text` · `Sonstiges` |
| `motiv_roh` | Originalangabe des Modells vor der Zuordnung |
| `anlass` | frei, z. B. `Hochzeit`, `Urlaub`, `Geburtstag`; sonst `kein erkennbarer Anlass` |
| `innen_aussen` | `innen` · `außen` · `unklar` |
| `tageszeit` | `Morgen` · `Tag` · `Abend/Dämmerung` · `Nacht` · `unklar` |
| `jahreszeit` | `Frühling` · `Sommer` · `Herbst` · `Winter` · `unklar` |
| `wetter` | `sonnig` · `leicht bewölkt` · `bedeckt` · `Regen` · `Schnee` · `Nebel` · `Gewitter` · `entfällt` (innen) · `unklar` |
| `szene` | frei, wenige Worte |
| `stimmung` | **JSON-Liste**, 1–3 Begriffe |
| `personen_anzahl` | INTEGER (nur echte Menschen) |
| `personen_merkmale` | **JSON-Liste**, z. B. `Brille`, `Bart`, `Kind`, `Senior`, `Brautkleid` |
| `schlagworte` | **JSON-Liste**, 15–25 konkrete Begriffe |
| `sichtbarer_text` | Schilder, Wasserzeichen, Handschrift auf Albumseiten |
| `einzelbilder` | **JSON-Liste**, nur bei Albumseiten: `"oben links: …"` je Foto |

**Verwaltung**

| Spalte | Inhalt |
|---|---|
| `status` | `ok` · `duplikat` · `fehler` |
| `fehler` | Fehlermeldung (nur bei `fehler`) |
| `duplikat_von` | `id` des Originals (nur bei `duplikat`) |
| `modell`, `prompt_version` | z. B. `qwen/qwen3.5-9b`, `4` – für spätere Nachanalysen |
| `tokens`, `dauer_s`, `erfasst` | Ausgabe-Tokens, Analysedauer, Zeitpunkt (ISO) |

### 5.2 Tabelle `begriffe` (Einzelbegriffe für schnelle Filter)

| Spalte | Inhalt |
|---|---|
| `bild_id` | → `bilder.id` |
| `typ` | `schlagwort` · `merkmal` · `stimmung` · `stichwort` (aus Metadaten) |
| `wert` | **kleingeschrieben** |

### 5.3 Volltextindex `suche` (FTS5, Tokenizer `trigram`)

`rowid` = `bilder.id`. Spalten:

- `ablage` – relativer Pfad + Dateiname
- `ki_text` – Titel, Beschreibung, Kategorien, Schlagworte, Merkmale, sichtbarer Text, Einzelbilder, Farbgebung
- `meta_text` – Kamera, Objektiv, Urheber, Metadaten-Titel/-Stichworte, Datum

**Trigram** findet auch Wortteile (`landschaft` trifft `Berglandschaft`, `boot` trifft `Boote`) – wichtig für deutsche Komposita. Suchbegriffe brauchen **mindestens 3 Zeichen**; kürzere per `LIKE` auf `beschreibung`/`schlagworte`.

---

## 6. Abfragebeispiele

```sql
-- Volltext (alle Begriffe müssen vorkommen)
SELECT b.id, b.relativer_pfad, b.kurztitel
FROM bilder b
WHERE b.status IN ('ok','duplikat')
  AND b.id IN (SELECT rowid FROM suche WHERE suche MATCH '"baum" AND "landschaft"');

-- Nur in einer Spalte suchen (z. B. Ordnernamen)
SELECT rowid FROM suche WHERE suche MATCH 'ablage : "hochzeit"';

-- Kategorien kombinieren
SELECT id, relativer_pfad FROM bilder
WHERE status = 'ok' AND farbgebung = 'schwarzweiß' AND ausrichtung = 'hoch';

-- Personenmerkmal über die Begriffstabelle
SELECT b.* FROM bilder b
JOIN begriffe g ON g.bild_id = b.id
WHERE g.typ = 'merkmal' AND g.wert = 'brille';

-- JSON-Listen direkt auswerten
SELECT b.id, j.value AS schlagwort
FROM bilder b, json_each(b.schlagworte) j
WHERE b.id = 42;

-- Zeitraum
SELECT id, aufnahme_datum, relativer_pfad FROM bilder
WHERE aufnahme_datum >= '2015' AND aufnahme_datum < '2016';

-- Alle Seiten eines Fotobuchs
SELECT seite, kurztitel, einzelbilder FROM bilder
WHERE dateiname = 'Album.pdf' ORDER BY seite;
```

**Duplikate:** Für Suchergebnisse meist `status = 'ok'` verwenden (sonst erscheint dasselbe Bild mehrfach); `status IN ('ok','duplikat')` zeigt alle Ablageorte.

---

## 7. Regeln für ein Programm, das die Datenbank nutzt

1. **Nur lesend öffnen**, solange der Katalog-Lauf aktiv sein kann:
   ```python
   db = sqlite3.connect(f"file:{pfad}?mode=ro", uri=True)
   ```
   Dank WAL ist gleichzeitiges Lesen während des Laufs sicher.
2. **Schema nicht verändern.** Eigene Daten (Favoriten, Sammlungen, Collagen-Listen …) in eine **eigene Datenbank** legen und über `bilder.id` oder besser `bilder.pfad` verknüpfen (`pfad` bleibt bei Neuaufbau stabil, `id` nicht zwingend). Alternativ per `ATTACH DATABASE`.
3. **Für Auswertungen unterwegs** kann die Sicherungskopie (falls eingerichtet, z. B. in Dropbox) genutzt werden – ebenfalls nur lesend.
4. **Datei zu einem Eintrag auflösen:**
   ```python
   def quelle(pfad: str):
       if "#seite=" in pfad:
           datei, seite = pfad.rsplit("#seite=", 1)
           return datei, int(seite)
       return pfad, None
   ```
   (identisch zu `bildanalyse.quelle_aufloesen`)
5. **Vorschau/Thumbnail erzeugen:**
   - Bild: `sips -Z 400 -s format jpeg <datei> --out <ziel.jpg>` (kann HEIC/WebP)
   - PDF-Seite: `pdftoppm -f N -l N -scale-to 400 -jpeg -singlefile <datei.pdf> <ziel>`
6. Pfade sind **absolute macOS-Pfade** (teils in Dropbox). Liegt eine Datei nicht lokal vor (Dropbox „nur online“), schlägt der Zugriff fehl → abfangen.

---

## 8. Datenqualität und bekannte Grenzen

- **Beschreibungen** sind detailliert und gut durchsuchbar; Kategorien sind gut, aber nicht fehlerfrei (9B-Modell). Typische Schwächen: Tageszeit/Jahreszeit eher vorsichtig (`unklar`), Anlass nur erkannt, wenn im Bild sichtbar – der Ordnername hilft hier oft mehr (Volltext auf `ablage`).
- **Personen** werden nur beschrieben, **nie identifiziert**. Wer auf einem Bild ist, steht nicht in der Datenbank (geplant: Abgleich mit Apple Fotos, s. u.).
- **Farbgebung** wird gemessen; kalibriert u. a. gegen warmes Kunstlicht (darf nicht als Sepia gelten) und graue Motive mit Farbinseln (nicht Schwarzweiß). Im Zweifel → `farbig`.
- **Aufnahmedatum** kann falsch sein, wenn die Kamera-Uhr nicht gestellt war (z. B. `2000-01-01`). `datum_quelle = 'dateiname'` heißt: aus dem Dateinamen abgeleitet.
- **Handschrift** (Albumseiten) wird nicht immer korrekt gelesen.
- **PDF-Verarbeitung** ist implementiert und mit Testdaten geprüft; Praxistest mit großen Fotobuch-PDFs steht noch aus.
- Einträge mit `status = 'fehler'` haben keine KI-Felder; sie werden beim nächsten Lauf über denselben Ordner erneut versucht.

---

## 9. Bedienung (Kurzfassung)

```bash
cd "/Users/kim/KI-Laufzeitumgebungen/Bildkatalg erstellen"
# Oberfläche (oder Doppelklick auf „Bilderwerk Work starten.command“)
/opt/homebrew/bin/python3 katalog_gui.py

# Suche / Auswertung
python3 suche.py brille --farbe schwarzweiß
python3 suche.py --motiv Landschaft --wetter sonnig --ausrichtung quer
python3 suche.py --statistik
python3 suche.py --fehler
python3 suche.py --zeige 123
```

Voraussetzung für Analyseläufe: LM Studio läuft, Modell geladen, Server gestartet (Developer-Bereich → Status *Running*), Stromsparmodus aus.

---

## 10. Offene Punkte und Ideen

- **Dauerlauf auf dem Mac mini** (ca. 81.000 Bilder, mehrere Wochen bis Monate, abschnittsweise nach Priorität)
- **Ortsnamen aus GPS** in Spalte `ort` nachtragen (ohne KI-Lauf)
- **Apple-Fotos-Mediathek** einbinden (z. B. per `osxphotos`): Personennamen, Orte, Alben
- **Semantische Suche** über Embeddings (LM Studio bringt `nomic-embed-text-v1.5` mit) – findet „Sonnenuntergang am Meer“ auch bei „Abendstimmung an der Küste“
- **Fotobuch Stufe 2:** Einzelfotos auf Albumseiten automatisch ausschneiden und einzeln katalogisieren
- **Such-Oberfläche** mit Vorschaubildern (Grundlage für das Folgeprojekt)
