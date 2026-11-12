//! I-ABL: Abläufe hinter den Tauri-Befehlen `start` und `db_setzen`.
mod gemeinsam;
use gemeinsam::*;
use bilderwerk_view_lib::{ablauf, settings};

/// I-ABL-01
#[test]
fn start_ohne_einstellung() {
    let d = tmp("abl-leer");
    let e = ablauf::start(&d.join("konfig")).err().unwrap();
    assert!(e.contains("Noch keine Datenbank"), "{e}");
}

/// I-ABL-02 (v0.3): kein eingebauter DB-Vorschlag mehr – ohne Einstellung immer die Auswahl
#[test]
fn start_ohne_vorschlag() {
    let d = tmp("abl-vorschlag");
    test_db(&d, "k.db");
    assert!(ablauf::start(&d.join("konfig")).is_err());
    assert!(!d.join("konfig").exists(), "nichts gespeichert");
}

/// I-ABL-03
#[test]
fn db_setzen_speichert_und_start_nutzt() {
    let d = tmp("abl-setzen");
    let k = d.join("konfig");
    let p = test_db(&d, "k.db");
    ablauf::db_setzen(&k, p.to_str().unwrap()).unwrap();
    assert_eq!(settings::laden(&k).db_pfad.as_deref(), p.to_str());
    assert!(ablauf::start(&k).is_ok());
}

/// I-ABL-04
#[test]
fn ungueltige_db_aendert_nichts() {
    let d = tmp("abl-ungueltig");
    let k = d.join("konfig");
    let p = test_db(&d, "k.db");
    ablauf::db_setzen(&k, p.to_str().unwrap()).unwrap();
    assert!(ablauf::db_setzen(&k, "/gibt/es/nicht.db").is_err());
    assert_eq!(settings::laden(&k).db_pfad.as_deref(), p.to_str(), "alte Wahl bleibt");
}

/// I-ABL-05
#[test]
fn start_mit_verschwundener_db() {
    let d = tmp("abl-weg");
    let k = d.join("konfig");
    settings::speichern(&k, &settings::Einstellungen { db_pfad: Some("/weg/k.db".into()), ..Default::default() }).unwrap();
    let e = ablauf::start(&k).err().unwrap();
    assert!(e.contains("nicht gefunden"), "{e}");
}

/// I-ABL-06 (v0.0.7): Dropbox ersetzt die Sicherung → alte Verbindung liest weiter den alten Stand,
/// `neu_laden` zeigt den neuen (hier: Vorschauen dazugekommen); der Beobachter erkennt das Ersetzen.
#[test]
fn ersetzte_db_neu_laden() {
    use bilderwerk_view_lib::beobachter::{stand, Beobachter};
    let d = tmp("abl-neu");
    let p = test_db(&d, "k.db");
    let pfad = p.to_str().unwrap();
    let (_, alt, info) = ablauf::db_setzen(&d.join("konfig"), pfad).unwrap();
    assert_eq!(info.vorschauen, None);
    let mut b = Beobachter::default();
    b.geladen(stand(&p));

    // neue Fassung daneben bauen und umbenennen (wie Dropbox)
    let neu = test_db(&d, "neu.db");
    vorschauen_anlegen(&neu, &["h1"]);
    std::fs::rename(&neu, &p).unwrap();

    assert!(!b.pruefen(stand(&p)));
    assert!(b.pruefen(stand(&p)), "stabil → neu laden");
    assert_eq!(bilderwerk_view_lib::vorschau_db::anzahl(&alt), None, "alte Verbindung bleibt beim alten Stand");
    let (_, _, info) = ablauf::neu_laden(pfad).unwrap();
    assert_eq!(info.vorschauen, Some(1));
    assert_eq!(info.schema_version, 4);
    assert!(ablauf::neu_laden("/gibt/es/nicht.db").is_err());
}
