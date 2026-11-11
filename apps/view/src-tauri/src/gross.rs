//! Große Vorschau für den Detailbereich (v0.0.6) – nur auf ausdrücklichen Wunsch („Groß laden“).
//! Quelle ist das Original (`original_bereitstellen`, Dropbox „nur online“ → Download),
//! daraus entsteht per sips/PDFKit ein JPEG mit 2048 px. Ablage in einem eigenen kleinen Ordner
//! (`<AppCache>/gross`), NICHT im Vorschau-Cache: höchstens `MAX_DATEIEN`, beim Start geleert.

use crate::speicher::Speicheranbieter;
use crate::thumbs::{self, Vorschau};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Längste Kante der großen Vorschau
pub const GROESSE: u32 = 2048;
/// So viele große Vorschauen bleiben liegen (Zurückblättern ohne Neuberechnung)
pub const MAX_DATEIEN: usize = 30;

/// Große Vorschau holen (vorhanden → sofort, sonst Original bereitstellen und umwandeln).
pub fn holen(anbieter: &dyn Speicheranbieter, ordner: &Path, pfad: &str) -> Vorschau {
    let datei = match anbieter.original_bereitstellen(pfad) {
        Ok(d) => d,
        Err(m) if m.contains("nicht gefunden") => return Vorschau::Fehlt,
        Err(meldung) => return Vorschau::Fehler { meldung },
    };
    let meta = match std::fs::metadata(&datei) {
        Ok(m) => m,
        Err(_) => return Vorschau::Fehlt,
    };
    let ziel = thumbs::cache_ziel(ordner, pfad, GROESSE, meta.mtime(), meta.len());
    if ziel.is_file() {
        crate::cache::beruehren(&ziel);
        return Vorschau::Bild { datei: ziel.to_string_lossy().into() };
    }
    let seite = thumbs::quelle(pfad).1;
    let v = thumbs::erzeugen(&datei, seite, GROESSE, &ziel);
    begrenzen(ordner, MAX_DATEIEN);
    v
}

/// Alle JPEGs unter `ordner` (rekursiv)
fn dateien(ordner: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(ordner) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                v.extend(dateien(&p));
            } else if p.extension().is_some_and(|x| x == "jpg") {
                v.push(p);
            }
        }
    }
    v
}

/// Nur die `max` zuletzt benutzten Dateien behalten. Liefert die Zahl der gelöschten.
pub fn begrenzen(ordner: &Path, max: usize) -> usize {
    let mut v: Vec<(std::time::SystemTime, PathBuf)> = dateien(ordner)
        .into_iter()
        .map(|p| (std::fs::metadata(&p).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH), p))
        .collect();
    if v.len() <= max {
        return 0;
    }
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.drain(max..).filter(|(_, p)| std::fs::remove_file(p).is_ok()).count()
}

/// Beim Start: Ordner leeren (große Vorschauen sind bewusst nur für die laufende Sitzung).
pub fn leeren(ordner: &Path) {
    let _ = std::fs::remove_dir_all(ordner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::speicher::Lokal;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("view-gross-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// R-GROSS-01
    #[test]
    fn fehlende_datei() {
        let d = tmp("fehlt");
        assert_eq!(holen(&Lokal::new(&d), &d.join("g"), "/gibt/es/nicht.jpg"), Vorschau::Fehlt);
    }

    /// R-GROSS-02
    #[test]
    fn vorhandene_grosse_vorschau_ohne_werkzeug() {
        let d = tmp("vorhanden");
        let bild = d.join("a.jpg");
        std::fs::write(&bild, b"original").unwrap();
        let ordner = d.join("gross");
        let m = std::fs::metadata(&bild).unwrap();
        let pfad = bild.to_str().unwrap();
        let ziel = thumbs::cache_ziel(&ordner, pfad, GROESSE, m.mtime(), m.len());
        std::fs::create_dir_all(ziel.parent().unwrap()).unwrap();
        std::fs::write(&ziel, b"jpeg").unwrap();
        assert_eq!(holen(&Lokal::new(&d), &ordner, pfad), Vorschau::Bild { datei: ziel.to_string_lossy().into() });
        assert!(ziel.starts_with(&ordner), "liegt im eigenen Ordner, nicht im Vorschau-Cache");
    }

    /// R-GROSS-03
    #[test]
    fn nur_online_meldet_fehler() {
        let d = tmp("online");
        let null = d.join("platzhalter.jpg");
        std::fs::write(&null, b"").unwrap();
        let v = holen(&crate::speicher::FileProvider::new(&d), &d.join("g"), null.to_str().unwrap());
        assert!(matches!(v, Vorschau::Fehler { ref meldung } if meldung.contains("nur online")), "{v:?}");
    }

    /// R-GROSS-04
    #[test]
    fn begrenzen_behaelt_die_neuesten() {
        let d = tmp("grenze");
        let sub = d.join("2048");
        std::fs::create_dir_all(&sub).unwrap();
        for i in 0..5 {
            std::fs::write(sub.join(format!("{i}.jpg")), b"x").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        std::fs::write(sub.join("anderes.txt"), b"x").unwrap();
        assert_eq!(begrenzen(&d, 3), 2);
        let mut rest: Vec<String> = std::fs::read_dir(&sub).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into()).collect();
        rest.sort();
        assert_eq!(rest, vec!["2.jpg", "3.jpg", "4.jpg", "anderes.txt"]);
        assert_eq!(begrenzen(&d, 3), 0);
    }

    /// R-GROSS-05
    #[test]
    fn leeren_entfernt_ordner() {
        let d = tmp("leeren");
        let g = d.join("gross");
        std::fs::create_dir_all(g.join("2048")).unwrap();
        leeren(&g);
        assert!(!g.exists());
        leeren(&g); // fehlt schon → kein Fehler
    }
}
