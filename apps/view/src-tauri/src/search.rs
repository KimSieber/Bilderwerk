//! Suchkern: baut aus den Filterkriterien eine SQL-Abfrage gegen die
//! Bildkatalog-Datenbank. Bewusst ohne Tauri-Abhängigkeit, damit testbar.

use rusqlite::{params_from_iter, types::Value, Connection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Kriterien aus der Oberfläche. Alle gesetzten Kriterien werden mit UND verknüpft.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SuchFilter {
    /// Freitext: Wörter (oder "Phrasen") durchsuchen Ablage, KI-Text und Metadaten.
    /// Ein vorangestelltes "-" schließt Treffer aus.
    pub freitext: String,
    /// Bestandteil des Dateinamens
    pub name: String,
    /// Bestandteil des Ablageordners (inkl. Archiv-Wurzel)
    pub ordner: String,
    /// Aufnahmedatum ab (YYYY, YYYY-MM oder YYYY-MM-DD)
    pub datum_von: String,
    /// Aufnahmedatum bis einschließlich (YYYY, YYYY-MM oder YYYY-MM-DD)
    pub datum_bis: String,
    /// Dateiendungen inkl. Punkt, z. B. [".jpg", ".jpeg"]; leer = alle
    pub endungen: Vec<String>,
    /// Auch Duplikate (weitere Ablageorte) anzeigen
    pub mit_duplikaten: bool,
    /// Höchstzahl gelieferter Treffer (0 = Standard)
    pub limit: u32,
    /// Facetten-Auswahl (Stufe 3): Feld → gewählte Werte. Innerhalb eines Feldes ODER,
    /// zwischen Feldern UND. Unbekannte Felder werden ignoriert (siehe facetten::FACETTEN).
    pub facetten: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Treffer {
    pub id: i64,
    pub pfad: String,
    pub relativer_pfad: Option<String>,
    pub archiv_wurzel: Option<String>,
    pub ordner: Option<String>,
    pub dateiname: Option<String>,
    pub endung: Option<String>,
    pub kurztitel: Option<String>,
    pub aufnahme_datum: Option<String>,
    pub seite: Option<i64>,
    pub breite: Option<i64>,
    pub hoehe: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ergebnis {
    pub gesamt: i64,
    pub treffer: Vec<Treffer>,
    pub dauer_ms: u128,
}

const STANDARD_LIMIT: u32 = 3000;

/// Datum gilt als plausibel, wenn es ein gültiges Kalenderdatum ist und nicht in der
/// Zukunft liegt (der Katalog enthält teils aus Dateinamen geratene Werte wie 2056-55-10).
/// Unplausible Daten und Bilder ohne Datum werden ans Ende sortiert.
const DATUM_PLAUSIBEL: &str = "(date(substr(b.aufnahme_datum,1,10)) IS NOT NULL \
     AND b.aufnahme_datum <= strftime('%Y-%m-%dT%H:%M:%S','now'))";

impl SuchFilter {
    /// true, wenn kein einziges Kriterium gesetzt ist (→ keine Suche, leere Fläche).
    pub fn ist_leer(&self) -> bool {
        self.freitext.trim().is_empty()
            && self.name.trim().is_empty()
            && self.ordner.trim().is_empty()
            && self.datum_von.trim().is_empty()
            && self.datum_bis.trim().is_empty()
            && self.endungen.is_empty()
            && self.facetten.values().all(|v| v.is_empty())
    }
}

/// Zerlegt den Freitext in Begriffe; "in Anführungszeichen" bleibt zusammen.
/// Liefert (begriff, ausschliessen).
pub fn begriffe(text: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        let mut neg = false;
        if c == '-' {
            neg = true;
            chars.next();
        }
        let mut wort = String::new();
        if chars.peek() == Some(&'"') {
            chars.next();
            while let Some(c) = chars.next() {
                if c == '"' {
                    break;
                }
                wort.push(c);
            }
        } else {
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                wort.push(c);
                chars.next();
            }
        }
        let wort = wort.trim().to_string();
        if !wort.is_empty() {
            out.push((wort, neg));
        }
    }
    out
}

/// FTS5-Phrase: in Anführungszeichen, innere Anführungszeichen verdoppelt.
fn fts_phrase(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Obergrenze für "bis einschließlich": '2015' → '2015Z' (größer als jedes '2015-…').
fn datum_bis(s: &str) -> String {
    format!("{}Z", s.trim())
}

/// Baut WHERE-Klausel und Parameter.
pub fn where_klausel(f: &SuchFilter) -> (String, Vec<Value>) {
    let mut teile: Vec<String> = Vec::new();
    let mut p: Vec<Value> = Vec::new();

    if f.mit_duplikaten {
        teile.push("b.status IN ('ok','duplikat')".into());
    } else {
        teile.push("b.status = 'ok'".into());
    }

    for (wort, neg) in begriffe(&f.freitext) {
        let bedingung = if wort.chars().count() >= 3 {
            // Trigram-Volltext über alle Spalten + Archiv-Wurzel (nicht im Index)
            p.push(Value::Text(fts_phrase(&wort)));
            p.push(Value::Text(wort.clone()));
            "(b.id IN (SELECT rowid FROM suche WHERE suche MATCH ?) \
              OR instr(lower(b.archiv_wurzel), lower(?)) > 0)"
                .to_string()
        } else {
            // < 3 Zeichen: Trigram greift nicht → LIKE auf die wichtigsten Felder
            for _ in 0..4 {
                p.push(Value::Text(wort.clone()));
            }
            "(instr(lower(b.dateiname), lower(?)) > 0 \
              OR instr(lower(b.kurztitel), lower(?)) > 0 \
              OR instr(lower(b.beschreibung), lower(?)) > 0 \
              OR instr(lower(b.schlagworte), lower(?)) > 0)"
                .to_string()
        };
        teile.push(if neg { format!("NOT {bedingung}") } else { bedingung });
    }

    if !f.name.trim().is_empty() {
        teile.push("instr(lower(b.dateiname), lower(?)) > 0".into());
        p.push(Value::Text(f.name.trim().into()));
    }
    if !f.ordner.trim().is_empty() {
        teile.push(
            "instr(lower(coalesce(b.archiv_wurzel,'') || '/' || coalesce(b.ordner,'')), lower(?)) > 0"
                .into(),
        );
        p.push(Value::Text(f.ordner.trim().into()));
    }
    if !f.datum_von.trim().is_empty() {
        teile.push("b.aufnahme_datum >= ?".into());
        p.push(Value::Text(f.datum_von.trim().into()));
    }
    if !f.datum_bis.trim().is_empty() {
        teile.push("b.aufnahme_datum <= ?".into());
        p.push(Value::Text(datum_bis(&f.datum_bis)));
    }
    if !f.endungen.is_empty() {
        let platzhalter = vec!["?"; f.endungen.len()].join(",");
        teile.push(format!("lower(b.endung) IN ({platzhalter})"));
        for e in &f.endungen {
            p.push(Value::Text(e.to_lowercase()));
        }
    }

    for (feld, werte) in &f.facetten {
        if let Some(b) = crate::facetten::bedingung(feld, werte.len()) {
            teile.push(b);
            p.extend(werte.iter().map(|w| Value::Text(w.clone())));
        }
    }

    (teile.join(" AND "), p)
}

pub fn suchen(db: &Connection, f: &SuchFilter) -> rusqlite::Result<Ergebnis> {
    let start = std::time::Instant::now();
    if f.ist_leer() {
        return Ok(Ergebnis { gesamt: 0, treffer: vec![], dauer_ms: 0 });
    }
    if !crate::facetten::auswertbar(db, f)? {
        // Auswahl in einer Kategorie, die diese (ältere) DB nicht hat → keine Treffer statt SQL-Fehler
        return Ok(Ergebnis { gesamt: 0, treffer: vec![], dauer_ms: start.elapsed().as_millis() });
    }
    let (wo, p) = where_klausel(f);

    let gesamt: i64 = db.query_row(
        &format!("SELECT count(*) FROM bilder b WHERE {wo}"),
        params_from_iter(p.iter()),
        |r| r.get(0),
    )?;

    let limit = if f.limit == 0 { STANDARD_LIMIT } else { f.limit };
    let sql = format!(
        "SELECT b.id, b.pfad, b.relativer_pfad, b.archiv_wurzel, b.ordner, b.dateiname, b.endung, \
                b.kurztitel, b.aufnahme_datum, b.seite, b.breite, b.hoehe, b.status \
         FROM bilder b WHERE {wo} \
         ORDER BY {plausibel} DESC, b.aufnahme_datum DESC, b.relativer_pfad \
         LIMIT {limit}",
        plausibel = DATUM_PLAUSIBEL
    );
    let mut stmt = db.prepare(&sql)?;
    let treffer = stmt
        .query_map(params_from_iter(p.iter()), |r| {
            Ok(Treffer {
                id: r.get(0)?,
                pfad: r.get(1)?,
                relativer_pfad: r.get(2)?,
                archiv_wurzel: r.get(3)?,
                ordner: r.get(4)?,
                dateiname: r.get(5)?,
                endung: r.get(6)?,
                kurztitel: r.get(7)?,
                aufnahme_datum: r.get(8)?,
                seite: r.get(9)?,
                breite: r.get(10)?,
                hoehe: r.get(11)?,
                status: r.get(12)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(Ergebnis { gesamt, treffer, dauer_ms: start.elapsed().as_millis() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R-SUCH-01
    #[test]
    fn begriffe_zerlegen() {
        assert_eq!(
            begriffe(r#"baum "rotes boot" -winter"#),
            vec![
                ("baum".to_string(), false),
                ("rotes boot".to_string(), false),
                ("winter".to_string(), true)
            ]
        );
    }

    /// R-SUCH-02
    #[test]
    fn datum_obergrenze() {
        assert!(datum_bis("2015").as_str() > "2015-12-31T23:59:59");
        assert!(datum_bis("2015-06").as_str() > "2015-06-30T23:59:59");
        assert!(datum_bis("2015-06").as_str() < "2015-07");
    }

    /// R-SUCH-03
    #[test]
    fn leerer_filter() {
        assert!(SuchFilter::default().ist_leer());
        // nur Leerzeichen bzw. nur „Duplikate“ gelten als leer
        let f = SuchFilter { freitext: "   ".into(), mit_duplikaten: true, ..Default::default() };
        assert!(f.ist_leer());
        for f in [
            SuchFilter { freitext: "a".into(), ..Default::default() },
            SuchFilter { name: "a".into(), ..Default::default() },
            SuchFilter { ordner: "a".into(), ..Default::default() },
            SuchFilter { datum_von: "2015".into(), ..Default::default() },
            SuchFilter { datum_bis: "2015".into(), ..Default::default() },
            SuchFilter { endungen: vec![".jpg".into()], ..Default::default() },
        ] {
            assert!(!f.ist_leer(), "{f:?}");
        }
    }

    /// R-SUCH-04
    #[test]
    fn begriffe_randfaelle() {
        assert!(begriffe("").is_empty());
        assert!(begriffe("   -  \"\" ").is_empty());
        // offene Anführungszeichen: Rest bis zum Ende ist die Phrase
        assert_eq!(begriffe("\"rotes boot"), vec![("rotes boot".to_string(), false)]);
        // Ausschluss einer Phrase
        assert_eq!(begriffe("-\"a b\""), vec![("a b".to_string(), true)]);
        // Umlaute und Sonderzeichen bleiben erhalten
        assert_eq!(begriffe("Großmütter c++"), vec![("Großmütter".to_string(), false), ("c++".to_string(), false)]);
    }

    /// R-SUCH-05
    #[test]
    fn fts_phrase_maskiert() {
        assert_eq!(fts_phrase("baum"), "\"baum\"");
        assert_eq!(fts_phrase("a\"b"), "\"a\"\"b\"");
        assert_eq!(fts_phrase("AND OR"), "\"AND OR\"");
    }

    /// R-SUCH-06
    #[test]
    fn where_status_und_parameter() {
        let f = SuchFilter { name: " x ".into(), ..Default::default() };
        let (w, p) = where_klausel(&f);
        assert!(w.starts_with("b.status = 'ok'"), "{w}");
        assert_eq!(p, vec![Value::Text("x".into())], "Werte werden getrimmt");

        let f = SuchFilter { name: "x".into(), mit_duplikaten: true, ..Default::default() };
        assert!(where_klausel(&f).0.starts_with("b.status IN ('ok','duplikat')"));
    }

    /// R-SUCH-07
    #[test]
    fn where_kurze_und_lange_begriffe() {
        let (w, p) = where_klausel(&SuchFilter { freitext: "ab".into(), ..Default::default() });
        assert!(!w.contains("MATCH") && w.contains("instr"), "{w}");
        assert_eq!(p.len(), 4);
        let (w, p) = where_klausel(&SuchFilter { freitext: "abc -xyz".into(), ..Default::default() });
        assert_eq!(w.matches("MATCH").count(), 2);
        assert!(w.contains("NOT (b.id IN"), "{w}");
        assert_eq!(p[0], Value::Text("\"abc\"".into()));
        assert_eq!(p.len(), 4);
    }

    /// R-SUCH-08
    #[test]
    fn where_endungen_und_datum() {
        let f = SuchFilter {
            endungen: vec![".JPG".into(), ".jpeg".into()],
            datum_von: "2015".into(),
            datum_bis: "2016-03".into(),
            ..Default::default()
        };
        let (w, p) = where_klausel(&f);
        assert!(w.contains("lower(b.endung) IN (?,?)"), "{w}");
        assert!(p.contains(&Value::Text(".jpg".into())), "Endungen kleingeschrieben");
        assert!(p.contains(&Value::Text("2015".into())));
        assert!(p.contains(&Value::Text("2016-03Z".into())));
    }

    fn facetten(paare: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        paare.iter().map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect())).collect()
    }

    /// R-SUCH-09
    #[test]
    fn facetten_zaehlen_als_kriterium() {
        let f = SuchFilter { facetten: facetten(&[("motiv", &["Tier"])]), ..Default::default() };
        assert!(!f.ist_leer());
        let f = SuchFilter { facetten: facetten(&[("motiv", &[])]), ..Default::default() };
        assert!(f.ist_leer(), "leere Auswahl zählt nicht");
    }

    /// R-SUCH-10
    #[test]
    fn where_facetten() {
        let f = SuchFilter {
            facetten: facetten(&[("motiv", &["Tier", "Porträt"]), ("merkmale", &["brille"])]),
            ..Default::default()
        };
        let (w, p) = where_klausel(&f);
        assert!(w.contains("b.motiv IN (?,?)"), "{w}");
        assert!(w.contains("typ = 'merkmal' AND wert IN (?)"), "{w}");
        assert_eq!(p, vec![Value::Text("brille".into()), Value::Text("Tier".into()), Value::Text("Porträt".into())]);
    }

    /// R-SUCH-11
    #[test]
    fn where_facetten_unbekannt_und_leer_ignoriert() {
        let f = SuchFilter {
            facetten: facetten(&[("motiv; DROP TABLE bilder", &["x"]), ("wetter", &[])]),
            ..Default::default()
        };
        let (w, p) = where_klausel(&f);
        assert_eq!(w, "b.status = 'ok'");
        assert!(p.is_empty());
    }
}
