// öffentlich, damit die Integrationstests (src-tauri/tests/) sie erreichen
pub mod ablauf;
pub mod beobachter;
pub mod aktionen;
pub mod cache;
pub mod db;
pub mod details;
pub mod facetten;
pub mod gross;
pub mod pdf;
pub mod search;
pub mod settings;
pub mod speicher;
pub mod teilen;
pub mod thumbs;
pub mod vorschau_db;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tauri::{Emitter, Manager, State};

/// Geöffnete Datenbank (nur lesend) samt Pfad.
struct AppState {
    db: Mutex<Option<(String, Connection)>>,
    /// zweite Verbindung nur für Vorschauen aus der DB – None, wenn die DB keine Tabelle `vorschauen` hat
    vorschau_db: Mutex<Option<Connection>>,
    /// Ordner des Vorschau-Caches (…/Caches/de.sieber-bw.bilderwerk.view/vorschau)
    cache: PathBuf,
    /// erkennt, wenn Dropbox die DB-Sicherung ersetzt (v0.0.7)
    beobachter: Mutex<beobachter::Beobachter>,
    /// Ordner der großen Vorschauen (…/Caches/…/gross) – nicht Teil des Vorschau-Caches, beim Start geleert
    gross: PathBuf,
    /// Pfad → Speicheranbieter und Dropbox-Konto nach den Einstellungen – beim Speichern im
    /// Einstellungsdialog neu aufgebaut (v0.3), laufende Vorschauen behalten ihren Stand
    speicher: RwLock<speicher::Speicher>,
}

impl AppState {
    fn verteiler(&self) -> Arc<speicher::Verteiler> {
        self.speicher.read().map(|s| s.verteiler.clone()).unwrap_or_else(|e| e.into_inner().verteiler.clone())
    }
    fn konto(&self) -> Option<Arc<speicher::Konto>> {
        self.speicher.read().map(|s| s.konto.clone()).unwrap_or_else(|e| e.into_inner().konto.clone())
    }
}

/// Dropbox-Konto mit Token im macOS-Schlüsselbund
fn konto_anlegen(key: &str) -> Arc<speicher::Konto> {
    Arc::new(speicher::Konto::neu(key, Box::new(speicher::dropbox::token::Schluesselbund)))
}

fn config_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

/// Liefert die Einstellungen und versucht, die konfigurierte DB zu öffnen.
/// Antwort: Info zur DB oder Fehlertext (dann zeigt die Oberfläche die DB-Auswahl).
#[tauri::command]
async fn start(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<db::DbInfo, String> {
    let r = ablauf::start(&config_dir(&app)?)?;
    merken(&state, r)
}

/// Neue DB wählen: öffnen, prüfen, im Erfolgsfall als Einstellung speichern.
#[tauri::command]
async fn db_setzen(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    pfad: String,
) -> Result<db::DbInfo, String> {
    let r = ablauf::db_setzen(&config_dir(&app)?, &pfad)?;
    merken(&state, r)
}

fn merken(state: &AppState, (pfad, conn, info): (String, Connection, db::DbInfo)) -> Result<db::DbInfo, String> {
    state
        .beobachter
        .lock()
        .map_err(|e| e.to_string())?
        .geladen(beobachter::stand(std::path::Path::new(&pfad)));
    // Vorschauen aus der DB (Katalog-Schema ≥ 4) – eigene Verbindung, damit Suche und Bilder sich nicht blockieren
    let vdb = db::oeffnen(&pfad).ok().filter(vorschau_db::hat_tabelle);
    *state.vorschau_db.lock().map_err(|e| e.to_string())? = vdb;
    *state.db.lock().map_err(|e| e.to_string())? = Some((pfad, conn));
    Ok(info)
}

/// Geöffnete DB neu laden und der Oberfläche melden („db-neu“ mit DbInfo).
fn neu_laden_und_melden(app: &tauri::AppHandle) -> Result<db::DbInfo, String> {
    let state = app.state::<AppState>();
    let pfad = {
        let g = state.db.lock().map_err(|e| e.to_string())?;
        g.as_ref().map(|(p, _)| p.clone()).ok_or("Keine Datenbank geöffnet.")?
    };
    let info = merken(&state, ablauf::neu_laden(&pfad)?)?;
    eprintln!("View: Katalog neu geladen – {} Bilder, {:?} Vorschauen", info.bilder, info.vorschauen);
    let _ = app.emit("db-neu", &info);
    Ok(info)
}

/// „Datenbank neu laden“ (DB-Menü, App-Menü ⌘R)
#[tauri::command]
async fn db_neu_laden(app: tauri::AppHandle) -> Result<db::DbInfo, String> {
    tauri::async_runtime::spawn_blocking(move || neu_laden_und_melden(&app)).await.map_err(|e| e.to_string())?
}

/// Hintergrund: ersetzt Dropbox die Sicherung, wird neu geladen (erst wenn die Datei stabil ist).
fn beobachten(app: tauri::AppHandle) {
    loop {
        std::thread::sleep(beobachter::ABSTAND);
        let state = app.state::<AppState>();
        let pfad = match state.db.lock() {
            Ok(g) => g.as_ref().map(|(p, _)| p.clone()),
            Err(_) => None,
        };
        let Some(pfad) = pfad else { continue };
        let jetzt = beobachter::stand(std::path::Path::new(&pfad));
        let laden = state.beobachter.lock().map(|mut b| b.pruefen(jetzt)).unwrap_or(false);
        if laden {
            if let Err(e) = neu_laden_und_melden(&app) {
                eprintln!("View: Neu laden fehlgeschlagen ({e}) – neuer Versuch folgt");
                if let Ok(mut b) = state.beobachter.lock() {
                    b.fehlgeschlagen();
                }
            }
        }
    }
}

#[tauri::command]
async fn suchen(
    state: State<'_, AppState>,
    filter: search::SuchFilter,
) -> Result<search::Ergebnis, String> {
    let guard = state.db.lock().map_err(|e| e.to_string())?;
    let (_, conn) = guard.as_ref().ok_or("Keine Datenbank geöffnet.")?;
    search::suchen(conn, &filter).map_err(|e| format!("Suchfehler: {e}"))
}

/// Facetten mit Anzahl für die aktuellen Kriterien (Stufe 3).
/// Eigene Verbindung im Thread-Pool: die Zählung (einige 100 ms bei Schlagworten) blockiert weder Suche noch Oberfläche.
#[tauri::command]
async fn facetten(
    state: State<'_, AppState>,
    filter: search::SuchFilter,
) -> Result<Vec<facetten::Facette>, String> {
    let pfad = {
        let guard = state.db.lock().map_err(|e| e.to_string())?;
        guard.as_ref().map(|(p, _)| p.clone()).ok_or("Keine Datenbank geöffnet.")?
    };
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db::oeffnen(&pfad)?;
        facetten::facetten(&conn, &filter).map_err(|e| format!("Facetten: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Vorschaubild holen. Reihenfolge: Katalog-DB (Tabelle `vorschauen`) → Cache → Anbieter (lokal, Dropbox …).
/// Läuft im Thread-Pool, blockiert die Oberfläche nicht.
#[tauri::command]
async fn vorschau(
    state: State<'_, AppState>,
    pfad: String,
    groesse: u32,
) -> Result<thumbs::Vorschau, String> {
    if let Some(schluessel) = state
        .vorschau_db
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .and_then(|c| vorschau_db::schluessel_fuer(c, &pfad))
    {
        return Ok(thumbs::Vorschau::Datenbank { schluessel });
    }
    let verteiler = state.verteiler();
    tauri::async_runtime::spawn_blocking(move || verteiler.fuer(&pfad).vorschau(&pfad, groesse.clamp(64, 2400)))
    .await
    .map_err(|e| e.to_string())
}

/// Alle Angaben eines Bildes für den Detailbereich (v0.0.6).
#[tauri::command]
async fn details(state: State<'_, AppState>, id: i64) -> Result<details::Details, String> {
    let guard = state.db.lock().map_err(|e| e.to_string())?;
    let (_, conn) = guard.as_ref().ok_or("Keine Datenbank geöffnet.")?;
    details::details(conn, id)
}

/// Große Vorschau (2048 px) auf Wunsch – aus dem Original, nicht im Vorschau-Cache.
#[tauri::command]
async fn vorschau_gross(state: State<'_, AppState>, pfad: String) -> Result<thumbs::Vorschau, String> {
    let verteiler = state.verteiler();
    let ordner = state.gross.clone();
    tauri::async_runtime::spawn_blocking(move || gross::holen(verteiler.fuer(&pfad), &ordner, &pfad))
        .await
        .map_err(|e| e.to_string())
}

/// Original bereitstellen (Dropbox „nur online“ → Download) – im Thread-Pool.
async fn original(state: &State<'_, AppState>, pfad: String) -> Result<PathBuf, String> {
    let verteiler = state.verteiler();
    tauri::async_runtime::spawn_blocking(move || verteiler.fuer(&pfad).original_bereitstellen(&pfad))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn original_oeffnen(state: State<'_, AppState>, pfad: String) -> Result<(), String> {
    aktionen::ausfuehren(aktionen::oeffnen_befehl(&original(&state, pfad).await?))
}

#[tauri::command]
async fn im_finder_zeigen(state: State<'_, AppState>, pfad: String) -> Result<(), String> {
    aktionen::ausfuehren(aktionen::finder_befehl(&original(&state, pfad).await?))
}

/// Vorschlag für den Sichern-Dialog
#[tauri::command]
fn dateiname_vorschlag(pfad: String) -> String {
    aktionen::dateiname_vorschlag(&pfad)
}

#[tauri::command]
async fn sichern_unter(state: State<'_, AppState>, pfad: String, ziel: String) -> Result<String, String> {
    let quelle = original(&state, pfad).await?;
    tauri::async_runtime::spawn_blocking(move || aktionen::sichern(&quelle, std::path::Path::new(&ziel)))
        .await
        .map_err(|e| e.to_string())?
        .map(|p| p.to_string_lossy().into_owned())
}

/// macOS-Teilen am Knopf (x, y, breite, höhe in CSS-Pixeln des Fensters)
#[tauri::command]
async fn teilen(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    pfad: String,
    knopf: (f64, f64, f64, f64),
) -> Result<(), String> {
    let datei = original(&state, pfad).await?;
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let w = window.clone();
        window
            .run_on_main_thread(move || {
                let r = w.ns_window().map_err(|e| e.to_string()).and_then(|f| teilen::zeigen(f, &datei, knopf));
                let _ = tx.send(r);
            })
            .map_err(|e| e.to_string())?;
        return rx.recv().map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, datei, knopf);
        Err("Teilen gibt es nur unter macOS".into())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DropboxStatus {
    /// App-Schlüssel in den Einstellungen (oder mitgelieferter Standard)
    eingerichtet: bool,
    /// Refresh-Token im Schlüsselbund
    angemeldet: bool,
}

#[tauri::command]
async fn dropbox_status(state: State<'_, AppState>) -> Result<DropboxStatus, String> {
    let k = state.konto();
    let eingerichtet = k.is_some();
    // Schlüsselbund-Zugriff kann kurz dauern (Freigabe-Dialog) → nicht im Hauptthread
    let angemeldet = tauri::async_runtime::spawn_blocking(move || k.is_some_and(|k| k.angemeldet()))
        .await
        .map_err(|e| e.to_string())?;
    Ok(DropboxStatus { eingerichtet, angemeldet })
}

/// Öffnet die Dropbox-Anmeldung im Browser und wartet (max. 5 min) auf den Rückruf.
#[tauri::command]
async fn dropbox_anmelden(state: State<'_, AppState>) -> Result<DropboxStatus, String> {
    let k = state.konto().ok_or("Kein Dropbox-App-Schlüssel eingetragen (Einstellungen).")?;
    let k2 = k.clone();
    tauri::async_runtime::spawn_blocking(move || {
        k2.anmelden(
            speicher::dropbox::REDIRECT_PORT,
            |url| {
                let _ = std::process::Command::new("/usr/bin/open").arg(url).status();
            },
            std::time::Duration::from_secs(300),
        )
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(DropboxStatus { eingerichtet: true, angemeldet: k.angemeldet() })
}

#[tauri::command]
async fn dropbox_abmelden(state: State<'_, AppState>) -> Result<DropboxStatus, String> {
    let k = state.konto();
    let eingerichtet = k.is_some();
    if let Some(k) = k {
        tauri::async_runtime::spawn_blocking(move || k.abmelden()).await.map_err(|e| e.to_string())??;
    }
    Ok(DropboxStatus { eingerichtet, angemeldet: false })
}

/// Einstellungen für den Dialog (fehlende Werte durch Standards ersetzt).
#[tauri::command]
fn einstellungen(app: tauri::AppHandle) -> Result<settings::Ansicht, String> {
    Ok(settings::laden(&config_dir(&app)?).ansicht())
}

/// Einstellungen aus dem Dialog speichern und sofort wirksam machen: Verteiler und Dropbox-Konto neu
/// aufbauen, Cache auf die neue Grenze bringen. Meldet „speicher-neu“, damit die Oberfläche Vorschauen neu anfordert.
#[tauri::command]
async fn einstellungen_setzen(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    aenderung: settings::Aenderung,
) -> Result<settings::Ansicht, String> {
    let dir = config_dir(&app)?;
    let cache = state.cache.clone();
    let alt = state.speicher.read().map_err(|e| e.to_string())?.clone();
    // Abmelden der alten Dropbox-App kann das Netz brauchen → Thread-Pool
    let (e, neu) = tauri::async_runtime::spawn_blocking(move || {
        let e = settings::aendern(&dir, aenderung)?;
        let (neu, fehler) = speicher::Speicher::aufbauen(&e, &cache, Some(&alt), konto_anlegen);
        if let Some(f) = fehler {
            eprintln!("View: {f} – alle Pfade werden als lokal behandelt");
        }
        let (n, frei) = cache::begrenzen(&cache, e.cache_grenze_bytes());
        if n > 0 {
            eprintln!("View: Vorschau-Cache begrenzt – {n} Dateien, {} MB freigegeben", frei / 1024 / 1024);
        }
        Ok::<_, String>((e, neu))
    })
    .await
    .map_err(|e| e.to_string())??;
    *state.speicher.write().map_err(|e| e.to_string())? = neu;
    let _ = app.emit("speicher-neu", ());
    Ok(e.ansicht())
}

/// Vorschläge für erkannte Cloud-Ordner (~/Dropbox, ~/Library/CloudStorage/…, iCloud) – für den Dialog.
#[tauri::command]
fn anbieter_vorschlaege(app: tauri::AppHandle) -> Result<Vec<speicher::Regel>, String> {
    Ok(speicher::zuordnung::vorschlaege(&app.path().home_dir().map_err(|e| e.to_string())?))
}

/// „Über Bilderwerk View“ als eigenes Fenster (v0.3) – vorhandenes Fenster nach vorn holen.
fn ueber_zeigen(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("ueber") {
        return w.set_focus();
    }
    tauri::WebviewWindowBuilder::new(app, "ueber", tauri::WebviewUrl::App("index.html?fenster=ueber".into()))
        .title("Über Bilderwerk View")
        .inner_size(420.0, 400.0)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .center()
        .build()
        .map(|_| ())
}

/// Größe des Vorschau-Caches (für das DB-Menü).
#[tauri::command]
async fn cache_info(state: State<'_, AppState>) -> Result<cache::CacheInfo, String> {
    let dir = state.cache.clone();
    tauri::async_runtime::spawn_blocking(move || cache::info(&dir)).await.map_err(|e| e.to_string())
}

/// Vorschau-Cache leeren (DB-Menü und App-Menü). Meldet „cache-geleert“, damit die Oberfläche neu lädt.
#[tauri::command]
async fn cache_leeren(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<cache::CacheInfo, String> {
    let dir = state.cache.clone();
    let info = tauri::async_runtime::spawn_blocking(move || cache::leeren(&dir)).await.map_err(|e| e.to_string())?;
    let _ = app.emit("cache-geleert", info);
    Ok(info)
}

/// Antwort auf `vorschau://localhost/<hash>` – JPEG direkt aus der Katalog-DB, ohne Umweg über Dateien.
fn vorschau_protokoll(app: &tauri::AppHandle, uri_pfad: &str) -> tauri::http::Response<Vec<u8>> {
    let hash = vorschau_db::hash_aus_uri(uri_pfad);
    let jpeg = app
        .state::<AppState>()
        .vorschau_db
        .lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(|c| vorschau_db::jpeg(c, &hash)));
    let b = tauri::http::Response::builder();
    match jpeg {
        Some(daten) => b
            .status(200)
            .header("Content-Type", "image/jpeg")
            .header("Cache-Control", "max-age=31536000, immutable")
            .body(daten),
        None => b.status(404).body(Vec::new()),
    }
    .unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    thumbs::download_beim_lesen_erlauben();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol("vorschau", |ctx, anfrage, antwort| {
            let app = ctx.app_handle().clone();
            let pfad = anfrage.uri().path().to_string();
            std::thread::spawn(move || antwort.respond(vorschau_protokoll(&app, &pfad)));
        })
        .on_menu_event(|app, ereignis| {
            if ereignis.id().0 == "db_neu_laden" {
                let app = app.clone();
                std::thread::spawn(move || {
                    if let Err(e) = neu_laden_und_melden(&app) {
                        eprintln!("View: Neu laden fehlgeschlagen ({e})");
                    }
                });
            }
            if ereignis.id().0 == "ueber" {
                if let Err(e) = ueber_zeigen(app) {
                    eprintln!("View: Über-Fenster ({e})");
                }
            }
            if ereignis.id().0 == "einstellungen" {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.set_focus();
                }
                let _ = app.emit_to("main", "einstellungen-oeffnen", ());
            }
            if ereignis.id().0 == "cache_leeren" {
                let app = app.clone();
                std::thread::spawn(move || {
                    let dir = app.state::<AppState>().cache.clone();
                    let info = cache::leeren(&dir);
                    let _ = app.emit("cache-geleert", info);
                });
            }
        })
        .setup(|app| {
            let cache = app.path().app_cache_dir()?.join("vorschau");
            let gross = app.path().app_cache_dir()?.join("gross");
            gross::leeren(&gross);

            // App-Menü: Standard + „Katalog → Datenbank neu laden (⌘R) · Vorschau-Cache leeren“
            use tauri::menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu};
            let neu = MenuItem::with_id(app, "db_neu_laden", "Datenbank neu laden", true, Some("CmdOrCtrl+R"))?;
            let leeren = MenuItem::with_id(app, "cache_leeren", "Vorschau-Cache leeren", true, None::<&str>)?;
            let untermenue = Submenu::with_items(app, "Katalog", true, &[&neu, &leeren])?;
            let menue = Menu::default(app.handle())?;
            // „Über Bilderwerk View“ (eigenes Fenster) und „Einstellungen…“ (⌘,) im App-Menü (v0.3)
            if let Some(MenuItemKind::Submenu(app_menue)) = menue.items()?.into_iter().next() {
                let ueber = MenuItem::with_id(app, "ueber", "Über Bilderwerk View", true, None::<&str>)?;
                let einst = MenuItem::with_id(app, "einstellungen", "Einstellungen…", true, Some("CmdOrCtrl+,"))?;
                app_menue.remove_at(0)?;
                app_menue.insert(&ueber, 0)?;
                app_menue.insert(&PredefinedMenuItem::separator(app)?, 1)?;
                app_menue.insert(&einst, 2)?;
            }
            menue.append(&untermenue)?;
            app.handle().set_menu(menue)?;

            let einst = settings::laden(&app.path().app_config_dir()?);

            // Cache beim Start auf die Obergrenze bringen (im Hintergrund)
            let grenze = einst.cache_grenze_bytes();
            let dir = cache.clone();
            std::thread::spawn(move || {
                let (n, frei) = cache::begrenzen(&dir, grenze);
                if n > 0 {
                    eprintln!("View: Vorschau-Cache begrenzt – {n} Dateien, {} MB freigegeben", frei / 1024 / 1024);
                }
            });

            let (speicher, fehler) = speicher::Speicher::aufbauen(&einst, &cache, None, konto_anlegen);
            if let Some(f) = fehler {
                eprintln!("View: {f} – alle Pfade werden als lokal behandelt");
            }
            app.manage(AppState {
                db: Mutex::new(None),
                vorschau_db: Mutex::new(None),
                beobachter: Mutex::new(beobachter::Beobachter::default()),
                cache,
                gross,
                speicher: RwLock::new(speicher),
            });
            let h = app.handle().clone();
            std::thread::spawn(move || beobachten(h));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start,
            db_setzen,
            einstellungen,
            einstellungen_setzen,
            anbieter_vorschlaege,
            db_neu_laden,
            suchen,
            facetten,
            vorschau,
            details,
            vorschau_gross,
            original_oeffnen,
            im_finder_zeigen,
            dateiname_vorschlag,
            sichern_unter,
            teilen,
            dropbox_status,
            dropbox_anmelden,
            dropbox_abmelden,
            cache_info,
            cache_leeren
        ])
        .run(tauri::generate_context!())
        .expect("Fehler beim Start von Bilderwerk View");
}
