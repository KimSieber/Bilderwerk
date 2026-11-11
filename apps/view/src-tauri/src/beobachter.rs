//! DB neu laden, wenn Dropbox die Sicherungskopie ersetzt (v0.0.7).
//! Die DB ist mit `immutable=1` geöffnet: SQLite liest die alte Datei weiter, bis neu geöffnet wird.
//! Ein Hintergrund-Thread prüft regelmäßig Änderungszeit, Größe und Inode; neu geladen wird erst,
//! wenn der neue Stand bei zwei Prüfungen gleich bleibt (Dropbox schreibt nicht in einem Zug).

use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// Prüfabstand des Hintergrund-Threads
pub const ABSTAND: std::time::Duration = std::time::Duration::from_secs(3);

/// Kennzeichen einer Datei-Fassung
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stand {
    pub mtime_ns: i128,
    pub laenge: u64,
    pub inode: u64,
}

/// Aktueller Stand der Datei (None, wenn sie gerade fehlt – z. B. mitten im Ersetzen)
pub fn stand(pfad: &Path) -> Option<Stand> {
    let m = std::fs::metadata(pfad).ok()?;
    Some(Stand { mtime_ns: m.mtime() as i128 * 1_000_000_000 + m.mtime_nsec() as i128, laenge: m.len(), inode: m.ino() })
}

/// Entscheidet, wann neu geladen wird.
#[derive(Debug, Default)]
pub struct Beobachter {
    geladen: Option<Stand>,
    kandidat: Option<Stand>,
}

impl Beobachter {
    /// Nach dem (Neu-)Öffnen: dieser Stand ist geladen.
    pub fn geladen(&mut self, s: Option<Stand>) {
        self.geladen = s;
        self.kandidat = None;
    }

    /// Eine Prüfung. `true` = jetzt neu laden (der neue Stand ist stabil).
    /// Gelingt das Öffnen danach nicht, `fehlgeschlagen()` aufrufen – dann wird es später erneut versucht.
    pub fn pruefen(&mut self, aktuell: Option<Stand>) -> bool {
        let Some(a) = aktuell else {
            self.kandidat = None;
            return false;
        };
        if self.geladen == Some(a) {
            self.kandidat = None;
            return false;
        }
        if self.kandidat == Some(a) {
            self.kandidat = None;
            return true;
        }
        self.kandidat = Some(a);
        false
    }

    /// Neu öffnen ging nicht (Datei unvollständig?) → beim nächsten stabilen Stand wieder versuchen.
    pub fn fehlgeschlagen(&mut self) {
        self.kandidat = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(n: i128) -> Option<Stand> {
        Some(Stand { mtime_ns: n, laenge: 100, inode: 1 })
    }

    /// R-BEO-01
    #[test]
    fn unveraendert_nie() {
        let mut b = Beobachter::default();
        b.geladen(s(1));
        assert!(!b.pruefen(s(1)));
        assert!(!b.pruefen(s(1)));
    }

    /// R-BEO-02
    #[test]
    fn neu_erst_wenn_stabil() {
        let mut b = Beobachter::default();
        b.geladen(s(1));
        assert!(!b.pruefen(s(2)), "erste Sichtung: noch warten");
        assert!(!b.pruefen(s(3)), "ändert sich noch");
        assert!(b.pruefen(s(3)), "zweimal gleich → laden");
        b.geladen(s(3));
        assert!(!b.pruefen(s(3)));
    }

    /// R-BEO-03
    #[test]
    fn fehlende_datei_wartet() {
        let mut b = Beobachter::default();
        b.geladen(s(1));
        assert!(!b.pruefen(s(2)));
        assert!(!b.pruefen(None), "mitten im Ersetzen");
        assert!(!b.pruefen(s(2)), "nach Lücke wieder von vorn");
        assert!(b.pruefen(s(2)));
    }

    /// R-BEO-04
    #[test]
    fn fehlschlag_wird_wiederholt() {
        let mut b = Beobachter::default();
        b.geladen(s(1));
        b.pruefen(s(2));
        assert!(b.pruefen(s(2)));
        b.fehlgeschlagen();
        assert!(!b.pruefen(s(2)));
        assert!(b.pruefen(s(2)), "nächster Versuch");
    }

    /// R-BEO-05
    #[test]
    fn stand_erkennt_ersetzen() {
        let d = std::env::temp_dir().join(format!("view-beo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("k.db");
        assert_eq!(stand(&p), None);
        std::fs::write(&p, b"alt").unwrap();
        let a = stand(&p).unwrap();
        // wie Dropbox: neue Datei daneben schreiben und umbenennen (neuer Inode, gleiche Länge)
        let t = d.join(".k.db.tmp");
        std::fs::write(&t, b"neu").unwrap();
        std::fs::rename(&t, &p).unwrap();
        let n = stand(&p).unwrap();
        assert_eq!(n.laenge, a.laenge);
        assert_ne!(a, n);
    }
}
