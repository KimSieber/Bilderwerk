# Bilderwerk – ToDo

Offene Punkte für die Reifephase. Erledigtes wird gestrichen bzw. in die Versionsdoku übernommen.

## Übergreifend

- [ ] **Verteilung / Anleitung KI-Modell:** Dokumentieren, welche lokalen KI-Werkzeuge und Modelle funktionieren
  (Voraussetzung: OpenAI-kompatible Schnittstelle `/v1/chat/completions` mit Bildeingabe – z. B. LM Studio, Ollama,
  llama.cpp-Server). Hinweise zu Modellwahl (Vision-fähig, JSON-Ausgabe), Hardware/RAM, Geschwindigkeit,
  Qualität; Referenz: LM Studio + `qwen/qwen3.5-9b`. Ggf. Prompt-Robustheit für andere Modelle prüfen.

## Work

- [x] ~~**KI-Endpunkt einstellbar**~~ (v0.2: Dialog „KI-Server“, Verbindungstest, Kompatibilitätsstufen)
- [x] ~~**RAW-Formate**~~ (v0.2: über `sips`; v0.3.2 mit echten CR2 von zwei Canon-Kameras geprüft, View-Filter „RAW“)
- [ ] **RAW „nur online“ in View (Dropbox klassisch):** Dropbox erzeugt für RAW keine Vorschau → View lädt die
  ganze Datei (oft 20–60 MB). Unkritisch, solange die Vorschau aus der DB kommt (Work legt sie an); sonst ggf.
  eingebettetes JPEG per HTTP-Range laden.
- [ ] **Kategorien/Prompt auslagern:** geprüft, zurückgestellt. View liest die Filterwerte dynamisch
  aus der DB (nur die Feldnamen sind fest) – technisch möglich, aber wechselnde Wertelisten in einer DB
  zersplittern die Filter. Wenn, dann als Konfigurationsdatei mit eigener `PROMPT_VERSION`.

## View

- [x] ~~**Werte zur Laufzeit**~~ (v0.3: Einstellungsdialog statt `view.local.json`, Speicherorte, Dropbox-App-Schlüssel)
- [x] ~~**About-Fenster ausbauen**~~ (v0.3: eigenes Fenster mit Webseite, E-Mail, Lizenz, Version)
- [ ] **Verteilung:** Notarisierung; Dropbox-App „Produktion beantragen“, sobald sich ~50 Konten verbinden
  (vorher in der App-Console „Enable additional users“ für andere Konten)

## Sweep

- [x] ~~**Datenbank wählbar**~~ (v0.2: „Datenbank …“, gespeichert in den gemeinsamen Einstellungen)
- [x] ~~Kopplung an Work~~ (v0.2: gemeinsamer Code in `shared/bilderwerk`)
