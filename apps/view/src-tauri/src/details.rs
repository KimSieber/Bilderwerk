//! Detailbereich (v0.0.6): alle Angaben eines Bildes aus der Katalog-DB.
//! Bewusst generisch über die Spalten von `bilder` – die Oberfläche ordnet bekannte Felder,
//! unbekannte (neue Work-Spalten) zeigt sie unter „Weitere“.

use rusqlite::{params, types::ValueRef, Connection};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feld {
    /// Spaltenname, z. B. "kamera_modell"
    pub name: String,
    pub wert: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Verwandt {
    pub id: i64,
    pub pfad: String,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub id: i64,
    pub pfad: String,
    /// Spalten mit Wert in Tabellenreihenfolge (NULL, leerer Text und BLOBs entfallen)
    pub felder: Vec<Feld>,
    /// Einzelbegriffe je Typ (schlagwort, merkmal, stimmung, stichwort)
    pub begriffe: BTreeMap<String, Vec<String>>,
    /// Original und weitere Duplikate desselben Inhalts (ohne das Bild selbst)
    pub duplikate: Vec<Verwandt>,
}

fn json(v: ValueRef) -> Option<serde_json::Value> {
    match v {
        ValueRef::Null | ValueRef::Blob(_) => None,
        ValueRef::Integer(i) => Some(i.into()),
        ValueRef::Real(f) => serde_json::Number::from_f64(f).map(Into::into),
        ValueRef::Text(t) => {
            let s = String::from_utf8_lossy(t);
            (!s.trim().is_empty()).then(|| s.into_owned().into())
        }
    }
}

pub fn details(db: &Connection, id: i64) -> Result<Details, String> {
    let e = |e: rusqlite::Error| e.to_string();
    let mut st = db.prepare("SELECT * FROM bilder WHERE id = ?1").map_err(e)?;
    let namen: Vec<String> = st.column_names().map(|s| s.to_string()).collect();
    let felder: Vec<Feld> = st
        .query_row(params![id], |r| {
            Ok(namen
                .iter()
                .enumerate()
                .filter_map(|(i, n)| json(r.get_ref(i).ok()?).map(|wert| Feld { name: n.clone(), wert }))
                .collect())
        })
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => format!("Bild {id} nicht im Katalog."),
            x => x.to_string(),
        })?;
    let text = |n: &str| felder.iter().find(|f| f.name == n).and_then(|f| f.wert.as_str().map(String::from));
    let pfad = text("pfad").unwrap_or_default();

    let mut begriffe: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut st = db.prepare("SELECT typ, wert FROM begriffe WHERE bild_id = ?1 ORDER BY rowid").map_err(e)?;
    for z in st.query_map(params![id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(e)? {
        let (typ, wert) = z.map_err(e)?;
        let liste = begriffe.entry(typ).or_default();
        if !liste.contains(&wert) {
            liste.push(wert);
        }
    }

    let mut duplikate = Vec::new();
    if namen.iter().any(|n| n == "duplikat_von") {
        let original = felder
            .iter()
            .find(|f| f.name == "duplikat_von")
            .and_then(|f| f.wert.as_i64())
            .unwrap_or(id);
        let mut st = db
            .prepare("SELECT id, pfad, status FROM bilder WHERE (id = ?1 OR duplikat_von = ?1) AND id <> ?2 ORDER BY id")
            .map_err(e)?;
        for z in st
            .query_map(params![original, id], |r| Ok(Verwandt { id: r.get(0)?, pfad: r.get(1)?, status: r.get(2)? }))
            .map_err(e)?
        {
            duplikate.push(z.map_err(e)?);
        }
    }
    Ok(Details { id, pfad, felder, begriffe, duplikate })
}
