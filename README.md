# Bilderwerk

Die eigene Bildersammlung per lokaler KI katalogisieren, durchsuchen und aufräumen.
Alle Daten bleiben auf dem eigenen Rechner.

| Komponente | Aufgabe | Technik | Ordner |
|---|---|---|---|
| **Bilderwerk Work** | Bilder mit einem lokalen KI-Modell (z. B. LM Studio) analysieren und in der Katalog-DB erfassen | Python / tkinter | [`apps/work/`](apps/work/) |
| **Bilderwerk View** | Katalog durchsuchen (Freitext, Facetten) und Bilder anzeigen | Tauri 2 · React · Rust | [`apps/view/`](apps/view/) |
| **Bilderwerk Sweep** | Dubletten sichten und bereinigen | Python / tkinter | [`apps/sweep/`](apps/sweep/) |

Gemeinsame Grundlage ist die SQLite-Datenbank `Bildkatalog.db`.

**Einstieg:** [Anleitung](docs/Anleitung.md) – was die Programme tun, was man braucht, wie man eine lokale KI anbindet.

## Aufbau

```
Bilderwerk/
  apps/work/    Bildanalyse, erzeugt Bildkatalog.db
  apps/view/    Suche und Anzeige (liest die DB nur)
  apps/sweep/   Dublettenbereinigung
  shared/       gemeinsamer Python-Code von Work und Sweep (Einstellungen, DB-Zugriff, Vorschau)
  docs/         übergreifende Dokumentation
```

Einrichtung, Start und Bau stehen im README der jeweiligen Komponente.

## Lizenz

Bilderwerk steht unter der **[PolyForm Noncommercial License 1.0.0](LICENSE)**.
Nutzung, Änderung und Weitergabe sind für nicht-kommerzielle Zwecke erlaubt
(privat, Forschung, gemeinnützige Organisationen, Bildung). Kommerzielle Nutzung ist nicht gestattet.

**Ausgenommen** sind die Icons und App-Symbole (u. a. `apps/view/src-tauri/icons/`,
`apps/view/src-tauri/app-icon.png`, `apps/view/src-tauri/app-icon.svg`).
Sie sind urheberrechtlich geschützt (© Kim Sieber, alle Rechte vorbehalten) und dürfen
nicht in abgewandelten oder weitergegebenen Fassungen verwendet werden.

© 2026 Kim Sieber
