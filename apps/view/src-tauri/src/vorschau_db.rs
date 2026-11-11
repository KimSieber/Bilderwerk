//! Vorschauen aus der Katalog-DB (Tabelle `vorschauen`, Katalog-Schema ≥ 4).
//! Schlüssel ist der Datei-Hash (`bilder.hash`, PDF-Seite `<hash>#<seite>`) – Duplikate teilen sich eine Vorschau.
//! Fehlt die Tabelle (ältere Katalog-DB), liefert alles `None`: View nutzt dann Cache und Anbieter.

use rusqlite::{Connection, OptionalExtension};

/// Hat die DB die Tabelle `vorschauen`?
pub fn hat_tabelle(db: &Connection) -> bool {
    db.query_row("SELECT 1 FROM sqlite_master WHERE type='table' AND name='vorschauen'", [], |_| Ok(()))
        .optional()
        .ok()
        .flatten()
        .is_some()
}

/// Hash der Vorschau zu einem Katalogpfad – nur, wenn es die Vorschau auch gibt.
pub fn schluessel_fuer(db: &Connection, pfad: &str) -> Option<String> {
    db.query_row(
        "SELECT v.hash FROM bilder b JOIN vorschauen v ON v.hash = b.hash WHERE b.pfad = ?1 LIMIT 1",
        [pfad],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

/// JPEG-Daten einer Vorschau.
pub fn jpeg(db: &Connection, hash: &str) -> Option<Vec<u8>> {
    db.query_row("SELECT jpeg FROM vorschauen WHERE hash = ?1", [hash], |r| r.get(0))
        .optional()
        .ok()
        .flatten()
}

/// Anzahl der Vorschauen (None ohne Tabelle) – für die Anzeige im DB-Menü.
pub fn anzahl(db: &Connection) -> Option<i64> {
    if !hat_tabelle(db) {
        return None;
    }
    db.query_row("SELECT count(*) FROM vorschauen", [], |r| r.get(0)).ok()
}

/// Pfad einer Anfrage `vorschau://localhost/<kodierter hash>` → Hash.
pub fn hash_aus_uri(pfad: &str) -> String {
    let s = pfad.trim_start_matches('/');
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(x) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(x);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    /// R-VDB-01
    #[test]
    fn hash_aus_uri() {
        assert_eq!(super::hash_aus_uri("/abc123"), "abc123");
        assert_eq!(super::hash_aus_uri("/abc%23seite"), "abc#seite");
        assert_eq!(super::hash_aus_uri("/h%C3%A4"), "hä");
        assert_eq!(super::hash_aus_uri("/kaputt%2"), "kaputt%2");
        assert_eq!(super::hash_aus_uri("/x%zz"), "x%zz");
    }
}
