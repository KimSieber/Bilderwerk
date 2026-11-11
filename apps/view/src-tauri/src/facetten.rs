//! Stufe 3: Facetten mit Anzahl (Motiv, Bildart, Farbgebung, Ausrichtung, Jahreszeit, Wetter,
//! Merkmale, Schlagworte). Die Felder sind fest hinterlegt – nur sie gelangen in SQL.
//!
//! Zählung „disjunktiv“: Jede Facette zählt mit allen Kriterien außer ihrer eigenen Auswahl.
//! So bleiben Alternativen innerhalb einer Facette sichtbar (Porträt gewählt → Landschaft zeigt weiter
//! ihre Anzahl), während die übrigen Facetten auf die Auswahl eingeschränkt werden.

use crate::search::{where_klausel, SuchFilter};
use rusqlite::{params_from_iter, types::Value, Connection};
use serde::Serialize;

/// Woher die Werte einer Facette kommen.
#[derive(Debug, Clone, Copy)]
pub enum Quelle {
    /// Spalte in `bilder` (ein Wert je Bild)
    Spalte(&'static str),
    /// Einträge in `begriffe` mit diesem `typ` (mehrere Werte je Bild, kleingeschrieben)
    Begriff(&'static str),
}

pub struct Definition {
    pub feld: &'static str,
    pub titel: &'static str,
    pub quelle: Quelle,
}

/// Reihenfolge = Anzeige-Reihenfolge.
pub const FACETTEN: [Definition; 8] = [
    Definition { feld: "motiv", titel: "Motiv", quelle: Quelle::Spalte("motiv") },
    Definition { feld: "bildart", titel: "Bildart", quelle: Quelle::Spalte("bildart") },
    Definition { feld: "farbgebung", titel: "Farbgebung", quelle: Quelle::Spalte("farbgebung") },
    Definition { feld: "ausrichtung", titel: "Ausrichtung", quelle: Quelle::Spalte("ausrichtung") },
    Definition { feld: "jahreszeit", titel: "Jahreszeit", quelle: Quelle::Spalte("jahreszeit") },
    Definition { feld: "wetter", titel: "Wetter", quelle: Quelle::Spalte("wetter") },
    Definition { feld: "merkmale", titel: "Merkmale", quelle: Quelle::Begriff("merkmal") },
    Definition { feld: "schlagworte", titel: "Schlagworte", quelle: Quelle::Begriff("schlagwort") },
];

/// Höchstzahl angezeigter Werte bei Begriffs-Facetten (Schlagworte haben Tausende Werte).
pub const MAX_BEGRIFFE: usize = 40;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FacettenWert {
    pub wert: String,
    pub anzahl: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Facette {
    pub feld: String,
    pub titel: String,
    pub werte: Vec<FacettenWert>,
}

fn definition(feld: &str) -> Option<&'static Definition> {
    FACETTEN.iter().find(|d| d.feld == feld)
}

/// SQL-Bedingung für eine Auswahl von `n` Werten; None bei unbekanntem Feld oder n = 0.
pub fn bedingung(feld: &str, n: usize) -> Option<String> {
    if n == 0 {
        return None;
    }
    let ph = vec!["?"; n].join(",");
    Some(match definition(feld)?.quelle {
        Quelle::Spalte(s) => format!("b.{s} IN ({ph})"),
        Quelle::Begriff(t) => format!("b.id IN (SELECT bild_id FROM begriffe WHERE typ = '{t}' AND wert IN ({ph}))"),
    })
}

/// Spalten der Tabelle `bilder` (ältere Kataloge haben nicht alle Kategorien).
fn spalten(db: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut st = db.prepare("SELECT name FROM pragma_table_info('bilder')")?;
    let v = st.query_map([], |r| r.get(0))?.collect();
    v
}

/// false, wenn eine Auswahl eine Spalte betrifft, die diese DB nicht hat.
pub fn auswertbar(db: &Connection, f: &SuchFilter) -> rusqlite::Result<bool> {
    let gewaehlte: Vec<&str> = f
        .facetten
        .iter()
        .filter(|(_, w)| !w.is_empty())
        .filter_map(|(k, _)| match definition(k)?.quelle {
            Quelle::Spalte(s) => Some(s),
            Quelle::Begriff(_) => None,
        })
        .collect();
    if gewaehlte.is_empty() {
        return Ok(true);
    }
    let vorhanden = spalten(db)?;
    Ok(gewaehlte.iter().all(|s| vorhanden.iter().any(|v| v == s)))
}

/// Alle Facetten mit Anzahl für die aktuellen Kriterien. Gewählte Werte bleiben auch ohne Treffer sichtbar.
pub fn facetten(db: &Connection, f: &SuchFilter) -> rusqlite::Result<Vec<Facette>> {
    let vorhanden = spalten(db)?;
    let mut out = Vec::new();
    for d in &FACETTEN {
        if let Quelle::Spalte(s) = d.quelle {
            if !vorhanden.iter().any(|v| v == s) {
                continue;
            }
        }
        // eigene Auswahl herausnehmen (disjunktive Zählung)
        let mut ohne = f.clone();
        let gewaehlt = ohne.facetten.remove(d.feld).unwrap_or_default();
        if !auswertbar(db, &ohne)? {
            ohne.facetten.retain(|k, _| definition(k).is_some_and(|d| matches!(d.quelle, Quelle::Begriff(_))));
        }
        let (wo, p) = where_klausel(&ohne);
        let sql = match d.quelle {
            Quelle::Spalte(s) => format!(
                "SELECT b.{s}, count(*) FROM bilder b WHERE {wo} AND coalesce(b.{s},'') <> '' \
                 GROUP BY b.{s} ORDER BY 2 DESC, 1"
            ),
            Quelle::Begriff(t) => format!(
                "SELECT g.wert, count(DISTINCT g.bild_id) FROM begriffe g JOIN bilder b ON b.id = g.bild_id \
                 WHERE g.typ = '{t}' AND coalesce(g.wert,'') <> '' AND {wo} \
                 GROUP BY g.wert ORDER BY 2 DESC, 1 LIMIT {MAX_BEGRIFFE}"
            ),
        };
        let mut st = db.prepare(&sql)?;
        let mut werte = st
            .query_map(params_from_iter(p.iter()), |r| Ok(FacettenWert { wert: r.get(0)?, anzahl: r.get(1)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for g in gewaehlt {
            if !werte.iter().any(|w| w.wert == g) {
                let anzahl = anzahl_fuer(db, d, &wo, &p, &g)?;
                werte.push(FacettenWert { wert: g, anzahl });
            }
        }
        out.push(Facette { feld: d.feld.into(), titel: d.titel.into(), werte });
    }
    Ok(out)
}

/// Anzahl für einen einzelnen (gewählten) Wert – z. B. ein Schlagwort jenseits der Top-Liste.
fn anzahl_fuer(db: &Connection, d: &Definition, wo: &str, p: &[Value], wert: &str) -> rusqlite::Result<i64> {
    let b = bedingung(d.feld, 1).expect("bekanntes Feld");
    let mut q: Vec<Value> = p.to_vec();
    q.push(Value::Text(wert.into()));
    db.query_row(&format!("SELECT count(*) FROM bilder b WHERE {wo} AND {b}"), params_from_iter(q.iter()), |r| r.get(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R-FAC-01
    #[test]
    fn bedingungen() {
        assert_eq!(bedingung("motiv", 2).unwrap(), "b.motiv IN (?,?)");
        assert!(bedingung("schlagworte", 1).unwrap().contains("typ = 'schlagwort' AND wert IN (?)"));
        assert!(bedingung("motiv", 0).is_none());
        assert!(bedingung("b.id); DROP TABLE x; --", 1).is_none());
    }

    /// R-FAC-02
    #[test]
    fn felder_eindeutig_und_vollstaendig() {
        let mut f: Vec<_> = FACETTEN.iter().map(|d| d.feld).collect();
        f.sort();
        f.dedup();
        assert_eq!(f.len(), FACETTEN.len());
        for d in &FACETTEN {
            assert!(d.feld.chars().all(|c| c.is_ascii_lowercase()), "{}", d.feld);
            assert!(!d.titel.is_empty());
        }
    }
}
