# Bilderwerk View – Testdokumentation (Testfallkatalog)

Jeder automatische Testfall wird hier mit **ID, Ort und Zweck** eingetragen, sobald er eingeführt wird.
Manuelle Prüfungen je Version stehen am Ende.

## 1. Testausführung (lokal)

| Befehl | Wirkung |
|---|---|
| `npm test` | alle Schichten: `cargo test` → `vitest run` → `tsc --noEmit`, am Ende **Testsumme** (`scripts/tests.mjs`) |
| `npm run test:rust` | nur Rust (Unit-, Integrations- und Vertragstests) |
| `npm run test:frontend` | nur Frontend (Vitest + Testing Library, jsdom) |
| `npm run test:watch` | Frontend-Tests laufen beim Speichern automatisch mit |

Grundsatz ab v0.0.3: **erst der Test, dann der Code.** Eine Version ist erst fertig, wenn `npm test` grün ist.
Ziel seit v0.0.3: **jede Funktion und jeder Ablauf hat mindestens einen Test.** Ausnahmen stehen in 3.9.

## 2. Aufbau und Kennungen

| Präfix | Schicht | Werkzeug | Ort |
|---|---|---|---|
| `R-SUCH` · `R-FAC` · `R-DB` · `R-VOR` · `R-EINST` · `R-ANB` · `R-ZUO` · `R-DBX` · `R-CACHE` · `R-VDB` · `R-GROSS` · `R-AKT` · `R-TEIL` · `R-BEO` | Rust – Unit-Tests je Modul (`R-DBX` gegen nachgebildeten HTTP-Server auf 127.0.0.1) | `cargo test` | `#[cfg(test)]` in `src-tauri/src/*.rs` |
| `I-DB` · `I-SUCH` · `I-FAC` · `I-ABL` · `I-VDB` · `I-DET` | Integration Rust + SQLite mit Test-DB | `cargo test` | `src-tauri/tests/{db,suche,facetten,ablauf,vorschau_db,details}.rs` |
| `I-VOR` | Integration Rust + sips/PDFKit mit Testbildern (nur macOS) | `cargo test` | `src-tauri/tests/vorschau.rs` |
| `F-LOG` | Frontend – Logik, Hooks, Hilfsfunktionen | Vitest | `src/**/*.test.ts(x)` |
| `F-UI` | Frontend – Komponenten | Vitest + Testing Library | `src/**/*.test.tsx` |
| `V` | Vertrag Rust ↔ Frontend | beide | `src-tauri/tests/vertrag.rs` → `tests/vertrag/*.json` → `src/vertrag.test.ts` |

**Testdaten**

| Baustein | Inhalt |
|---|---|
| Test-DB (`src-tauri/tests/gemeinsam/mod.rs`) | wird je Test frisch erzeugt; Schema wie der Katalog (`bilder` inkl. `hash`, `begriffe`, FTS5 `suche` mit trigram, `user_version = 3`); optional Tabelle `vorschauen` (Schema 4) über `vorschauen_anlegen`; 30 Einträge: Strand/Boot (1), Tempel mit Endung `.JPG` (2), Duplikat von 1 (3), HEIC Hochzeit (4), PNG Berglandschaft mit Kamera-Metadaten (5), PDF-Album Seite 1+2 (6, 7), unplausibles Datum `2056-55-10` (8), ohne Datum (9), Status `fehler` (10), WebP mit kurzem Dateinamen `XQ_…` (11), Serie Mai 2018 (12–30); seit v0.0.5 Kategorie-Spalten (`motiv`, `bildart`, `farbgebung`, `ausrichtung`, `jahreszeit`, `wetter`) und Merkmale (`begriffe.typ = 'merkmal'`: brautkleid 4, brille 8/9, bart 9) je Eintrag aus `kategorien()`; `test_db_ohne_kategorien` = älterer Katalog ohne diese Spalten |
| Testbilder (`src-tauri/tests/daten/`) | `foto.jpg` 300 × 200, `foto-gedreht.jpg` (EXIF-Drehung 6), `grafik.png` 160 × 240 mit Transparenz, `album.pdf` (2 Seiten); 0-Byte-Platzhalter und kaputte Dateien erzeugt der Test selbst. HEIC lässt sich unter Linux nicht erzeugen → folgt bei Bedarf auf dem Mac |
| Vertrag (`tests/vertrag/`) | echte JSON-Antworten (`ergebnis`, `ergebnis-leer`, `dbinfo`, `vorschau-*`, `facetten`, `details`), deterministisch, gehören ins Repo; `cargo test` schreibt sie nur bei Änderung |
| Frontend-Hilfen (`src/test/`) | `setup.ts` (jest-dom, Aufräumen, ResizeObserver), `virtuosoMock.tsx` (rendert alle Einträge, da jsdom nichts misst), `daten.ts` (Treffer, Ergebnis, DbInfo, Facetten), `knoten.d.ts` (Typ für `readFileSync` in F-UI-50) |

**Status:** ✅ aktiv · ⏸ vorhanden, aber vorerst abgeschaltet (`#[ignore]`) · ⏳ geplant · ❌ entfernt (mit Grund)

## 3. Testfallkatalog

### 3.1 Rust – Suchkern (`search.rs`)

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| R-SUCH-01 | `begriffe_zerlegen` | Freitext: Wörter, `"Phrase"` bleibt zusammen, `-wort` = Ausschluss | v0.0.1 | ✅ |
| R-SUCH-02 | `datum_obergrenze` | „bis“-Grenze schließt den ganzen Zeitraum ein | v0.0.1 | ✅ |
| R-SUCH-03 | `leerer_filter` | `ist_leer`: ohne Kriterien / nur Leerzeichen / nur „Duplikate“ leer; jedes einzelne Kriterium nicht leer | v0.0.1 | ✅ |
| R-SUCH-04 | `begriffe_randfaelle` | leere Eingabe, einzelnes `-`, leere Phrase, offenes Anführungszeichen, ausgeschlossene Phrase, Umlaute | v0.0.3 | ✅ |
| R-SUCH-05 | `fts_phrase_maskiert` | FTS5-Phrase: Anführungszeichen verdoppelt, Operatoren (`AND`, `OR`) wirkungslos | v0.0.3 | ✅ |
| R-SUCH-06 | `where_status_und_parameter` | nur `ok` bzw. `ok`+`duplikat`; Werte getrimmt | v0.0.3 | ✅ |
| R-SUCH-07 | `where_kurze_und_lange_begriffe` | < 3 Zeichen → `instr` auf 4 Felder, ≥ 3 → FTS + Archiv-Wurzel; Ausschluss als `NOT` | v0.0.3 | ✅ |
| R-SUCH-08 | `where_endungen_und_datum` | Endungen kleingeschrieben als `IN (?,?)`; Datum von/bis als Parameter | v0.0.3 | ✅ |
| R-SUCH-09 | `facetten_zaehlen_als_kriterium` | gewählte Facette macht den Filter nicht leer; leere Auswahl schon | v0.0.5 | ✅ |
| R-SUCH-10 | `where_facetten` | Spalten-Facette als `IN (?,?)`, Begriffs-Facette als Unterabfrage auf `begriffe`, Werte als Parameter | v0.0.5 | ✅ |
| R-SUCH-11 | `where_facetten_unbekannt_und_leer_ignoriert` | unbekannte Felder (SQL-Einschub) und leere Auswahl erzeugen keine Bedingung | v0.0.5 | ✅ |

### 3.1a Rust – Facetten (`facetten.rs`)

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| R-FAC-01 | `bedingungen` | SQL-Bedingung je Facettenart; 0 Werte und unbekannte Felder → keine | v0.0.5 | ✅ |
| R-FAC-02 | `felder_eindeutig_und_vollstaendig` | Feldnamen eindeutig, nur Kleinbuchstaben, Titel vorhanden | v0.0.5 | ✅ |

### 3.2 Rust – Datenbankzugriff (`db.rs`)

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| R-DB-01 | `uri` | Leerzeichen, `#`, `?` werden für die SQLite-URI kodiert | v0.0.1 | ✅ |
| R-DB-02 | `uri_umlaute_und_prozent` | Umlaute (UTF-8 je Byte) und `%` kodiert, erlaubte Zeichen bleiben | v0.0.3 | ✅ |
| R-DB-03 | `fehlende_datei` | nicht vorhandene Datei → verständliche Meldung | v0.0.3 | ✅ |

### 3.3 Rust – Vorschaubilder (`thumbs.rs`)

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| R-VOR-01 | `quelle_aufloesen` | `…/Album.pdf#seite=3` → Datei + Seite; ungültige Seitenangabe bleibt Teil des Pfads | v0.0.2 | ✅ |
| R-VOR-02 | `fehlende_datei` | nicht vorhandene Datei → `Fehlt` | v0.0.2 | ✅ |
| R-VOR-03 | `hash_stabil` | FNV-1a stabil (Cache bleibt über Versionen gültig) | v0.0.2 | ✅ |
| R-VOR-04 | `nur_online_erkennen` | 0 Byte, `SF_DATALESS`, Größe ohne Blöcke; lokale Datei nicht | v0.0.2 | ✅ |
| R-VOR-05 | `sips_meldung_platzhalter` | sips „not a valid file“ → Nur online | v0.0.2 | ✅ |
| R-VOR-06 | `applescript_maskieren` | – | v0.0.2 | ❌ entfernt mit dem Klick-Download |
| R-VOR-07 | `cache_ziel_aufbau` | Cache-Pfad `<cache>/<größe>/<16 Hex>.jpg`; neuer Schlüssel bei anderer Zeit, Größe, Datei oder PDF-Seite | v0.0.3 | ✅ |
| R-VOR-08 | `cache_treffer_ohne_werkzeug` | vorhandene Cache-Datei wird direkt geliefert (ohne sips) | v0.0.3 | ✅ |
| R-VOR-09 | `platzhalter_wird_nicht_angefasst` | echte 0-Byte-Datei → `NurOnline`, im Cache entsteht nichts | v0.0.3 | ✅ |
| R-VOR-10 | `werkzeug_suchen` | – | v0.0.3 | ❌ entfernt mit pdftoppm (PDFKit braucht kein Werkzeug) |
| R-VOR-11 | `eindeutig_steigt` | Zwischendatei-Namen kollidieren nicht | v0.0.3 | ✅ |
| R-VOR-12 | `download_erlauben_laeuft` | `download_beim_lesen_erlauben` läuft (mehrfach) ohne Absturz | v0.0.3 | ✅ |
| R-VOR-13 | `vorschau_json_format` | JSON-Form `{"art":"bild","datei":…}` usw. für alle fünf Fälle (inkl. `datenbank`) | v0.0.3 | ✅ |
| R-VOR-14 | `jpeg_masse_lesen` | Maße aus dem JPEG-Kopf; kein JPEG/abgeschnitten → None ohne Absturz | v0.0.3 | ✅ |
| R-VOR-15 | `verkleinern_nur_wenn_noetig` | nur größer als 640 wird verkleinert, nie vergrößert; kleine Datei bleibt unangetastet | v0.0.3 | ✅ |

### 3.4 Rust – Einstellungen (`settings.rs`)

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| R-EINST-01 | `speichern_und_laden` | Rundlauf; JSON in camelCase (`dbPfad`) | v0.0.3 | ✅ |
| R-EINST-02 | `fehlende_oder_kaputte_datei` | fehlende/kaputte Datei → Standardwerte, kein Absturz | v0.0.3 | ✅ |
| R-EINST-03 | `vorschlag_nur_wenn_datei_existiert` | Vorschlag aus `view.local.json` nur, wenn die Datei existiert; gespeicherter Pfad hat Vorrang | v0.0.3 | ✅ |
| R-EINST-05 | `cache_grenze` | Cache-Obergrenze: Standard 2 GB, eigener Wert, ältere einstellungen.json bleibt lesbar | v0.0.3 | ✅ |
| R-EINST-04 | `speichern_legt_ordner_an` | Konfigurationsordner wird angelegt; `laden` (mit eingebautem Vorschlag) läuft | v0.0.3 | ✅ |

### 3.4a Rust – Speicheranbieter (`speicher/`)

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| R-ANB-01 | `zustand_erkennen` | Lokal · NurOnline (0 Byte, „dataless“) · Fehlt; PDF-Seite → Datei – für beide Anbieter | v0.0.3 | ✅ |
| R-ANB-02 | `vorschau_fehlt_und_platzhalter` | Fehlt; 0 Byte → Wolke ohne Cache-Eintrag – beide Anbieter | v0.0.3 | ✅ |
| R-ANB-03 | `vorschau_aus_cache` | Cache-Treffer auch für „dataless“-Dateien – beide Anbieter | v0.0.3 | ✅ |
| R-ANB-04 | `dataless_lokal_wolke_file_provider_liest` | „dataless“: `lokal` → Wolke ohne Versuch; `file-provider` versucht zu erzeugen (Lesen lädt nach) | v0.0.3 | ✅ |
| R-ANB-05 | `original_bereitstellen` | Original: Datei, PDF-Seite → ganze PDF, fehlt, nur online; `file-provider` liefert auch „dataless“ | v0.0.3 | ✅ |
| R-ANB-06 | `art_und_name` | Kurznamen; Anbieter `Send + Sync`, als `Arc<dyn …>` teilbar | v0.0.3 | ✅ |

### 3.4b Rust – Zuordnung (`speicher/zuordnung.rs`)

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| R-ZUO-01 | `regeln_lesen_gueltig` | `anbieter` aus view.local.json; abschließendes `/` entfernt | v0.0.3 | ✅ |
| R-ZUO-02 | `regeln_lesen_fehler` | kaputtes JSON, unbekannte Art, relativer Pfad, fehlende Art → verständliche Meldung | v0.0.3 | ✅ |
| R-ZUO-03 | `laengster_praefix_gewinnt` | längster passender Ordner gewinnt (Unterordner-Ausnahme) | v0.0.3 | ✅ |
| R-ZUO-04 | `nur_ganze_ordnernamen` | `/…/Dropbox` passt nicht auf `/…/Dropbox (Firma)` oder `/…/Dropbox2` | v0.0.3 | ✅ |
| R-ZUO-05 | `ohne_passende_regel_lokal` | ohne Regel → `lokal` | v0.0.3 | ✅ |
| R-ZUO-06 | `pdf_seite_nach_datei` | `…pdf#seite=N` wird nach der Datei zugeordnet | v0.0.3 | ✅ |
| R-ZUO-07 | `dropbox_pfad_umsetzen` | `/Users/kim/Dropbox/Archiv/x.jpg` → `/Archiv/x.jpg`; PDF-Seite → Datei; fremde Pfade → keiner | v0.0.3 | ✅ |
| R-ZUO-08 | `verteiler_liefert_anbieter` | Verteiler liefert je Pfad den Anbieter (inkl. `dropbox-klassisch`, PDF-Seiten) | v0.0.3 | ✅ |
| R-ZUO-09 | `aus_konfiguration` | ohne Konfiguration alles `lokal`; kaputte Konfiguration → Fehlermeldung + Rückfall `lokal` | v0.0.3 | ✅ |

### 3.4c Rust – Dropbox (`speicher/dropbox/`)

Alle Tests laufen gegen einen nachgebildeten Server (`Server::starte` in `dropbox/tests.rs`) – kein echtes Konto, kein Netz.

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| R-DBX-01 | `pkce_challenge_und_zufall` | S256-Challenge (Beispiel RFC 7636), Zufallswert mit erlaubten Zeichen | v0.0.3 | ✅ |
| R-DBX-02 | `kodieren_und_dekodieren` | Prozentkodierung hin und zurück, Umlaute, kaputte Kodierung | v0.0.3 | ✅ |
| R-DBX-03 | `anmelde_url_und_rueckruf` | Anmelde-URL (offline, S256, redirect, state); Rückruf: Code, falscher state, abgelehnt, ohne Code | v0.0.3 | ✅ |
| R-DBX-04 | `api_arg_nur_ascii` | `Dropbox-API-Arg` reines ASCII (Umlaut, Emoji als \\u), bleibt gültiges JSON | v0.0.3 | ✅ |
| R-DBX-05 | `fehlerarten` | HTTP-Status → Fehlerart (401, 429/503 + Retry-After, 409 not_found/sonst, invalid_grant) | v0.0.3 | ✅ |
| R-DBX-06 | `wiederholen_bei_drosselung` | Retry-After beachtet, sonst exponentiell; max. 5 Versuche; andere Fehler sofort | v0.0.3 | ✅ |
| R-DBX-07 | `anmelden_mit_pkce` | ganzer Anmeldeablauf mit simuliertem Browser: Code-Tausch, Verifier passt zur Challenge, Refresh-Token abgelegt | v0.0.3 | ✅ |
| R-DBX-08 | `anmelden_falscher_state_und_frist` | fremder state → kein Code-Tausch; Frist abgelaufen → Meldung | v0.0.3 | ✅ |
| R-DBX-09 | `token_wird_gemerkt_und_erneuert` | Access-Token aus Refresh-Token, einmal geholt und gemerkt; Kopfzeilen und Argumente der Vorschau-Anfrage (`w960h640` → jede Ausrichtung ≥ 640 px) | v0.0.3 | ✅ |
| R-DBX-10 | `abgelaufenes_token_wird_ersetzt` | 401 → neues Token, Anfrage wiederholt | v0.0.3 | ✅ |
| R-DBX-11 | `drosselung_wird_wiederholt` | 429 mit Retry-After → wiederholt bis Erfolg | v0.0.3 | ✅ |
| R-DBX-12 | `abmelden_widerruft_und_loescht` | Abmelden widerruft das Token und löscht die Ablage; doppelt harmlos | v0.0.3 | ✅ |
| R-DBX-13 | `widerrufene_anmeldung` | `invalid_grant` beim Erneuern → Anmeldefehler | v0.0.3 | ✅ |
| R-DBX-14 | `ohne_konto_oder_anmeldung_wolke` | ohne App-Schlüssel oder Anmeldung → Wolke, keine Netzanfrage; fehlende Datei → Fehlt | v0.0.3 | ✅ |
| R-DBX-15 | `vorschau_ueber_api_und_cache` | Platzhalter → Vorschau über API in den Cache, danach ohne Netz; Pfad lokal → Dropbox mit Umlaut; Datei bleibt Platzhalter | v0.0.3 | ✅ |
| R-DBX-16 | `lokale_datei_ohne_netz` | lokal vorhandene Dropbox-Dateien nie über das Netz | v0.0.3 | ✅ |
| R-DBX-17 | `nicht_gefunden_und_offline` | 409 not_found → Fehlt; Server nicht erreichbar → Wolke | v0.0.3 | ✅ |
| R-DBX-18 | `heic_abgelehnt_und_nicht_umwandelbar_per_download` | HEIC zuerst über die Vorschau-API; `unsupported_extension` → Download und für weitere HEIC kein API-Versuch mehr; JPEG, das Dropbox nicht umwandeln kann (> 20 MB …) → Download; Kopien gelöscht | v0.0.3, geändert v0.0.5 | ✅ |
| R-DBX-19 | `pdf_seiten_laden_einmal` | 4 Seiten parallel → ein Download, Kopie bleibt kurz vorgehalten | v0.0.3 | ✅ |
| R-DBX-20 | `original_bereitstellen` | nur online → Download mit Originalnamen; lokal → Datei selbst; nicht angemeldet → Hinweis | v0.0.3 | ✅ |
| R-DBX-21 | `heic_ueber_vorschau_api` | HEIC-Vorschau direkt von Dropbox – kein Download des Originals | v0.0.5 | ✅ |
| R-DBX-22 | `vorschau_endungen` | HEIC/HEIF gehören zu den API-Endungen, PDF nicht | v0.0.5 | ✅ |

### 3.4d Rust – Cache und DB-Vorschauen (`cache.rs`, `vorschau_db.rs`)

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| R-CACHE-01 | `info_zaehlt_rekursiv` | Größe und Anzahl über alle Unterordner; fehlender Ordner → 0 | v0.0.3 | ✅ |
| R-CACHE-02 | `leeren_loescht_alles` | alle Dateien weg, Ordner bleiben | v0.0.3 | ✅ |
| R-CACHE-03 | `begrenzen_aelteste_zuerst` | unter der Grenze nichts; sonst älteste zuerst, bis die Grenze erreicht ist | v0.0.3 | ✅ |
| R-CACHE-04 | `beruehren_schuetzt_vor_loeschen` | Cache-Treffer frischt die Datei auf → bleibt beim Begrenzen erhalten | v0.0.3 | ✅ |
| R-VDB-01 | `hash_aus_uri` | `vorschau://localhost/<kodiert>` → Hash (`#`, Umlaut, kaputte Kodierung) | v0.0.3 | ✅ |
| I-VDB-01 | `ohne_tabelle` | ältere Katalog-DB ohne `vorschauen`: kein Fehler, nur „keine Vorschau“ | v0.0.3 | ✅ |
| I-VDB-02 | `mit_tabelle_nach_hash` | Vorschau über `bilder.hash`; Duplikat findet die Vorschau des Originals; PDF je Seite; fehlende → keine | v0.0.3 | ✅ |
| I-VDB-03 | `jpeg_lesen` | JPEG-Daten nach Hash | v0.0.3 | ✅ |
| I-VDB-04 | `info_meldet_vorschauen` | DbInfo `vorschauen`: ohne Tabelle `null`, sonst Anzahl | v0.0.3 | ✅ |

### 3.4e Rust – Detailbereich (`details.rs`, `gross.rs`, `aktionen.rs`, `teilen.rs`) – v0.0.6

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| I-DET-01 | `alle_spalten_in_tabellenreihenfolge` | alle Spalten von `bilder` mit Wert, Reihenfolge wie in der Tabelle; Zahlen als Zahl, JSON-Listen als Text | v0.0.6 | ✅ |
| I-DET-02 | `leere_werte_fehlen` | NULL und leerer Text entfallen | v0.0.6 | ✅ |
| I-DET-03 | `begriffe_nach_typ` | `begriffe` je Typ (schlagwort, merkmal), Reihenfolge wie erfasst | v0.0.6 | ✅ |
| I-DET-04 | `duplikate_in_beide_richtungen` | Original zeigt seine Duplikate, Duplikat zeigt sein Original; ohne Duplikat leer | v0.0.6 | ✅ |
| I-DET-05 | `unbekannte_id` | Fehlertext nennt die ID | v0.0.6 | ✅ |
| I-DET-06 | `aeltere_db_ohne_kategorien` | ältere DB: fehlende Spalten fehlen einfach | v0.0.6 | ✅ |
| R-GROSS-01 | `fehlende_datei` | Original fehlt → `Fehlt` | v0.0.6 | ✅ |
| R-GROSS-02 | `vorhandene_grosse_vorschau_ohne_werkzeug` | schon erzeugte große Vorschau wird ohne sips geliefert; liegt im eigenen Ordner, nicht im Vorschau-Cache | v0.0.6 | ✅ |
| R-GROSS-03 | `nur_online_meldet_fehler` | Platzhalter ohne Download-Möglichkeit → Fehlermeldung „nur online“ | v0.0.6 | ✅ |
| R-GROSS-04 | `begrenzen_behaelt_die_neuesten` | höchstens N große Vorschauen, die zuletzt benutzten bleiben; andere Dateien unberührt | v0.0.6 | ✅ |
| R-GROSS-05 | `leeren_entfernt_ordner` | Ordner beim Start leeren, auch wenn er fehlt | v0.0.6 | ✅ |
| R-AKT-01 | `befehle` | `open <datei>` bzw. `open -R <datei>` (Leerzeichen im Pfad) | v0.0.6 | ✅ |
| R-AKT-02 | `vorschlag` | Dateiname für „Sichern unter …“, bei PDF-Seiten die PDF-Datei | v0.0.6 | ✅ |
| R-AKT-03 | `sichern_kopiert` | Kopie am Ziel, Original bleibt; Quelle = Ziel → nichts; fehlende Quelle → Fehler | v0.0.6 | ✅ |
| R-AKT-04 | `ausfuehren_meldet_fehler` | Programm fehlt bzw. Exit-Code ≠ 0 → Fehler | v0.0.6 | ✅ |
| R-TEIL-01 | `koordinaten_umrechnen` | Knopf-Rechteck (oben-links) → AppKit (unten-links), begrenzt | v0.0.6 | ✅ |

### 3.4f Rust – DB neu laden (`beobachter.rs`, `ablauf.rs`) – v0.0.7

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| R-BEO-01 | `unveraendert_nie` | gleicher Stand → nie neu laden | v0.0.7 | ✅ |
| R-BEO-02 | `neu_erst_wenn_stabil` | neuer Stand erst nach zwei gleichen Prüfungen; danach Ruhe | v0.0.7 | ✅ |
| R-BEO-03 | `fehlende_datei_wartet` | Datei kurz weg (mitten im Ersetzen) → warten, danach von vorn | v0.0.7 | ✅ |
| R-BEO-04 | `fehlschlag_wird_wiederholt` | Öffnen scheitert → nächster stabiler Stand versucht es erneut | v0.0.7 | ✅ |
| R-BEO-05 | `stand_erkennt_ersetzen` | Ersetzen per Umbenennen (neuer Inode, gleiche Länge) wird erkannt | v0.0.7 | ✅ |
| I-ABL-06 | `ersetzte_db_neu_laden` | Sicherung ersetzt: alte Verbindung liest weiter den alten Stand, `neu_laden` zeigt den neuen (Vorschauen, Schema 4); Beobachter erkennt es; fehlende DB → Fehler | v0.0.7 | ✅ |

### 3.5 Integration Rust + SQLite (Test-DB)

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| I-DB-01 | `katalog_oeffnen_und_info` | Öffnen; DbInfo: Schema 3, 28 Bilder, 1 Duplikat, Größe, Änderungszeit | v0.0.3 | ✅ |
| I-DB-02 | `pfad_mit_sonderzeichen` | DB in Ordner mit Leerzeichen, `#`, `?`, `&`, Umlaut | v0.0.3 | ✅ |
| I-DB-03 | `nur_lesend` | Schreiben scheitert; keine `-wal`/`-shm`-Dateien | v0.0.3 | ✅ |
| I-DB-04 | `fremde_sqlite_db_abgelehnt` | SQLite ohne Katalog-Tabellen → Meldung | v0.0.3 | ✅ |
| I-DB-05 | `keine_sqlite_datei_abgelehnt` | Textdatei mit `.db` → Fehler | v0.0.3 | ✅ |
| I-DB-06 | `ordner_statt_datei_abgelehnt` | Ordner statt Datei → „nicht gefunden“ | v0.0.3 | ✅ |
| I-SUCH-01 | `freitext_wortteil_und_gross_klein` | Trigram findet Wortteile (`landschaft` → Berglandschaft), Groß/klein egal | v0.0.3 | ✅ |
| I-SUCH-02 | `phrase_und_ausschluss` | Phrase in Reihenfolge, `-wort`, mehrere Begriffe = UND | v0.0.3 | ✅ |
| I-SUCH-03 | `kurze_begriffe` | < 3 Zeichen über Dateiname | v0.0.3 | ✅ |
| I-SUCH-04 | `metadaten_und_archiv_wurzel` | Kamera-Metadaten; Archiv-Wurzel (nicht im Index) | v0.0.3 | ✅ |
| I-SUCH-05 | `dateiname_und_ordner` | Dateiname; Ordner inkl. Archiv-Wurzel | v0.0.3 | ✅ |
| I-SUCH-06 | `datum_von_bis` | Jahr, Monat, Tag; Grenzen eingeschlossen | v0.0.3 | ✅ |
| I-SUCH-07 | `dateitypen` | Endungsgruppen; `.JPG` zählt als `.jpg` | v0.0.3 | ✅ |
| I-SUCH-08 | `duplikate_und_fehler` | Duplikate nur auf Wunsch; Status `fehler` nie | v0.0.3 | ✅ |
| I-SUCH-09 | `sortierung_unplausibel_ans_ende` | neueste zuerst; unplausibles Datum, dann ohne Datum am Ende | v0.0.3 | ✅ |
| I-SUCH-10 | `limit_und_gesamt` | Limit begrenzt Treffer, Gesamtzahl bleibt; 0 = Standard | v0.0.3 | ✅ |
| I-SUCH-11 | `leerer_filter_ohne_abfrage` | leerer Filter → 0 Treffer ohne Abfrage | v0.0.3 | ✅ |
| I-SUCH-12 | `trefferfelder_vollstaendig` | alle Trefferfelder inkl. PDF-Seite gefüllt | v0.0.3 | ✅ |
| I-SUCH-13 | `sonderzeichen_ohne_fehler` | `"`, `AND`, `(*)`, `c++`, SQL-Einschub, `NEAR(…)` → kein Fehler | v0.0.3 | ✅ |
| I-FAC-01 | `ohne_filter_alle_facetten_mit_anzahl` | alle 8 Facetten in fester Reihenfolge; Anzahl je Wert, sortiert nach Anzahl, dann Wert; Duplikat und Fehler zählen nicht | v0.0.5 | ✅ |
| I-FAC-02 | `auswahl_oder_innerhalb_und_zwischen_facetten` | Auswahl filtert die Suche: ODER innerhalb, UND zwischen Facetten, kombiniert mit Freitext | v0.0.5 | ✅ |
| I-FAC-03 | `merkmale_und_schlagworte_ueber_begriffe` | Merkmale und Schlagworte über `begriffe` | v0.0.5 | ✅ |
| I-FAC-04 | `eigene_auswahl_zaehlt_nicht_gegen_sich_selbst` | disjunktive Zählung: gewählte Facette zeigt weiter alle Werte, andere werden eingeschränkt | v0.0.5 | ✅ |
| I-FAC-05 | `facetten_folgen_den_suchkriterien` | Anzahl folgt Freitext und „Duplikate einbeziehen“ | v0.0.5 | ✅ |
| I-FAC-06 | `gewaehlter_wert_ohne_treffer_bleibt_sichtbar` | gewählter Wert mit 0 Treffern bleibt in der Liste | v0.0.5 | ✅ |
| I-FAC-07 | `aeltere_db_ohne_kategorie_spalten` | älterer Katalog: nur Merkmale/Schlagworte; Auswahl einer fehlenden Spalte → 0 Treffer statt SQL-Fehler | v0.0.5 | ✅ |
| I-ABL-01 | `start_ohne_einstellung` | Ablauf `start` ohne DB → Meldung „Noch keine Datenbank“ | v0.0.3 | ✅ |
| I-ABL-02 | `start_mit_vorschlag` | Vorschlag aus `view.local.json` wird geöffnet, aber nicht gespeichert | v0.0.3 | ✅ |
| I-ABL-03 | `db_setzen_speichert_und_start_nutzt` | `db_setzen` speichert; nächster `start` öffnet sie | v0.0.3 | ✅ |
| I-ABL-04 | `ungueltige_db_aendert_nichts` | ungültige Wahl → Fehler, alte Einstellung bleibt | v0.0.3 | ✅ |
| I-ABL-05 | `start_mit_verschwundener_db` | gespeicherte DB fehlt → Meldung (Oberfläche zeigt Auswahl) | v0.0.3 | ✅ |

### 3.6 Integration Vorschau (nur macOS, echte Werkzeuge)

| ID | Test | Prüft | Seit | Status |
|---|---|---|---|---|
| I-VOR-01 | `jpeg_verkleinern_und_cache` | 300 × 200 → 120 × 80; zweiter Aufruf aus dem Cache; keine `.part`-Reste | v0.0.3 | ✅ |
| I-VOR-02 | `png_umwandeln` | PNG mit Transparenz → JPEG, längste Kante = Größe | v0.0.3 | ✅ |
| I-VOR-03 | `pdf_seite` | PDFKit: Seite 1 und 2 als eigene Vorschauen, hochkant ≤ Größe; Seite 3 (gibt es nicht) → verständlicher Fehler | v0.0.3 | ✅ |
| I-VOR-04 | `kaputte_datei_meldet_fehler` | kaputte Datei → `Fehler` mit Meldung | v0.0.3 | ✅ |
| I-VOR-05 | `exif_drehung` | Hochkant-Foto (EXIF 6) erscheint hochkant: Pixel gedreht oder Drehangabe bleibt erhalten (WebKit dreht beim Anzeigen) | v0.0.3 | ✅ ohne Codeänderung grün |
| I-VOR-06 | `file_provider_lokale_datei` | Anbieter `file-provider` erzeugt lokale Bilder wie `lokal` | v0.0.3 | ✅ |
| I-VOR-07 | `exif_helfer` | Testhelfer liest EXIF-Orientation korrekt (6 bzw. 1) | v0.0.3 | ✅ |

### 3.7 Frontend

| ID | Ort | Prüft | Seit | Status |
|---|---|---|---|---|
| F-LOG-01 | `api.test.ts` | `datumDe`: ISO → TT.MM.JJJJ, leer, unplausible Werte roh | v0.0.3 | ✅ |
| F-LOG-02 | `api.test.ts` | `zahlDe`: Tausenderpunkt | v0.0.3 | ✅ |
| F-LOG-03 | `api.test.ts` | `api.*` ruft `start`, `db_setzen`, `suchen`, `dropbox_*` mit richtigen Argumenten; Fehler durchgereicht | v0.0.3 | ✅ |
| F-LOG-04 | `types.test.ts` | `DATEITYPEN` decken alle Katalog-Endungen ab, ohne Dopplung | v0.0.3 | ✅ |
| F-LOG-05 | `types.test.ts` | `istLeer` (App) – Gegenstück zu R-SUCH-03 | v0.0.3 | ✅ |
| F-LOG-06 | `vorschau.test.ts` | Warteschlange: höchstens 6 gleichzeitig | v0.0.3 | ✅ |
| F-LOG-07 | `vorschau.test.ts` | LIFO: jüngste Anfrage zuerst | v0.0.3 | ✅ |
| F-LOG-08 | `vorschau.test.ts` | abgebrochene Aufträge werden übersprungen | v0.0.3 | ✅ |
| F-LOG-09 | `vorschau.test.ts` | Antworten → Status, Datei → Asset-URL, Fehler abgefangen | v0.0.3 | ✅ |
| F-LOG-10 | `vorschau.test.ts` | `useVorschau`: lädt → Bild, zweites Mal aus dem Speicher | v0.0.3 | ✅ |
| F-LOG-11 | `vorschau.test.ts` | Nur online/Fehler nicht gemerkt; Unmount bricht ab | v0.0.3 | ✅ |
| F-LOG-12 … 18 | `useTastatur.test.ts` | ↑/↓ Zeile mit Begrenzung · erste Taste wählt Bild 1 · ←/→ nur im Raster · im Suchfeld ↓ verlässt Feld, ←/→ nicht · Bild↑/↓, Pos1/Ende · Modifier und 0 Treffer ohne Wirkung | v0.0.3 | ✅ |
| F-LOG-19 | `Ergebnisansicht.test.tsx` | `ablage()`: letzter Teil der Wurzel + Ordner | v0.0.3 | ✅ |
| F-LOG-20 | `vorschau.test.ts` | `nurOnlineNeuLaden`: Wolken-Kacheln fragen erneut, gemerkte Bilder nicht | v0.0.3 | ✅ |
| F-LOG-21 | `api.test.ts` | `groesseDe`: B/KB/MB/GB mit deutschem Komma | v0.0.3 | ✅ |
| F-LOG-22 | `vorschau.test.ts` | Größe 640; DB-Vorschau → `vorschau://`-Adresse, wird gemerkt | v0.0.3 | ✅ |
| F-LOG-23 | `vorschau.test.ts` | `alleNeuLaden`: auch gemerkte Bilder werden neu angefordert | v0.0.3 | ✅ |
| F-LOG-24 | `scripts/tests.test.ts` | Testsumme: cargo-Blöcke summiert, Vitest-Zeile gelesen (grün und rot), auch bei eingefärbter Ausgabe | v0.0.3 | ✅ |
| F-LOG-25 | `types.test.ts` | `istLeer` mit Facetten: gewählter Wert zählt, leere Auswahl nicht; `LEERER_FILTER.facetten` leer | v0.0.5 | ✅ |
| F-LOG-26 | `api.test.ts` | `api.details`, `vorschauGross`, `originalOeffnen`, `imFinderZeigen`, `dateinameVorschlag`, `sichernUnter`, `teilen` mit richtigen Befehlen/Argumenten | v0.0.6 | ✅ |
| F-LOG-27 | `detailfelder.test.ts` | Formate: Datum mit Uhrzeit, Änderungszeit in Ortszeit, Belichtung als Bruch, GPS mit Himmelsrichtung, Listen aus JSON/Text | v0.0.6 | ✅ |
| F-LOG-29 | `teiler.test.ts` | Detailbreite: Ergebnis-Minimum (eine Kachel in aktueller Größe, mind. Kopfzeile 250 px; Liste 420 px), Begrenzung nach unten/oben, schmales Fenster | v0.0.6 | ✅ |
| F-LOG-30 | `api.test.ts` | `api.dbNeuLaden` → `db_neu_laden` | v0.0.7 | ✅ |
| F-LOG-28 | `detailfelder.test.ts` | `aufbereiten`: Kopf, Gruppen Aufnahme/Bildinhalt/Datei/Text/Metadaten, Analyse und „Alle Metadaten“ zugeklappt, Chips (Begriffe vor Spalte), unbekannte Spalten unter „Weitere“, nichts doppelt, leere Gruppen entfallen | v0.0.6 | ✅ |
| F-UI-01 … 09 | `SearchPanel.test.tsx` | Fokus beim Start · Kriterien auf/zu · alle Felder · Dateityp-Gruppen · Zähler · × leert (Duplikate bleiben) · Esc · ⌘F · kompakt | v0.0.3 | ✅ |
| F-UI-10 … 13 | `DbMenu.test.tsx` | Anzeige Bilder/Duplikate/Stand/Pfad · „unbekannt“ · DB-Wechsel · Klick außerhalb schließt | v0.0.3 | ✅ |
| F-UI-14 … 17 | `Vorschaubild.test.tsx` | Bild mit Einblenden · Wolke · fehlt/Fehler · leer beim Laden | v0.0.3 | ✅ |
| F-UI-18 … 21 | `Ergebnisansicht.test.tsx` | Keine Treffer · Kopfzeile (Einzahl, „die ersten …“, ms) · Ansicht gemerkt · Kachelgröße gemerkt, kaputter Speicherwert | v0.0.3 | ✅ |
| F-UI-22 … 24 | `Ergebnisansicht.test.tsx` | Raster: Titel/Rückfall Dateiname, Datum, Seite · Klick und Markierung · Spaltenzahl aus Breite, Tastatur, Sichtbarhalten | v0.0.3 | ✅ |
| F-UI-25 … 26 | `Ergebnisansicht.test.tsx` | Liste: Zeileninhalt · Klick, ↓, Sichtbarhalten | v0.0.3 | ✅ |
| F-UI-40 | `DbMenu.test.tsx` | Zeile „Vorschauen in der DB“: Anzahl bzw. „keine“ | v0.0.3 | ✅ |
| F-UI-41 … 43 | `DbMenu.cache.test.tsx` | Cache-Größe · „Cache leeren“ (0 B, alle neu laden, Knopf aus) · Größe nicht abfragbar → Bereich weg | v0.0.3 | ✅ |
| F-UI-44 | `App.test.tsx` | Ereignis „cache-geleert“ (App-Menü) lädt alle Vorschauen neu; Abmelden beim Schließen | v0.0.3 | ✅ |
| F-UI-35 … 39 | `DbMenu.dropbox.test.tsx` | Dropbox-Bereich: ohne App-Schlüssel unsichtbar · Anmelden (Warten, danach Wolken neu laden) · Abmelden · Fehlermeldung · Status nicht abfragbar | v0.0.3 | ✅ |
| F-UI-27 … 34 | `App.test.tsx` | ohne DB → Auswahl → bereit · Auswahl abgebrochen · ungültige DB · verzögerte Suche · nur jüngste Antwort zählt · Suchfehler · Leeren → Startfläche · DB-Wechsel über Menü | v0.0.3 | ✅ |
| F-UI-45 … 49 | `Facetten.test.tsx` | kompakte Leiste: ein Knopf je Facette, Werte mit Anzahl (Tausenderpunkt) erst nach Klick, nur eine Facette offen · Klick wählt/entfernt, Knopf zeigt Anzahl gewählter Werte, andere Auswahl bleibt · gewählter Wert mit 0 sichtbar und gedämpft · lange Listen: erste 12 + „weitere“/„weniger“, gewählte immer sichtbar · Facetten ohne Werte ausgeblendet | v0.0.5 | ✅ |
| F-UI-54 | `Facetten.test.tsx` | gewählte Werte als entfernbare Chips in der Leiste; Esc und Klick außerhalb schließen den Aufklapper | v0.0.5 | ✅ |
| F-UI-50 | `styles.test.ts` | ganzes Bild: `object-fit: contain`, nirgends `cover`; Kachel bleibt 4:3; ruhiger Bildgrund (`--bildgrund`, hell und dunkel) in Kachel und Liste | v0.0.5 | ✅ |
| F-UI-51 … 52 | `App.test.tsx` | Facetten werden auch ohne Suche geladen, Auswahl startet Suche und neue Zählung · Facetten-Fehler stört die Suche nicht | v0.0.5 | ✅ |
| F-UI-53 | `SearchPanel.test.tsx` | Facetten im Kriterienbereich: Auswahl im Filter, Zähler zählt gewählte Werte, × leert auch die Facetten | v0.0.5 | ✅ |
| F-UI-55 … 58, 61 … 64 | `Detailbereich.test.tsx` | ohne Auswahl Hinweis · Auswahl: 640-px-Vorschau, Titel, Beschreibung, Gruppen, Schlagworte, Position, kein Nachladen · nur jüngste Antwort · Knopf „Großansicht“ und Doppelklick aufs Bild öffnen die Großansicht (F-UI-58; F-UI-59/60 „Groß laden“ im Detailbereich ❌ entfallen, ersetzt durch F-UI-71 … 77) · Öffnen/Finder/Teilen (Knopfposition), Fehler sichtbar · Sichern unter (Vorschlag, Abbruch) · Duplikat anzeigen und zurück · Ladefehler | v0.0.6 | ✅ |
| F-UI-65 … 67 | `App.test.tsx` | ohne Auswahl keine Detailfläche (und kein Schieber) · Klick und Pfeiltasten füllen den Detailbereich, Position „n / gesamt“ · Detailbereich ein/aus gemerkt · **Suchzeile unverändert**: Leertaste und ←/→ im Suchfeld bleiben Text, Suche läuft, Kriterien/Facetten mit Esc wie gehabt | v0.0.6 | ✅ |
| F-UI-68 | `styles.test.ts` | Detailbereich: ganzes Bild, Bildgrund, Breite rechts, eigener Bildlauf; Schieber mit `col-resize` | v0.0.6 | ✅ |
| F-UI-69 | `Teiler.test.tsx` | Schieber: Breite ab rechter Kante beim Ziehen, nichts ohne Drücken, Loslassen merkt, Doppelklick → Standard | v0.0.6 | ✅ |
| F-UI-71 … 75 | `Grossansicht.test.tsx` | Großansicht: sofort 640 px, 2048 px blendet ein, Titel/Position · Blättern lädt neu, nur jüngste Antwort · ohne große Fassung 640 px mit Hinweis · ←/↑ zurück, →/↓ weiter, Esc/Leertaste schließen, andere Tastenhörer (Raster, Suchfeld, Facetten) merken nichts · Hintergrund/× schließt, Kopfzeile nicht | v0.0.6 | ✅ |
| F-UI-76 … 77 | `App.test.tsx` | Leertaste öffnet die Großansicht (nicht im Suchfeld, nicht ohne Auswahl), Pfeile blättern bis zum Ende, Detailbereich folgt, Esc schließt ohne die Suche zu leeren · Knopf „Großansicht“ | v0.0.6 | ✅ |
| F-UI-78 | `App.test.tsx` | Ereignis „db-neu“: DbInfo neu, Suche und Details frisch geholt, Vorschauen neu angefordert, Auswahl bleibt, Meldung „Katalog aktualisiert · …“ | v0.0.7 | ✅ |
| F-UI-79 | `App.test.tsx` | Esc außerhalb des Suchfelds hebt die Auswahl auf (Detailbereich zu, Suche bleibt); bei offenem Facetten-Aufklapper schließt Esc zuerst diesen | v0.0.7 | ✅ |
| F-UI-80 | `App.test.tsx` | Programmname „Bilderwerk View“ in Titelleiste und DB-Auswahl | v0.0.7 | ✅ |
| F-UI-81 | `DbMenu.neu.test.tsx` | DB-Menü „Neu laden“ ruft `db_neu_laden`, Fehler sichtbar | v0.0.7 | ✅ |
| F-UI-82 … 83 | `Detailbereich.test.tsx` | Tastenkürzel ⌘O Öffnen, ⌥⌘R Im Finder, ⇧⌘S Sichern unter (ohne ⌘ nichts) · neuer DB-Stand holt die Angaben neu | v0.0.7 | ✅ |
| F-UI-70 | `Ergebnisansicht.test.tsx` | `Results` meldet seine Mindestbreite (Kachel bzw. Liste), auch nach Kachelgröße/Ansicht ändern | v0.0.6 | ✅ |

### 3.8 Vertrag Rust ↔ Frontend

| ID | Ort | Prüft | Seit | Status |
|---|---|---|---|---|
| V-01 | `vertrag.rs` | erzeugt `ergebnis.json`, `ergebnis-leer.json` aus echter Suche | v0.0.3 | ✅ |
| V-02 | `vertrag.rs` | erzeugt `dbinfo.json` | v0.0.3 | ✅ |
| V-03 | `vertrag.rs` | erzeugt `vorschau-*.json` (alle fünf Fälle) | v0.0.3 | ✅ |
| V-04 | `vertrag.rs` | Filter aus dem Frontend (camelCase) wird gelesen; fehlende Felder = Standard | v0.0.3 | ✅ |
| V-05 | `vertrag.test.ts` | Ergebnis/Treffer haben genau die Felder der TS-Typen | v0.0.3 | ✅ |
| V-06 | `vertrag.test.ts` | DbInfo hat genau die Felder des TS-Typs | v0.0.3 | ✅ |
| V-07 | `vertrag.test.ts` | echte Rust-Antwort wird angezeigt | v0.0.3 | ✅ |
| V-08 | `vertrag.test.ts` | alle Vorschau-Antworten werden verstanden | v0.0.3 | ✅ |
| V-09 | `vertrag.rs` | erzeugt `dropbox-status.json` | v0.0.3 | ✅ |
| V-10 | `vertrag.test.ts` | DropboxStatus hat genau die Felder des TS-Typs | v0.0.3 | ✅ |
| V-11 | `vertrag.rs` | erzeugt `cache-info.json`, `vorschau-datenbank.json`; DbInfo mit `vorschauen` | v0.0.3 | ✅ |
| V-12 | `vertrag.test.ts` | CacheInfo-Felder; DB-Vorschau-Antwort wird verstanden | v0.0.3 | ✅ |
| V-13 | `vertrag.rs` | erzeugt `facetten.json` (Ausschnitt Motiv/Merkmale mit Auswahl „Porträt“); V-04 liest `facetten` aus dem Filter | v0.0.5 | ✅ |
| V-14 | `vertrag.test.ts` | Facette/FacettenWert haben genau die Felder der TS-Typen; echte Antwort wird angezeigt | v0.0.5 | ✅ |
| V-15 | `vertrag.rs` | erzeugt `details.json` (Duplikat 3 mit Original und Schlagworten) | v0.0.6 | ✅ |
| V-16 | `vertrag.test.ts` | Details/DetailFeld/Verwandt haben genau die Felder der TS-Typen; echte Antwort lässt sich aufbereiten | v0.0.6 | ✅ |

### 3.9 Bewusst ohne automatischen Test

| Teil | Grund | Abdeckung stattdessen |
|---|---|---|
| Tauri-Befehle in `lib.rs` (`start`, `db_setzen`, `db_neu_laden`, Hintergrund-Thread `beobachten`, Menü „Katalog“, `suchen`, `facetten`, `vorschau`, `details`, `vorschau_gross`, `original_oeffnen`, `im_finder_zeigen`, `dateiname_vorschlag`, `sichern_unter`, `teilen`, `dropbox_*`, `cache_info`, `cache_leeren`), Protokoll `vorschau://`, App-Menü, Cache-Begrenzung beim Start | brauchen ein laufendes Tauri | nur noch Durchreichen; Logik in `ablauf.rs`, `search.rs`, `facetten.rs`, `details.rs`, `gross.rs`, `aktionen.rs`, `speicher/`, `thumbs.rs` getestet (I-ABL, I-SUCH, I-DET, R-GROSS, R-AKT, R-ANB, R-ZUO, I-VOR) |
| `build.rs` – `anbieter` aus view.local.json | Build-Skript | Regeln werden beim Start geprüft (R-ZUO-02/09); Fehler → Meldung im Terminal, alles `lokal` |
| `main.rs`, `main.tsx`, `build.rs` | Programmstart bzw. Build | manueller Start je Version |
| `pdf.rs` (PDFKit) | nur macOS | I-VOR-03 auf dem Mac; unter Linux nur Typprüfung für macOS |
| `token::Schluesselbund` (macOS-Schlüsselbund) | würde Kims echten Schlüsselbund verändern | Tests nutzen `ImSpeicher`; manuelle Prüfung Anmelden/Abmelden |
| Echte Dropbox (Anmeldung im Browser, echte Vorschauen) | echtes Konto | manuelle Prüfung v0.0.3; Zeiten messen |
| `App.dbWaehlen` – macOS-Dateidialog | Systemdialog | Dialog gemockt (F-UI-27 … 29, 34) |
| `teilen.rs` – NSSharingServicePicker, Öffnen/Finder per `open`, Sichern-Dialog | Systemfenster bzw. Finder | Umrechnung (R-TEIL-01), Befehle (R-AKT), Dialog gemockt (F-UI-62); Wirkung manuell (Abschnitt 5, v0.0.6) |
| große Vorschau über sips/PDFKit (`gross::holen` mit echtem Bild) | nur macOS | Wiederverwendung und Grenzen (R-GROSS); Umwandlung wie I-VOR; manuell |
| Darstellung (CSS, Animationen, echtes Scrollen) | jsdom rendert nicht | Kernregeln als Text geprüft (F-UI-50); Wirkung manuell (Abschnitt 5) |
| HEIC über die echte Dropbox-API | echtes Konto; ob Dropbox HEIC umwandelt, ist nicht dokumentiert | R-DBX-18/21 mit nachgebildetem Server; manuelle Prüfung v0.0.5 |

## 4. Bisherige Prüfungen außerhalb des Repos

| Datum | Version | Prüfung | Ergebnis |
|---|---|---|---|
| 03.11.2026 | v0.0.1 | Suchkern gegen echte Katalog-DB (23.742 Bilder): 9 Suchfälle inkl. Sonderzeichen, Phrase, Ausschluss, Datum, Dateityp, Ordner | alle fehlerfrei, 10–70 ms; Befund: unplausible Datumswerte → Sortierung angepasst |

## 5. Manuelle Prüfungen je Version

Was automatisch nicht prüfbar ist (Fenster, Finder, Teilen, Gesamteindruck).

### v0.0.1

| # | Prüfung | Ergebnis |
|---|---|---|
| 1 | App startet, DB wird automatisch geöffnet | ✅ |
| 2 | Leere Startfläche, Suchfeld gleitet bei Treffern nach oben | ✅ |
| 3 | Freitext + Kriterien kombiniert, Trefferzahl plausibel | ✅ |
| 4 | ↑/↓ verschiebt Auswahl, Bild↑/↓ blättert | ✅ (nach Korrektur) |

### v0.0.2

| # | Prüfung | Ergebnis |
|---|---|---|
| 1 | Vorschauen erscheinen beim Scrollen, weiches Einblenden | ✅ |
| 2 | Hochkant-Fotos richtig gedreht | ⏳ |
| 3 | PDF-Seiten zeigen Vorschau | ⏳ |
| 4 | Nur-online-Datei zeigt Wolke (ohne Klick-Funktion) | ✅ |
| 5 | Raster/Liste umschalten, Kachelgröße, Einstellung bleibt nach Neustart | ✅ |
| 6 | Tastatur im Raster: ↑/↓ Zeile, ←/→ Bild, Bild↑/↓ | ⏳ |
| 7 | ×-Knopf im Suchfeld leert alle Kriterien → leere Startfläche | ✅ |

**Befund v0.0.2 (erster Test):** Nur-online-Dateien wurden nicht erkannt (macOS meldet für Dropbox-Platzhalter belegte Blöcke) → sips „not a valid file“. Korrektur: Flag `SF_DATALESS` + Rückfall auf sips-Meldung (R-VOR-04/05).

**Befund v0.0.2 (zweiter Test):** Klassische Dropbox legt Platzhalter als **0-Byte-Datei** mit Attribut `com.dropbox.placeholder` an; Lesen lädt nichts nach, nur *Öffnen* (LaunchServices → Dropbox-App). Korrektur: 0 Byte = nur online; Download per `open -g` + Warten auf vollständige Datei.

**Befund v0.0.2 (dritter Test):** Dropbox öffnet die Datei nach dem Download zusätzlich in der Vorschau. Korrektur: genau dieses Dokument per AppleScript schließen, Vorschau beenden, falls sie vorher nicht lief; Fokus zurück zu View (R-VOR-06).

**Entscheidung v0.0.2 (Abschluss):** Vorschau blitzt trotzdem 1–2 s auf, Serienbetrieb nicht möglich → Klick-Download wieder entfernt. Nur-online-Dateien zeigen bis v0.0.3 nur die Wolke. Lösung in v0.0.3: Anbieter-Schnittstelle mit Dropbox-API (Variante A).

### v0.0.3

Geprüft am 04.11.2026 auf dem MacBook Air M3 (Kim).

| # | Prüfung | Ergebnis |
|---|---|---|
| 1 | `npm test` grün (Rust 110 inkl. 7 macOS-Tests, Frontend 81, tsc) | ✅ |
| 2 | Hochkant-Fotos richtig gedreht | ✅ (I-VOR-05) |
| 3 | Dropbox: Anmelden im Browser, Vorschauen für „nur online“, Neustart angemeldet, Ab-/Anmelden | ✅ |
| 4 | PDF-Seiten (Dropbox und lokal) mit PDFKit, ohne poppler | ✅ |
| 5 | Schärfe der 640-px-Vorschauen (Hochkant aus Dropbox) | ⏸ nicht beurteilbar – größte Anzeige ist die Kachel; Prüfung mit Detailansicht (Stufe 4) |
| 6 | DB-Menü: Vorschauen in der DB („keine“), Cache-Größe, „Cache leeren“ → Vorschauen kommen neu | ✅ |
| 7 | App-Menü „Vorschau → Vorschau-Cache leeren“ | ✅ |
| 8 | Suche, lokale Bilder, Raster/Liste unverändert | ✅ |

**Befund v0.0.3:** Kacheln (4:3) und Listen-Vorschau schneiden das Bild zu (`object-fit: cover`) – Hoch- und Querformat
sind nicht zu unterscheiden. → v0.0.4: ganzes Bild zeigen (`contain`, ruhiger Hintergrund), Kachel bleibt 4:3.

**Messung v0.0.3 (MacBook Air M3, Dropbox-Protokoll):** Dropbox-Vorschau 0,5–1,4 s (meist ~1 s, 30–90 KB) ·
HEIC per Download 1,5–2,5 s (2–8 MB) + 0,2 s Umwandeln · PDF 190 MB einmal 10,8 s, danach je Seite ~65 ms ·
keine Drosselung (kein 429). Bei 6 parallelen Anfragen 3–6 Bilder/s. Konsequenz: Vorschauen künftig im Katalog
(Tabelle `vorschauen`), View nutzt sie vorrangig.

### v0.0.5

Geprüft am 04.11.2026 auf dem MacBook Air M3 (Kim).

| # | Prüfung | Ergebnis |
|---|---|---|
| 1 | `npm test` grün (Rust 125 inkl. 7 macOS-Tests, Frontend 94, tsc) | ✅ 219 Tests grün |
| 2 | Kachel und Liste zeigen das ganze Bild; Hoch-/Querformat erkennbar; Ränder ruhig (hell und dunkel) | ✅ |
| 3 | HEIC „nur online“: Vorschau kommt über die Dropbox-API (schneller als 1,5–2,5 s) – oder sauber per Download, falls Dropbox ablehnt | ✅ Dropbox liefert HEIC-Vorschauen (keine Ablehnung); Zeiten siehe Messung |
| 4 | Kriterien → Eigenschaften: Leiste mit 8 Knöpfen, Werte klappen kompakt auf; Ergebnis bleibt mit mehreren Bildreihen sichtbar; ohne Suchtext stöbern | ✅ Darstellung passt (Kim, 04.11.2026) |
| 4a | HEIC: Terminal von `npm run tauri dev` zeigt „HEIC-Vorschau über Dropbox-API in … ms“ bzw. „lehnt HEIC-Vorschau ab“ | ✅ |
| 5 | Facette wählen → Treffer und Anzahlen der übrigen Facetten passen; zweiter Wert derselben Facette erweitert (ODER) | ⏳ |
| 6 | Schlagworte/Merkmale: „weitere“ klappt auf; gewählter Wert bleibt sichtbar | ⏳ |
| 7 | Antwortzeit der Facetten mit echter DB (ohne Filter und mit Freitext) – Wert notieren | ⏸ nicht gesondert gemessen; Arbeiten insgesamt akzeptabel |
| 8 | × / Esc leert auch die Facetten; Zähler am Knopf „Kriterien“ stimmt | ⏳ |

**Befund v0.0.5 (erste Prüfung, Design):** Erste Fassung zeigte alle Facetten-Gruppen offen untereinander – nur noch eine
Bildreihe sichtbar. Korrektur: einzeilige Leiste mit Aufklappern (F-UI-45 … 49, 54).

**Messung v0.0.5 (HEIC über Dropbox-API, 32 Bilder aus `/Kamera-Uploads`, 6 parallel):** alle über die API, keine
Ablehnung, kein Download. 0,34–4,0 s je Anfrage, Median ~2,2 s; rund ein Viertel unter 1 s (vermutlich von Dropbox
bereits erzeugt). Je Bild nicht schneller als der Download (1,5–2,5 s), aber nur ~50 KB statt 2–8 MB und keine
Umwandlung auf dem Mac. Dauerhafte Lösung bleiben die Vorschauen in der Katalog-DB.

**Fazit v0.0.5 (Kim):** Antwortzeiten mit Dropbox-Vorschauen akzeptabel; flüssiges Arbeiten erst, wenn die Vorschauen
in der Katalog-DB stehen (Work-Zweig).

### v0.0.6

Geprüft am 05.11.2026 auf dem Mac (Kim): Gesamtfunktion ✅ (Detailbereich, Schieber, Großansicht). Einzelpunkte unten, soweit nicht gesondert bestätigt, ⏳.

| # | Prüfung | Ergebnis |
|---|---|---|
| 1 | `npm test` grün (Rust 142 inkl. 7 macOS-Tests, Frontend 134, tsc) | ⏳ |
| 2 | Suchzeile, Kriterien und Leiste „Eigenschaften“ bedienen sich wie in v0.0.5 (Tippen inkl. Leertaste, ⌘F, Esc, ×) | ⏳ |
| 3 | Einfach-Klick zeigt rechts die 640-px-Vorschau sofort (aus DB/Cache) und alle Angaben; ←/→/↑/↓ blättern flüssig | ⏳ |
| 4 | Angaben vollständig und lesbar: Aufnahme, Bildinhalt, Schlagworte/Merkmale, Datei, Sichtbarer Text, Analyse/Alle Metadaten zugeklappt, „Weitere“ | ⏳ |
| 5 | Schärfe der 640-px-Vorschau im Detailbereich ausreichend (Retina), auch bei breit gezogener Detailsicht | ⏳ |
| 6 | Großansicht (Leertaste, Doppelklick, Knopf): ganzes Fenster, 640 px sofort, 2048 px blendet ein – lokal, Dropbox „nur online“ (Download), PDF-Seite, HEIC; Zeit notieren · ←/→ blättern · Esc/Leertaste/Klick schließen, Suche bleibt | ⏳ |
| 7 | Öffnen (Standard-App), Im Finder (Datei markiert), Sichern unter … (Dateiname vorgeschlagen), Teilen (Systemmenü am Knopf) – lokal und „nur online“ | ⏳ |
| 8 | Duplikat-Link zeigt das andere Exemplar, „zurück zur Auswahl“ | ⏳ |
| 9 | Detailbereich ausblenden/einblenden (Knopf rechts neben Raster/Liste), Einstellung bleibt nach Neustart; Fensterbreite 900 px noch bedienbar | ⏳ |
| 11 | Ohne Auswahl: keine Detailfläche, Ergebnis über die ganze Breite (Startfläche unverändert „clean“) | ⏳ |
| 12 | Schieber: Ziehen ändert Breite und Bildgröße; Grenze = eine Kachel in aktueller Größe bzw. Mindestbreite der Liste; Breite bleibt nach Neustart; Doppelklick → Standard; Fenster verkleinern hält die Grenze ein | ⏳ |
| 10 | hell und dunkel | ⏳ |

### v0.0.7

Offen – Prüfung durch Kim auf dem Mac.

| # | Prüfung | Ergebnis |
|---|---|---|
| 1 | `npm test` grün (Rust 148 inkl. 7 macOS-Tests, Frontend 141, tsc) | ⏳ |
| 2 | Titelleiste zeigt „Bilderwerk View“ (hell/dunkel), Fenster weiter an der Leiste verschiebbar | ⏳ |
| 3 | DB-Sicherung ersetzen lassen (Work-Lauf → Dropbox): nach wenigen Sekunden Meldung „Katalog aktualisiert“, Vorschauen-Zahl im DB-Menü steigt, Suche/Auswahl bleiben, Terminal „View: Katalog neu geladen …“ | ⏳ |
| 4 | Menü „Katalog → Datenbank neu laden“ (⌘R) und DB-Menü „Neu laden“ | ⏳ |
| 5 | ⌘O, ⌥⌘R, ⇧⌘S bei ausgewähltem Bild; Esc hebt Auswahl auf (Detailbereich zu), Esc im Suchfeld leert wie bisher | ⏳ |
| 6 | Bau: `npm run bauen` → `.app` und `.dmg` entstehen; `.app` startet aus `/Programme`, findet die DB, Dropbox-Anmeldung bleibt | ⏳ |
| 7 | Installation auf dem Mac mini M1 aus dem `.dmg`: Start, DB, Vorschauen | ⏳ |

