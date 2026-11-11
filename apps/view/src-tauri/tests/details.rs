//! I-DET (v0.0.6): Detailbereich – alle Angaben eines Bildes aus der Katalog-DB.
//! Generisch über die Spalten von `bilder`: neue Spalten von Work erscheinen ohne Code-Änderung.
mod gemeinsam;
use gemeinsam::*;
use bilderwerk_view_lib::{db, details::{details, Details}};

fn conn(name: &str) -> rusqlite::Connection {
    let d = tmp(&format!("det-{name}"));
    db::oeffnen(test_db(&d, "k.db").to_str().unwrap()).unwrap()
}

fn wert<'a>(d: &'a Details, name: &str) -> Option<&'a serde_json::Value> {
    d.felder.iter().find(|f| f.name == name).map(|f| &f.wert)
}

/// I-DET-01
#[test]
fn alle_spalten_in_tabellenreihenfolge() {
    let c = conn("alle");
    let d = details(&c, 1).unwrap();
    assert_eq!(d.id, 1);
    assert_eq!(d.pfad, "/Archiv/Globetrotter/2015 Thailand/IMG_0001.jpg");
    assert_eq!(wert(&d, "kurztitel").unwrap(), "Strand mit Booten");
    assert_eq!(wert(&d, "beschreibung").unwrap(), "Ein rotes Boot liegt am Strand.");
    assert_eq!(wert(&d, "breite").unwrap(), 4000);
    assert_eq!(wert(&d, "motiv").unwrap(), "Landschaft");
    // JSON-Listen bleiben Text – die Oberfläche zerlegt sie
    assert_eq!(wert(&d, "schlagworte").unwrap(), r#"["boot","strand","meer"]"#);
    let namen: Vec<&str> = d.felder.iter().map(|f| f.name.as_str()).collect();
    let pos = |n: &str| namen.iter().position(|x| *x == n).unwrap();
    assert!(pos("pfad") < pos("kurztitel") && pos("kurztitel") < pos("motiv"), "Reihenfolge wie in der Tabelle: {namen:?}");
}

/// I-DET-02
#[test]
fn leere_werte_fehlen() {
    let c = conn("leer");
    let d = details(&c, 9).unwrap(); // ohne Datum, ohne Ordner
    assert!(wert(&d, "aufnahme_datum").is_none());
    assert!(wert(&d, "ordner").is_none(), "leerer Text zählt als leer");
    assert!(wert(&d, "seite").is_none());
    assert!(wert(&d, "duplikat_von").is_none());
}

/// I-DET-03
#[test]
fn begriffe_nach_typ() {
    let c = conn("begriffe");
    let d = details(&c, 4).unwrap();
    assert_eq!(d.begriffe.get("schlagwort").unwrap(), &vec!["hochzeit".to_string(), "kirche".to_string()]);
    assert_eq!(d.begriffe.get("merkmal").unwrap(), &vec!["brautkleid".to_string()]);
}

/// I-DET-04
#[test]
fn duplikate_in_beide_richtungen() {
    let c = conn("dup");
    let o = details(&c, 1).unwrap();
    assert_eq!(o.duplikate.iter().map(|x| x.id).collect::<Vec<_>>(), vec![3]);
    assert_eq!(o.duplikate[0].pfad, "/Sicherung/Alt/IMG_0001.jpg");
    assert_eq!(o.duplikate[0].status.as_deref(), Some("duplikat"));
    let k = details(&c, 3).unwrap();
    assert_eq!(k.duplikate.iter().map(|x| x.id).collect::<Vec<_>>(), vec![1], "Duplikat zeigt sein Original");
    assert!(details(&c, 2).unwrap().duplikate.is_empty());
}

/// I-DET-05
#[test]
fn unbekannte_id() {
    let c = conn("unbekannt");
    assert!(details(&c, 999).unwrap_err().contains("999"));
}

/// I-DET-06
#[test]
fn aeltere_db_ohne_kategorien() {
    let d = tmp("det-alt");
    let c = db::oeffnen(test_db_ohne_kategorien(&d, "k.db").to_str().unwrap()).unwrap();
    let x = details(&c, 1).unwrap();
    assert!(wert(&x, "motiv").is_none());
    assert_eq!(wert(&x, "kurztitel").unwrap(), "Strand mit Booten");
}
