# Bilderwerk – Anleitung

Bilderwerk katalogisiert eine eigene Bildersammlung mit einer **lokalen KI**, macht sie durchsuchbar
und hilft beim Aufräumen. Bilder und Daten bleiben auf dem eigenen Rechner – es wird nichts in eine
Cloud geschickt.

Diese Anleitung richtet sich an technisch versierte Nutzer. Sie erklärt, was die Teile tun und was
man braucht; sie ersetzt nicht die Dokumentation der verwendeten Werkzeuge (Homebrew, LM Studio …).

---

## 1. Die drei Programme

| Programm | Wofür | Wann |
|---|---|---|
| **Bilderwerk Work** | Analysiert jedes Bild mit einem lokalen KI-Modell und schreibt Beschreibung, Schlagworte, Motiv, Bildart, Stimmung, Personenmerkmale, sichtbaren Text, Farbgebung, Aufnahmedaten (EXIF/GPS) und ein Vorschaubild in die Katalog-Datenbank `Bildkatalog.db`. | einmalig für die ganze Sammlung, danach regelmäßig für Neues |
| **Bilderwerk View** | Durchsucht den Katalog (Freitext, Filter wie Motiv, Jahreszeit, Wetter, Farbgebung) und zeigt die Bilder an. Liest die Datenbank nur. | im Alltag, auch auf anderen Rechnern |
| **Bilderwerk Sweep** | Zeigt Dubletten (byte-gleiche Dateien an verschiedenen Orten) und löscht auf Wunsch einzelne Kopien – Datei und Katalogeintrag gemeinsam. | bei Bedarf |

```
Bildordner ──► Work (+ lokale KI) ──► Bildkatalog.db ──► View  (Suche, Anzeige)
                                             └─────────► Sweep (Dubletten)
```

Work und Sweep laufen auf **dem** Rechner, auf dem die Bilder liegen und die KI läuft. View kann
überall laufen, wo die Datenbank (z. B. eine Sicherungskopie in Dropbox) und die Bilder erreichbar sind.

---

## 2. Voraussetzungen

- **macOS** (genutzt werden `sips`, `caffeinate`, PDFKit). Apple Silicon empfohlen.
- **Homebrew** und daraus:
  ```bash
  brew install python@3.14 python-tk@3.14 pillow numpy exiftool poppler
  ```
  ExifTool liest Aufnahmedaten (inkl. HEIC/RAW), poppler wird nur für PDF-Fotobücher gebraucht.
- **Eine lokale KI** – siehe Abschnitt 3.
- **Für View** derzeit: Node.js, Rust (rustup) und Xcode Command Line Tools zum Bauen
  (siehe `apps/view/README.md`). Fertige Installationsdateien gibt es noch nicht.
- Das Repository als Ordner auf dem Rechner (Work und Sweep nutzen den gemeinsamen Code in `shared/`).

---

## 3. Lokale KI

### Was gebraucht wird
Work schickt jedes Bild mit einer Anweisung an einen **lokalen KI-Server** und erwartet die Antwort
als JSON. Nötig sind:

1. ein Programm, das eine **OpenAI-kompatible Schnittstelle** (`/v1/chat/completions`) bereitstellt, und
2. ein **Vision-Modell** (versteht Bilder), das brauchbar **Deutsch** schreibt und **JSON** liefert.

Geeignete Programme sind z. B. **LM Studio** (einfachster Einstieg, grafische Oberfläche),
**Ollama** oder der **llama.cpp-Server**.

### Referenz: LM Studio mit Qwen 3.5 9B
Bilderwerk wurde mit **LM Studio** und dem Modell **`qwen/qwen3.5-9b`** entwickelt und an einem
Archiv mit rund 80.000 Bildern erprobt. Andere Vision-Modelle sollten funktionieren, sind aber nicht
getestet – die Qualität der Beschreibungen hängt stark vom Modell ab.

Einrichtung in Kurzform:
1. LM Studio installieren (lmstudio.ai).
2. Im Modellkatalog `qwen3.5-9b` (Vision-Variante) suchen und laden.
3. Bereich *Developer*: Modell laden, **Kontextlänge mindestens 8192** einstellen (rechts unter
   *Load → Context and Offload*; Bild, Anweisung und Antwort brauchen zusammen mehrere tausend Token,
   8192 reicht, mehr kostet nur Speicher), Server starten (Status *Running*).
   Standardadresse: `http://127.0.0.1:1234/v1`.
4. In Work: **KI-Server → Einstellen …**, Adresse eintragen, **Verbindung testen**, Modell aus der
   Liste wählen, **Übernehmen**.

Andere Server – nur die Adresse ändert sich:

| Server | Adresse (Standard) |
|---|---|
| LM Studio | `http://127.0.0.1:1234/v1` |
| Ollama | `http://127.0.0.1:11434/v1` |
| llama.cpp-Server | `http://127.0.0.1:8080/v1` |

Ein API-Schlüssel ist lokal meist nicht nötig; das Feld ist für Server gedacht, die einen verlangen.

### Hinweise zur Modellwahl
- **Vision-fähig** muss es sein – reine Textmodelle melden einen Fehler oder erfinden Inhalte.
- **Größe:** Modelle um 7–9 Mrd. Parameter (quantisiert) sind ein guter Kompromiss. Qwen 3.5 9B belegt
  geladen rund 6 GB (LM Studio zeigt „Size 5.98 GB“); mit Kontext 8192 sollten grob 8 GB Arbeitsspeicher
  frei sein. Größere Modelle beschreiben genauer, sind aber deutlich langsamer.
- **„Denkende“ Modelle** (Reasoning) schreiben vor der Antwort lange Gedankengänge – das kostet viel
  Zeit. Work schaltet das Denken bei Qwen ab; bei anderen Modellen ggf. eine Variante ohne Reasoning wählen.
- **Schnittstelle:** Work verlangt die Antwort zuerst mit festem JSON-Schema. Lehnt ein Server das ab,
  versucht Work automatisch einfachere Anfragen und gleicht unvollständige Antworten an.
- **Dauer:** Erfahrungswert mit Qwen 3.5 9B auf einem Mac mini: rund 40–90 Sekunden je neuem Bild.
  Große Sammlungen laufen also Tage – Work ist für den unbeaufsichtigten Dauerlauf gebaut.

---

## 4. Bilderwerk Work

### Start
Doppelklick auf `apps/work/Bilderwerk Work starten.command`. Das Skript sucht ein passendes Python
(ab 3.10 mit Tk) und weist auf fehlende Pakete hin. Beim ersten Mal ggf. das Ausführungsrecht setzen:
`chmod +x "Bilderwerk Work starten.command"`.

### Ein Lauf
1. **Bildordner** wählen – alle Unterordner werden einbezogen.
2. **Datenbank** wählen oder neu anlegen. Empfehlung: lokal ablegen, nicht in einem Cloud-Ordner.
3. Optional **Sicherung**: Ziel für eine regelmäßige Kopie der Datenbank (z. B. in Dropbox – diese
   Kopie kann View auf anderen Rechnern nutzen).
4. **KI-Server** einstellen (Abschnitt 3), **▶ Start**.

Je Bild: Prüfsumme bilden (erkennt Dubletten – sie werden nicht erneut analysiert) → Aufnahmedaten
lesen → Bild verkleinern und an die KI schicken → Antwort prüfen und vereinheitlichen → Farbgebung
aus den Pixeln messen → Vorschaubild erzeugen → speichern.

Erfasst werden JPEG, PNG, HEIC/HEIF, TIFF, WebP, gängige RAW-Formate (CR2/CR3, NEF, ARW, DNG, ORF,
RAF, RW2 …) und jede Seite von PDF-Dateien (z. B. digitalisierte Fotoalben). Sehr kleine Dateien
(Icons, Thumbnails) und versteckte Ordner werden übergangen.
RAW-Bilder werden in voller Auflösung gelesen (macOS `sips`). Die Drehung kommt aus der Datei: Hat die
Kamera keine gespeichert (Drehsensor aus), erscheinen Hochformatbilder quer – wie in jedem anderen Programm.

### Gut zu wissen
- **Pause/Stopp** wirken nach dem aktuellen Bild; der Fortschritt ist immer gespeichert.
- **Erneut starten** mit demselben Ordner analysiert nur Neues und Geändertes, ergänzt fehlende
  Vorschaubilder und entfernt Einträge gelöschter Dateien (Schutz: nicht, wenn auffällig viele fehlen).
- Ist der KI-Server weg, wartet Work und versucht es weiter, statt Fehler zu buchen.
- **Ruhezustand** wird während eines Laufs verhindert (abschaltbar). Mac am Netzteil betreiben.
- **Cloud-Ordner:** Bilder müssen lokal vorliegen. „Nur online“-Dateien (Dropbox, iCloud) werden für
  die Vorschau übersprungen und gezählt – den Ordner vorher offline verfügbar machen.
- Personen werden **nie identifiziert** – die KI beschreibt nur Sichtbares (Anzahl, Merkmale, Kleidung).

---

## 5. Bilderwerk View

Bauen und installieren siehe `apps/view/README.md`. Beim ersten Start die `Bildkatalog.db` (oder
ihre Sicherungskopie) wählen. Suche per Freitext über Beschreibung, Schlagworte, Text im Bild und
Ablageort; Filter links schränken weiter ein. Bilder lassen sich groß anzeigen, im Finder zeigen,
öffnen und teilen. View ändert die Datenbank nicht und bemerkt, wenn eine Sicherungskopie ersetzt wurde.

### Einrichtung (⌘, oder DB-Menü → „Einstellungen…“)

View läuft auf jedem Mac mit eigenen Einstellungen. Änderungen gelten nach **Übernehmen** sofort.

- **Datenbank** – Katalog oder Sicherungskopie wechseln.
- **Vorschau-Cache** – Obergrenze in MB (Standard 2048), „Cache leeren“.
- **Speicherorte** – je Ordner, wie View an Vorschau und Original kommt. Es gilt der längste passende
  Ordner, ohne passende Regel „Lokal“. **Erkannte Ordner vorschlagen** trägt gefundene Cloud-Ordner ein.

  | Art | Für | Datei „nur online“ |
  |---|---|---|
  | Lokal | eigene Platte, externe Laufwerke | Wolke, es wird nichts geladen |
  | Cloud (File Provider) | iCloud, OneDrive, Google Drive, neue Dropbox (`~/Library/CloudStorage`) | lädt beim Lesen still nach |
  | Dropbox (klassisch) | Dropbox im Ordner `~/Dropbox` | Vorschau über die Dropbox-API, Original wird bei Bedarf geladen |

- **Dropbox** – mitgelieferter App-Schlüssel ist eingetragen; **Bei Dropbox anmelden…** öffnet den
  Browser. Der Zugang gilt nur für das eigene Konto, das Token liegt im Schlüsselbund. Ohne Schlüssel
  behandelt View Dropbox-Ordner wie lokale Ordner (Dateien „nur online“ ohne Vorschau).

**Über Bilderwerk View** (App-Menü) zeigt Version, Webseite, Kontakt und Lizenz.

---

## 6. Bilderwerk Sweep

Doppelklick auf `apps/sweep/Bilderwerk Sweep starten.command`. Sweep öffnet dieselbe Datenbank wie
Work (über **Datenbank …** wechselbar – die Wahl gilt dann auch für Work). **Immer mit der
Arbeits-Datenbank, nie mit einer Sicherungskopie arbeiten.**

Links stehen Gruppen identischer Dateien, rechts Vorschau, Ablageort, Auflösung und Größe je Kopie.
**Löschen …** entfernt nach Rückfrage die Datei vom Laufwerk und ihren Eintrag aus dem Katalog; die
KI-Analyse geht nicht verloren, eine verbleibende Kopie übernimmt sie. Jede Löschung wird in
`sweep_protokoll.csv` neben der Datenbank protokolliert. PDFs werden nur angezeigt, nicht gelöscht.

---

## 7. Wo liegt was

| Was | Ort |
|---|---|
| Einstellungen Work und Sweep | `~/Library/Application Support/Bilderwerk/einstellungen.json` |
| Einstellungen View | `~/Library/Application Support/de.sieber-bw.bilderwerk.view/einstellungen.json` (Dropbox-Token im Schlüsselbund) |
| Katalog-Datenbank | frei wählbar; Vorschlag: `~/Library/Application Support/Bilderwerk/Bildkatalog.db` |
| Protokoll eines Laufs | `<Datenbankname>_protokoll.txt` neben der Datenbank |
| Unbrauchbare KI-Antworten | `fehler_antworten/` neben der Datenbank |
| Lösch-Protokoll Sweep | `sweep_protokoll.csv` neben der Datenbank |

---

## 8. Lizenz und Kontakt

Nicht-kommerzielle Nutzung nach der [PolyForm Noncommercial License 1.0.0](../LICENSE); Icons und
App-Symbole sind ausgenommen. Webseite: bilderwerk.sieber-bw.de · Kontakt: Bilderwerk@sieber-bw.de
