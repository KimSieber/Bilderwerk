//! I-VDB: Vorschauen aus der Katalog-DB (Tabelle `vorschauen`, ab Katalog-Schema 4).
//! View muss mit und ohne diese Tabelle arbeiten.
mod gemeinsam;
use gemeinsam::*;
use bilderwerk_view_lib::{db, vorschau_db};

/// I-VDB-01
#[test]
fn ohne_tabelle() {
    let d = tmp("vdb-ohne");
    let conn = db::oeffnen(test_db(&d, "k.db").to_str().unwrap()).unwrap();
    assert!(!vorschau_db::hat_tabelle(&conn));
    assert_eq!(vorschau_db::schluessel_fuer(&conn, "/Archiv/Globetrotter/2015 Thailand/IMG_0001.jpg"), None);
    assert_eq!(vorschau_db::jpeg(&conn, "h1"), None, "keine Tabelle → kein Fehler, nur None");
}

/// I-VDB-02
#[test]
fn mit_tabelle_nach_hash() {
    let d = tmp("vdb-mit");
    let p = test_db(&d, "k.db");
    vorschauen_anlegen(&p, &["h1", "hAlbum#2"]);
    let conn = db::oeffnen(p.to_str().unwrap()).unwrap();
    assert!(vorschau_db::hat_tabelle(&conn));
    // Original und Duplikat (anderer Pfad, gleicher Inhalt) finden dieselbe Vorschau
    assert_eq!(vorschau_db::schluessel_fuer(&conn, "/Archiv/Globetrotter/2015 Thailand/IMG_0001.jpg").as_deref(), Some("h1"));
    assert_eq!(vorschau_db::schluessel_fuer(&conn, "/Sicherung/Alt/IMG_0001.jpg").as_deref(), Some("h1"));
    // PDF: je Seite eigene Vorschau
    assert_eq!(vorschau_db::schluessel_fuer(&conn, "/Archiv/Familie/Alben/Album.pdf#seite=2").as_deref(), Some("hAlbum#2"));
    assert_eq!(vorschau_db::schluessel_fuer(&conn, "/Archiv/Familie/Alben/Album.pdf#seite=1"), None, "Seite 1 fehlt noch");
    // Bild ohne Vorschau und unbekannter Pfad
    assert_eq!(vorschau_db::schluessel_fuer(&conn, "/Archiv/Tiere/Haus/XQ_0001.webp"), None);
    assert_eq!(vorschau_db::schluessel_fuer(&conn, "/gibt/es/nicht.jpg"), None);
}

/// I-VDB-03
#[test]
fn jpeg_lesen() {
    let d = tmp("vdb-jpeg");
    let p = test_db(&d, "k.db");
    vorschauen_anlegen(&p, &["h1", "hAlbum#2"]);
    let conn = db::oeffnen(p.to_str().unwrap()).unwrap();
    assert_eq!(vorschau_db::jpeg(&conn, "hAlbum#2").unwrap(), b"JPEG:hAlbum#2");
    assert_eq!(vorschau_db::jpeg(&conn, "unbekannt"), None);
}

/// I-VDB-04
#[test]
fn info_meldet_vorschauen() {
    let d = tmp("vdb-info");
    let p = test_db(&d, "k.db");
    let pfad = p.to_str().unwrap();
    assert_eq!(db::info(&db::oeffnen(pfad).unwrap(), pfad).unwrap().vorschauen, None);
    vorschauen_anlegen(&p, &["h1", "h2", "hAlbum#1"]);
    assert_eq!(db::info(&db::oeffnen(pfad).unwrap(), pfad).unwrap().vorschauen, Some(3));
}
