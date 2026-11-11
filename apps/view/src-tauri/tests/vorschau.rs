//! I-VOR: Vorschau-Erzeugung mit den macOS-Bordmitteln (sips, PDFKit) und Testbildern.
//! Nur auf macOS (sips gibt es nur dort).
#![cfg(target_os = "macos")]
mod gemeinsam;
use gemeinsam::*;
use bilderwerk_view_lib::thumbs::{holen, Vorschau};

/// Breite × Höhe aus einem JPEG (SOF-Marker) – ohne zusätzliche Bibliothek.
fn jpeg_masse(datei: &str) -> (u32, u32) {
    let b = std::fs::read(datei).unwrap();
    assert_eq!(&b[..2], &[0xFF, 0xD8], "kein JPEG");
    let mut i = 2;
    while i + 9 < b.len() {
        if b[i] != 0xFF { i += 1; continue; }
        let m = b[i + 1];
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        if (0xC0..=0xC3).contains(&m) {
            let h = u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32;
            let w = u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32;
            return (w, h);
        }
        i += 2 + len;
    }
    panic!("keine Bildmaße gefunden");
}

fn bild(v: Vorschau) -> String {
    match v {
        Vorschau::Bild { datei } => datei,
        x => panic!("erwartet Bild, erhalten {x:?}"),
    }
}

/// I-VOR-01
#[test]
fn jpeg_verkleinern_und_cache() {
    let c = tmp("vor-jpg");
    let quelle = daten().join("foto.jpg"); // 300 × 200
    let a = bild(holen(&c, quelle.to_str().unwrap(), 120));
    assert_eq!(jpeg_masse(&a), (120, 80));
    let b = bild(holen(&c, quelle.to_str().unwrap(), 120));
    assert_eq!(a, b, "zweiter Aufruf aus dem Cache");
    assert_eq!(std::fs::read_dir(c.join("120")).unwrap().count(), 1, "keine .part-Reste");
}

/// I-VOR-02
#[test]
fn png_umwandeln() {
    let c = tmp("vor-png");
    let a = bild(holen(&c, daten().join("grafik.png").to_str().unwrap(), 100));
    let (w, h) = jpeg_masse(&a);
    assert!(w.max(h) == 100, "{w}×{h}");
}

/// I-VOR-03
#[test]
fn pdf_seite() {
    let c = tmp("vor-pdf");
    let pdf = daten().join("album.pdf");
    let s1 = bild(holen(&c, &format!("{}#seite=1", pdf.display()), 100));
    let s2 = bild(holen(&c, &format!("{}#seite=2", pdf.display()), 100));
    assert_ne!(s1, s2);
    // album.pdf: Seiten 400 × 560 pt → hochkant, längste Kante 100 px
    let (w, h) = jpeg_masse(&s2);
    assert!(h > w && h <= 100 && h >= 99, "{w}×{h}");
    match holen(&c, &format!("{}#seite=3", pdf.display()), 100) {
        Vorschau::Fehler { meldung } => assert!(meldung.contains("Seite 3"), "{meldung}"),
        x => panic!("Seite 3 gibt es nicht, erhalten {x:?}"),
    }
}

/// I-VOR-04
#[test]
fn kaputte_datei_meldet_fehler() {
    let c = tmp("vor-kaputt");
    let f = c.join("kaputt.jpg");
    std::fs::write(&f, vec![0x42u8; 40_000]).unwrap();
    match holen(&c.join("cache"), f.to_str().unwrap(), 100) {
        Vorschau::Fehler { meldung } => assert!(!meldung.is_empty()),
        x => panic!("erwartet Fehler, erhalten {x:?}"),
    }
}

/// EXIF-Orientation (Tag 0x0112) aus einem JPEG, 1 wenn keine Angabe.
fn exif_orientierung(datei: &str) -> u16 {
    let b = std::fs::read(datei).unwrap();
    let Some(pos) = b.windows(6).position(|w| w == b"Exif\0\0") else { return 1 };
    let t = &b[pos + 6..];
    let le = &t[..2] == b"II";
    let u16_ = |o: usize| if le { u16::from_le_bytes([t[o], t[o + 1]]) } else { u16::from_be_bytes([t[o], t[o + 1]]) };
    let u32_ = |o: usize| {
        let x = [t[o], t[o + 1], t[o + 2], t[o + 3]];
        if le { u32::from_le_bytes(x) } else { u32::from_be_bytes(x) }
    };
    let ifd = u32_(4) as usize;
    for i in 0..u16_(ifd) as usize {
        let e = ifd + 2 + i * 12;
        if u16_(e) == 0x0112 {
            return u16_(e + 8);
        }
    }
    1
}

/// I-VOR-05: Hochkant-Foto (EXIF Orientation 6, Pixel quer) erscheint hochkant –
/// entweder sind die Pixel gedreht, oder die Vorschau trägt die Drehangabe weiter (WebKit dreht beim Anzeigen).
#[test]
fn exif_drehung() {
    let c = tmp("vor-exif");
    let a = bild(holen(&c, daten().join("foto-gedreht.jpg").to_str().unwrap(), 120));
    let (w, h) = jpeg_masse(&a);
    let o = exif_orientierung(&a);
    let hochkant = match o {
        5..=8 => w > h, // Drehangabe vorhanden: Pixel quer, Anzeige hochkant
        _ => h > w,     // keine Drehangabe: Pixel müssen hochkant sein
    };
    assert!(hochkant, "nicht hochkant: {w}×{h}, Orientation {o}");
}

/// Prüft den Helfer selbst am Testbild.
#[test]
fn exif_helfer() {
    assert_eq!(exif_orientierung(daten().join("foto-gedreht.jpg").to_str().unwrap()), 6);
    assert_eq!(exif_orientierung(daten().join("foto.jpg").to_str().unwrap()), 1);
}

/// I-VOR-06: File-Provider-Anbieter erzeugt lokale Bilder genauso wie `lokal`.
#[test]
fn file_provider_lokale_datei() {
    use bilderwerk_view_lib::speicher::{FileProvider, Speicheranbieter};
    let c = tmp("vor-fp");
    let a = bild(FileProvider::new(&c).vorschau(daten().join("foto.jpg").to_str().unwrap(), 120));
    assert_eq!(jpeg_masse(&a), (120, 80));
}
