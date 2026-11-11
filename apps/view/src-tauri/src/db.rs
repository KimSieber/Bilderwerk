//! Öffnen der Katalog-Datenbank – strikt nur lesend.

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::path::Path;

/// Prozentkodiert einen Pfad für eine SQLite-URI (Leerzeichen, #, ? …).
fn uri_kodieren(pfad: &str) -> String {
    let mut s = String::new();
    for b in pfad.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                s.push(b as char)
            }
            _ => s.push_str(&format!("%{:02X}", b)),
        }
    }
    s
}

/// Öffnet die DB nur lesend. `immutable=1`: keine Sperren, keine -shm/-wal-Dateien –
/// passend für die Sicherungskopie in Dropbox, die nur von außen ersetzt wird.
pub fn oeffnen(pfad: &str) -> Result<Connection, String> {
    if !Path::new(pfad).is_file() {
        return Err(format!("Datei nicht gefunden: {pfad}"));
    }
    let uri = format!("file:{}?mode=ro&immutable=1", uri_kodieren(pfad));
    let db = Connection::open_with_flags(
        &uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("Datenbank lässt sich nicht öffnen: {e}"))?;

    // Plausibilität: ist das ein Bildkatalog?
    let ok: i64 = db
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name IN ('bilder','suche','begriffe')",
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("Keine gültige SQLite-Datenbank: {e}"))?;
    if ok < 3 {
        return Err("Die Datei ist keine Bildkatalog-Datenbank (Tabellen fehlen).".into());
    }
    Ok(db)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbInfo {
    pub pfad: String,
    pub schema_version: i64,
    pub bilder: i64,
    pub duplikate: i64,
    /// Änderungszeit der Datei (Unix-Sekunden) – zeigt das Alter der Sicherung
    pub geaendert: Option<u64>,
    pub groesse: u64,
    /// Anzahl der Vorschauen in der DB (Tabelle `vorschauen`); None = ältere DB ohne Vorschauen
    pub vorschauen: Option<i64>,
}

pub fn info(db: &Connection, pfad: &str) -> Result<DbInfo, String> {
    let e = |e: rusqlite::Error| e.to_string();
    let schema_version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(e)?;
    let (bilder, duplikate): (i64, i64) = db
        .query_row(
            "SELECT sum(status='ok'), sum(status='duplikat') FROM bilder",
            [],
            |r| Ok((r.get::<_, Option<i64>>(0)?.unwrap_or(0), r.get::<_, Option<i64>>(1)?.unwrap_or(0))),
        )
        .map_err(e)?;
    let meta = std::fs::metadata(pfad).ok();
    let geaendert = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    Ok(DbInfo {
        pfad: pfad.to_string(),
        schema_version,
        bilder,
        duplikate,
        geaendert,
        groesse: meta.map(|m| m.len()).unwrap_or(0),
        vorschauen: crate::vorschau_db::anzahl(db),
    })
}

#[cfg(test)]
mod tests {
    /// R-DB-01
    #[test]
    fn uri() {
        assert_eq!(
            super::uri_kodieren("/a b/2026-10 #x?.db"),
            "/a%20b/2026-10%20%23x%3F.db"
        );
    }

    /// R-DB-02
    #[test]
    fn uri_umlaute_und_prozent() {
        // UTF-8-Bytes einzeln kodiert, % selbst ebenfalls
        assert_eq!(super::uri_kodieren("/Bilder/Ä 100%.db"), "/Bilder/%C3%84%20100%25.db");
        assert_eq!(super::uri_kodieren("/a-b_c.d~e"), "/a-b_c.d~e");
    }

    /// R-DB-03
    #[test]
    fn fehlende_datei() {
        let e = super::oeffnen("/gibt/es/nicht.db").unwrap_err();
        assert!(e.contains("nicht gefunden"), "{e}");
    }
}
