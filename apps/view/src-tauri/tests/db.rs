//! I-DB: Öffnen und Prüfen der Katalog-DB gegen eine echte SQLite-Datei.
mod gemeinsam;
use gemeinsam::*;
use bilderwerk_view_lib::db;

/// I-DB-01
#[test]
fn katalog_oeffnen_und_info() {
    let d = tmp("db-info");
    let p = test_db(&d, "Katalog.db");
    let pfad = p.to_str().unwrap();
    let conn = db::oeffnen(pfad).unwrap();
    let i = db::info(&conn, pfad).unwrap();
    assert_eq!(i.pfad, pfad);
    assert_eq!(i.schema_version, 3);
    assert_eq!(i.bilder, 28, "status ok"); // 30 − Duplikat − Fehler
    assert_eq!(i.duplikate, 1);
    assert!(i.groesse > 0);
    assert!(i.geaendert.unwrap() > 1_700_000_000);
}

/// I-DB-02
#[test]
fn pfad_mit_sonderzeichen() {
    let d = tmp("db-sonder").join("Ordner mit #Raute? & Ä");
    std::fs::create_dir_all(&d).unwrap();
    let p = test_db(&d, "Sicherung 2026-11 #1.db");
    assert!(db::oeffnen(p.to_str().unwrap()).is_ok());
}

/// I-DB-03
#[test]
fn nur_lesend() {
    let d = tmp("db-ro");
    let p = test_db(&d, "k.db");
    let conn = db::oeffnen(p.to_str().unwrap()).unwrap();
    assert!(conn.execute("DELETE FROM bilder", []).is_err(), "Schreiben muss scheitern");
    // keine Nebendateien (immutable)
    assert!(!d.join("k.db-wal").exists() && !d.join("k.db-shm").exists());
}

/// I-DB-04
#[test]
fn fremde_sqlite_db_abgelehnt() {
    let d = tmp("db-fremd");
    let p = d.join("andere.db");
    rusqlite::Connection::open(&p).unwrap().execute_batch("CREATE TABLE bilder(id); CREATE TABLE x(y);").unwrap();
    let e = db::oeffnen(p.to_str().unwrap()).unwrap_err();
    assert!(e.contains("keine Bildkatalog"), "{e}");
}

/// I-DB-05
#[test]
fn keine_sqlite_datei_abgelehnt() {
    let d = tmp("db-text");
    let p = d.join("text.db");
    std::fs::write(&p, "das ist keine Datenbank, sondern Text ".repeat(200)).unwrap();
    assert!(db::oeffnen(p.to_str().unwrap()).is_err());
}

/// I-DB-06
#[test]
fn ordner_statt_datei_abgelehnt() {
    let d = tmp("db-ordner");
    let e = db::oeffnen(d.to_str().unwrap()).unwrap_err();
    assert!(e.contains("nicht gefunden"), "{e}");
}
