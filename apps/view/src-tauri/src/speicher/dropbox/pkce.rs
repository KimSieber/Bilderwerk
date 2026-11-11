//! OAuth 2 mit PKCE (RFC 7636): Prüfwert erzeugen, Anmelde-URL bauen, Rückruf auswerten.
//! Kein App-Secret in der App – nur der öffentliche App-Schlüssel.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

pub const AUTORISIEREN: &str = "https://www.dropbox.com/oauth2/authorize";

/// Zufallswert aus den in RFC 7636 erlaubten Zeichen.
pub fn zufall(laenge: usize) -> String {
    const Z: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let mut b = vec![0u8; laenge];
    getrandom::getrandom(&mut b).expect("Zufallsquelle");
    b.iter().map(|x| Z[*x as usize % Z.len()] as char).collect()
}

/// code_challenge = BASE64URL(SHA256(verifier)) ohne Auffüllzeichen
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// Prozentkodierung für URL-Parameter und Formulare (alles außer A–Z a–z 0–9 - . _ ~).
pub fn kodieren(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn dekodieren(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16) {
                    Ok(x) => {
                        out.push(x);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            x => out.push(x),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Anmelde-URL für den Browser. `token_access_type=offline` liefert ein Refresh-Token.
pub fn anmelde_url(basis: &str, app_key: &str, challenge: &str, redirect: &str, state: &str) -> String {
    format!(
        "{basis}?client_id={}&response_type=code&code_challenge={}&code_challenge_method=S256\
         &token_access_type=offline&redirect_uri={}&state={}",
        kodieren(app_key),
        kodieren(challenge),
        kodieren(redirect),
        kodieren(state)
    )
}

/// Wert eines Parameters aus einer Query („a=1&b=2“).
pub fn parameter(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|p| {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        (k == name).then(|| dekodieren(v))
    })
}

/// Erste Zeile einer HTTP-Anfrage („GET /pfad?query HTTP/1.1“) → (pfad, query)
pub fn anfrage_zeile(zeile: &str) -> Option<(String, String)> {
    let mut t = zeile.split_whitespace();
    if t.next()? != "GET" {
        return None;
    }
    let ziel = t.next()?;
    let (pfad, query) = ziel.split_once('?').unwrap_or((ziel, ""));
    Some((pfad.to_string(), query.to_string()))
}

/// Ergebnis des Rückrufs: Code bei passendem state, sonst Fehlertext.
pub fn rueckruf_auswerten(query: &str, state: &str) -> Result<String, String> {
    if let Some(f) = parameter(query, "error") {
        let text = parameter(query, "error_description").unwrap_or_default();
        return Err(if f == "access_denied" { "Anmeldung bei Dropbox abgelehnt.".into() } else { format!("Dropbox meldet: {f} {text}") });
    }
    if parameter(query, "state").as_deref() != Some(state) {
        return Err("Antwort passt nicht zur Anmeldung (state) – bitte erneut versuchen.".into());
    }
    parameter(query, "code").filter(|c| !c.is_empty()).ok_or_else(|| "Dropbox hat keinen Code geliefert.".into())
}
