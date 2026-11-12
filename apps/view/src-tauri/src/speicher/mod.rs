//! Anbieter-Schnittstelle: kapselt, wie View an Vorschau und Original einer Datei kommt.
//! Welcher Anbieter für einen Pfad gilt, entscheiden die Regeln in den Einstellungen (seit v0.3, vorher view.local.json).
//!
//! | Anbieter        | Wann                                        | Nur-online-Datei                          |
//! |-----------------|---------------------------------------------|-------------------------------------------|
//! | `lokal`         | Standard                                    | Wolke, nichts wird angefasst              |
//! | `file-provider` | iCloud, OneDrive, Google Drive, neue Dropbox | Lesen lädt still nach → wie lokal         |
//! | `dropbox-klassisch` (Schritt 4) | Kims Dropbox                | Vorschau/Original über die Dropbox-API    |

pub mod dropbox;
pub mod zuordnung;
pub use dropbox::{DropboxKlassisch, Konto};
pub use zuordnung::{Regel, Verteiler};

use crate::thumbs::{self, quelle, Vorschau};
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Verteiler und Dropbox-Konto, wie sie aus den Einstellungen folgen. Im AppState austauschbar (v0.3):
/// Änderungen im Einstellungsdialog wirken ohne Neustart.
#[derive(Clone)]
pub struct Speicher {
    pub verteiler: Arc<Verteiler>,
    /// None = kein App-Schlüssel → Dropbox-Ordner verhalten sich wie `lokal`
    pub konto: Option<Arc<Konto>>,
}

impl Speicher {
    /// Baut Verteiler und Konto aus den Einstellungen. `alt`: bisheriger Stand – bleibt der App-Schlüssel gleich,
    /// wird das Konto weiterverwendet (Anmeldung bleibt); wechselt er, wird das alte Konto abgemeldet, denn
    /// sein Token gehört zur alten Dropbox-App. `neues_konto` legt ein Konto für einen Schlüssel an.
    /// Zweiter Rückgabewert: Fehler in den Regeln (dann alles `lokal`).
    pub fn aufbauen(
        e: &crate::settings::Einstellungen,
        cache: &Path,
        alt: Option<&Speicher>,
        neues_konto: impl FnOnce(&str) -> Arc<Konto>,
    ) -> (Self, Option<String>) {
        let key = e.app_key();
        let alt_konto = alt.and_then(|s| s.konto.clone());
        let konto = match (alt_konto, key) {
            (Some(k), Some(neu)) if k.app_key() == neu => Some(k),
            (alt_konto, neu) => {
                if let Some(k) = alt_konto {
                    if let Err(f) = k.abmelden() {
                        eprintln!("View: Abmelden der bisherigen Dropbox-App fehlgeschlagen ({f})");
                    }
                }
                neu.map(neues_konto)
            }
        };
        let (verteiler, fehler) = Verteiler::aus_regeln(e.regeln(), cache, konto.clone());
        (Self { verteiler: Arc::new(verteiler), konto }, fehler)
    }
}

/// Wo liegt der Inhalt einer Datei?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zustand {
    /// vollständig auf diesem Rechner
    Lokal,
    /// nur beim Cloud-Anbieter (Platzhalter bzw. „dataless“)
    NurOnline,
    /// existiert nicht (mehr)
    Fehlt,
}

pub trait Speicheranbieter: Send + Sync {
    /// Kurzname wie in den Regeln (`lokal`, `file-provider`, …)
    fn art(&self) -> &'static str;
    /// Zustand der Datei hinter `pfad` (PDF-Seiten `…#seite=N` → die PDF-Datei)
    fn zustand(&self, pfad: &str) -> Zustand;
    /// Vorschaubild mit längster Kante `groesse` (Cache oder neu erzeugt)
    fn vorschau(&self, pfad: &str, groesse: u32) -> Vorschau;
    /// Pfad einer lesbaren Originaldatei (für Stufe 4: Öffnen, Sichern unter, Teilen)
    fn original_bereitstellen(&self, pfad: &str) -> Result<PathBuf, String>;
}

/// Datei hinter einem Katalogpfad (ohne `#seite=N`).
pub fn quelle_datei(pfad: &str) -> &str {
    quelle(pfad).0
}

fn zustand_nach(pfad: &str) -> (Zustand, Option<Metadata>) {
    let (datei, _) = quelle(pfad);
    match std::fs::metadata(datei) {
        Err(_) => (Zustand::Fehlt, None),
        Ok(m) if thumbs::ist_nur_online(&m) => (Zustand::NurOnline, Some(m)),
        Ok(m) => (Zustand::Lokal, Some(m)),
    }
}

fn fehlt(pfad: &str) -> String {
    format!("Datei nicht gefunden: {}", quelle(pfad).0)
}

/// Dateien auf der eigenen Platte (und alles ohne eigene Zuordnung).
pub struct Lokal {
    cache: PathBuf,
}

impl Lokal {
    pub fn new(cache: &Path) -> Self {
        Self { cache: cache.to_path_buf() }
    }
}

impl Speicheranbieter for Lokal {
    fn art(&self) -> &'static str {
        "lokal"
    }
    fn zustand(&self, pfad: &str) -> Zustand {
        zustand_nach(pfad).0
    }
    fn vorschau(&self, pfad: &str, groesse: u32) -> Vorschau {
        thumbs::holen(&self.cache, pfad, groesse)
    }
    fn original_bereitstellen(&self, pfad: &str) -> Result<PathBuf, String> {
        match zustand_nach(pfad).0 {
            Zustand::Lokal => Ok(PathBuf::from(quelle(pfad).0)),
            Zustand::NurOnline => Err(format!("Datei liegt nur online: {}", quelle(pfad).0)),
            Zustand::Fehlt => Err(fehlt(pfad)),
        }
    }
}

/// macOS File Provider (iCloud Drive, OneDrive, Google Drive, neue Dropbox):
/// „dataless“-Dateien werden beim Lesen still heruntergeladen – dafür sorgt
/// `thumbs::download_beim_lesen_erlauben()` beim Programmstart.
pub struct FileProvider {
    cache: PathBuf,
}

impl FileProvider {
    pub fn new(cache: &Path) -> Self {
        Self { cache: cache.to_path_buf() }
    }
}

/// 0 Byte ist auch beim File Provider nie lesbar (echte Platzhalter melden ihre volle Größe).
fn leer(m: &Metadata) -> bool {
    m.len() == 0
}

impl Speicheranbieter for FileProvider {
    fn art(&self) -> &'static str {
        "file-provider"
    }
    fn zustand(&self, pfad: &str) -> Zustand {
        zustand_nach(pfad).0
    }
    fn vorschau(&self, pfad: &str, groesse: u32) -> Vorschau {
        thumbs::holen_mit(&self.cache, pfad, groesse, leer)
    }
    fn original_bereitstellen(&self, pfad: &str) -> Result<PathBuf, String> {
        let datei = PathBuf::from(quelle(pfad).0);
        match zustand_nach(pfad) {
            (Zustand::Fehlt, _) => Err(fehlt(pfad)),
            (_, Some(m)) if leer(&m) => Err(format!("Datei liegt nur online: {}", datei.display())),
            (Zustand::NurOnline, _) => {
                // ein Byte lesen stößt den Download an
                use std::io::Read;
                std::fs::File::open(&datei)
                    .and_then(|mut f| f.read(&mut [0u8; 1]))
                    .map_err(|e| format!("Datei lässt sich nicht laden: {e}"))?;
                Ok(datei)
            }
            _ => Ok(datei),
        }
    }
}

#[cfg(test)]
mod tests;
