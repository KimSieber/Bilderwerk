//! Einstellungen pro Rechner (DB-Pfad, Cache-Grenze, Speicheranbieter, Dropbox-App-Schlüssel), gespeichert als JSON im App-Konfigurationsordner:
//! ~/Library/Application Support/de.sieber-bw.bilderwerk.view/einstellungen.json

use crate::speicher::zuordnung::Regel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// App-Schlüssel der mitgelieferten Dropbox-App „Bilderwerk View“ (öffentlich, kein Secret – Anmeldung über PKCE).
/// Gilt, solange in den Einstellungen kein eigener Schlüssel steht.
pub const STANDARD_DROPBOX_APP_KEY: &str = "sm36zb0y3puyff6";

/// Untergrenze für die Cache-Grenze im Dialog (MB)
pub const CACHE_MIN_MB: u64 = 100;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Einstellungen {
    pub db_pfad: Option<String>,
    /// Obergrenze des Vorschau-Caches in MB (fehlt → 2048)
    pub cache_grenze_mb: Option<u64>,
    /// Dropbox-App-Schlüssel: fehlt → mitgelieferter Schlüssel, leer → keine Dropbox-API (v0.3)
    pub dropbox_app_key: Option<String>,
    /// Regeln Pfad → Speicheranbieter: fehlt → `/` → `lokal` (v0.3, vorher view.local.json)
    pub anbieter: Option<Vec<Regel>>,
}

/// Einstellungen, wie die Oberfläche sie anzeigt – fehlende Werte durch Standards ersetzt.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ansicht {
    pub db_pfad: Option<String>,
    pub cache_grenze_mb: u64,
    pub anbieter: Vec<Regel>,
    /// wirksamer App-Schlüssel ("" = Dropbox aus)
    pub dropbox_app_key: String,
    pub standard_app_key: String,
}

impl Einstellungen {
    /// Obergrenze des Vorschau-Caches in Bytes
    pub fn cache_grenze_bytes(&self) -> u64 {
        self.cache_grenze_mb() * 1024 * 1024
    }

    pub fn cache_grenze_mb(&self) -> u64 {
        self.cache_grenze_mb.unwrap_or(crate::cache::STANDARD_GRENZE_MB)
    }

    /// Wirksamer Dropbox-App-Schlüssel; None = keine Dropbox-API
    pub fn app_key(&self) -> Option<&str> {
        match self.dropbox_app_key.as_deref() {
            None => Some(STANDARD_DROPBOX_APP_KEY),
            Some(k) if k.trim().is_empty() => None,
            Some(k) => Some(k.trim()),
        }
    }

    /// Wirksame Regeln; ohne Eintrag eine Regel `/` → `lokal`
    pub fn regeln(&self) -> Vec<Regel> {
        self.anbieter.clone().unwrap_or_else(standard_regeln)
    }

    pub fn ansicht(&self) -> Ansicht {
        Ansicht {
            db_pfad: self.db_pfad.clone(),
            cache_grenze_mb: self.cache_grenze_mb(),
            anbieter: self.regeln(),
            dropbox_app_key: self.app_key().unwrap_or("").to_string(),
            standard_app_key: STANDARD_DROPBOX_APP_KEY.to_string(),
        }
    }
}

/// Standard für neue Nutzer: alles lokal
pub fn standard_regeln() -> Vec<Regel> {
    vec![Regel { pfad: "/".into(), art: "lokal".into() }]
}

fn datei(dir: &Path) -> PathBuf {
    dir.join("einstellungen.json")
}

/// Lädt die Einstellungen. Fehlende oder kaputte Einstellungsdatei → Standardwerte.
/// Ungültige Regeln bleiben lesbar (Prüfung beim Aufbau des Verteilers), damit der DB-Pfad nicht verloren geht.
pub fn laden(dir: &Path) -> Einstellungen {
    std::fs::read_to_string(datei(dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn speichern(dir: &Path, e: &Einstellungen) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(e).map_err(|e| e.to_string())?;
    std::fs::write(datei(dir), json).map_err(|e| e.to_string())
}

/// Änderung aus dem Einstellungsdialog (DB-Pfad wird getrennt über `db_setzen` gewählt).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Aenderung {
    pub cache_grenze_mb: u64,
    pub anbieter: Vec<Regel>,
    /// "" = Dropbox aus; gleich dem Standard → Standard (nicht gespeichert)
    pub dropbox_app_key: String,
}

/// Prüft die Änderung, übernimmt sie in die Einstellungen und speichert. Bei Fehlern bleibt alles, wie es war.
pub fn aendern(dir: &Path, a: Aenderung) -> Result<Einstellungen, String> {
    let regeln = crate::speicher::zuordnung::regeln_pruefen(a.anbieter)?;
    if a.cache_grenze_mb < CACHE_MIN_MB {
        return Err(format!("Die Cache-Grenze muss mindestens {CACHE_MIN_MB} MB betragen."));
    }
    let key = a.dropbox_app_key.trim();
    if key.chars().any(|c| !c.is_ascii_alphanumeric()) {
        return Err("Der Dropbox-App-Schlüssel darf nur Buchstaben und Ziffern enthalten.".into());
    }
    let mut e = laden(dir);
    e.cache_grenze_mb = Some(a.cache_grenze_mb);
    e.anbieter = Some(regeln);
    e.dropbox_app_key = (key != STANDARD_DROPBOX_APP_KEY).then(|| key.to_string());
    speichern(dir, &e)?;
    Ok(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("view-einst-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn r(pfad: &str, art: &str) -> Regel {
        Regel { pfad: pfad.into(), art: art.into() }
    }

    fn aenderung(key: &str, anbieter: Vec<Regel>) -> Aenderung {
        Aenderung { cache_grenze_mb: 500, anbieter, dropbox_app_key: key.into() }
    }

    /// R-EINST-01
    #[test]
    fn speichern_und_laden() {
        let d = tmp("rund");
        let e = Einstellungen {
            db_pfad: Some("/x/Katalog.db".into()),
            cache_grenze_mb: Some(500),
            dropbox_app_key: Some("abc123".into()),
            anbieter: Some(vec![r("/D", "file-provider")]),
        };
        speichern(&d, &e).unwrap();
        assert_eq!(laden(&d), e);
        let json = std::fs::read_to_string(d.join("einstellungen.json")).unwrap();
        assert!(json.contains("\"dbPfad\"") && json.contains("\"dropboxAppKey\""), "camelCase im JSON: {json}");
    }

    /// R-EINST-02
    #[test]
    fn fehlende_oder_kaputte_datei() {
        let d = tmp("kaputt");
        assert_eq!(laden(&d), Einstellungen::default());
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("einstellungen.json"), "{kein json").unwrap();
        assert_eq!(laden(&d), Einstellungen::default());
    }

    /// R-EINST-03: Datei aus v0.2 (nur dbPfad/cacheGrenzeMb) bleibt lesbar, Standards greifen
    #[test]
    fn alte_datei_und_standards() {
        let e: Einstellungen = serde_json::from_str(r#"{"dbPfad":"/a.db","cacheGrenzeMb":null}"#).unwrap();
        assert_eq!(e.db_pfad.as_deref(), Some("/a.db"));
        assert_eq!(e.app_key(), Some(STANDARD_DROPBOX_APP_KEY));
        assert_eq!(e.regeln(), vec![r("/", "lokal")]);
        let a = e.ansicht();
        assert_eq!((a.cache_grenze_mb, a.dropbox_app_key.as_str()), (2048, STANDARD_DROPBOX_APP_KEY));
    }

    /// R-EINST-05
    #[test]
    fn cache_grenze() {
        assert_eq!(Einstellungen::default().cache_grenze_bytes(), 2048 * 1024 * 1024);
        let e = Einstellungen { cache_grenze_mb: Some(100), ..Default::default() };
        assert_eq!(e.cache_grenze_bytes(), 100 * 1024 * 1024);
    }

    /// R-EINST-04
    #[test]
    fn speichern_legt_ordner_an() {
        let d = tmp("ordner").join("a").join("b");
        speichern(&d, &Einstellungen::default()).unwrap();
        assert!(d.join("einstellungen.json").is_file());
    }

    /// R-EINST-06: App-Schlüssel – Standard, eigener, leer = aus
    #[test]
    fn app_key() {
        let mit = |k: Option<&str>| Einstellungen { dropbox_app_key: k.map(String::from), ..Default::default() };
        assert_eq!(mit(None).app_key(), Some(STANDARD_DROPBOX_APP_KEY));
        assert_eq!(mit(Some(" eigen1 ")).app_key(), Some("eigen1"));
        assert_eq!(mit(Some("")).app_key(), None);
        assert_eq!(mit(Some("  ")).ansicht().dropbox_app_key, "");
    }

    /// R-EINST-07: Änderung aus dem Dialog speichern, DB-Pfad bleibt
    #[test]
    fn aendern_speichert() {
        let d = tmp("aendern");
        speichern(&d, &Einstellungen { db_pfad: Some("/k.db".into()), ..Default::default() }).unwrap();
        let e = aendern(&d, aenderung("eigen1", vec![r("/D/", "dropbox-klassisch"), r("/", "lokal")])).unwrap();
        assert_eq!(e.db_pfad.as_deref(), Some("/k.db"));
        assert_eq!(e.anbieter, Some(vec![r("/D", "dropbox-klassisch"), r("/", "lokal")]), "Schrägstrich am Ende entfernt");
        assert_eq!(e.cache_grenze_mb, Some(500));
        assert_eq!(laden(&d), e, "gespeichert");
        // Standard-Schlüssel wird nicht festgeschrieben, leer = aus
        assert_eq!(aendern(&d, aenderung(STANDARD_DROPBOX_APP_KEY, vec![])).unwrap().dropbox_app_key, None);
        assert_eq!(aendern(&d, aenderung("", vec![])).unwrap().app_key(), None);
    }

    /// R-EINST-08: ungültige Änderung ändert nichts
    #[test]
    fn aendern_fehler() {
        let d = tmp("aendern-fehler");
        let vorher = Einstellungen { db_pfad: Some("/k.db".into()), cache_grenze_mb: Some(700), ..Default::default() };
        speichern(&d, &vorher).unwrap();
        assert!(aendern(&d, aenderung("x", vec![r("rel", "lokal")])).unwrap_err().contains("absolut"));
        assert!(aendern(&d, aenderung("x/y", vec![])).unwrap_err().contains("App-Schlüssel"));
        let mut a = aenderung("x", vec![]);
        a.cache_grenze_mb = 10;
        assert!(aendern(&d, a).unwrap_err().contains("100 MB"));
        assert_eq!(laden(&d), vorher);
    }
}
