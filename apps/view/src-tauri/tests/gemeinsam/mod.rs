//! Gemeinsame Testhilfen: temporäre Ordner und eine kleine Katalog-DB mit bekanntem Inhalt.
//! Die DB hat dasselbe Schema wie der echte Bildkatalog (soweit View es nutzt):
//! Tabelle `bilder`, Begriffstabelle `begriffe`, Volltext `suche` (FTS5 trigram), user_version 3.
//! Seit v0.0.5 auch die Kategorie-Spalten für die Facetten (Motiv, Bildart, Farbgebung …)
//! und Personenmerkmale (`begriffe.typ = 'merkmal'`), siehe `kategorien()`.
#![allow(dead_code)]

use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// Frischer, leerer Ordner unter dem System-Temp (je Test eindeutig).
pub fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("view-it-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Ordner mit den eingecheckten Testbildern.
pub fn daten() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("daten")
}

/// Ein Eintrag der Test-DB.
pub struct Eintrag {
    pub id: i64,
    pub wurzel: &'static str,
    pub ordner: &'static str,
    pub dateiname: &'static str,
    pub endung: &'static str,
    pub kurztitel: &'static str,
    pub beschreibung: &'static str,
    pub schlagworte: &'static [&'static str],
    pub datum: Option<&'static str>,
    pub seite: Option<i64>,
    pub status: &'static str,
    pub meta: &'static str,
}

const fn e(
    id: i64,
    wurzel: &'static str,
    ordner: &'static str,
    dateiname: &'static str,
    endung: &'static str,
    kurztitel: &'static str,
    beschreibung: &'static str,
    schlagworte: &'static [&'static str],
    datum: Option<&'static str>,
) -> Eintrag {
    Eintrag { id, wurzel, ordner, dateiname, endung, kurztitel, beschreibung, schlagworte, datum, seite: None, status: "ok", meta: "" }
}

pub const GLOBE: &str = "/Archiv/Globetrotter";

/// Die festen Einträge 1–11. Dazu kommen 12–30 („Serienbild“) aus `serie()`.
pub fn eintraege() -> Vec<Eintrag> {
    let mut v = vec![
        e(1, GLOBE, "2015 Thailand", "IMG_0001.jpg", ".jpg", "Strand mit Booten",
          "Ein rotes Boot liegt am Strand.", &["boot", "strand", "meer"], Some("2015-03-10T10:00:00")),
        e(2, GLOBE, "2015 Thailand", "IMG_0002.JPG", ".JPG", "Goldener Tempel",
          "Ein Tempel nahe dem Strand.", &["tempel", "gold"], Some("2015-03-12T09:00:00")),
        Eintrag { status: "duplikat", ..e(3, "/Sicherung", "Alt", "IMG_0001.jpg", ".jpg", "Strand mit Booten",
          "Ein rotes Boot liegt am Strand.", &["boot", "strand", "meer"], Some("2015-03-10T10:00:00")) },
        e(4, "/Archiv/Familie", "Hochzeit", "Kirche.heic", ".heic", "Brautpaar vor Kirche",
          "Brautpaar mit Brautkleid vor der Kirche.", &["hochzeit", "kirche"], Some("2010-06-05T14:00:00")),
        Eintrag { meta: "Canon EOS 5D", ..e(5, "/Archiv/Natur", "Alpen", "Berge.png", ".png", "Verschneite Berglandschaft",
          "Schnee auf den Gipfeln.", &["winter", "schnee"], Some("2012-01-20T12:00:00")) },
        Eintrag { seite: Some(1), ..e(6, "/Archiv/Familie", "Alben", "Album.pdf", ".pdf", "Albumseite mit Fotos",
          "Mehrere eingeklebte Fotos.", &["album"], Some("2008-08-01T00:00:00")) },
        Eintrag { seite: Some(2), ..e(7, "/Archiv/Familie", "Alben", "Album.pdf", ".pdf", "Zweite Albumseite",
          "Handschrift unter den Fotos.", &["album"], Some("2008-08-01T00:00:00")) },
        e(8, "/Archiv/Alt", "", "Scan_8.jpg", ".jpg", "Altes Foto",
          "Ein vergilbtes Bild.", &["alt"], Some("2056-55-10T00:00:00")),
        e(9, "/Archiv/Alt", "", "Scan_9.jpg", ".jpg", "Foto ohne Datum",
          "Ein Bild ohne Datum.", &["alt"], None),
        Eintrag { status: "fehler", ..e(10, "/Archiv/Alt", "", "kaputt.jpg", ".jpg", "Kaputtes Bild",
          "", &[], Some("2011-01-01T00:00:00")) },
        e(11, "/Archiv/Tiere", "Haus", "XQ_0001.webp", ".webp", "Katze auf Sofa",
          "Eine Katze schläft.", &["katze", "sofa"], Some("2020-11-01T08:00:00")),
    ];
    v.extend(serie());
    v
}

/// 19 gleichartige Einträge für Limit-Tests (ids 12–30, Mai 2018).
fn serie() -> Vec<Eintrag> {
    const TAGE: [&str; 19] = [
        "2018-05-01", "2018-05-02", "2018-05-03", "2018-05-04", "2018-05-05", "2018-05-06", "2018-05-07",
        "2018-05-08", "2018-05-09", "2018-05-10", "2018-05-11", "2018-05-12", "2018-05-13", "2018-05-14",
        "2018-05-15", "2018-05-16", "2018-05-17", "2018-05-18", "2018-05-19",
    ];
    TAGE.iter()
        .enumerate()
        .map(|(i, t)| e(12 + i as i64, "/Archiv/Serie", "Serie", Box::leak(format!("Serienbild_{:02}.jpg", i + 1).into_boxed_str()), ".jpg", "Serienbild",
            "Teil einer Serie.", &["serie"], Some(Box::leak(format!("{t}T12:00:00").into_boxed_str()))))
        .collect()
}

/// Kategorien je Eintrag: (motiv, bildart, farbgebung, ausrichtung, jahreszeit, wetter, merkmale).
/// Eintrag 10 (Fehler) hat keine Kategorien.
pub fn kategorien(id: i64) -> Option<(&'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static [&'static str])> {
    Some(match id {
        1 | 3 => ("Landschaft", "Foto", "farbig", "quer", "Sommer", "sonnig", &[]),
        2 => ("Stadt/Architektur", "Foto", "farbig", "quer", "Sommer", "sonnig", &[]),
        4 => ("Gruppenbild", "Foto", "farbig", "hoch", "Sommer", "bedeckt", &["brautkleid"]),
        5 => ("Landschaft", "Foto", "farbig", "quer", "Winter", "Schnee", &[]),
        6 | 7 => ("Dokument/Text", "Fotoalbum/Collage", "sepia", "hoch", "unklar", "entfällt", &[]),
        8 => ("Porträt", "Scan/Dokument", "schwarzweiß", "hoch", "unklar", "unklar", &["brille"]),
        9 => ("Porträt", "Foto", "schwarzweiß", "quer", "unklar", "unklar", &["brille", "bart"]),
        10 => return None,
        11 => ("Tier", "Foto", "farbig", "quer", "Herbst", "entfällt", &[]),
        _ => ("Menschen in Aktion", "Foto", "farbig", "quer", "Frühling", "sonnig", &[]),
    })
}

/// Datei-Hash wie im Katalog: Duplikat 3 = Inhalt von 1; PDF-Seiten '<hash>#<seite>'.
pub fn hash_von(x: &Eintrag) -> String {
    let basis = match x.id {
        3 => "h1".to_string(),
        6 | 7 => "hAlbum".to_string(),
        id => format!("h{id}"),
    };
    match x.seite {
        Some(s) => format!("{basis}#{s}"),
        None => basis,
    }
}

/// Ergänzt die Test-DB um die Tabelle `vorschauen` (Schema-Version 4 des Katalogs)
/// mit Vorschauen für die angegebenen Hashes. Inhalt je Vorschau: b"JPEG:<hash>".
pub fn vorschauen_anlegen(db: &Path, hashes: &[&str]) {
    let c = Connection::open(db).unwrap();
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS vorschauen (hash TEXT PRIMARY KEY, jpeg BLOB NOT NULL, breite INTEGER NOT NULL,
            hoehe INTEGER NOT NULL, version INTEGER NOT NULL, erzeugt TEXT NOT NULL) WITHOUT ROWID;
         PRAGMA user_version = 4;",
    )
    .unwrap();
    for h in hashes {
        c.execute(
            "INSERT INTO vorschauen VALUES (?1, ?2, 640, 480, 1, '2026-11-04T12:00:00')",
            params![h, format!("JPEG:{h}").into_bytes()],
        )
        .unwrap();
    }
}

/// Ältere Katalog-DB ohne Kategorie-Spalten (Facetten fehlen dann still).
pub fn test_db_ohne_kategorien(dir: &Path, name: &str) -> PathBuf {
    let p = test_db(dir, name);
    let c = Connection::open(&p).unwrap();
    for s in ["motiv", "bildart", "farbgebung", "ausrichtung", "jahreszeit", "wetter", "personen_merkmale"] {
        c.execute_batch(&format!("ALTER TABLE bilder DROP COLUMN {s}")).unwrap();
    }
    p
}

/// Legt die Test-DB an und liefert ihren Pfad. `name` darf Leerzeichen/Sonderzeichen enthalten.
pub fn test_db(dir: &Path, name: &str) -> PathBuf {
    let pfad = dir.join(name);
    let _ = std::fs::remove_file(&pfad);
    let db = Connection::open(&pfad).unwrap();
    db.execute_batch(
        "CREATE TABLE bilder (
            id INTEGER PRIMARY KEY, pfad TEXT UNIQUE, archiv_wurzel TEXT, relativer_pfad TEXT,
            ordner TEXT, dateiname TEXT, endung TEXT, seite INTEGER, seiten_gesamt INTEGER,
            breite INTEGER, hoehe INTEGER, aufnahme_datum TEXT, kurztitel TEXT, beschreibung TEXT,
            schlagworte TEXT, status TEXT, duplikat_von INTEGER, hash TEXT,
            motiv TEXT, bildart TEXT, farbgebung TEXT, ausrichtung TEXT, jahreszeit TEXT, wetter TEXT,
            personen_merkmale TEXT);
         CREATE TABLE begriffe (bild_id INTEGER, typ TEXT, wert TEXT);
         CREATE VIRTUAL TABLE suche USING fts5(ablage, ki_text, meta_text, tokenize='trigram');
         PRAGMA user_version = 3;",
    )
    .unwrap();
    for x in eintraege() {
        let rel = if x.ordner.is_empty() { x.dateiname.to_string() } else { format!("{}/{}", x.ordner, x.dateiname) };
        let mut pfad = format!("{}/{rel}", x.wurzel);
        let mut relativ = rel.clone();
        if let Some(s) = x.seite {
            pfad = format!("{pfad}#seite={s}");
            relativ = format!("{rel} [Seite {s}/2]");
        }
        let schlag = serde_json::to_string(x.schlagworte).unwrap();
        db.execute(
            "INSERT INTO bilder (id,pfad,archiv_wurzel,relativer_pfad,ordner,dateiname,endung,seite,
                seiten_gesamt,breite,hoehe,aufnahme_datum,kurztitel,beschreibung,schlagworte,status,duplikat_von,hash)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,4000,3000,?10,?11,?12,?13,?14,?15,?16)",
            params![
                x.id, pfad, x.wurzel, relativ, x.ordner, x.dateiname, x.endung, x.seite,
                x.seite.map(|_| 2), x.datum, x.kurztitel, x.beschreibung, schlag, x.status,
                if x.status == "duplikat" { Some(1) } else { None },
                hash_von(&x)
            ],
        )
        .unwrap();
        if x.status != "fehler" {
            db.execute(
                "INSERT INTO suche (rowid, ablage, ki_text, meta_text) VALUES (?1, ?2, ?3, ?4)",
                params![x.id, relativ, format!("{} {} {}", x.kurztitel, x.beschreibung, x.schlagworte.join(" ")), x.meta],
            )
            .unwrap();
            for w in x.schlagworte {
                db.execute("INSERT INTO begriffe VALUES (?1,'schlagwort',?2)", params![x.id, w]).unwrap();
            }
        }
        if let Some((motiv, bildart, farbe, ausr, jz, wetter, merkmale)) = kategorien(x.id) {
            db.execute(
                "UPDATE bilder SET motiv=?2, bildart=?3, farbgebung=?4, ausrichtung=?5, jahreszeit=?6, wetter=?7,
                    personen_merkmale=?8 WHERE id=?1",
                params![x.id, motiv, bildart, farbe, ausr, jz, wetter, serde_json::to_string(merkmale).unwrap()],
            )
            .unwrap();
            if x.status != "fehler" {
                for m in merkmale {
                    db.execute("INSERT INTO begriffe VALUES (?1,'merkmal',?2)", params![x.id, m]).unwrap();
                }
            }
        }
    }
    drop(db);
    pfad
}
