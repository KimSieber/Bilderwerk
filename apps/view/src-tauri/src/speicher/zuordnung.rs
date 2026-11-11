//! Zuordnung Pfad → Anbieter nach Pfad-Anfang (längster passender Ordner gewinnt).
//! Regeln stehen in view.local.json unter "anbieter" (build.rs reicht sie als PSV_ANBIETER durch):
//!
//! ```json
//! "anbieter": [
//!   { "pfad": "/Users/kim/Dropbox", "art": "dropbox-klassisch" },
//!   { "pfad": "/", "art": "lokal" }
//! ]
//! ```
//! Ohne passende Regel gilt `lokal`. Anbieterwechsel = Zeile ändern, kein Code.

use super::{quelle_datei, DropboxKlassisch, FileProvider, Konto, Lokal, Speicheranbieter};
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;

pub const ARTEN: [&str; 3] = ["lokal", "file-provider", "dropbox-klassisch"];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Regel {
    pub pfad: String,
    pub art: String,
}

/// Liest und prüft die Regeln (JSON-Liste).
pub fn regeln_lesen(json: &str) -> Result<Vec<Regel>, String> {
    let mut v: Vec<Regel> =
        serde_json::from_str(json).map_err(|e| format!("view.local.json – \"anbieter\" ungültig: {e}"))?;
    for r in &mut v {
        if !ARTEN.contains(&r.art.as_str()) {
            return Err(format!("view.local.json – unbekannte Anbieter-Art \"{}\" (erlaubt: {})", r.art, ARTEN.join(", ")));
        }
        if !r.pfad.starts_with('/') {
            return Err(format!("view.local.json – Anbieter-Pfad muss absolut sein: \"{}\"", r.pfad));
        }
        while r.pfad.len() > 1 && r.pfad.ends_with('/') {
            r.pfad.pop();
        }
    }
    Ok(v)
}

/// Liegt `pfad` in `ordner` (oder ist er es)? Nur ganze Ordnernamen zählen.
fn liegt_in(ordner: &str, pfad: &str) -> bool {
    ordner == "/" || pfad == ordner || pfad.strip_prefix(ordner).is_some_and(|rest| rest.starts_with('/'))
}

/// Art des Anbieters für `pfad` (PDF-Seiten zählen als ihre Datei).
pub fn art_fuer<'a>(regeln: &'a [Regel], pfad: &str) -> &'a str {
    let datei = quelle_datei(pfad);
    regeln
        .iter()
        .filter(|r| liegt_in(&r.pfad, datei))
        .max_by_key(|r| r.pfad.len())
        .map(|r| r.art.as_str())
        .unwrap_or("lokal")
}

/// Lokaler Pfad → Pfad in der Dropbox (relativ zur Dropbox-Wurzel, mit führendem "/").
/// Dropbox-Pfade sind nicht case-sensitiv; die Schreibweise bleibt wie im Katalog.
pub fn dropbox_pfad(wurzel: &str, pfad: &str) -> Option<String> {
    let datei = quelle_datei(pfad);
    let rest = datei.strip_prefix(wurzel.trim_end_matches('/'))?;
    (rest.starts_with('/') && rest.len() > 1).then(|| rest.to_string())
}

/// Hält je Art einen Anbieter und verteilt Pfade nach den Regeln.
pub struct Verteiler {
    regeln: Vec<Regel>,
    lokal: Arc<dyn Speicheranbieter>,
    file_provider: Arc<dyn Speicheranbieter>,
    /// je Dropbox-Regel (Wurzel) ein Anbieter; alle teilen dasselbe Konto
    dropbox: Vec<(String, Arc<dyn Speicheranbieter>)>,
}

impl Verteiler {
    /// `konto` = None (kein App-Schlüssel): Dropbox-Pfade verhalten sich wie `lokal` (Wolke).
    pub fn neu(regeln: &[Regel], cache: &Path, konto: Option<Arc<Konto>>) -> Self {
        let dropbox = regeln
            .iter()
            .filter(|r| r.art == "dropbox-klassisch")
            .map(|r| {
                let a: Arc<dyn Speicheranbieter> = Arc::new(DropboxKlassisch::new(&r.pfad, cache, konto.clone()));
                (r.pfad.clone(), a)
            })
            .collect();
        Self {
            regeln: regeln.to_vec(),
            lokal: Arc::new(Lokal::new(cache)),
            file_provider: Arc::new(FileProvider::new(cache)),
            dropbox,
        }
    }

    /// Alles `lokal`.
    pub fn standard(cache: &Path) -> Self {
        Self::neu(&[], cache, None)
    }

    /// Aus dem JSON in view.local.json; bei Fehlern alles `lokal` und Fehlertext zurück.
    pub fn aus_konfiguration(json: Option<&str>, cache: &Path, konto: Option<Arc<Konto>>) -> (Self, Option<String>) {
        match json.map(regeln_lesen) {
            None => (Self::standard(cache), None),
            Some(Ok(r)) => (Self::neu(&r, cache, konto), None),
            Some(Err(e)) => (Self::standard(cache), Some(e)),
        }
    }

    pub fn fuer(&self, pfad: &str) -> &dyn Speicheranbieter {
        let datei = quelle_datei(pfad);
        match art_fuer(&self.regeln, pfad) {
            "file-provider" => self.file_provider.as_ref(),
            "dropbox-klassisch" => self
                .dropbox
                .iter()
                .filter(|(w, _)| liegt_in(w, datei))
                .max_by_key(|(w, _)| w.len())
                .map(|(_, a)| a.as_ref())
                .unwrap_or(self.lokal.as_ref()),
            _ => self.lokal.as_ref(),
        }
    }
}

#[cfg(test)]
#[path = "zuordnung_tests.rs"]
mod tests;
