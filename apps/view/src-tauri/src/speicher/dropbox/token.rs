//! Ablage des Refresh-Tokens: im macOS-Schlüsselbund, nie in einer Datei.

use std::sync::Mutex;

pub trait Tokenablage: Send + Sync {
    fn lesen(&self) -> Option<String>;
    fn schreiben(&self, token: &str) -> Result<(), String>;
    fn loeschen(&self) -> Result<(), String>;
}

pub const DIENST: &str = "de.sieber-bw.bilderwerk.view";
pub const KONTO: &str = "dropbox-refresh-token";

/// macOS-Schlüsselbund (Eintrag „de.sieber-bw.bilderwerk.view“ / „dropbox-refresh-token“).
pub struct Schluesselbund;

impl Schluesselbund {
    fn eintrag() -> Result<keyring::Entry, String> {
        keyring::Entry::new(DIENST, KONTO).map_err(|e| e.to_string())
    }
}

impl Tokenablage for Schluesselbund {
    fn lesen(&self) -> Option<String> {
        Self::eintrag().ok()?.get_password().ok()
    }
    fn schreiben(&self, token: &str) -> Result<(), String> {
        Self::eintrag()?.set_password(token).map_err(|e| format!("Schlüsselbund: {e}"))
    }
    fn loeschen(&self) -> Result<(), String> {
        match Self::eintrag()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Schlüsselbund: {e}")),
        }
    }
}

/// Für Tests: nur im Arbeitsspeicher.
#[derive(Default)]
pub struct ImSpeicher(pub Mutex<Option<String>>);

impl Tokenablage for ImSpeicher {
    fn lesen(&self) -> Option<String> {
        self.0.lock().unwrap().clone()
    }
    fn schreiben(&self, token: &str) -> Result<(), String> {
        *self.0.lock().unwrap() = Some(token.into());
        Ok(())
    }
    fn loeschen(&self) -> Result<(), String> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}
