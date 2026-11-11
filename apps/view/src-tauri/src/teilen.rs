//! macOS-Teilen (v0.0.6): Systemmenü „Teilen“ (NSSharingServicePicker) für das Original.
//! Muss im Hauptthread laufen; die Position kommt vom Knopf in der Oberfläche (CSS-Pixel = Punkte).

/// Knopf-Rechteck aus der Oberfläche (oben-links-Ursprung) → AppKit-Rechteck (unten-links) im Inhaltsbereich.
pub fn rechteck_appkit(x: f64, y: f64, b: f64, h: f64, hoehe_ansicht: f64) -> (f64, f64, f64, f64) {
    (x, (hoehe_ansicht - y - h).max(0.0), b.max(1.0), h.max(1.0))
}

#[cfg(target_os = "macos")]
thread_local! {
    /// Auswahlmenü bleibt am Leben, solange es offen ist (nur im Hauptthread benutzt)
    static OFFEN: std::cell::RefCell<Option<objc2::rc::Retained<objc2_app_kit::NSSharingServicePicker>>> =
        const { std::cell::RefCell::new(None) };
}

/// Zeigt „Teilen“ für `datei` am Knopf. `ns_window` = Zeiger auf das NSWindow (Tauri `ns_window()`).
/// Aufruf nur im Hauptthread.
#[cfg(target_os = "macos")]
pub fn zeigen(ns_window: *mut std::ffi::c_void, datei: &std::path::Path, knopf: (f64, f64, f64, f64)) -> Result<(), String> {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{AnyThread, MainThreadMarker};
    use objc2_app_kit::{NSSharingServicePicker, NSWindow};
    use objc2_foundation::{NSArray, NSPoint, NSRect, NSRectEdge, NSSize, NSString, NSURL};

    let _mtm = MainThreadMarker::new().ok_or("Teilen nur im Hauptthread")?;
    if ns_window.is_null() {
        return Err("Kein Fenster".into());
    }
    unsafe {
        let fenster: &NSWindow = &*(ns_window as *const NSWindow);
        let ansicht = fenster.contentView().ok_or("Fenster ohne Inhalt")?;
        let hoehe = ansicht.bounds().size.height;
        let (x, y, b, h) = rechteck_appkit(knopf.0, knopf.1, knopf.2, knopf.3, hoehe);
        let url = NSURL::fileURLWithPath(&NSString::from_str(&datei.to_string_lossy()));
        let obj: Retained<AnyObject> = Retained::into_super(Retained::into_super(url));
        let liste = NSArray::from_retained_slice(&[obj]);
        let picker = NSSharingServicePicker::initWithItems(NSSharingServicePicker::alloc(), &liste);
        picker.showRelativeToRect_ofView_preferredEdge(
            NSRect::new(NSPoint::new(x, y), NSSize::new(b, h)),
            &ansicht,
            NSRectEdge::MinY,
        );
        OFFEN.with(|o| *o.borrow_mut() = Some(picker));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// R-TEIL-01
    #[test]
    fn koordinaten_umrechnen() {
        // Knopf 20 px unter der Oberkante, 24 hoch, Fenster 800 hoch → AppKit-y = 800-20-24
        assert_eq!(super::rechteck_appkit(100.0, 20.0, 30.0, 24.0, 800.0), (100.0, 756.0, 30.0, 24.0));
        assert_eq!(super::rechteck_appkit(0.0, 900.0, 0.0, 0.0, 800.0), (0.0, 0.0, 1.0, 1.0), "begrenzt");
    }
}
