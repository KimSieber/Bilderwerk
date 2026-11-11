//! Aktionen am Original (v0.0.6): Öffnen, Im Finder zeigen, Sichern unter …
//! Das Original kommt immer über `Speicheranbieter::original_bereitstellen`
//! (lokal direkt, Dropbox „nur online“ → Download in tmp). Teilen: `teilen.rs` (nur macOS).

use std::path::{Path, PathBuf};

/// Befehl zum Öffnen in der Standard-App (`open <datei>`)
pub fn oeffnen_befehl(datei: &Path) -> (&'static str, Vec<String>) {
    ("/usr/bin/open", vec![datei.to_string_lossy().into()])
}

/// Befehl „Im Finder zeigen“ (`open -R <datei>` – markiert die Datei im Finder)
pub fn finder_befehl(datei: &Path) -> (&'static str, Vec<String>) {
    ("/usr/bin/open", vec!["-R".into(), datei.to_string_lossy().into()])
}

pub fn ausfuehren((prog, args): (&str, Vec<String>)) -> Result<(), String> {
    let s = std::process::Command::new(prog).args(&args).status().map_err(|e| e.to_string())?;
    if s.success() { Ok(()) } else { Err(format!("{prog} meldet Fehler ({s})")) }
}

/// Vorschlag für „Sichern unter …“: Dateiname des Originals (bei PDF-Seiten die PDF-Datei)
pub fn dateiname_vorschlag(pfad: &str) -> String {
    Path::new(crate::thumbs::quelle(pfad).0)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Bild".into())
}

/// Kopie des Originals an `ziel`; liefert das Ziel. Quelle und Ziel gleich → nichts tun.
pub fn sichern(quelle: &Path, ziel: &Path) -> Result<PathBuf, String> {
    if quelle == ziel {
        return Ok(ziel.to_path_buf());
    }
    std::fs::copy(quelle, ziel).map_err(|e| format!("Sichern fehlgeschlagen: {e}"))?;
    Ok(ziel.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R-AKT-01
    #[test]
    fn befehle() {
        let d = Path::new("/a b/c.jpg");
        assert_eq!(oeffnen_befehl(d), ("/usr/bin/open", vec!["/a b/c.jpg".to_string()]));
        assert_eq!(finder_befehl(d), ("/usr/bin/open", vec!["-R".to_string(), "/a b/c.jpg".to_string()]));
    }

    /// R-AKT-02
    #[test]
    fn vorschlag() {
        assert_eq!(dateiname_vorschlag("/A/IMG_1.heic"), "IMG_1.heic");
        assert_eq!(dateiname_vorschlag("/A/Album.pdf#seite=3"), "Album.pdf");
        assert_eq!(dateiname_vorschlag("/"), "Bild");
    }

    /// R-AKT-03
    #[test]
    fn sichern_kopiert() {
        let d = std::env::temp_dir().join(format!("view-akt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let q = d.join("q.jpg");
        std::fs::write(&q, b"inhalt").unwrap();
        let z = d.join("kopie.jpg");
        assert_eq!(sichern(&q, &z).unwrap(), z);
        assert_eq!(std::fs::read(&z).unwrap(), b"inhalt");
        assert_eq!(std::fs::read(&q).unwrap(), b"inhalt", "Original bleibt");
        assert_eq!(sichern(&q, &q).unwrap(), q, "gleiche Datei → nichts tun");
        assert!(sichern(&d.join("fehlt.jpg"), &z).unwrap_err().contains("fehlgeschlagen"));
    }

    /// R-AKT-04
    #[test]
    fn ausfuehren_meldet_fehler() {
        assert!(ausfuehren(("/gibt/es/nicht", vec![])).is_err());
        assert!(ausfuehren(("/bin/sh", vec!["-c".into(), "exit 3".into()])).unwrap_err().contains("Fehler"));
        assert!(ausfuehren(("/bin/sh", vec!["-c".into(), "exit 0".into()])).is_ok());
    }
}
