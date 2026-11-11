# Repo-Umstrukturierung und feste Code-Signatur

Stand: 04.11.2026, nach Version 0.0.3

## 1. Neuer Repo-Aufbau

Das Repo enthält jetzt zwei eigenständige Programme, die sich die Bilddatenbank teilen:

| Ordner | Inhalt |
|---|---|
| `apps/view/` | Bilderwerk View – Tauri + React (inkl. `apps/view/docs/`) |
| `apps/work/` | Analyseprogramm (Python) – erzeugt und aktualisiert die Bilddatenbank, eigene Doku unter `apps/work/docs/` |
| `shared/` | Gemeinsamer Vertrag beider Programme (DB-Schema, `schema.sql` folgt) |

- Alle App-Dateien wurden mit Umbenennungen (git rename) nach `apps/view/` verschoben – die Historie bleibt erhalten (`git log --follow`).
- Relative Pfade (`../view.local.json`, `../dist`, `tests/vertrag/`) sind unverändert gültig, weil sie mitgewandert sind.
- `view.local.json` liegt jetzt unter `apps/view/`.
- npm-Befehle laufen im Ordner `apps/view/` (`cd app && npm run tauri dev`).
- `.gitignore`: Pfade auf `apps/view/…` angepasst, Python-Einträge ergänzt (`__pycache__/`, `.venv/` …).

### Icons
Die Regel `Icon?` (macOS-Vorlage) hat wegen des Platzhalters `?` und der Groß-/Kleinschreibungs-Toleranz
auf macOS auch den Ordner `src-tauri/icons/` ausgeschlossen – die Icons waren nie im Repo. Die Regel wurde
entfernt, die Icons liegen jetzt versioniert unter `apps/view/src-tauri/icons/`.

## 2. Feste Code-Signatur (Schlüsselbund-Abfrage)

**Problem:** Bei jedem Start fragte macOS nach dem Schlüsselbund-Passwort (Dropbox-Token über `keyring`),
trotz „Immer erlauben“. Ursache: Dev-Builds sind nur ad hoc signiert, die Signatur ändert sich mit jedem
Build – macOS sieht jedes Mal eine neue App.

**Lösung:** Signieren mit einem festen „Apple Development“-Zertifikat (in Xcode angelegt, liegt im
Schlüsselbund; Name ermitteln mit `security find-identity -v -p codesigning`).

| Datei | Zweck |
|---|---|
| `apps/view/src-tauri/tauri.conf.json` → `bundle.macOS.signingIdentity` | Signatur für `npm run tauri build` |
| `apps/view/src-tauri/.cargo/config.toml` → `[env] PSV_SIGN_IDENTITY` | Zertifikat für Dev-Läufe |
| `apps/view/src-tauri/scripts/dev-signieren.sh` | Cargo-Runner: signiert vor jedem Start (`tauri dev`, `cargo run`, `cargo test`) und startet dann |

- An beiden Stellen steht derselbe Zertifikatsname. Er ist kein Geheimnis (der private Schlüssel bleibt im Schlüsselbund).
- Ohne gültiges Zertifikat startet das Skript unsigniert weiter; `tauri build` würde dagegen abbrechen.
- Für spätere CI (GitHub Actions): Eintrag aus `tauri.conf.json` entfernen und `APPLE_SIGNING_IDENTITY` setzen.
- Nach Zertifikatswechsel einmal erneut „Immer erlauben“ wählen.
