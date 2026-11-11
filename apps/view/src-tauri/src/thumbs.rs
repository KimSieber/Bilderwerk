//! Vorschaubilder: erzeugt verkleinerte JPEGs per `sips` (Bilder) bzw. PDFKit
//! (PDF-Seiten) und legt sie im Cache ab. Schlüssel: Pfad + Größe + Änderungszeit + Dateigröße.

use serde::Serialize;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "art", rename_all = "camelCase")]
pub enum Vorschau {
    /// Fertige Vorschau im Cache
    Bild { datei: String },
    /// Vorschau aus der Katalog-DB (Tabelle `vorschauen`), Abruf über `vorschau://localhost/<hash>`
    Datenbank { schluessel: String },
    /// Datei liegt nur in der Cloud (Dropbox „nur online“)
    NurOnline,
    /// Datei existiert nicht (mehr)
    Fehlt,
    Fehler { meldung: String },
}

/// `…/Album.pdf#seite=3` → (`…/Album.pdf`, Some(3))
pub fn quelle(pfad: &str) -> (&str, Option<u32>) {
    if let Some((datei, seite)) = pfad.rsplit_once("#seite=") {
        if let Ok(n) = seite.parse() {
            return (datei, Some(n));
        }
    }
    (pfad, None)
}

/// macOS-Dateiflag SF_DATALESS: Inhalt liegt nur beim Cloud-Anbieter (File Provider).
const SF_DATALESS: u32 = 0x4000_0000;

/// Entscheidet anhand von Dateiflags, Größe und belegten Blöcken, ob eine Datei nur online liegt.
/// - klassische Dropbox: Platzhalter mit 0 Byte (+ Attribut com.dropbox.placeholder).
///   Unbedenklich, weil der Katalog nur Dateien ab 30 KB aufnimmt.
/// - File Provider (neue Dropbox, iCloud): Flag SF_DATALESS bzw. Größe ohne Blöcke.
pub fn nur_online_nach(flags: u32, laenge: u64, bloecke: u64) -> bool {
    laenge == 0 || flags & SF_DATALESS != 0 || bloecke == 0
}

#[cfg(target_os = "macos")]
fn dateiflags(meta: &Metadata) -> u32 {
    use std::os::macos::fs::MetadataExt as _;
    meta.st_flags()
}

#[cfg(not(target_os = "macos"))]
fn dateiflags(_meta: &Metadata) -> u32 {
    0
}

/// Platzhalter einer Cloud-Datei (Dropbox „nur online“)?
pub fn ist_nur_online(meta: &Metadata) -> bool {
    nur_online_nach(dateiflags(meta), meta.len(), meta.blocks())
}

/// Erlaubt diesem Prozess (und den von ihm gestarteten Werkzeugen), Platzhalter-Dateien
/// beim Lesen herunterzuladen. Ohne diese Einstellung verweigert macOS das teilweise.
/// Einmal beim Programmstart aufrufen.
#[cfg(target_os = "macos")]
pub fn download_beim_lesen_erlauben() {
    // sys/resource.h
    const IOPOL_TYPE_VFS_MATERIALIZE_DATALESS_FILES: i32 = 3;
    const IOPOL_SCOPE_PROCESS: i32 = 0;
    const IOPOL_MATERIALIZE_DATALESS_FILES_ON: i32 = 2;
    extern "C" {
        fn setiopolicy_np(iotype: i32, scope: i32, policy: i32) -> i32;
    }
    unsafe {
        setiopolicy_np(
            IOPOL_TYPE_VFS_MATERIALIZE_DATALESS_FILES,
            IOPOL_SCOPE_PROCESS,
            IOPOL_MATERIALIZE_DATALESS_FILES_ON,
        );
    }
}

#[cfg(not(target_os = "macos"))]
pub fn download_beim_lesen_erlauben() {}

/// sips meldet bei Platzhaltern, die die Erkennung nicht erfasst hat, „not a valid file“.
pub fn fehler_heisst_nur_online(meldung: &str) -> bool {
    meldung.contains("not a valid file")
}

/// FNV-1a 64 Bit – stabil über Programmversionen hinweg (anders als DefaultHasher).
pub fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Cache-Datei einer Vorschau: `<cache>/<größe>/<fnv1a(pfad|größe|mtime|länge)>.jpg`.
/// Ändert sich die Datei (Zeit oder Größe), entsteht ein neuer Schlüssel.
pub fn cache_ziel(cache: &Path, pfad: &str, groesse: u32, mtime: i64, laenge: u64) -> PathBuf {
    let schluessel = fnv1a(&format!("{pfad}|{groesse}|{mtime}|{laenge}"));
    cache.join(groesse.to_string()).join(format!("{schluessel:016x}.jpg"))
}

pub fn eindeutig() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Liefert die Vorschau (aus dem Cache oder neu erzeugt) für eine lokal lesbare Datei.
/// Nur-online-Dateien werden nicht angefasst → Status `NurOnline`.
/// Entspricht dem Anbieter `lokal` (siehe speicher.rs).
pub fn holen(cache: &Path, pfad: &str, groesse: u32) -> Vorschau {
    holen_mit(cache, pfad, groesse, ist_nur_online)
}

/// Wie `holen`, aber der Anbieter entscheidet, wann eine Datei als nicht lesbar gilt
/// (`nicht_lesbar` → `NurOnline`, ohne Werkzeug zu starten). Der Cache wird immer zuerst geprüft.
pub fn holen_mit(cache: &Path, pfad: &str, groesse: u32, nicht_lesbar: impl Fn(&Metadata) -> bool) -> Vorschau {
    let (datei, seite) = quelle(pfad);
    let meta = match std::fs::metadata(datei) {
        Ok(m) => m,
        Err(_) => return Vorschau::Fehlt,
    };

    let ziel = cache_ziel(cache, pfad, groesse, meta.mtime(), meta.len());
    if ziel.is_file() {
        crate::cache::beruehren(&ziel);
        return Vorschau::Bild { datei: ziel.to_string_lossy().into() };
    }

    if nicht_lesbar(&meta) {
        return Vorschau::NurOnline;
    }

    erzeugen(Path::new(datei), seite, groesse, &ziel)
}

/// Erzeugt die Vorschau von `datei` (bei PDF Seite `seite`) als JPEG nach `ziel`.
/// Schreibt erst in eine Zwischendatei und benennt dann um – halbe Dateien landen nie im Cache.
pub fn erzeugen(datei: &Path, seite: Option<u32>, groesse: u32, ziel: &Path) -> Vorschau {
    let dir = ziel.parent().expect("Ziel hat Ordner").to_path_buf();
    let stamm = ziel.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Vorschau::Fehler { meldung: e.to_string() };
    }
    let basis = dir.join(format!("{stamm}-{}.part", eindeutig()));
    let tmp = basis.with_extension("part.jpg");
    let g = groesse.to_string();

    let ausgabe = match seite {
        None => Command::new("/usr/bin/sips")
            .args(["-Z", &g, "-s", "format", "jpeg", "-s", "formatOptions", "82"])
            .arg(datei)
            .arg("--out")
            .arg(&tmp)
            .output(),
        Some(n) => {
            // PDF-Seiten über PDFKit (Teil von macOS) – kein externes Werkzeug nötig
            let _ = &basis;
            return match crate::pdf::seite_als_jpeg(datei, n, groesse, &tmp).and_then(|_| auf_groesse(&tmp, groesse)) {
                Ok(()) => match std::fs::rename(&tmp, ziel) {
                    Ok(()) => Vorschau::Bild { datei: ziel.to_string_lossy().into() },
                    Err(e) => Vorschau::Fehler { meldung: e.to_string() },
                },
                Err(meldung) => {
                    let _ = std::fs::remove_file(&tmp);
                    Vorschau::Fehler { meldung }
                }
            };
        }
    };

    match ausgabe {
        Ok(o) if o.status.success() && tmp.is_file() => match std::fs::rename(&tmp, ziel) {
            Ok(()) => Vorschau::Bild { datei: ziel.to_string_lossy().into() },
            Err(e) => Vorschau::Fehler { meldung: e.to_string() },
        },
        Ok(o) => {
            let _ = std::fs::remove_file(&tmp);
            let meldung: String = String::from_utf8_lossy(&o.stderr).trim().chars().take(300).collect();
            if fehler_heisst_nur_online(&meldung) {
                Vorschau::NurOnline
            } else {
                Vorschau::Fehler { meldung }
            }
        }
        Err(e) => Vorschau::Fehler { meldung: e.to_string() },
    }
}

/// Breite × Höhe eines JPEG aus dem SOF-Marker (None, wenn kein lesbares JPEG).
pub fn jpeg_masse(b: &[u8]) -> Option<(u32, u32)> {
    if b.len() < 4 || b[..2] != [0xFF, 0xD8] {
        return None;
    }
    let mut i = 2;
    while i + 9 < b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = b[i + 1];
        if m == 0xD8 || m == 0x01 || (0xD0..=0xD7).contains(&m) || m == 0xFF {
            i += if m == 0xFF { 1 } else { 2 };
            continue;
        }
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        if matches!(m, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            let h = u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32;
            let w = u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32;
            return Some((w, h));
        }
        i += 2 + len;
    }
    None
}

/// Muss ein Bild mit diesen Maßen auf `groesse` verkleinert werden? (nie vergrößern)
pub fn zu_gross(masse: Option<(u32, u32)>, groesse: u32) -> bool {
    masse.is_some_and(|(w, h)| w.max(h) > groesse)
}

/// JPEG-Datei in place auf längste Kante `groesse` bringen, falls größer (sips, Teil von macOS).
pub fn auf_groesse(jpeg: &Path, groesse: u32) -> Result<(), String> {
    let daten = std::fs::read(jpeg).map_err(|e| e.to_string())?;
    if !zu_gross(jpeg_masse(&daten), groesse) {
        return Ok(());
    }
    let o = Command::new("/usr/bin/sips")
        .args(["-Z", &groesse.to_string()])
        .arg(jpeg)
        .arg("--out")
        .arg(jpeg)
        .output()
        .map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().chars().take(300).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R-VOR-01
    #[test]
    fn quelle_aufloesen() {
        assert_eq!(quelle("/a/b.jpg"), ("/a/b.jpg", None));
        assert_eq!(quelle("/a/Album.pdf#seite=3"), ("/a/Album.pdf", Some(3)));
        assert_eq!(quelle("/a/x#seite=y.jpg"), ("/a/x#seite=y.jpg", None));
    }

    /// R-VOR-02
    #[test]
    fn fehlende_datei() {
        assert_eq!(holen(Path::new("/tmp"), "/gibt/es/nicht.jpg", 100), Vorschau::Fehlt);
    }

    /// R-VOR-04
    #[test]
    fn nur_online_erkennen() {
        assert!(nur_online_nach(SF_DATALESS, 5_000_000, 8)); // Flag gesetzt, Blöcke gemeldet
        assert!(nur_online_nach(0, 5_000_000, 0)); // keine Blöcke trotz Größe
        assert!(nur_online_nach(0, 0, 0)); // klassische Dropbox: Platzhalter mit 0 Byte
        assert!(!nur_online_nach(0, 5_000_000, 9800)); // normal lokal
    }

    /// R-VOR-05
    #[test]
    fn sips_meldung_platzhalter() {
        assert!(fehler_heisst_nur_online(
            "Warning: /Users/kim/x.jpg not a valid file - skipping"
        ));
        assert!(!fehler_heisst_nur_online("Error: unsupported format"));
    }

    /// R-VOR-03
    #[test]
    fn hash_stabil() {
        assert_eq!(fnv1a("abc"), 0xe71fa2190541574b);
        assert_eq!(fnv1a(""), 0xcbf29ce484222325);
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("view-thumbs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// R-VOR-07
    #[test]
    fn cache_ziel_aufbau() {
        let z = cache_ziel(Path::new("/c"), "/a/b.jpg", 640, 100, 5000);
        assert!(z.starts_with("/c/640"));
        assert_eq!(z.extension().unwrap(), "jpg");
        assert_eq!(z.file_stem().unwrap().len(), 16);
        // gleiche Eingabe → gleicher Schlüssel; Änderung an Zeit, Größe oder Pfad → neuer Schlüssel
        assert_eq!(z, cache_ziel(Path::new("/c"), "/a/b.jpg", 640, 100, 5000));
        assert_ne!(z, cache_ziel(Path::new("/c"), "/a/b.jpg", 640, 101, 5000));
        assert_ne!(z, cache_ziel(Path::new("/c"), "/a/b.jpg", 640, 100, 5001));
        assert_ne!(z, cache_ziel(Path::new("/c"), "/a/c.jpg", 640, 100, 5000));
        assert_ne!(
            cache_ziel(Path::new("/c"), "/a/x.pdf#seite=1", 640, 1, 1),
            cache_ziel(Path::new("/c"), "/a/x.pdf#seite=2", 640, 1, 1)
        );
    }

    /// R-VOR-08
    #[test]
    fn cache_treffer_ohne_werkzeug() {
        let d = tmp("treffer");
        let bild = d.join("foto.jpg");
        std::fs::write(&bild, vec![1u8; 40_000]).unwrap();
        let meta = std::fs::metadata(&bild).unwrap();
        let pfad = bild.to_str().unwrap();
        let cache = d.join("cache");
        let ziel = cache_ziel(&cache, pfad, 200, meta.mtime(), meta.len());
        std::fs::create_dir_all(ziel.parent().unwrap()).unwrap();
        std::fs::write(&ziel, b"jpeg").unwrap();
        assert_eq!(holen(&cache, pfad, 200), Vorschau::Bild { datei: ziel.to_string_lossy().into() });
    }

    /// R-VOR-09
    #[test]
    fn platzhalter_wird_nicht_angefasst() {
        let d = tmp("platzhalter");
        let bild = d.join("nur-online.jpg");
        std::fs::write(&bild, b"").unwrap();
        let cache = d.join("cache");
        assert_eq!(holen(&cache, bild.to_str().unwrap(), 200), Vorschau::NurOnline);
        assert!(ist_nur_online(&std::fs::metadata(&bild).unwrap()));
        assert!(!cache.join("200").exists(), "für Platzhalter wird nichts erzeugt");
    }

    /// R-VOR-14
    #[test]
    fn jpeg_masse_lesen() {
        let foto = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/daten/foto.jpg")).unwrap();
        assert_eq!(jpeg_masse(&foto), Some((300, 200)));
        assert_eq!(jpeg_masse(b"kein jpeg"), None);
        assert_eq!(jpeg_masse(b"\xFF\xD8kurz"), None);
        assert_eq!(jpeg_masse(&foto[..40]), None, "abgeschnitten → None, kein Absturz");
    }

    /// R-VOR-15
    #[test]
    fn verkleinern_nur_wenn_noetig() {
        assert!(zu_gross(Some((960, 640)), 640));
        assert!(zu_gross(Some((427, 641)), 640));
        assert!(!zu_gross(Some((640, 480)), 640));
        assert!(!zu_gross(Some((300, 200)), 640), "nie vergrößern");
        assert!(!zu_gross(None, 640), "unbekannte Maße → unverändert lassen");
        // kleine Datei bleibt unangetastet (ohne sips)
        let d = tmp("aufgroesse");
        let f = d.join("klein.jpg");
        std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/daten/foto.jpg"), &f).unwrap();
        let vorher = std::fs::read(&f).unwrap();
        auf_groesse(&f, 640).unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), vorher);
    }

    /// R-VOR-11
    #[test]
    fn eindeutig_steigt() {
        let a = eindeutig();
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(eindeutig() > a);
    }

    /// R-VOR-12
    #[test]
    fn download_erlauben_laeuft() {
        download_beim_lesen_erlauben(); // darf nicht abstürzen, mehrfach aufrufbar
        download_beim_lesen_erlauben();
    }

    /// R-VOR-13
    #[test]
    fn vorschau_json_format() {
        let j = |v: &Vorschau| serde_json::to_string(v).unwrap();
        assert_eq!(j(&Vorschau::Bild { datei: "/x.jpg".into() }), r#"{"art":"bild","datei":"/x.jpg"}"#);
        assert_eq!(j(&Vorschau::NurOnline), r#"{"art":"nurOnline"}"#);
        assert_eq!(j(&Vorschau::Fehlt), r#"{"art":"fehlt"}"#);
        assert_eq!(j(&Vorschau::Fehler { meldung: "m".into() }), r#"{"art":"fehler","meldung":"m"}"#);
        assert_eq!(j(&Vorschau::Datenbank { schluessel: "h#1".into() }), r#"{"art":"datenbank","schluessel":"h#1"}"#);
    }
}
