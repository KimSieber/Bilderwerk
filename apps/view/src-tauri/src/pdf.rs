//! PDF-Seiten mit dem macOS-eigenen PDFKit als JPEG ausgeben – ohne externes Werkzeug.

use std::path::Path;

/// Seite `seite` (ab 1) von `datei` als JPEG nach `ziel`, längste Kante `groesse` px.
#[cfg(target_os = "macos")]
pub fn seite_als_jpeg(datei: &Path, seite: u32, groesse: u32, ziel: &Path) -> Result<(), String> {
    use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImageCompressionFactor};
    use objc2_foundation::{NSDictionary, NSNumber, NSSize, NSString, NSURL};
    use objc2::AnyThread;
    use objc2_pdf_kit::{PDFDisplayBox, PDFDocument};

    objc2::rc::autoreleasepool(|_| unsafe {
        let pfad = NSString::from_str(&datei.to_string_lossy());
        let url = NSURL::fileURLWithPath(&pfad);
        let doc = PDFDocument::initWithURL(PDFDocument::alloc(), &url)
            .ok_or_else(|| format!("PDF lässt sich nicht öffnen: {}", datei.display()))?;
        let anzahl = doc.pageCount();
        if seite == 0 || seite as usize > anzahl {
            return Err(format!("Seite {seite} gibt es nicht (PDF hat {anzahl} Seiten)"));
        }
        let page = doc.pageAtIndex(seite as usize - 1).ok_or("Seite nicht lesbar")?;

        // Seitenmaße (Drehung 90°/270° beachten) → Vorschaugröße ohne Ränder
        let r = page.boundsForBox(PDFDisplayBox::MediaBox);
        let (mut w, mut h) = (r.size.width, r.size.height);
        if page.rotation().rem_euclid(180) == 90 {
            std::mem::swap(&mut w, &mut h);
        }
        if w <= 0.0 || h <= 0.0 {
            return Err("Seite hat keine Fläche".into());
        }
        let s = groesse as f64 / w.max(h);
        let bild = page.thumbnailOfSize_forBox(NSSize::new((w * s).round(), (h * s).round()), PDFDisplayBox::MediaBox);

        let tiff = bild.TIFFRepresentation().ok_or("Seite ließ sich nicht zeichnen")?;
        let rep = NSBitmapImageRep::imageRepWithData(&tiff).ok_or("Bilddaten ungültig")?;
        let qualitaet = NSNumber::new_f64(0.82);
        let props = NSDictionary::from_slices(&[NSImageCompressionFactor], &[qualitaet.as_ref() as &objc2::runtime::AnyObject]);
        let jpeg = rep
            .representationUsingType_properties(NSBitmapImageFileType::JPEG, &props)
            .ok_or("JPEG ließ sich nicht erzeugen")?;
        std::fs::write(ziel, jpeg.to_vec()).map_err(|e| e.to_string())
    })
}

#[cfg(not(target_os = "macos"))]
pub fn seite_als_jpeg(_datei: &Path, _seite: u32, _groesse: u32, _ziel: &Path) -> Result<(), String> {
    Err("PDF-Vorschau gibt es nur unter macOS".into())
}
