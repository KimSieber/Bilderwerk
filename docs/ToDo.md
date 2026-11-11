# Bilderwerk – ToDo

Offene Punkte für die Reifephase. Erledigtes wird gestrichen bzw. in die Versionsdoku übernommen.

## Übergreifend

- [ ] **Verteilung / Anleitung KI-Modell:** Dokumentieren, welche lokalen KI-Werkzeuge und Modelle funktionieren
  (Voraussetzung: OpenAI-kompatible Schnittstelle `/v1/chat/completions` mit Bildeingabe – z. B. LM Studio, Ollama,
  llama.cpp-Server). Hinweise zu Modellwahl (Vision-fähig, JSON-Ausgabe), Hardware/RAM, Geschwindigkeit,
  Qualität; Referenz: LM Studio + `qwen/qwen3.5-9b`. Ggf. Prompt-Robustheit für andere Modelle prüfen.

## Work

- [ ] **KI-Endpunkt einstellbar:** URL (`API_URL`, derzeit fest `http://127.0.0.1:1234/v1/chat/completions`)
  und Modellname (`MODEL`, derzeit fest `qwen/qwen3.5-9b`) gesamthaft in den Einstellungen der Oberfläche
  pflegbar machen (`bildanalyse.py`), inkl. Verbindungstest.
- [ ] **RAW-Formate:** CR2/CR3, NEF, ARW, DNG, ORF, RAF usw. unterstützen (derzeit nicht in `ENDUNGEN`).
  Umwandlung prüfen (macOS `sips` / Core Image kann viele RAW-Formate).

## View

- [ ] **About-Fenster ausbauen:** eigenes Fenster mit klickbarer Webseite (bilderwerk.sieber-bw.de),
  E-Mail (Bilderwerk@sieber-bw.de), Lizenzhinweis (PolyForm Noncommercial), Version.

## Sweep

- [ ] **Datenbank wählbar:** Auswahl der DB in der Oberfläche (derzeit nur über Work-Einstellungen
  `gui_einstellungen.json`, Standardort `apps/work/Katalog-DB/` oder Startparameter `sweep.py <pfad>`).
