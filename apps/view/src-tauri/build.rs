//! Liest lokale, nicht versionierte Werte aus ../view.local.json und gibt sie
//! als Umgebungsvariablen an den Compiler weiter (option_env! im Code).
//! Fehlt die Datei, baut die App trotzdem – ohne Vorschlag für die Datenbank, alle Pfade `lokal`.

fn main() {
    let datei = std::path::Path::new("../view.local.json");
    println!("cargo:rerun-if-changed=../view.local.json");
    if let Ok(text) = std::fs::read_to_string(datei) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(db) = json.get("standardDb").and_then(|v| v.as_str()) {
                println!("cargo:rustc-env=PSV_STANDARD_DB={db}");
            }
            // Speicheranbieter je Pfad (geprüft wird beim Programmstart)
            if let Some(k) = json.get("dropboxAppKey").and_then(|v| v.as_str()) {
                println!("cargo:rustc-env=PSV_DROPBOX_APP_KEY={k}");
            }
            if let Some(a) = json.get("anbieter") {
                println!("cargo:rustc-env=PSV_ANBIETER={a}");
            }
        }
    }
    tauri_build::build()
}
