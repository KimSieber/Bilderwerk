# Auftrag Bildkatalog – Vorschaubilder in der Datenbank

**Stand:** 04.11.2026 · **Auftraggeber:** Kim · **Projekt:** Bildkatalog (Python, `katalog.py` u. a.)
**Abnehmer:** Bilderwerk View (View) – liest die Vorschauen aus der Sicherungskopie der Katalog-DB

---

## 1. Ziel

Der Katalog-Lauf erzeugt zu jedem erfassten Bild (und jeder PDF-Seite) ein **Vorschaubild mit 640 px** und legt es
**in der Katalog-Datenbank** ab. Damit kann View suchen und blättern, ohne die Originale zu laden – alle Bilder in
Dropbox dürfen auf „nur online“ stehen. Originale werden nur noch zur Weiterverwendung (Öffnen, Sichern, Teilen) geladen.

Für die ca. 24.000 bereits erfassten Bilder werden die Vorschauen in einem eigenen Nachhol-Lauf ohne KI erzeugt.

## 2. Festlegungen (entschieden)

| Punkt | Festlegung |
|---|---|
| Ablage | **in der Katalog-DB** (keine separate Datei) |
| Größe | längste Kante **640 px** (nie vergrößern; kleinere Bilder bleiben in Originalgröße) |
| Erzeugung | im **Katalog-Lauf**, zusätzlich Nachhol-Skript für den Bestand |
| Sicherung | Sicherungskopie künftig **alle 1.000 Bilder** statt alle 50 (einstellbar) |

## 3. Datenmodell

Neue Tabelle – bewusst **nicht** als Spalte in `bilder`:
große BLOBs in `bilder` würden jede Suche über die Tabelle verlangsamen (SQLite liest Zeilen samt Überlaufseiten).

```sql
CREATE TABLE IF NOT EXISTS vorschauen (
    hash     TEXT PRIMARY KEY,   -- gleicher Wert wie bilder.hash (PDF-Seite: '<hash>#<seite>')
    jpeg     BLOB NOT NULL,      -- JPEG, längste Kante ≤ 640 px
    breite   INTEGER NOT NULL,   -- Maße der Vorschau (nach EXIF-Drehung)
    hoehe    INTEGER NOT NULL,
    version  INTEGER NOT NULL,   -- Verfahrensversion, Start: 1 (erlaubt späteres Neuerzeugen)
    erzeugt  TEXT NOT NULL       -- ISO-Zeitpunkt
) WITHOUT ROWID;
```

- **Schlüssel = Datei-Hash**, nicht Pfad: Duplikate (≈ 2.500) teilen sich eine Vorschau; Verschieben/Umbenennen
  einer Datei macht die Vorschau nicht ungültig. View verknüpft über `bilder.hash = vorschauen.hash`.
- **Schema-Version:** `PRAGMA user_version = 4`. Migration 3 → 4 legt nur die Tabelle an (bestehende Daten unverändert).
- Keine weiteren Änderungen an `bilder`, `begriffe`, `suche`.

## 4. Erzeugung der Vorschau

| Schritt | Vorgabe |
|---|---|
| Quelle | das Original (Bild) bzw. die gerenderte PDF-Seite – **nicht** das 1024-px-Modellbild (das ist für die KI optimiert und ggf. anders beschnitten/komprimiert). Wenn das 1024-px-Bild nachweislich unverändert aus dem Original entsteht, darf es als Quelle dienen (spart Zeit). |
| Drehung | **EXIF-Orientation anwenden** (`ImageOps.exif_transpose`), Ergebnis ohne Drehangabe speichern |
| Farbraum | nach **RGB/sRGB** wandeln (CMYK, Palette, Graustufen mit Alpha …); Transparenz auf **Weiß** legen |
| Verkleinern | längste Kante 640 px, Filter **LANCZOS**; nie vergrößern |
| Format | **JPEG, Qualität 80**, `optimize=True`, nicht progressiv, **ohne Metadaten** (kein EXIF/GPS in der Vorschau) |
| HEIC/HEIF | wie bisher im Katalog gelesen (pillow-heif bzw. bestehender Weg) |
| PDF | je Seite ein Eintrag, Quelle = die vom Katalog gerenderte Seite (bisher 1280 px) |
| Duplikate | existiert `hash` schon in `vorschauen` → nichts tun |
| Fehler | Vorschau-Fehler darf den Katalog-Eintrag **nicht** verhindern: protokollieren, Bild ohne Vorschau weiter erfassen; der Nachhol-Lauf versucht es erneut |

Erwartung: **40–70 KB je Vorschau**, bei 81.000 Bildern **ca. 4–5 GB** zusätzlich in der DB.

## 5. Katalog-Lauf

1. Nach erfolgreicher Analyse (Status `ok`) bzw. beim Anlegen eines Duplikats: Vorschau erzeugen, falls `hash` noch fehlt.
2. Schreiben in **derselben Transaktion** wie der `bilder`-Eintrag (beides oder nichts).
3. **Sicherungskopie:** Intervall von 50 auf **1.000 Bilder** (Einstellung/Konstante, nicht hart verdrahtet).
   Die Kopie muss **konsistent** sein – per SQLite-Backup-API (`sqlite3.Connection.backup`) oder `VACUUM INTO`,
   nicht per Dateikopie bei offener WAL-DB.

## 6. Nachhol-Skript `vorschau_nachholen.py`

Für alle Einträge mit `status IN ('ok','duplikat')`, deren `hash` nicht in `vorschauen` steht:

- Datei auflösen (`…pdf#seite=N` → Datei + Seite, wie `quelle_aufloesen`), Vorschau nach Abschnitt 4 erzeugen.
- **Fortsetzbar:** jederzeit abbrechen und neu starten; Commit alle 200 Vorschauen.
- **Nur lokal vorhandene Dateien** verarbeiten; Dropbox-Platzhalter („nur online“: 0 Byte) **überspringen und zählen**
  (nicht herunterladen). Option `--auch-online` darf Platzhalter bewusst laden.
- PDFs: Datei **einmal** öffnen, alle fehlenden Seiten erzeugen.
- Fortschritt mit Restzeit (wie GUI/Katalog), Abschlussbericht: erzeugt · übersprungen (nur online) · fehlerhaft (mit Pfad).
- Optionen: `--limit N`, `--nur-ordner PFAD`, `--neu` (alle mit `version` < aktueller Version neu erzeugen).
- Am Ende eine Sicherungskopie anstoßen.

Erwartete Dauer: 0,1–0,3 s je Bild → ca. 1–2 h für 24.000 Bilder (Mac mini, Dateien lokal).

## 7. Prüfkriterien (Abnahme)

| # | Kriterium |
|---|---|
| 1 | Neue DB und bestehende DB (Version 3) werden auf Version 4 gebracht; `bilder`/`suche`/`begriffe` unverändert |
| 2 | Hochkant-Foto mit EXIF-Orientation 6 → Vorschau hat Höhe > Breite und keine EXIF-Daten |
| 3 | JPEG, PNG mit Transparenz, HEIC, WebP, TIFF, CMYK-JPEG, PDF (2 Seiten) → je eine gültige Vorschau ≤ 640 px |
| 4 | Bild kleiner als 640 px wird nicht vergrößert |
| 5 | Duplikat erzeugt keine zweite Vorschau; `bilder.hash` des Duplikats findet die Vorschau des Originals |
| 6 | Nachhol-Lauf: Abbruch und Neustart setzt fort, verarbeitet nichts doppelt; Platzhalter werden gezählt, nicht geladen |
| 7 | Kaputte Bilddatei: Katalog-Eintrag entsteht trotzdem, Fehler im Protokoll |
| 8 | Sicherungskopie ist eine gültige, konsistente SQLite-DB (`PRAGMA integrity_check` = ok) und enthält die Vorschauen |
| 9 | Stichprobe 1.000 Bilder: mittlere Vorschaugröße im Bereich 40–70 KB |

## 8. Nicht Teil dieses Auftrags

- Anzeige in View (View liest `vorschauen` per `bilder.hash`; Rückfall auf den bisherigen Cache/Dropbox-Weg für Einträge ohne Vorschau – wird in View umgesetzt).
- Änderungen an KI-Analyse, Prompt, Kategorien.

## 9. Hinweise für die Umsetzung

- Doku des Katalogs (Abschnitt Datenbank) um Tabelle `vorschauen` und Schema-Version 4 ergänzen;
  Regel „Schema nicht verändern“ für Fremdprogramme bleibt – die Änderung kommt vom Katalog selbst.
- Die DB wird durch die Vorschauen groß: `VACUUM` nach dem Nachhol-Lauf einmalig sinnvoll.
- View öffnet die Sicherungskopie strikt nur lesend (`mode=ro&immutable=1`).
