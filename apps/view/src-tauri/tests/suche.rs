//! I-DB (Suche): Suchkern gegen die Test-DB mit bekanntem Inhalt.
mod gemeinsam;
use gemeinsam::*;
use bilderwerk_view_lib::{db, search::{suchen, SuchFilter}};

fn ids(f: SuchFilter) -> Vec<i64> {
    let d = tmp(&format!("suche-{}", std::thread::current().name().unwrap_or("x").replace("::", "-")));
    let p = test_db(&d, "k.db");
    let conn = db::oeffnen(p.to_str().unwrap()).unwrap();
    let mut v: Vec<i64> = suchen(&conn, &f).unwrap().treffer.iter().map(|t| t.id).collect();
    v.sort();
    v
}

fn frei(s: &str) -> SuchFilter {
    SuchFilter { freitext: s.into(), ..Default::default() }
}

/// I-SUCH-01
#[test]
fn freitext_wortteil_und_gross_klein() {
    assert_eq!(ids(frei("boot")), vec![1]);
    assert_eq!(ids(frei("BOOT")), vec![1]);
    assert_eq!(ids(frei("landschaft")), vec![5], "Wortteil in Berglandschaft");
}

/// I-SUCH-02
#[test]
fn phrase_und_ausschluss() {
    assert_eq!(ids(frei("\"rotes boot\"")), vec![1]);
    assert_eq!(ids(frei("\"boot rotes\"")), Vec::<i64>::new());
    assert_eq!(ids(frei("strand -boot")), vec![2]);
    assert_eq!(ids(frei("strand tempel")), vec![2], "alle Begriffe müssen passen");
}

/// I-SUCH-03
#[test]
fn kurze_begriffe() {
    assert_eq!(ids(frei("xq")), vec![11], "< 3 Zeichen über Dateiname");
}

/// I-SUCH-04
#[test]
fn metadaten_und_archiv_wurzel() {
    assert_eq!(ids(frei("canon")), vec![5]);
    assert_eq!(ids(frei("globetrotter")), vec![1, 2], "Archiv-Wurzel ist nicht im Index");
}

/// I-SUCH-05
#[test]
fn dateiname_und_ordner() {
    assert_eq!(ids(SuchFilter { name: "ALBUM".into(), ..Default::default() }), vec![6, 7]);
    assert_eq!(ids(SuchFilter { ordner: "globetrotter/2015".into(), ..Default::default() }), vec![1, 2]);
}

/// I-SUCH-06
#[test]
fn datum_von_bis() {
    let f = |von: &str, bis: &str| SuchFilter { datum_von: von.into(), datum_bis: bis.into(), ..Default::default() };
    assert_eq!(ids(f("2015", "2015")), vec![1, 2]);
    assert_eq!(ids(f("", "2010")), vec![4, 6, 7]);
    assert_eq!(ids(f("2010-06-05", "2010-06-05")), vec![4], "Tagesgrenzen eingeschlossen");
    assert_eq!(ids(f("2015-03-11", "2015-03")), vec![2]);
}

/// I-SUCH-07
#[test]
fn dateitypen() {
    let f = |e: &[&str]| SuchFilter { endungen: e.iter().map(|s| s.to_string()).collect(), ..Default::default() };
    assert_eq!(ids(f(&[".pdf"])), vec![6, 7]);
    assert_eq!(ids(f(&[".heic", ".heif"])), vec![4]);
    assert!(ids(f(&[".jpg", ".jpeg"])).contains(&2), "Endung .JPG zählt als .jpg");
}

/// I-SUCH-08
#[test]
fn duplikate_und_fehler() {
    assert_eq!(ids(SuchFilter { freitext: "boot".into(), mit_duplikaten: true, ..Default::default() }), vec![1, 3]);
    assert_eq!(ids(frei("kaputt")), Vec::<i64>::new(), "Status fehler erscheint nie");
}

/// I-SUCH-09
#[test]
fn sortierung_unplausibel_ans_ende() {
    let d = tmp("suche-sort");
    let conn = db::oeffnen(test_db(&d, "k.db").to_str().unwrap()).unwrap();
    let e = suchen(&conn, &SuchFilter { endungen: vec![".jpg".into()], ..Default::default() }).unwrap();
    let t: Vec<i64> = e.treffer.iter().map(|t| t.id).collect();
    assert_eq!(&t[t.len() - 2..], &[8, 9], "unplausibles Datum, dann ohne Datum");
    let daten: Vec<&str> = e.treffer[..t.len() - 2].iter().map(|t| t.aufnahme_datum.as_deref().unwrap()).collect();
    assert!(daten.windows(2).all(|w| w[0] >= w[1]), "neueste zuerst: {daten:?}");
}

/// I-SUCH-10
#[test]
fn limit_und_gesamt() {
    let d = tmp("suche-limit");
    let conn = db::oeffnen(test_db(&d, "k.db").to_str().unwrap()).unwrap();
    let e = suchen(&conn, &SuchFilter { name: "serien".into(), limit: 5, ..Default::default() }).unwrap();
    assert_eq!(e.gesamt, 19);
    assert_eq!(e.treffer.len(), 5);
    let e = suchen(&conn, &SuchFilter { name: "serien".into(), ..Default::default() }).unwrap();
    assert_eq!(e.treffer.len(), 19, "limit 0 = Standard (3000)");
}

/// I-SUCH-11
#[test]
fn leerer_filter_ohne_abfrage() {
    let d = tmp("suche-leer");
    let conn = db::oeffnen(test_db(&d, "k.db").to_str().unwrap()).unwrap();
    let e = suchen(&conn, &SuchFilter { mit_duplikaten: true, ..Default::default() }).unwrap();
    assert_eq!((e.gesamt, e.treffer.len()), (0, 0));
}

/// I-SUCH-12
#[test]
fn trefferfelder_vollstaendig() {
    let d = tmp("suche-felder");
    let conn = db::oeffnen(test_db(&d, "k.db").to_str().unwrap()).unwrap();
    let e = suchen(&conn, &SuchFilter { name: "album".into(), ..Default::default() }).unwrap();
    let t = e.treffer.iter().find(|t| t.id == 7).unwrap();
    assert_eq!(t.pfad, "/Archiv/Familie/Alben/Album.pdf#seite=2");
    assert_eq!(t.seite, Some(2));
    assert_eq!(t.endung.as_deref(), Some(".pdf"));
    assert_eq!(t.archiv_wurzel.as_deref(), Some("/Archiv/Familie"));
    assert_eq!(t.status.as_deref(), Some("ok"));
    assert_eq!((t.breite, t.hoehe), (Some(4000), Some(3000)));
}

/// I-SUCH-13
#[test]
fn sonderzeichen_ohne_fehler() {
    let d = tmp("suche-sonder");
    let conn = db::oeffnen(test_db(&d, "k.db").to_str().unwrap()).unwrap();
    for s in ["\"", "a\"b", "AND", "OR NOT", "(*)", "c++", "%_", "'; DROP TABLE bilder; --", "NEAR(x y)"] {
        assert!(suchen(&conn, &frei(s)).is_ok(), "Suche nach {s:?} darf nicht scheitern");
    }
}
