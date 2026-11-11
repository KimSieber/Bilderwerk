//! I-FAC (Stufe 3): Facetten mit Anzahl gegen die Test-DB (Kategorien siehe gemeinsam::kategorien).
mod gemeinsam;
use gemeinsam::*;
use bilderwerk_view_lib::{db, facetten::{facetten, Facette}, search::{suchen, SuchFilter}};
use std::collections::BTreeMap;

fn verbindung(name: &str) -> rusqlite::Connection {
    let d = tmp(&format!("fac-{name}"));
    db::oeffnen(test_db(&d, "k.db").to_str().unwrap()).unwrap()
}

fn auswahl(paare: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    paare.iter().map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect())).collect()
}

fn mit(paare: &[(&str, &[&str])]) -> SuchFilter {
    SuchFilter { facetten: auswahl(paare), ..Default::default() }
}

/// (wert, anzahl) einer Facette
fn werte(f: &[Facette], feld: &str) -> Vec<(String, i64)> {
    f.iter().find(|x| x.feld == feld).unwrap_or_else(|| panic!("Facette {feld} fehlt"))
        .werte.iter().map(|w| (w.wert.clone(), w.anzahl)).collect()
}

fn w(v: &[(&str, i64)]) -> Vec<(String, i64)> {
    v.iter().map(|(s, n)| (s.to_string(), *n)).collect()
}

fn ids(c: &rusqlite::Connection, f: &SuchFilter) -> Vec<i64> {
    let mut v: Vec<i64> = suchen(c, f).unwrap().treffer.iter().map(|t| t.id).collect();
    v.sort();
    v
}

/// I-FAC-01
#[test]
fn ohne_filter_alle_facetten_mit_anzahl() {
    let c = verbindung("01");
    let f = facetten(&c, &SuchFilter::default()).unwrap();
    let felder: Vec<_> = f.iter().map(|x| x.feld.as_str()).collect();
    assert_eq!(felder, ["motiv", "bildart", "farbgebung", "ausrichtung", "jahreszeit", "wetter", "merkmale", "schlagworte"]);
    assert_eq!(f[0].titel, "Motiv");
    assert_eq!(
        werte(&f, "motiv"),
        w(&[("Menschen in Aktion", 19), ("Dokument/Text", 2), ("Landschaft", 2), ("Porträt", 2),
            ("Gruppenbild", 1), ("Stadt/Architektur", 1), ("Tier", 1)]),
        "nach Anzahl, dann alphabetisch; Duplikat und Fehler zählen nicht"
    );
    assert_eq!(werte(&f, "farbgebung"), w(&[("farbig", 24), ("schwarzweiß", 2), ("sepia", 2)]));
    assert_eq!(werte(&f, "merkmale"), w(&[("brille", 2), ("bart", 1), ("brautkleid", 1)]));
    assert_eq!(werte(&f, "schlagworte")[0], ("serie".to_string(), 19));
}

/// I-FAC-02
#[test]
fn auswahl_oder_innerhalb_und_zwischen_facetten() {
    let c = verbindung("02");
    assert_eq!(ids(&c, &mit(&[("motiv", &["Porträt"])])), vec![8, 9]);
    assert_eq!(ids(&c, &mit(&[("motiv", &["Porträt", "Landschaft"])])), vec![1, 5, 8, 9], "ODER innerhalb");
    assert_eq!(
        ids(&c, &mit(&[("motiv", &["Porträt", "Landschaft"]), ("farbgebung", &["farbig"])])),
        vec![1, 5],
        "UND zwischen Facetten"
    );
    let f = SuchFilter { freitext: "foto".into(), ..mit(&[("ausrichtung", &["hoch"])]) };
    assert_eq!(ids(&c, &f), vec![6, 7, 8], "mit Freitext kombiniert");
}

/// I-FAC-03
#[test]
fn merkmale_und_schlagworte_ueber_begriffe() {
    let c = verbindung("03");
    assert_eq!(ids(&c, &mit(&[("merkmale", &["brille"])])), vec![8, 9]);
    assert_eq!(ids(&c, &mit(&[("merkmale", &["brille", "brautkleid"])])), vec![4, 8, 9]);
    assert_eq!(ids(&c, &mit(&[("schlagworte", &["strand"])])), vec![1]);
    assert_eq!(ids(&c, &mit(&[("schlagworte", &["strand"]), ("merkmale", &["brille"])])), Vec::<i64>::new());
}

/// I-FAC-04
#[test]
fn eigene_auswahl_zaehlt_nicht_gegen_sich_selbst() {
    let c = verbindung("04");
    let f = facetten(&c, &mit(&[("motiv", &["Porträt"])])).unwrap();
    assert_eq!(werte(&f, "motiv")[0], ("Menschen in Aktion".to_string(), 19), "Motiv zeigt weiter alle Werte");
    assert_eq!(werte(&f, "farbgebung"), w(&[("schwarzweiß", 2)]), "andere Facetten nur innerhalb der Auswahl");
    assert_eq!(werte(&f, "merkmale"), w(&[("brille", 2), ("bart", 1)]));
}

/// I-FAC-05
#[test]
fn facetten_folgen_den_suchkriterien() {
    let c = verbindung("05");
    let f = facetten(&c, &SuchFilter { freitext: "album".into(), ..Default::default() }).unwrap();
    assert_eq!(werte(&f, "motiv"), w(&[("Dokument/Text", 2)]));
    assert!(werte(&f, "merkmale").is_empty());
    let f = facetten(&c, &SuchFilter { mit_duplikaten: true, ..Default::default() }).unwrap();
    assert!(werte(&f, "motiv").contains(&("Landschaft".to_string(), 3)), "Duplikate zählen, wenn gewählt");
}

/// I-FAC-06
#[test]
fn gewaehlter_wert_ohne_treffer_bleibt_sichtbar() {
    let c = verbindung("06");
    let f = SuchFilter { freitext: "album".into(), ..mit(&[("jahreszeit", &["Winter"]), ("merkmale", &["bart"])]) };
    let fa = facetten(&c, &f).unwrap();
    assert!(werte(&fa, "jahreszeit").contains(&("Winter".to_string(), 0)), "{:?}", werte(&fa, "jahreszeit"));
    assert!(werte(&fa, "merkmale").contains(&("bart".to_string(), 0)));
    assert!(ids(&c, &f).is_empty());
}

/// I-FAC-07
#[test]
fn aeltere_db_ohne_kategorie_spalten() {
    let d = tmp("fac-07");
    let c = db::oeffnen(test_db_ohne_kategorien(&d, "alt.db").to_str().unwrap()).unwrap();
    let f = facetten(&c, &SuchFilter::default()).unwrap();
    let felder: Vec<_> = f.iter().map(|x| x.feld.as_str()).collect();
    assert_eq!(felder, ["merkmale", "schlagworte"], "nur Facetten, deren Daten es gibt");
    assert_eq!(ids(&c, &mit(&[("motiv", &["Porträt"])])), Vec::<i64>::new(), "Auswahl einer fehlenden Spalte → keine Treffer, kein Fehler");
}
