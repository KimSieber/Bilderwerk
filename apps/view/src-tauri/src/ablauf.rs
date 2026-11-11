//! Abläufe hinter den Tauri-Befehlen – ohne Tauri-Abhängigkeit, damit testbar.
//! lib.rs reicht nur noch Ordner und Zustand durch.

use crate::{db, settings};
use rusqlite::Connection;
use std::path::Path;

/// Programmstart: DB aus den Einstellungen (oder dem Vorschlag) öffnen.
pub fn start(config_dir: &Path, vorschlag: Option<&str>) -> Result<(String, Connection, db::DbInfo), String> {
    let e = settings::laden_mit(config_dir, vorschlag);
    let pfad = e.db_pfad.ok_or("Noch keine Datenbank ausgewählt.")?;
    oeffnen(&pfad)
}

/// Neue DB wählen: öffnen, prüfen, nur im Erfolgsfall als Einstellung speichern.
pub fn db_setzen(config_dir: &Path, pfad: &str) -> Result<(String, Connection, db::DbInfo), String> {
    let r = oeffnen(pfad)?;
    let mut e = settings::laden_mit(config_dir, None);
    e.db_pfad = Some(pfad.to_string());
    settings::speichern(config_dir, &e)?;
    Ok(r)
}

fn oeffnen(pfad: &str) -> Result<(String, Connection, db::DbInfo), String> {
    let conn = db::oeffnen(pfad)?;
    let info = db::info(&conn, pfad)?;
    Ok((pfad.to_string(), conn, info))
}

/// Dieselbe DB neu öffnen (Dropbox hat die Sicherung ersetzt, oder „Neu laden“ im Menü).
pub fn neu_laden(pfad: &str) -> Result<(String, Connection, db::DbInfo), String> {
    oeffnen(pfad)
}
