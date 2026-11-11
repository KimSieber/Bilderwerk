//! V: Vertrag Rust ↔ Frontend. Die Tests erzeugen echte JSON-Antworten der Befehle und
//! legen sie unter tests/vertrag/ ab. Die Frontend-Tests (src/vertrag.test.ts) nutzen genau
//! diese Dateien als Mock – ändert sich ein Feld in Rust, schlägt der Frontend-Test an.
//! Die Dateien sind deterministisch (feste Zeiten/Pfade) und gehören ins Repo.
mod gemeinsam;
use gemeinsam::*;
use bilderwerk_view_lib::{db, search, thumbs::Vorschau};
use std::path::PathBuf;

fn ziel(name: &str) -> PathBuf {
    let d = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("tests").join("vertrag");
    std::fs::create_dir_all(&d).unwrap();
    d.join(name)
}

/// Schreibt nur bei Änderung (hält Dropbox und git ruhig).
fn schreiben(name: &str, wert: &impl serde::Serialize) {
    let json = serde_json::to_string_pretty(wert).unwrap() + "\n";
    let p = ziel(name);
    if std::fs::read_to_string(&p).ok().as_deref() != Some(json.as_str()) {
        std::fs::write(&p, json).unwrap();
    }
}

/// V-01
#[test]
fn ergebnis() {
    let d = tmp("vertrag-erg");
    let conn = db::oeffnen(test_db(&d, "k.db").to_str().unwrap()).unwrap();
    let f = search::SuchFilter { name: "album".into(), limit: 1, ..Default::default() };
    let mut e = search::suchen(&conn, &f).unwrap();
    assert_eq!((e.gesamt, e.treffer.len()), (2, 1));
    e.dauer_ms = 12;
    schreiben("ergebnis.json", &e);
    let mut leer = search::suchen(&conn, &search::SuchFilter::default()).unwrap();
    leer.dauer_ms = 0;
    schreiben("ergebnis-leer.json", &leer);
}

/// V-02
#[test]
fn dbinfo() {
    let d = tmp("vertrag-info");
    let p = test_db(&d, "k.db");
    let conn = db::oeffnen(p.to_str().unwrap()).unwrap();
    let mut i = db::info(&conn, p.to_str().unwrap()).unwrap();
    i.pfad = "/Users/test/Dropbox/Bildkatalog.db".into();
    i.geaendert = Some(1_793_000_000);
    i.groesse = 123_456_789;
    assert_eq!(i.vorschauen, None);
    i.vorschauen = Some(24_000);
    schreiben("dbinfo.json", &i);
}

/// V-03
#[test]
fn vorschau() {
    schreiben("vorschau-bild.json", &Vorschau::Bild { datei: "/Users/test/Library/Caches/vorschau/600/0123456789abcdef.jpg".into() });
    schreiben("vorschau-nuronline.json", &Vorschau::NurOnline);
    schreiben("vorschau-fehlt.json", &Vorschau::Fehlt);
    schreiben("vorschau-fehler.json", &Vorschau::Fehler { meldung: "sips: unsupported format".into() });
    schreiben("vorschau-datenbank.json", &Vorschau::Datenbank { schluessel: "3f2a9c#2".into() });
}

/// V-04: Filter aus dem Frontend (camelCase) wird korrekt gelesen, fehlende Felder = Standard.
#[test]
fn filter_lesen() {
    let f: search::SuchFilter = serde_json::from_str(
        r#"{"freitext":"a","name":"b","ordner":"c","datumVon":"2015","datumBis":"2016","endungen":[".jpg"],"mitDuplikaten":true}"#,
    )
    .unwrap();
    assert_eq!((f.datum_von.as_str(), f.datum_bis.as_str(), f.mit_duplikaten, f.limit), ("2015", "2016", true, 0));
    let f: search::SuchFilter = serde_json::from_str(r#"{"freitext":"x"}"#).unwrap();
    assert!(f.endungen.is_empty() && !f.mit_duplikaten && f.facetten.is_empty());
    let f: search::SuchFilter =
        serde_json::from_str(r#"{"facetten":{"motiv":["Tier","Porträt"],"merkmale":["brille"]}}"#).unwrap();
    assert_eq!(f.facetten["motiv"], vec!["Tier", "Porträt"]);
    assert_eq!(f.facetten["merkmale"], vec!["brille"]);
}

/// V-13: Facetten (Befehl facetten, Stufe 3) – Ausschnitt mit Auswahl „Porträt“
#[test]
fn facetten() {
    let d = tmp("vertrag-fac");
    let conn = db::oeffnen(test_db(&d, "k.db").to_str().unwrap()).unwrap();
    let mut f = search::SuchFilter::default();
    f.facetten.insert("motiv".into(), vec!["Porträt".into()]);
    let mut v = bilderwerk_view_lib::facetten::facetten(&conn, &f).unwrap();
    v.retain(|x| ["motiv", "merkmale"].contains(&x.feld.as_str()));
    v[0].werte.truncate(3);
    schreiben("facetten.json", &v);
}

/// V-09: Dropbox-Status (Form wie der Tauri-Befehl dropbox_status) – Frontend prüft die Felder.
#[test]
fn dropbox_status() {
    schreiben("dropbox-status.json", &serde_json::json!({ "eingerichtet": true, "angemeldet": false }));
}

/// V-11: Cache-Info (Befehle cache_info / cache_leeren, Ereignis cache-geleert)
#[test]
fn cache_info() {
    schreiben("cache-info.json", &bilderwerk_view_lib::cache::CacheInfo { bytes: 1_363_148_800, dateien: 24_000 });
}

/// V-15 (v0.0.6): Detailbereich – Duplikat 3 hat ein Original (1) und Schlagworte
#[test]
fn details() {
    let d = tmp("vertrag-details");
    let conn = db::oeffnen(test_db(&d, "k.db").to_str().unwrap()).unwrap();
    let x = bilderwerk_view_lib::details::details(&conn, 3).unwrap();
    assert_eq!(x.duplikate.len(), 1);
    schreiben("details.json", &x);
}
