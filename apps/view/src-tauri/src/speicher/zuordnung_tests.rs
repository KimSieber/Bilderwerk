//! R-ZUO: Zuordnung Pfad → Anbieter und Pfad-Umsetzung lokal → Dropbox.
use super::*;

fn r(pfad: &str, art: &str) -> Regel {
    Regel { pfad: pfad.into(), art: art.into() }
}

/// R-ZUO-01
#[test]
fn regeln_pruefen_gueltig() {
    let v = regeln_pruefen(vec![r("/Users/kim/Dropbox", "dropbox-klassisch"), r("/", "lokal")]).unwrap();
    assert_eq!(v, vec![r("/Users/kim/Dropbox", "dropbox-klassisch"), r("/", "lokal")]);
    // abschließender Schrägstrich und Leerzeichen werden entfernt (außer bei "/")
    assert_eq!(regeln_pruefen(vec![r(" /a/ ", "lokal")]).unwrap(), vec![r("/a", "lokal")]);
    assert_eq!(regeln_pruefen(vec![]).unwrap(), vec![], "leere Liste = alles lokal");
}

/// R-ZUO-02
#[test]
fn regeln_pruefen_fehler() {
    assert!(regeln_pruefen(vec![r("/a", "ftp")]).unwrap_err().contains("ftp"));
    assert!(regeln_pruefen(vec![r("relativ", "lokal")]).unwrap_err().contains("absolut"));
    assert!(regeln_pruefen(vec![r("", "lokal")]).unwrap_err().contains("absolut"));
    assert!(regeln_pruefen(vec![r("/a", "lokal"), r("/a/", "file-provider")]).unwrap_err().contains("mehreren"));
}

/// R-ZUO-03
#[test]
fn laengster_praefix_gewinnt() {
    let regeln = vec![r("/", "lokal"), r("/Users/kim/Dropbox", "dropbox-klassisch"), r("/Users/kim/Dropbox/Privat", "lokal")];
    assert_eq!(art_fuer(&regeln, "/Users/kim/Dropbox/Archiv/x.jpg"), "dropbox-klassisch");
    assert_eq!(art_fuer(&regeln, "/Users/kim/Dropbox/Privat/x.jpg"), "lokal");
    assert_eq!(art_fuer(&regeln, "/Users/kim/Bilder/x.jpg"), "lokal");
    assert_eq!(art_fuer(&regeln, "/Users/kim/Dropbox"), "dropbox-klassisch", "genau der Ordner");
}

/// R-ZUO-04
#[test]
fn nur_ganze_ordnernamen() {
    let regeln = vec![r("/", "lokal"), r("/Users/kim/Dropbox", "dropbox-klassisch")];
    assert_eq!(art_fuer(&regeln, "/Users/kim/Dropbox (Firma)/x.jpg"), "lokal");
    assert_eq!(art_fuer(&regeln, "/Users/kim/Dropbox2/x.jpg"), "lokal");
}

/// R-ZUO-05
#[test]
fn ohne_passende_regel_lokal() {
    assert_eq!(art_fuer(&[], "/x.jpg"), "lokal");
    assert_eq!(art_fuer(&[r("/Volumes/NAS", "file-provider")], "/Users/x.jpg"), "lokal");
}

/// R-ZUO-06
#[test]
fn pdf_seite_nach_datei() {
    let regeln = vec![r("/D", "file-provider")];
    assert_eq!(art_fuer(&regeln, "/D/Album.pdf#seite=3"), "file-provider");
}

/// R-ZUO-07
#[test]
fn dropbox_pfad_umsetzen() {
    assert_eq!(dropbox_pfad("/Users/kim/Dropbox", "/Users/kim/Dropbox/Archiv/x.jpg").as_deref(), Some("/Archiv/x.jpg"));
    assert_eq!(
        dropbox_pfad("/Users/kim/Dropbox", "/Users/kim/Dropbox/Album.pdf#seite=2").as_deref(),
        Some("/Album.pdf"),
        "PDF-Seite → Datei"
    );
    assert_eq!(dropbox_pfad("/Users/kim/Dropbox", "/Users/kim/Bilder/x.jpg"), None);
    assert_eq!(dropbox_pfad("/Users/kim/Dropbox", "/Users/kim/Dropbox2/x.jpg"), None);
    assert_eq!(dropbox_pfad("/Users/kim/Dropbox", "/Users/kim/Dropbox"), None, "Wurzel selbst ist keine Datei");
}

/// R-ZUO-08
#[test]
fn verteiler_liefert_anbieter() {
    let c = std::path::Path::new("/tmp/view-c");
    let v = Verteiler::neu(&[r("/", "lokal"), r("/D", "file-provider"), r("/X", "dropbox-klassisch")], c, None);
    assert_eq!(v.fuer("/a/b.jpg").art(), "lokal");
    assert_eq!(v.fuer("/D/b.jpg").art(), "file-provider");
    assert_eq!(v.fuer("/X/b.jpg").art(), "dropbox-klassisch");
    assert_eq!(v.fuer("/X/Album.pdf#seite=2").art(), "dropbox-klassisch");
    // Standard ohne Konfiguration
    assert_eq!(Verteiler::standard(c).fuer("/irgendwo.jpg").art(), "lokal");
}

/// R-ZUO-09
#[test]
fn aus_regeln() {
    let c = std::path::Path::new("/tmp/view-c");
    let (v, fehler) = Verteiler::aus_regeln(vec![r("/D", "file-provider")], c, None);
    assert!(fehler.is_none());
    assert_eq!(v.fuer("/D/x.jpg").art(), "file-provider");
    let (v, fehler) = Verteiler::aus_regeln(vec![r("/D", "ftp")], c, None);
    assert!(fehler.is_some(), "Fehler wird gemeldet");
    assert_eq!(v.fuer("/D/x.jpg").art(), "lokal", "Rückfall auf lokal");
}

/// R-ZUO-10: Vorschläge für erkannte Cloud-Ordner
#[test]
fn vorschlaege_erkannter_ordner() {
    let home = std::env::temp_dir().join(format!("view-home-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let p = |s: &str| home.join(s).to_string_lossy().into_owned();
    // leerer Benutzerordner → nur "/"
    assert_eq!(vorschlaege(&home), vec![r("/", "lokal")]);
    for o in ["Dropbox", "Library/CloudStorage/OneDrive-Persönlich", "Library/CloudStorage/GoogleDrive-x", "Library/Mobile Documents"] {
        std::fs::create_dir_all(home.join(o)).unwrap();
    }
    assert_eq!(
        vorschlaege(&home),
        vec![
            r(&p("Dropbox"), "dropbox-klassisch"),
            r(&p("Library/CloudStorage/GoogleDrive-x"), "file-provider"),
            r(&p("Library/CloudStorage/OneDrive-Persönlich"), "file-provider"),
            r(&p("Library/Mobile Documents"), "file-provider"),
            r("/", "lokal"),
        ]
    );
    // ~/Dropbox als Verknüpfung (neue Dropbox unter CloudStorage) → file-provider
    std::fs::remove_dir_all(home.join("Dropbox")).unwrap();
    std::os::unix::fs::symlink(home.join("Library/CloudStorage/GoogleDrive-x"), home.join("Dropbox")).unwrap();
    assert_eq!(vorschlaege(&home)[0], r(&p("Dropbox"), "file-provider"));
    let _ = std::fs::remove_dir_all(&home);
}
