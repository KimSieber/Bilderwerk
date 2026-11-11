//! Vorschau-Cache verwalten: Größe ermitteln, leeren, auf eine Obergrenze begrenzen.
//! „Zuletzt genutzt“ = Änderungszeit der Cache-Datei; bei jedem Cache-Treffer wird sie aufgefrischt
//! (`beruehren`). So verschwinden beim Begrenzen zuerst die Vorschauen, die lange keiner angesehen hat –
//! z. B. alle, die inzwischen aus der Katalog-DB kommen.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Standard-Obergrenze, wenn in den Einstellungen nichts steht.
pub const STANDARD_GRENZE_MB: u64 = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheInfo {
    pub bytes: u64,
    pub dateien: u64,
}

fn alle_dateien(dir: &Path, out: &mut Vec<(PathBuf, u64, SystemTime)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(m) = e.metadata() else { continue };
        if m.is_dir() {
            alle_dateien(&e.path(), out);
        } else if m.is_file() {
            out.push((e.path(), m.len(), m.modified().unwrap_or(SystemTime::UNIX_EPOCH)));
        }
    }
}

/// Größe und Anzahl der Dateien im Cache (inkl. Unterordner).
pub fn info(dir: &Path) -> CacheInfo {
    let mut v = Vec::new();
    alle_dateien(dir, &mut v);
    CacheInfo { bytes: v.iter().map(|x| x.1).sum(), dateien: v.len() as u64 }
}

/// Alle Dateien im Cache löschen (Ordner bleiben). Liefert den Stand danach.
pub fn leeren(dir: &Path) -> CacheInfo {
    let mut v = Vec::new();
    alle_dateien(dir, &mut v);
    for (p, _, _) in v {
        let _ = std::fs::remove_file(p);
    }
    info(dir)
}

/// Älteste Dateien löschen, bis der Cache höchstens `grenze` Bytes groß ist.
/// Liefert (gelöschte Dateien, freigegebene Bytes).
pub fn begrenzen(dir: &Path, grenze: u64) -> (u64, u64) {
    let mut v = Vec::new();
    alle_dateien(dir, &mut v);
    let mut summe: u64 = v.iter().map(|x| x.1).sum();
    if summe <= grenze {
        return (0, 0);
    }
    v.sort_by_key(|x| x.2);
    let (mut n, mut frei) = (0, 0);
    for (p, len, _) in v {
        if summe <= grenze {
            break;
        }
        if std::fs::remove_file(&p).is_ok() {
            summe -= len;
            frei += len;
            n += 1;
        }
    }
    (n, frei)
}

/// Cache-Treffer: Datei als „gerade genutzt“ markieren.
pub fn beruehren(datei: &Path) {
    if let Ok(f) = std::fs::File::options().write(true).open(datei) {
        let _ = f.set_modified(SystemTime::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("view-cache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("640")).unwrap();
        std::fs::create_dir_all(d.join("dropbox-tmp")).unwrap();
        d
    }

    /// Datei mit `kb` KB und Änderungszeit „vor `alter` Sekunden“.
    fn datei(p: &Path, kb: usize, alter: u64) {
        std::fs::write(p, vec![0u8; kb * 1024]).unwrap();
        let f = std::fs::File::options().write(true).open(p).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(alter)).unwrap();
    }

    /// R-CACHE-01
    #[test]
    fn info_zaehlt_rekursiv() {
        let d = tmp("info");
        datei(&d.join("640").join("a.jpg"), 10, 0);
        datei(&d.join("640").join("b.jpg"), 20, 0);
        datei(&d.join("dropbox-tmp").join("c.pdf"), 30, 0);
        assert_eq!(info(&d), CacheInfo { bytes: 60 * 1024, dateien: 3 });
        assert_eq!(info(&d.join("gibt-es-nicht")), CacheInfo { bytes: 0, dateien: 0 });
    }

    /// R-CACHE-02
    #[test]
    fn leeren_loescht_alles() {
        let d = tmp("leeren");
        datei(&d.join("640").join("a.jpg"), 10, 0);
        datei(&d.join("dropbox-tmp").join("c.pdf"), 30, 0);
        assert_eq!(leeren(&d), CacheInfo { bytes: 0, dateien: 0 });
        assert!(d.join("640").is_dir(), "Ordner bleiben");
    }

    /// R-CACHE-03
    #[test]
    fn begrenzen_aelteste_zuerst() {
        let d = tmp("grenze");
        let o = d.join("640");
        datei(&o.join("alt.jpg"), 40, 3000);
        datei(&o.join("mittel.jpg"), 40, 2000);
        datei(&o.join("neu.jpg"), 40, 10);
        assert_eq!(begrenzen(&d, 200 * 1024), (0, 0), "unter der Grenze: nichts");
        assert_eq!(begrenzen(&d, 90 * 1024), (1, 40 * 1024));
        assert!(!o.join("alt.jpg").exists() && o.join("mittel.jpg").exists() && o.join("neu.jpg").exists());
        assert_eq!(begrenzen(&d, 0), (2, 80 * 1024));
    }

    /// R-CACHE-04
    #[test]
    fn beruehren_schuetzt_vor_loeschen() {
        let d = tmp("beruehren");
        let o = d.join("640");
        datei(&o.join("alt-aber-genutzt.jpg"), 40, 5000);
        datei(&o.join("juenger.jpg"), 40, 100);
        beruehren(&o.join("alt-aber-genutzt.jpg"));
        begrenzen(&d, 50 * 1024);
        assert!(o.join("alt-aber-genutzt.jpg").exists());
        assert!(!o.join("juenger.jpg").exists());
        beruehren(&d.join("gibt-es-nicht.jpg")); // kein Absturz
    }
}
