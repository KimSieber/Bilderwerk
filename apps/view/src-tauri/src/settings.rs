//! Einstellungen pro Rechner (z. B. DB-Pfad), gespeichert als JSON im App-Konfigurationsordner:
//! ~/Library/Application Support/de.sieber-bw.bilderwerk.view/einstellungen.json

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Vorschlag beim ersten Start – kommt aus view.local.json (nicht im Repo), siehe build.rs.
const STANDARD_DB: Option<&str> = option_env!("PSV_STANDARD_DB");

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Einstellungen {
    pub db_pfad: Option<String>,
    /// Obergrenze des Vorschau-Caches in MB (fehlt → 2048)
    pub cache_grenze_mb: Option<u64>,
}

impl Einstellungen {
    /// Obergrenze des Vorschau-Caches in Bytes
    pub fn cache_grenze_bytes(&self) -> u64 {
        self.cache_grenze_mb.unwrap_or(crate::cache::STANDARD_GRENZE_MB) * 1024 * 1024
    }
}

fn datei(dir: &Path) -> PathBuf {
    dir.join("einstellungen.json")
}

/// Lädt die Einstellungen; ohne gespeicherten DB-Pfad gilt der Vorschlag aus view.local.json.
pub fn laden(dir: &Path) -> Einstellungen {
    laden_mit(dir, STANDARD_DB)
}

/// Wie `laden`, aber mit ausdrücklichem Vorschlag (testbar). Der Vorschlag gilt nur,
/// wenn die Datei existiert. Fehlende oder kaputte Einstellungsdatei → Standardwerte.
pub fn laden_mit(dir: &Path, vorschlag: Option<&str>) -> Einstellungen {
    let mut e: Einstellungen = std::fs::read_to_string(datei(dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if e.db_pfad.is_none() {
        if let Some(db) = vorschlag.filter(|p| Path::new(p).is_file()) {
            e.db_pfad = Some(db.to_string());
        }
    }
    e
}

pub fn speichern(dir: &Path, e: &Einstellungen) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(e).map_err(|e| e.to_string())?;
    std::fs::write(datei(dir), json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("view-einst-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// R-EINST-01
    #[test]
    fn speichern_und_laden() {
        let d = tmp("rund");
        let e = Einstellungen { db_pfad: Some("/x/Katalog.db".into()), cache_grenze_mb: Some(500) };
        speichern(&d, &e).unwrap();
        assert_eq!(laden_mit(&d, None), e);
        let json = std::fs::read_to_string(d.join("einstellungen.json")).unwrap();
        assert!(json.contains("\"dbPfad\""), "camelCase im JSON: {json}");
    }

    /// R-EINST-02
    #[test]
    fn fehlende_oder_kaputte_datei() {
        let d = tmp("kaputt");
        assert_eq!(laden_mit(&d, None), Einstellungen::default());
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("einstellungen.json"), "{kein json").unwrap();
        assert_eq!(laden_mit(&d, None), Einstellungen::default());
    }

    /// R-EINST-03
    #[test]
    fn vorschlag_nur_wenn_datei_existiert() {
        let d = tmp("vorschlag");
        std::fs::create_dir_all(&d).unwrap();
        let db = d.join("da.db");
        std::fs::write(&db, b"x").unwrap();
        let db = db.to_str().unwrap();
        assert_eq!(laden_mit(&d, Some(db)).db_pfad.as_deref(), Some(db));
        assert_eq!(laden_mit(&d, Some("/gibt/es/nicht.db")).db_pfad, None);
        // gespeicherter Pfad hat Vorrang vor dem Vorschlag
        speichern(&d, &Einstellungen { db_pfad: Some("/eigen.db".into()), ..Default::default() }).unwrap();
        assert_eq!(laden_mit(&d, Some(db)).db_pfad.as_deref(), Some("/eigen.db"));
    }

    /// R-EINST-05
    #[test]
    fn cache_grenze() {
        assert_eq!(Einstellungen::default().cache_grenze_bytes(), 2048 * 1024 * 1024);
        let e = Einstellungen { cache_grenze_mb: Some(100), ..Default::default() };
        assert_eq!(e.cache_grenze_bytes(), 100 * 1024 * 1024);
        // ältere einstellungen.json ohne das Feld bleibt lesbar
        let e: Einstellungen = serde_json::from_str(r#"{"dbPfad":"/a.db"}"#).unwrap();
        assert_eq!((e.db_pfad.as_deref(), e.cache_grenze_mb), (Some("/a.db"), None));
    }

    /// R-EINST-04
    #[test]
    fn speichern_legt_ordner_an() {
        let d = tmp("ordner").join("a").join("b");
        speichern(&d, &Einstellungen::default()).unwrap();
        assert!(d.join("einstellungen.json").is_file());
        let _ = laden(&d); // Standardweg mit eingebautem Vorschlag läuft durch
    }
}
