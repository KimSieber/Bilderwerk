//! Dropbox-HTTP-API: Token, Vorschau (get_thumbnail_v2), Download, Abmelden.
//! Basis-URLs sind austauschbar – die Tests nutzen einen nachgebildeten Server.

use super::pkce::kodieren;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Basis {
    /// api.dropboxapi.com – Token, Abmelden
    pub api: String,
    /// content.dropboxapi.com – Vorschau, Download
    pub inhalt: String,
}

impl Default for Basis {
    fn default() -> Self {
        Self { api: "https://api.dropboxapi.com".into(), inhalt: "https://content.dropboxapi.com".into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiFehler {
    /// Datei gibt es in der Dropbox nicht (409 path/not_found)
    NichtGefunden,
    /// Dropbox kann keine Vorschau erzeugen (Format, > 20 MB …) → Download-Rückfall
    NichtUmwandelbar(String),
    /// Access-Token abgelaufen (401) → erneuern
    Abgelaufen,
    /// Anmeldung ungültig (Refresh-Token widerrufen o. ä.) → neu anmelden
    Anmeldung(String),
    /// 429/503 – später wiederholen (Sekunden aus Retry-After)
    Drosselung(Option<u64>),
    /// keine Verbindung, Zeitüberschreitung
    Netz(String),
    Sonst(u16, String),
}

impl std::fmt::Display for ApiFehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NichtGefunden => write!(f, "Datei in der Dropbox nicht gefunden"),
            Self::NichtUmwandelbar(s) => write!(f, "Dropbox kann keine Vorschau erzeugen: {s}"),
            Self::Abgelaufen => write!(f, "Dropbox-Zugang abgelaufen"),
            Self::Anmeldung(s) => write!(f, "Dropbox-Anmeldung ungültig: {s}"),
            Self::Drosselung(_) => write!(f, "Dropbox bittet, es später erneut zu versuchen"),
            Self::Netz(s) => write!(f, "Keine Verbindung zu Dropbox: {s}"),
            Self::Sonst(c, s) => write!(f, "Dropbox-Fehler {c}: {s}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub access: String,
    pub laeuft_ab_in: u64,
    pub refresh: Option<String>,
}

/// Größte Datei, die heruntergeladen wird (Schutz vor Endlos-Antworten)
const MAX_DOWNLOAD: u64 = 2 * 1024 * 1024 * 1024;

pub struct Api {
    basis: Basis,
    agent: ureq::Agent,
}

/// Dropbox-API-Arg muss reines ASCII sein: Nicht-ASCII als \uXXXX (JSON-gültig).
pub fn api_arg(v: &serde_json::Value) -> String {
    let mut s = String::new();
    for c in v.to_string().chars() {
        if c.is_ascii() {
            s.push(c);
        } else {
            let mut buf = [0u16; 2];
            for u in c.encode_utf16(&mut buf) {
                s.push_str(&format!("\\u{u:04x}"));
            }
        }
    }
    s
}

/// HTTP-Antwort → Fehlerart (bei Status ≠ 2xx).
pub fn fehler_aus(status: u16, retry_after: Option<&str>, text: &str) -> ApiFehler {
    match status {
        401 => ApiFehler::Abgelaufen,
        429 | 503 => ApiFehler::Drosselung(retry_after.and_then(|s| s.trim().parse().ok())),
        400 if text.contains("invalid_grant") => ApiFehler::Anmeldung(text.chars().take(200).collect()),
        409 if text.contains("not_found") => ApiFehler::NichtGefunden,
        409 => ApiFehler::NichtUmwandelbar(text.chars().take(200).collect()),
        _ => ApiFehler::Sonst(status, text.chars().take(200).collect()),
    }
}

impl Api {
    pub fn neu(basis: Basis) -> Self {
        let lokal = basis.api.starts_with("http://127.0.0.1") || basis.inhalt.starts_with("http://127.0.0.1");
        let mut cfg = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(120)))
            .timeout_connect(Some(Duration::from_secs(10)));
        if lokal {
            cfg = cfg.proxy(None);
        }
        Self { basis, agent: cfg.build().into() }
    }

    fn antwort(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<ureq::http::Response<ureq::Body>, ApiFehler> {
        let mut r = r.map_err(|e| ApiFehler::Netz(e.to_string()))?;
        let status = r.status().as_u16();
        if (200..300).contains(&status) {
            return Ok(r);
        }
        let ra = r.headers().get("Retry-After").and_then(|v| v.to_str().ok()).map(str::to_string);
        let text = r.body_mut().read_to_string().unwrap_or_default();
        Err(fehler_aus(status, ra.as_deref(), &text))
    }

    fn token_aus(text: &str) -> Result<Token, ApiFehler> {
        let j: serde_json::Value = serde_json::from_str(text).map_err(|e| ApiFehler::Sonst(200, e.to_string()))?;
        Ok(Token {
            access: j["access_token"].as_str().ok_or_else(|| ApiFehler::Sonst(200, "access_token fehlt".into()))?.into(),
            laeuft_ab_in: j["expires_in"].as_u64().unwrap_or(14_400),
            refresh: j["refresh_token"].as_str().map(str::to_string),
        })
    }

    fn token_anfrage(&self, form: &[(&str, &str)]) -> Result<Token, ApiFehler> {
        let body = form.iter().map(|(k, v)| format!("{k}={}", kodieren(v))).collect::<Vec<_>>().join("&");
        let r = self
            .agent
            .post(format!("{}/oauth2/token", self.basis.api))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send(body.as_bytes());
        let mut r = Self::antwort(r)?;
        let text = r.body_mut().read_to_string().map_err(|e| ApiFehler::Netz(e.to_string()))?;
        Self::token_aus(&text)
    }

    /// Code aus dem Browser-Rückruf gegen Token tauschen.
    pub fn code_einloesen(&self, app_key: &str, code: &str, verifier: &str, redirect: &str) -> Result<Token, ApiFehler> {
        self.token_anfrage(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", verifier),
            ("client_id", app_key),
            ("redirect_uri", redirect),
        ])
    }

    /// Neues Access-Token aus dem Refresh-Token.
    pub fn erneuern(&self, app_key: &str, refresh: &str) -> Result<Token, ApiFehler> {
        self.token_anfrage(&[("grant_type", "refresh_token"), ("refresh_token", refresh), ("client_id", app_key)])
    }

    /// Token bei Dropbox widerrufen (Abmelden).
    pub fn widerrufen(&self, access: &str) -> Result<(), ApiFehler> {
        let r = self
            .agent
            .post(format!("{}/2/auth/token/revoke", self.basis.api))
            .header("Authorization", format!("Bearer {access}"))
            .send_empty();
        Self::antwort(r).map(|_| ())
    }

    /// Vorschau als JPEG, eingepasst in 960 × 640 – so erreicht jede Ausrichtung mindestens
    /// 640 px an der längsten Kante (quer wird danach lokal auf 640 verkleinert, hoch: 427 × 640).
    pub fn vorschau(&self, access: &str, pfad: &str) -> Result<Vec<u8>, ApiFehler> {
        let arg = serde_json::json!({
            "resource": {".tag": "path", "path": pfad},
            "format": "jpeg", "size": "w960h640", "mode": "bestfit"
        });
        let r = self
            .agent
            .post(format!("{}/2/files/get_thumbnail_v2", self.basis.inhalt))
            .header("Authorization", format!("Bearer {access}"))
            .header("Dropbox-API-Arg", api_arg(&arg))
            .send_empty();
        let mut r = Self::antwort(r)?;
        r.body_mut().with_config().limit(50 * 1024 * 1024).read_to_vec().map_err(|e| ApiFehler::Netz(e.to_string()))
    }

    /// Original herunterladen nach `ziel` (wird überschrieben).
    pub fn herunterladen(&self, access: &str, pfad: &str, ziel: &Path) -> Result<u64, ApiFehler> {
        let arg = serde_json::json!({ "path": pfad });
        let r = self
            .agent
            .post(format!("{}/2/files/download", self.basis.inhalt))
            .header("Authorization", format!("Bearer {access}"))
            .header("Dropbox-API-Arg", api_arg(&arg))
            .send_empty();
        let mut r = Self::antwort(r)?;
        let mut leser = r.body_mut().with_config().limit(MAX_DOWNLOAD).reader();
        let mut datei = std::fs::File::create(ziel).map_err(|e| ApiFehler::Sonst(0, e.to_string()))?;
        let n = std::io::copy(&mut leser, &mut datei).map_err(|e| ApiFehler::Netz(e.to_string()))?;
        let _ = leser.read(&mut [0u8; 0]);
        Ok(n)
    }
}

/// Wiederholt `f` bei Drosselung (429/503): Retry-After beachten, sonst 0,5 · 2ⁿ s (max. 8 s), höchstens 5 Versuche.
pub fn mit_wiederholung<T>(warte: &dyn Fn(Duration), mut f: impl FnMut() -> Result<T, ApiFehler>) -> Result<T, ApiFehler> {
    let mut versuch = 0u32;
    loop {
        match f() {
            Err(ApiFehler::Drosselung(ra)) if versuch < 4 => {
                let d = ra.map(Duration::from_secs).unwrap_or_else(|| Duration::from_millis((500u64 << versuch).min(8_000)));
                warte(d.min(Duration::from_secs(60)));
                versuch += 1;
            }
            r => return r,
        }
    }
}
