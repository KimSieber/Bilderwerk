//! Anbieter `dropbox-klassisch`: Kims Dropbox mit 0-Byte-Platzhaltern („nur online“).
//! Lokal vorhandene Dateien → wie `lokal`. Nur online → Vorschau über die Dropbox-API,
//! HEIC seit v0.0.5 ebenfalls zuerst über die API (lehnt Dropbox das Format ab, merkt sich der
//! Anbieter das bis zum Programmende). Bei PDF-Seiten oder wenn Dropbox nicht umwandeln kann → Original laden, lokal
//! mit sips/PDFKit verkleinern, Kopie löschen (PDF: kurz für weitere Seiten vorhalten).
//! Die Datei im Dropbox-Ordner bleibt „nur online“ – es öffnet sich nichts.

pub mod api;
pub mod pkce;
pub mod token;

use super::zuordnung::dropbox_pfad;
use super::{quelle_datei, Speicheranbieter, Zustand};
use crate::thumbs::{self, cache_ziel, quelle, Vorschau};
use api::{mit_wiederholung, Api, ApiFehler, Basis};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use token::Tokenablage;

pub const REDIRECT_PORT: u16 = 53682;
pub const REDIRECT_PFAD: &str = "/psv-anmeldung";

/// Formate, für die Dropbox selbst eine Vorschau erzeugen soll. HEIC/HEIF sind in der API-Doku nicht
/// genannt, werden aber versucht – bei `unsupported_extension` fällt der Anbieter auf den Download zurück.
pub const VORSCHAU_ENDUNGEN: [&str; 11] = ["jpg", "jpeg", "png", "tiff", "tif", "gif", "webp", "ppm", "bmp", "heic", "heif"];

fn ist_heic(endung: &str) -> bool {
    matches!(endung, "heic" | "heif")
}

/// Heruntergeladene PDFs so lange für weitere Seiten vorhalten.
const PDF_VORHALTEN: Duration = Duration::from_secs(600);

/// Anmeldung und Zugang zu einem Dropbox-Konto (einmal je Programm).
pub struct Konto {
    app_key: String,
    api: Api,
    auth_basis: String,
    ablage: Box<dyn Tokenablage>,
    access: Mutex<Option<(String, Instant)>>,
    warte: fn(Duration),
}

impl Konto {
    pub fn neu(app_key: &str, ablage: Box<dyn Tokenablage>) -> Self {
        Self::mit_basis(app_key, ablage, Basis::default(), pkce::AUTORISIEREN, std::thread::sleep)
    }

    /// Für Tests: eigene Server-Adressen und Wartefunktion.
    pub fn mit_basis(app_key: &str, ablage: Box<dyn Tokenablage>, basis: Basis, auth_basis: &str, warte: fn(Duration)) -> Self {
        Self {
            app_key: app_key.into(),
            api: Api::neu(basis),
            auth_basis: auth_basis.into(),
            ablage,
            access: Mutex::new(None),
            warte,
        }
    }

    pub fn app_key(&self) -> &str {
        &self.app_key
    }

    pub fn angemeldet(&self) -> bool {
        self.ablage.lesen().is_some()
    }

    /// Gültiges Access-Token (gemerkt bis 5 min vor Ablauf, sonst erneuert).
    fn access_token(&self) -> Result<String, ApiFehler> {
        if let Some((t, bis)) = self.access.lock().unwrap().as_ref() {
            if Instant::now() < *bis {
                return Ok(t.clone());
            }
        }
        let refresh = self.ablage.lesen().ok_or_else(|| ApiFehler::Anmeldung("nicht angemeldet".into()))?;
        let t = mit_wiederholung(&self.warte, || self.api.erneuern(&self.app_key, &refresh))?;
        let bis = Instant::now() + Duration::from_secs(t.laeuft_ab_in.saturating_sub(300));
        *self.access.lock().unwrap() = Some((t.access.clone(), bis));
        Ok(t.access)
    }

    /// Ruft `f` mit gültigem Token; bei 401 einmal erneuern; bei 429/503 wiederholen.
    fn mit_token<T>(&self, f: impl Fn(&str) -> Result<T, ApiFehler>) -> Result<T, ApiFehler> {
        let r = mit_wiederholung(&self.warte, || f(&self.access_token()?));
        match r {
            Err(ApiFehler::Abgelaufen) => {
                *self.access.lock().unwrap() = None;
                mit_wiederholung(&self.warte, || f(&self.access_token()?))
            }
            r => r,
        }
    }

    pub fn vorschau(&self, dpfad: &str) -> Result<Vec<u8>, ApiFehler> {
        self.mit_token(|t| self.api.vorschau(t, dpfad))
    }

    pub fn herunterladen(&self, dpfad: &str, ziel: &Path) -> Result<u64, ApiFehler> {
        self.mit_token(|t| self.api.herunterladen(t, dpfad, ziel))
    }

    /// Anmeldung im Browser: lokaler Rückruf-Server auf 127.0.0.1:`port`, `oeffnen` zeigt die URL an.
    /// Blockiert, bis Dropbox zurückruft oder `frist` abläuft.
    pub fn anmelden(&self, port: u16, oeffnen: impl FnOnce(&str), frist: Duration) -> Result<(), String> {
        let redirect = format!("http://127.0.0.1:{port}{REDIRECT_PFAD}");
        let server = TcpListener::bind(("127.0.0.1", port))
            .map_err(|e| format!("Anmeldung nicht möglich, Port {port} belegt: {e}"))?;
        server.set_nonblocking(true).map_err(|e| e.to_string())?;
        let verifier = pkce::zufall(64);
        let state = pkce::zufall(24);
        oeffnen(&pkce::anmelde_url(&self.auth_basis, &self.app_key, &pkce::challenge(&verifier), &redirect, &state));

        let ende = Instant::now() + frist;
        let query = loop {
            if Instant::now() > ende {
                return Err("Zeit für die Anmeldung abgelaufen.".into());
            }
            let (mut s, _) = match server.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(e) => return Err(e.to_string()),
            };
            let _ = s.set_nonblocking(false);
            let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
            let mut buf = [0u8; 8192];
            let n = s.read(&mut buf).unwrap_or(0);
            let text = String::from_utf8_lossy(&buf[..n]);
            match pkce::anfrage_zeile(text.lines().next().unwrap_or("")) {
                Some((pfad, q)) if pfad == REDIRECT_PFAD => {
                    let ok = pkce::rueckruf_auswerten(&q, &state).is_ok();
                    let html = if ok {
                        "<h2>Bilderwerk View ist mit Dropbox verbunden.</h2><p>Dieses Fenster kann geschlossen werden.</p>"
                    } else {
                        "<h2>Anmeldung nicht abgeschlossen.</h2><p>Bitte in Bilderwerk View erneut versuchen.</p>"
                    };
                    let _ = write!(
                        s,
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{html}",
                        html.len()
                    );
                    break q;
                }
                _ => {
                    let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n");
                }
            }
        };
        let code = pkce::rueckruf_auswerten(&query, &state)?;
        let t = self.api.code_einloesen(&self.app_key, &code, &verifier, &redirect).map_err(|e| e.to_string())?;
        let refresh = t.refresh.ok_or("Dropbox hat kein Refresh-Token geliefert.")?;
        self.ablage.schreiben(&refresh)?;
        let bis = Instant::now() + Duration::from_secs(t.laeuft_ab_in.saturating_sub(300));
        *self.access.lock().unwrap() = Some((t.access, bis));
        Ok(())
    }

    /// Abmelden: Token bei Dropbox widerrufen (wenn erreichbar) und aus dem Schlüsselbund löschen.
    pub fn abmelden(&self) -> Result<(), String> {
        if self.angemeldet() {
            if let Ok(t) = self.access_token() {
                let _ = self.api.widerrufen(&t);
            }
        }
        *self.access.lock().unwrap() = None;
        self.ablage.loeschen()
    }
}

/// Anbieter für eine Dropbox-Wurzel (z. B. /Users/kim/Dropbox).
pub struct DropboxKlassisch {
    wurzel: String,
    cache: PathBuf,
    tmp: PathBuf,
    konto: Option<Arc<Konto>>,
    /// je Dropbox-Datei eine Sperre – parallele Seiten derselben PDF laden nur einmal
    sperren: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    /// Dropbox hat HEIC als Format abgelehnt → für weitere HEIC-Bilder gleich Download
    heic_abgelehnt: AtomicBool,
}

fn endung(pfad: &str) -> String {
    Path::new(pfad).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

impl DropboxKlassisch {
    /// `konto` = None: kein App-Schlüssel eingerichtet → verhält sich wie `lokal`.
    pub fn new(wurzel: &str, cache: &Path, konto: Option<Arc<Konto>>) -> Self {
        Self {
            wurzel: wurzel.into(),
            cache: cache.to_path_buf(),
            tmp: cache.join("dropbox-tmp"),
            konto,
            sperren: Mutex::default(),
            heic_abgelehnt: AtomicBool::new(false),
        }
    }

    fn sperre(&self, dpfad: &str) -> Arc<Mutex<()>> {
        self.sperren.lock().unwrap().entry(dpfad.to_lowercase()).or_default().clone()
    }

    /// Alte Kopien (älter als PDF_VORHALTEN) entfernen.
    fn aufraeumen(&self) {
        let Ok(rd) = std::fs::read_dir(&self.tmp) else { return };
        for e in rd.flatten() {
            let alt = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| SystemTime::now().duration_since(t).ok())
                .is_some_and(|d| d > PDF_VORHALTEN);
            if alt {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }

    /// Original in den temporären Ordner laden (PDF: vorhandene, junge Kopie wiederverwenden).
    fn kopie(&self, konto: &Konto, dpfad: &str) -> Result<PathBuf, ApiFehler> {
        std::fs::create_dir_all(&self.tmp).map_err(|e| ApiFehler::Sonst(0, e.to_string()))?;
        self.aufraeumen();
        let name = format!("{:016x}.{}", thumbs::fnv1a(&dpfad.to_lowercase()), endung(dpfad));
        let ziel = self.tmp.join(name);
        if ziel.is_file() {
            return Ok(ziel);
        }
        let teil = ziel.with_extension("part");
        match konto.herunterladen(dpfad, &teil) {
            Ok(_) => std::fs::rename(&teil, &ziel).map(|_| ziel).map_err(|e| ApiFehler::Sonst(0, e.to_string())),
            Err(e) => {
                let _ = std::fs::remove_file(&teil);
                Err(e)
            }
        }
    }

    fn per_download(&self, konto: &Konto, pfad: &str, dpfad: &str, groesse: u32, ziel: &Path) -> Vorschau {
        let (_, seite) = quelle(pfad);
        if ist_heic(&endung(dpfad)) {
            eprintln!("View: HEIC per Download – {dpfad}");
        }
        let kopie = match self.kopie(konto, dpfad) {
            Ok(k) => k,
            Err(e) => return api_fehler(e),
        };
        let v = thumbs::erzeugen(&kopie, seite, groesse, ziel);
        if seite.is_none() {
            let _ = std::fs::remove_file(&kopie); // PDF bleibt kurz für weitere Seiten
        }
        v
    }
}

/// API-Fehler → Vorschau-Status. Offline/abgemeldet → Wolke wie bisher.
fn api_fehler(e: ApiFehler) -> Vorschau {
    match e {
        ApiFehler::NichtGefunden => Vorschau::Fehlt,
        ApiFehler::Netz(_) | ApiFehler::Anmeldung(_) | ApiFehler::Drosselung(_) | ApiFehler::Abgelaufen => Vorschau::NurOnline,
        e => Vorschau::Fehler { meldung: e.to_string() },
    }
}

/// Dropbox-Vorschau in den Cache: bei Bedarf auf `groesse` verkleinern, dann atomar umbenennen.
fn speichern(ziel: &Path, daten: &[u8], groesse: u32) -> Result<(), String> {
    std::fs::create_dir_all(ziel.parent().unwrap()).map_err(|e| e.to_string())?;
    let teil = ziel.with_extension(format!("{}.part.jpg", thumbs::eindeutig()));
    std::fs::write(&teil, daten).map_err(|e| e.to_string())?;
    if let Err(e) = thumbs::auf_groesse(&teil, groesse) {
        let _ = std::fs::remove_file(&teil);
        return Err(e);
    }
    std::fs::rename(&teil, ziel).map_err(|e| e.to_string())
}

impl Speicheranbieter for DropboxKlassisch {
    fn art(&self) -> &'static str {
        "dropbox-klassisch"
    }

    fn zustand(&self, pfad: &str) -> Zustand {
        super::Lokal::new(&self.cache).zustand(pfad)
    }

    fn vorschau(&self, pfad: &str, groesse: u32) -> Vorschau {
        let datei = quelle_datei(pfad);
        let meta = match std::fs::metadata(datei) {
            Ok(m) => m,
            Err(_) => return Vorschau::Fehlt,
        };
        let ziel = cache_ziel(&self.cache, pfad, groesse, meta.mtime(), meta.len());
        if ziel.is_file() {
            crate::cache::beruehren(&ziel);
            return Vorschau::Bild { datei: ziel.to_string_lossy().into() };
        }
        if !thumbs::ist_nur_online(&meta) {
            return thumbs::holen(&self.cache, pfad, groesse);
        }
        let (Some(konto), Some(dpfad)) = (self.konto.as_deref(), dropbox_pfad(&self.wurzel, pfad)) else {
            return Vorschau::NurOnline;
        };
        if !konto.angemeldet() {
            return Vorschau::NurOnline;
        }
        let sperre = self.sperre(&dpfad);
        let _g = sperre.lock().unwrap();
        if ziel.is_file() {
            return Vorschau::Bild { datei: ziel.to_string_lossy().into() };
        }
        let (_, seite) = quelle(pfad);
        let end = endung(datei);
        let heic = ist_heic(&end);
        let api_versuchen = !(heic && self.heic_abgelehnt.load(Ordering::Relaxed));
        if seite.is_none() && api_versuchen && VORSCHAU_ENDUNGEN.contains(&end.as_str()) {
            let start = Instant::now();
            match konto.vorschau(&dpfad) {
                Ok(jpeg) => {
                    if heic {
                        // Prüfhilfe v0.0.5: im Terminal von `npm run tauri dev` sichtbar
                        eprintln!("View: HEIC-Vorschau über Dropbox-API in {} ms – {dpfad}", start.elapsed().as_millis());
                    }
                    return match speichern(&ziel, &jpeg, groesse) {
                        Ok(()) => Vorschau::Bild { datei: ziel.to_string_lossy().into() },
                        Err(meldung) => Vorschau::Fehler { meldung },
                    }
                }
                Err(ApiFehler::NichtUmwandelbar(text)) => {
                    // z. B. > 20 MB → Download; HEIC grundsätzlich abgelehnt → merken
                    if heic && text.contains("unsupported_extension") {
                        eprintln!("View: Dropbox lehnt HEIC-Vorschau ab ({text}) – ab jetzt Download");
                        self.heic_abgelehnt.store(true, Ordering::Relaxed);
                    } else if heic {
                        eprintln!("View: HEIC nicht umwandelbar ({text}) – Download – {dpfad}");
                    }
                }
                Err(e) => return api_fehler(e),
            }
        }
        self.per_download(konto, pfad, &dpfad, groesse, &ziel)
    }

    fn original_bereitstellen(&self, pfad: &str) -> Result<PathBuf, String> {
        match self.zustand(pfad) {
            Zustand::Lokal => Ok(PathBuf::from(quelle_datei(pfad))),
            Zustand::Fehlt => Err(format!("Datei nicht gefunden: {}", quelle_datei(pfad))),
            Zustand::NurOnline => {
                let konto = self.konto.as_deref().filter(|k| k.angemeldet()).ok_or("Datei liegt nur online – bitte bei Dropbox anmelden.")?;
                let dpfad = dropbox_pfad(&self.wurzel, pfad).ok_or("Pfad liegt nicht in der Dropbox.")?;
                let ordner = self.tmp.join("originale");
                std::fs::create_dir_all(&ordner).map_err(|e| e.to_string())?;
                let name = Path::new(quelle_datei(pfad)).file_name().ok_or("Dateiname fehlt")?;
                let ziel = ordner.join(name);
                konto.herunterladen(&dpfad, &ziel).map_err(|e| e.to_string())?;
                Ok(ziel)
            }
        }
    }
}

#[cfg(test)]
mod tests;
