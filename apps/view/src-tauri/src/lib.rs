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
use std::sync::{Arc, Mutex};
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
    /// Pfad → Speicheranbieter, nach view.local.json "anbieter"; beim Start angelegt
    verteiler: Arc<speicher::Verteiler>,
    /// Dropbox-Konto – nur, wenn in view.local.json ein App-Schlüssel steht
    dropbox: Option<Arc<speicher::Konto>>,
}

/// Regeln aus view.local.json (build.rs), sonst None → alles lokal
const ANBIETER: Option<&str> = option_env!("PSV_ANBIETER");
/// App-Schlüssel der Dropbox-App (öffentlich, kein Secret) aus view.local.json
const DROPBOX_APP_KEY: Option<&str> = option_env!("PSV_DROPBOX_APP_KEY");

fn config_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

/// Liefert die Einstellungen und versucht, die konfigurierte DB zu öffnen.
/// Antwort: Info zur DB oder Fehlertext (dann zeigt die Oberfläche die DB-Auswahl).
#[tauri::command]
async fn start(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<db::DbInfo, String> {
    let r = ablauf::start(&config_dir(&app)?, option_env!("PSV_STANDARD_DB"))?;
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
    let verteiler = state.verteiler.clone();
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
    let verteiler = state.verteiler.clone();
    let ordner = state.gross.clone();
    tauri::async_runtime::spawn_blocking(move || gross::holen(verteiler.fuer(&pfad), &ordner, &pfad))
        .await
        .map_err(|e| e.to_string())
}

/// Original bereitstellen (Dropbox „nur online“ → Download) – im Thread-Pool.
async fn original(state: &State<'_, AppState>, pfad: String) -> Result<PathBuf, String> {
    let verteiler = state.verteiler.clone();
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
    /// App-Schlüssel in view.local.json vorhanden
    eingerichtet: bool,
    /// Refresh-Token im Schlüsselbund
    angemeldet: bool,
}

fn dropbox_zustand(state: &AppState) -> DropboxStatus {
    DropboxStatus {
        eingerichtet: state.dropbox.is_some(),
        angemeldet: state.dropbox.as_ref().is_some_and(|k| k.angemeldet()),
    }
}

#[tauri::command]
async fn dropbox_status(state: State<'_, AppState>) -> Result<DropboxStatus, String> {
    let k = state.dropbox.clone();
    // Schlüsselbund-Zugriff kann kurz dauern (Freigabe-Dialog) → nicht im Hauptthread
    let angemeldet = tauri::async_runtime::spawn_blocking(move || k.is_some_and(|k| k.angemeldet()))
        .await
        .map_err(|e| e.to_string())?;
    Ok(DropboxStatus { eingerichtet: state.dropbox.is_some(), angemeldet })
}

/// Öffnet die Dropbox-Anmeldung im Browser und wartet (max. 5 min) auf den Rückruf.
#[tauri::command]
async fn dropbox_anmelden(state: State<'_, AppState>) -> Result<DropboxStatus, String> {
    let k = state.dropbox.clone().ok_or("Kein Dropbox-App-Schlüssel in view.local.json.")?;
    tauri::async_runtime::spawn_blocking(move || {
        k.anmelden(
            speicher::dropbox::REDIRECT_PORT,
            |url| {
                let _ = std::process::Command::new("/usr/bin/open").arg(url).status();
            },
            std::time::Duration::from_secs(300),
        )
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(dropbox_zustand(&state))
}

#[tauri::command]
async fn dropbox_abmelden(state: State<'_, AppState>) -> Result<DropboxStatus, String> {
    if let Some(k) = state.dropbox.clone() {
        tauri::async_runtime::spawn_blocking(move || k.abmelden()).await.map_err(|e| e.to_string())??;
    }
    Ok(dropbox_zustand(&state))
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
            use tauri::menu::{AboutMetadata, Menu, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu};
            let neu = MenuItem::with_id(app, "db_neu_laden", "Datenbank neu laden", true, Some("CmdOrCtrl+R"))?;
            let leeren = MenuItem::with_id(app, "cache_leeren", "Vorschau-Cache leeren", true, None::<&str>)?;
            let untermenue = Submenu::with_items(app, "Katalog", true, &[&neu, &leeren])?;
            let menue = Menu::default(app.handle())?;
            // „Über Bilderwerk View“: Copyright, Webseite, Kontakt (macOS zeigt „credits“ unter dem Copyright)
            if let Some(MenuItemKind::Submenu(app_menue)) = menue.items()?.into_iter().next() {
                let info = PredefinedMenuItem::about(
                    app,
                    None,
                    Some(AboutMetadata {
                        copyright: Some("© 2026 Kim Sieber".into()),
                        website: Some("http://bilderwerk.sieber-bw.de".into()),
                        website_label: Some("bilderwerk.sieber-bw.de".into()),
                        credits: Some("http://bilderwerk.sieber-bw.de\nBilderwerk@sieber-bw.de".into()),
                        icon: app.default_window_icon().cloned(),
                        ..Default::default()
                    }),
                )?;
                app_menue.remove_at(0)?;
                app_menue.insert(&info, 0)?;
            }
            menue.append(&untermenue)?;
            app.handle().set_menu(menue)?;

            // Cache beim Start auf die Obergrenze bringen (im Hintergrund)
            let grenze = settings::laden(&app.path().app_config_dir()?).cache_grenze_bytes();
            let dir = cache.clone();
            std::thread::spawn(move || {
                let (n, frei) = cache::begrenzen(&dir, grenze);
                if n > 0 {
                    eprintln!("View: Vorschau-Cache begrenzt – {n} Dateien, {} MB freigegeben", frei / 1024 / 1024);
                }
            });

            let dropbox = DROPBOX_APP_KEY
                .filter(|k| !k.trim().is_empty())
                .map(|k| Arc::new(speicher::Konto::neu(k.trim(), Box::new(speicher::dropbox::token::Schluesselbund))));
            let (verteiler, fehler) = speicher::Verteiler::aus_konfiguration(ANBIETER, &cache, dropbox.clone());
            if let Some(f) = fehler {
                eprintln!("View: {f} – alle Pfade werden als lokal behandelt");
            }
            app.manage(AppState {
                db: Mutex::new(None),
                vorschau_db: Mutex::new(None),
                beobachter: Mutex::new(beobachter::Beobachter::default()),
                cache,
                gross,
                verteiler: Arc::new(verteiler),
                dropbox,
            });
            let h = app.handle().clone();
            std::thread::spawn(move || beobachten(h));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start,
            db_setzen,
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
