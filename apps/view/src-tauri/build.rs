//! Tauri-Build. Seit v0.3 werden keine rechnerabhängigen Werte mehr eingebaut (bis v0.2: view.local.json) –
//! alles steht in den Einstellungen der App.

fn main() {
    tauri_build::build()
}
