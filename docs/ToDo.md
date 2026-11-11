# Bilderwerk – ToDo

Offene Punkte für die Reifephase. Erledigtes wird gestrichen bzw. in die Versionsdoku übernommen.

## Übergreifend

- [ ] **Verteilung / Anleitung KI-Modell:** Dokumentieren, welche lokalen KI-Werkzeuge und Modelle funktionieren
  (Voraussetzung: OpenAI-kompatible Schnittstelle `/v1/chat/completions` mit Bildeingabe – z. B. LM Studio, Ollama,
  llama.cpp-Server). Hinweise zu Modellwahl (Vision-fähig, JSON-Ausgabe), Hardware/RAM, Geschwindigkeit,
  Qualität; Referenz: LM Studio + `qwen/qwen3.5-9b`. Ggf. Prompt-Robustheit für andere Modelle prüfen.

## Work

- [x] ~~**KI-Endpunkt einstellbar**~~ (v0.2: Dialog „KI-Server“, Verbindungstest, Kompatibilitätsstufen)
- [x] ~~**RAW-Formate**~~ (v0.2: über `sips`) – auf dem Mac mit echten RAW-Dateien prüfen (Drehung, Vorschau)
- [ ] **Kategorien/Prompt auslagern:** geprüft, zurückgestellt. View liest die Filterwerte dynamisch
  aus der DB (nur die Feldnamen sind fest) – technisch möglich, aber wechselnde Wertelisten in einer DB
  zersplittern die Filter. Wenn, dann als Konfigurationsdatei mit eigener `PROMPT_VERSION`.

## View

- [ ] **About-Fenster ausbauen:** eigenes Fenster mit klickbarer Webseite (bilderwerk.sieber-bw.de),
  E-Mail (Bilderwerk@sieber-bw.de), Lizenzhinweis (PolyForm Noncommercial), Version.

## Sweep

- [x] ~~**Datenbank wählbar**~~ (v0.2: „Datenbank …“, gespeichert in den gemeinsamen Einstellungen)
- [x] ~~Kopplung an Work~~ (v0.2: gemeinsamer Code in `shared/bilderwerk`)
