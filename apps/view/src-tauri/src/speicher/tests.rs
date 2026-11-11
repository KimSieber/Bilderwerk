//! R-ANB: Unit-Tests der Anbieter-Schnittstelle (laufen auch ohne macOS-Werkzeuge).
use super::*;
use crate::thumbs::{cache_ziel, Vorschau};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("view-anb-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Normale Datei (40 KB), 0-Byte-Platzhalter (klassische Dropbox),
/// „dataless“-Datei mit Größe, aber ohne belegte Blöcke (File Provider).
fn dateien(d: &Path) -> (String, String, String) {
    let normal = d.join("normal.jpg");
    std::fs::write(&normal, vec![7u8; 40_000]).unwrap();
    let null = d.join("platzhalter.jpg");
    std::fs::write(&null, b"").unwrap();
    let duenn = d.join("dataless.jpg");
    std::fs::File::create(&duenn).unwrap().set_len(5_000_000).unwrap();
    let s = |p: PathBuf| p.to_string_lossy().into_owned();
    (s(normal), s(null), s(duenn))
}

fn anbieter(cache: &Path) -> [(&'static str, Box<dyn Speicheranbieter>); 2] {
    [("lokal", Box::new(Lokal::new(cache))), ("file-provider", Box::new(FileProvider::new(cache)))]
}

/// R-ANB-01
#[test]
fn zustand_erkennen() {
    let d = tmp("zustand");
    let (normal, null, duenn) = dateien(&d);
    for (name, a) in anbieter(&d.join("c")) {
        assert_eq!(a.zustand(&normal), Zustand::Lokal, "{name}");
        assert_eq!(a.zustand(&null), Zustand::NurOnline, "{name}");
        assert_eq!(a.zustand("/gibt/es/nicht.jpg"), Zustand::Fehlt, "{name}");
        assert_eq!(a.zustand(&format!("{normal}#seite=2")), Zustand::Lokal, "{name}: PDF-Seite → Datei");
    }
    // dataless: lokal = nur online; File Provider = holt beim Lesen selbst → gilt als nur online, ist aber lesbar
    assert_eq!(Lokal::new(&d).zustand(&duenn), Zustand::NurOnline);
    assert_eq!(FileProvider::new(&d).zustand(&duenn), Zustand::NurOnline);
}

/// R-ANB-02
#[test]
fn vorschau_fehlt_und_platzhalter() {
    let d = tmp("vor");
    let (_, null, _) = dateien(&d);
    for (name, a) in anbieter(&d.join("c")) {
        assert_eq!(a.vorschau("/gibt/es/nicht.jpg", 200), Vorschau::Fehlt, "{name}");
        assert_eq!(a.vorschau(&null, 200), Vorschau::NurOnline, "{name}: 0 Byte ist nie lesbar");
    }
    assert!(!d.join("c").join("200").exists(), "für Platzhalter entsteht nichts im Cache");
}

/// R-ANB-03
#[test]
fn vorschau_aus_cache() {
    let d = tmp("cache");
    let (normal, _, duenn) = dateien(&d);
    let cache = d.join("c");
    for p in [&normal, &duenn] {
        let m = std::fs::metadata(p).unwrap();
        let z = cache_ziel(&cache, p, 200, m.mtime(), m.len());
        std::fs::create_dir_all(z.parent().unwrap()).unwrap();
        std::fs::write(&z, b"jpeg").unwrap();
    }
    for (name, a) in anbieter(&cache) {
        for p in [&normal, &duenn] {
            assert!(matches!(a.vorschau(p, 200), Vorschau::Bild { .. }), "{name}: {p}");
        }
    }
}

/// R-ANB-04
#[test]
fn dataless_lokal_wolke_file_provider_liest() {
    let d = tmp("dataless");
    let (_, _, duenn) = dateien(&d);
    let (cl, cf) = (d.join("cl"), d.join("cf"));
    assert_eq!(Lokal::new(&cl).vorschau(&duenn, 200), Vorschau::NurOnline);
    assert!(!cl.join("200").exists(), "lokal: kein Erzeugungsversuch");
    // File Provider versucht zu erzeugen (Lesen lädt nach) – der Inhalt ist hier kein Bild,
    // entscheidend ist nur, dass der Versuch stattfindet.
    let _ = FileProvider::new(&cf).vorschau(&duenn, 200);
    assert!(cf.join("200").is_dir(), "file-provider: Erzeugungsversuch");
}

/// R-ANB-05
#[test]
fn original_bereitstellen() {
    let d = tmp("original");
    let (normal, null, duenn) = dateien(&d);
    for (name, a) in anbieter(&d.join("c")) {
        assert_eq!(a.original_bereitstellen(&normal).unwrap(), PathBuf::from(&normal), "{name}");
        assert_eq!(
            a.original_bereitstellen(&format!("{normal}#seite=3")).unwrap(),
            PathBuf::from(&normal),
            "{name}: PDF-Seite → ganze Datei"
        );
        assert!(a.original_bereitstellen("/gibt/es/nicht.jpg").unwrap_err().contains("nicht gefunden"), "{name}");
        assert!(a.original_bereitstellen(&null).unwrap_err().contains("nur online"), "{name}");
    }
    assert!(Lokal::new(&d).original_bereitstellen(&duenn).unwrap_err().contains("nur online"));
    assert_eq!(FileProvider::new(&d).original_bereitstellen(&duenn).unwrap(), PathBuf::from(&duenn));
}

/// R-ANB-06
#[test]
fn art_und_name() {
    assert_eq!(Lokal::new(Path::new("/c")).art(), "lokal");
    assert_eq!(FileProvider::new(Path::new("/c")).art(), "file-provider");
    // Anbieter lassen sich zwischen Threads teilen (Befehle laufen im Thread-Pool)
    fn teilbar<T: Send + Sync>() {}
    teilbar::<Lokal>();
    teilbar::<FileProvider>();
    let _: std::sync::Arc<dyn Speicheranbieter> = std::sync::Arc::new(Lokal::new(Path::new("/c")));
}
