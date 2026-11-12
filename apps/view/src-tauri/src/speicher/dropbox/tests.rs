//! R-DBX: Dropbox-Anbieter gegen einen nachgebildeten HTTP-Server (kein echtes Konto).
use super::api::*;
use super::pkce::*;
use super::token::{ImSpeicher, Tokenablage};
use super::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// ---------- nachgebildeter Server ----------

#[derive(Debug, Clone)]
struct Anfrage {
    pfad: String,
    kopf: Vec<(String, String)>,
    body: String,
}
impl Anfrage {
    fn kopf(&self, n: &str) -> Option<&str> {
        self.kopf.iter().find(|(k, _)| k.eq_ignore_ascii_case(n)).map(|(_, v)| v.as_str())
    }
}
type Antwort = (u16, Vec<(&'static str, String)>, Vec<u8>);
type Handler = Arc<dyn Fn(&Anfrage, usize) -> Antwort + Send + Sync>;

struct Server {
    url: String,
    log: Arc<Mutex<Vec<Anfrage>>>,
}
impl Server {
    fn starte(h: impl Fn(&Anfrage, usize) -> Antwort + Send + Sync + 'static) -> Self {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
        let log = Arc::new(Mutex::new(Vec::new()));
        let (lg, h): (_, Handler) = (log.clone(), Arc::new(h));
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let (lg, h) = (lg.clone(), h.clone());
                std::thread::spawn(move || bediene(s, &lg, &h));
            }
        });
        Self { url, log }
    }
    fn anfragen(&self, pfad: &str) -> Vec<Anfrage> {
        self.log.lock().unwrap().iter().filter(|a| a.pfad == pfad).cloned().collect()
    }
    fn basis(&self) -> Basis {
        Basis { api: self.url.clone(), inhalt: self.url.clone() }
    }
}
fn bediene(s: TcpStream, log: &Mutex<Vec<Anfrage>>, h: &Handler) {
    let mut r = BufReader::new(s.try_clone().unwrap());
    let mut zeile = String::new();
    if r.read_line(&mut zeile).unwrap_or(0) == 0 {
        return;
    }
    let pfad = zeile.split_whitespace().nth(1).unwrap_or("").to_string();
    let mut kopf = Vec::new();
    loop {
        let mut z = String::new();
        r.read_line(&mut z).unwrap();
        let z = z.trim_end();
        if z.is_empty() {
            break;
        }
        if let Some((k, v)) = z.split_once(':') {
            kopf.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let laenge: usize = kopf.iter().find(|(k, _)| k.eq_ignore_ascii_case("content-length")).and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
    let mut body = vec![0u8; laenge];
    r.read_exact(&mut body).unwrap();
    let a = Anfrage { pfad, kopf, body: String::from_utf8_lossy(&body).into() };
    let n = {
        let mut l = log.lock().unwrap();
        l.push(a.clone());
        l.iter().filter(|x| x.pfad == a.pfad).count()
    };
    let (status, k, b) = h(&a, n);
    let mut s = s;
    let mut text = format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n", b.len());
    for (k, v) in k {
        text.push_str(&format!("{k}: {v}\r\n"));
    }
    text.push_str("\r\n");
    let _ = s.write_all(text.as_bytes());
    let _ = s.write_all(&b);
}

fn json(status: u16, j: serde_json::Value) -> Antwort {
    (status, vec![("Content-Type", "application/json".into())], j.to_string().into_bytes())
}
fn token_ok() -> Antwort {
    json(200, serde_json::json!({"access_token":"AT","expires_in":14400,"token_type":"bearer"}))
}

/// Standard-Dropbox: Token erneuern, Vorschau liefert JPEG-Bytes, Download liefert Inhalt.
fn standard(a: &Anfrage, _n: usize) -> Antwort {
    match a.pfad.as_str() {
        "/oauth2/token" => token_ok(),
        "/2/files/get_thumbnail_v2" => (200, vec![], b"\xFF\xD8vorschau".to_vec()),
        "/2/files/download" => (200, vec![], b"ORIGINAL".to_vec()),
        "/2/auth/token/revoke" => json(200, serde_json::json!(null)),
        _ => (404, vec![], vec![]),
    }
}

static GEWARTET: AtomicU64 = AtomicU64::new(0);
fn warte_merken(d: Duration) {
    GEWARTET.fetch_add(d.as_millis() as u64, Ordering::SeqCst);
}
fn warte_nicht(_: Duration) {}

fn konto(s: &Server, angemeldet: bool) -> Arc<Konto> {
    let ablage = ImSpeicher::default();
    if angemeldet {
        ablage.schreiben("RT").unwrap();
    }
    Arc::new(Konto::mit_basis("APPKEY", Box::new(ablage), s.basis(), &format!("{}/oauth2/authorize", s.url), warte_nicht))
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("view-dbx-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Dropbox-Ordner mit Platzhaltern (0 Byte) und einer lokalen Datei.
fn dropbox(d: &Path) -> String {
    let w = d.join("Dropbox");
    std::fs::create_dir_all(w.join("Archiv")).unwrap();
    for n in ["online.jpg", "groß.jpg", "foto.heic", "album.pdf", "weg.jpg"] {
        std::fs::write(w.join("Archiv").join(n), b"").unwrap();
    }
    std::fs::write(w.join("Archiv").join("lokal.jpg"), vec![1u8; 40_000]).unwrap();
    w.to_string_lossy().into_owned()
}

// ---------- PKCE und Hilfen ----------

/// R-DBX-01
#[test]
fn pkce_challenge_und_zufall() {
    // Beispiel aus RFC 7636, Anhang B
    assert_eq!(challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    let z = zufall(64);
    assert_eq!(z.len(), 64);
    assert!(z.chars().all(|c| c.is_ascii_alphanumeric() || "-._~".contains(c)));
    assert_ne!(zufall(64), z);
}

/// R-DBX-02
#[test]
fn kodieren_und_dekodieren() {
    let s = "http://127.0.0.1:53682/psv-anmeldung?x=ä b&c";
    assert_eq!(dekodieren(&kodieren(s)), s);
    assert_eq!(kodieren("a b/ä"), "a%20b%2F%C3%A4");
    assert_eq!(dekodieren("a+b%2"), "a b%2", "kaputte Kodierung bleibt stehen");
}

/// R-DBX-03
#[test]
fn anmelde_url_und_rueckruf() {
    let u = anmelde_url("https://x/authorize", "KEY", "CH", "http://127.0.0.1:1/p", "ST");
    let q = u.split_once('?').unwrap().1;
    for (k, v) in [("client_id", "KEY"), ("response_type", "code"), ("code_challenge", "CH"), ("code_challenge_method", "S256"),
                   ("token_access_type", "offline"), ("redirect_uri", "http://127.0.0.1:1/p"), ("state", "ST")] {
        assert_eq!(parameter(q, k).as_deref(), Some(v), "{k}");
    }
    assert_eq!(anfrage_zeile("GET /psv-anmeldung?code=1&state=2 HTTP/1.1"), Some(("/psv-anmeldung".into(), "code=1&state=2".into())));
    assert_eq!(anfrage_zeile("POST / HTTP/1.1"), None);
    assert_eq!(rueckruf_auswerten("code=C&state=S", "S"), Ok("C".into()));
    assert!(rueckruf_auswerten("code=C&state=X", "S").unwrap_err().contains("state"));
    assert!(rueckruf_auswerten("error=access_denied&state=S", "S").unwrap_err().contains("abgelehnt"));
    assert!(rueckruf_auswerten("state=S", "S").is_err());
}

/// R-DBX-04
#[test]
fn api_arg_nur_ascii() {
    let v = serde_json::json!({"path": "/Archiv/Größe 😀.jpg"});
    let a = api_arg(&v);
    assert!(a.is_ascii(), "{a}");
    assert!(a.contains("\\u00f6") && a.contains("\\ud83d\\ude00"), "{a}");
    assert_eq!(serde_json::from_str::<serde_json::Value>(&a).unwrap(), v, "bleibt gültiges JSON");
}

/// R-DBX-05
#[test]
fn fehlerarten() {
    assert_eq!(fehler_aus(401, None, ""), ApiFehler::Abgelaufen);
    assert_eq!(fehler_aus(429, Some("3"), ""), ApiFehler::Drosselung(Some(3)));
    assert_eq!(fehler_aus(503, None, ""), ApiFehler::Drosselung(None));
    assert_eq!(fehler_aus(409, None, r#"{"error_summary":"path/not_found/.."}"#), ApiFehler::NichtGefunden);
    assert!(matches!(fehler_aus(409, None, r#"{"error_summary":"unsupported_extension/"}"#), ApiFehler::NichtUmwandelbar(_)));
    assert!(matches!(fehler_aus(400, None, r#"{"error":"invalid_grant"}"#), ApiFehler::Anmeldung(_)));
    assert!(matches!(fehler_aus(500, None, "x"), ApiFehler::Sonst(500, _)));
}

/// R-DBX-06
#[test]
fn wiederholen_bei_drosselung() {
    GEWARTET.store(0, Ordering::SeqCst);
    let mut n = 0;
    let r = mit_wiederholung(&warte_merken, || {
        n += 1;
        if n < 3 { Err(ApiFehler::Drosselung(if n == 1 { Some(2) } else { None })) } else { Ok(n) }
    });
    assert_eq!(r, Ok(3));
    assert_eq!(GEWARTET.load(Ordering::SeqCst), 2_000 + 1_000, "Retry-After 2 s, dann 0,5·2¹ s");
    // höchstens 5 Versuche
    let mut n = 0;
    let r: Result<(), _> = mit_wiederholung(&warte_nicht, || {
        n += 1;
        Err(ApiFehler::Drosselung(None))
    });
    assert_eq!((r, n), (Err(ApiFehler::Drosselung(None)), 5));
    // andere Fehler sofort
    let mut n = 0;
    let _: Result<(), _> = mit_wiederholung(&warte_nicht, || {
        n += 1;
        Err(ApiFehler::NichtGefunden)
    });
    assert_eq!(n, 1);
}

// ---------- Konto ----------

/// Simuliert den Browser: Anmelde-URL lesen, Rückruf mit Code und state an View senden.
fn browser(url: &str, code: &str, state_ersetzen: Option<&str>) {
    let q = url.split_once('?').unwrap().1;
    let redirect = parameter(q, "redirect_uri").unwrap();
    let state = state_ersetzen.map(str::to_string).unwrap_or_else(|| parameter(q, "state").unwrap());
    let (host, pfad) = redirect.trim_start_matches("http://").split_once('/').unwrap();
    let ziel = format!("/{pfad}?code={code}&state={}", kodieren(&state));
    std::thread::spawn({
        let host = host.to_string();
        move || {
            let mut s = TcpStream::connect(host).unwrap();
            write!(s, "GET /favicon.ico HTTP/1.1\r\n\r\n").ok(); // fremde Anfrage wird abgewiesen
            let mut s = TcpStream::connect(s.peer_addr().unwrap()).unwrap();
            write!(s, "GET {ziel} HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
            let mut a = String::new();
            let _ = s.read_to_string(&mut a);
        }
    });
}

fn freier_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

/// R-DBX-07
#[test]
fn anmelden_mit_pkce() {
    let s = Server::starte(|a, _| match a.pfad.as_str() {
        "/oauth2/token" => json(200, serde_json::json!({"access_token":"AT","expires_in":14400,"refresh_token":"RT-NEU"})),
        _ => (404, vec![], vec![]),
    });
    let k = konto(&s, false);
    assert!(!k.angemeldet());
    let mut url = String::new();
    k.anmelden(freier_port(), |u| { url = u.to_string(); browser(u, "CODE1", None) }, Duration::from_secs(10)).unwrap();
    assert!(k.angemeldet());
    let t = &s.anfragen("/oauth2/token")[0];
    assert_eq!(parameter(&t.body, "grant_type").as_deref(), Some("authorization_code"));
    assert_eq!(parameter(&t.body, "code").as_deref(), Some("CODE1"));
    assert_eq!(parameter(&t.body, "client_id").as_deref(), Some("APPKEY"));
    let verifier = parameter(&t.body, "code_verifier").unwrap();
    let q = url.split_once('?').unwrap().1;
    assert_eq!(parameter(q, "code_challenge").unwrap(), challenge(&verifier), "Verifier passt zur Challenge");
    assert_eq!(parameter(&t.body, "redirect_uri"), parameter(q, "redirect_uri"));
}

/// R-DBX-08
#[test]
fn anmelden_falscher_state_und_frist() {
    let s = Server::starte(standard);
    let k = konto(&s, false);
    let e = k.anmelden(freier_port(), |u| browser(u, "C", Some("fremd")), Duration::from_secs(10)).unwrap_err();
    assert!(e.contains("state"), "{e}");
    assert!(!k.angemeldet());
    assert!(s.anfragen("/oauth2/token").is_empty(), "kein Code-Tausch");
    let e = k.anmelden(freier_port(), |_| {}, Duration::from_millis(200)).unwrap_err();
    assert!(e.contains("Zeit"), "{e}");
}

/// R-DBX-09
#[test]
fn token_wird_gemerkt_und_erneuert() {
    let s = Server::starte(standard);
    let k = konto(&s, true);
    k.vorschau("/a.jpg").unwrap();
    k.vorschau("/b.jpg").unwrap();
    assert_eq!(s.anfragen("/oauth2/token").len(), 1, "Access-Token wird gemerkt");
    let t = &s.anfragen("/oauth2/token")[0];
    assert_eq!(parameter(&t.body, "grant_type").as_deref(), Some("refresh_token"));
    assert_eq!(parameter(&t.body, "refresh_token").as_deref(), Some("RT"));
    let v = &s.anfragen("/2/files/get_thumbnail_v2")[0];
    assert_eq!(v.kopf("Authorization"), Some("Bearer AT"));
    let arg: serde_json::Value = serde_json::from_str(v.kopf("Dropbox-API-Arg").unwrap()).unwrap();
    assert_eq!(arg["resource"]["path"], "/a.jpg");
    assert_eq!((arg["size"].as_str(), arg["format"].as_str()), (Some("w960h640"), Some("jpeg")));
}

/// R-DBX-10
#[test]
fn abgelaufenes_token_wird_ersetzt() {
    let s = Server::starte(|a, n| match (a.pfad.as_str(), n) {
        ("/2/files/get_thumbnail_v2", 1) => json(401, serde_json::json!({"error_summary":"expired_access_token/"})),
        _ => standard(a, n),
    });
    let k = konto(&s, true);
    assert!(k.vorschau("/a.jpg").is_ok());
    assert_eq!(s.anfragen("/oauth2/token").len(), 2);
    assert_eq!(s.anfragen("/2/files/get_thumbnail_v2").len(), 2);
}

/// R-DBX-11
#[test]
fn drosselung_wird_wiederholt() {
    let s = Server::starte(|a, n| match (a.pfad.as_str(), n) {
        ("/2/files/get_thumbnail_v2", 1 | 2) => (429, vec![("Retry-After", "1".into())], vec![]),
        _ => standard(a, n),
    });
    assert!(konto(&s, true).vorschau("/a.jpg").is_ok());
    assert_eq!(s.anfragen("/2/files/get_thumbnail_v2").len(), 3);
}

/// R-DBX-12
#[test]
fn abmelden_widerruft_und_loescht() {
    let s = Server::starte(standard);
    let k = konto(&s, true);
    k.abmelden().unwrap();
    assert!(!k.angemeldet());
    assert_eq!(s.anfragen("/2/auth/token/revoke")[0].kopf("Authorization"), Some("Bearer AT"));
    k.abmelden().unwrap(); // zweimal ist harmlos
}

/// R-DBX-13
#[test]
fn widerrufene_anmeldung() {
    let s = Server::starte(|a, n| match a.pfad.as_str() {
        "/oauth2/token" => json(400, serde_json::json!({"error":"invalid_grant"})),
        _ => standard(a, n),
    });
    assert!(matches!(konto(&s, true).vorschau("/a.jpg"), Err(ApiFehler::Anmeldung(_))));
}

// ---------- Anbieter ----------

fn anbieter(d: &Path, k: Option<Arc<Konto>>) -> (DropboxKlassisch, String) {
    let w = dropbox(d);
    (DropboxKlassisch::new(&w, &d.join("cache"), k), w)
}

/// R-DBX-14
#[test]
fn ohne_konto_oder_anmeldung_wolke() {
    let d = tmp("ohne");
    let s = Server::starte(standard);
    let (a, w) = anbieter(&d, None);
    assert_eq!(a.vorschau(&format!("{w}/Archiv/online.jpg"), 200), Vorschau::NurOnline);
    let (a, w) = anbieter(&d, Some(konto(&s, false)));
    assert_eq!(a.vorschau(&format!("{w}/Archiv/online.jpg"), 200), Vorschau::NurOnline);
    assert_eq!(a.vorschau(&format!("{w}/Archiv/fehlt-ganz.jpg"), 200), Vorschau::Fehlt);
    assert!(s.log.lock().unwrap().is_empty(), "keine Netzanfrage");
    assert_eq!(a.art(), "dropbox-klassisch");
}

/// R-DBX-15
#[test]
fn vorschau_ueber_api_und_cache() {
    let d = tmp("api");
    let s = Server::starte(standard);
    let (a, w) = anbieter(&d, Some(konto(&s, true)));
    let p = format!("{w}/Archiv/groß.jpg");
    let Vorschau::Bild { datei } = a.vorschau(&p, 200) else { panic!() };
    assert_eq!(std::fs::read(&datei).unwrap(), b"\xFF\xD8vorschau");
    assert!(matches!(a.vorschau(&p, 200), Vorschau::Bild { .. }));
    let v = s.anfragen("/2/files/get_thumbnail_v2");
    assert_eq!(v.len(), 1, "zweites Mal aus dem Cache");
    let arg: serde_json::Value = serde_json::from_str(v[0].kopf("Dropbox-API-Arg").unwrap()).unwrap();
    assert_eq!(arg["resource"]["path"], "/Archiv/groß.jpg", "lokaler Pfad → Dropbox-Pfad, Umlaut erhalten");
    assert!(s.anfragen("/2/files/download").is_empty());
    // Datei bleibt Platzhalter
    assert_eq!(std::fs::metadata(format!("{w}/Archiv/groß.jpg")).unwrap().len(), 0);
}

/// R-DBX-16
#[test]
fn lokale_datei_ohne_netz() {
    let d = tmp("lokal");
    let s = Server::starte(standard);
    let (a, w) = anbieter(&d, Some(konto(&s, true)));
    let p = format!("{w}/Archiv/lokal.jpg");
    let m = std::fs::metadata(&p).unwrap();
    let z = cache_ziel(&d.join("cache"), &p, 200, m.mtime(), m.len());
    std::fs::create_dir_all(z.parent().unwrap()).unwrap();
    std::fs::write(&z, b"x").unwrap();
    assert!(matches!(a.vorschau(&p, 200), Vorschau::Bild { .. }));
    let _ = a.vorschau(&format!("{w}/Archiv/lokal.jpg#x"), 200); // nicht im Cache → lokal erzeugt, kein Netz
    assert!(s.log.lock().unwrap().is_empty(), "lokale Dateien nie über das Netz");
    assert_eq!(a.zustand(&p), Zustand::Lokal);
    assert_eq!(a.zustand(&format!("{w}/Archiv/online.jpg")), Zustand::NurOnline);
}

/// R-DBX-17
#[test]
fn nicht_gefunden_und_offline() {
    let d = tmp("weg");
    let s = Server::starte(|a, n| match a.pfad.as_str() {
        "/2/files/get_thumbnail_v2" => json(409, serde_json::json!({"error_summary":"path/not_found/"})),
        _ => standard(a, n),
    });
    let (a, w) = anbieter(&d, Some(konto(&s, true)));
    assert_eq!(a.vorschau(&format!("{w}/Archiv/weg.jpg"), 200), Vorschau::Fehlt);
    // offline: Server-Adresse ohne Dienst
    let tot = Basis { api: format!("http://127.0.0.1:{}", freier_port()), inhalt: format!("http://127.0.0.1:{}", freier_port()) };
    let ablage = ImSpeicher::default();
    ablage.schreiben("RT").unwrap();
    let k = Arc::new(Konto::mit_basis("K", Box::new(ablage), tot, "x", warte_nicht));
    let a = DropboxKlassisch::new(&w, &d.join("cache2"), Some(k));
    assert_eq!(a.vorschau(&format!("{w}/Archiv/online.jpg"), 200), Vorschau::NurOnline);
}

/// R-DBX-18: HEIC zuerst über die Vorschau-API; lehnt Dropbox das Format ab → Download,
/// und für weitere HEIC-Bilder wird die API nicht mehr gefragt (spart je Bild eine Anfrage).
#[test]
fn heic_abgelehnt_und_nicht_umwandelbar_per_download() {
    let d = tmp("heic");
    let s = Server::starte(|a, n| match a.pfad.as_str() {
        "/2/files/get_thumbnail_v2" if a.kopf("Dropbox-API-Arg").unwrap_or("").contains(".heic") || a.body.contains(".heic") => json(409, serde_json::json!({"error_summary":"unsupported_extension/"})),
        "/2/files/get_thumbnail_v2" => json(409, serde_json::json!({"error_summary":"unsupported_image/"})),
        _ => standard(a, n),
    });
    let (a, w) = anbieter(&d, Some(konto(&s, true)));
    std::fs::write(format!("{w}/Archiv/foto2.heic"), b"").unwrap();
    let _ = a.vorschau(&format!("{w}/Archiv/foto.heic"), 200); // Inhalt „ORIGINAL“ ist kein Bild → Fehler, entscheidend ist der Weg
    assert_eq!(s.anfragen("/2/files/get_thumbnail_v2").len(), 1, "HEIC: Vorschau-API zuerst");
    assert_eq!(s.anfragen("/2/files/download").len(), 1, "abgelehnt → Download");
    let _ = a.vorschau(&format!("{w}/Archiv/foto2.heic"), 200);
    assert_eq!(s.anfragen("/2/files/get_thumbnail_v2").len(), 1, "nach Ablehnung kein weiterer API-Versuch für HEIC");
    assert_eq!(s.anfragen("/2/files/download").len(), 2);
    let _ = a.vorschau(&format!("{w}/Archiv/online.jpg"), 200);
    assert_eq!(s.anfragen("/2/files/get_thumbnail_v2").len(), 2, "JPEG fragt weiter die API");
    assert_eq!(s.anfragen("/2/files/download").len(), 3, "nicht umwandelbar → Download");
    let rest: Vec<_> = std::fs::read_dir(d.join("cache").join("dropbox-tmp")).unwrap().flatten().collect();
    assert!(rest.is_empty(), "Kopien von Einzelbildern werden gelöscht: {rest:?}");
}

/// R-DBX-21: HEIC-Vorschau direkt von Dropbox – kein Download des Originals.
#[test]
fn heic_ueber_vorschau_api() {
    let d = tmp("heic-api");
    let s = Server::starte(standard);
    let (a, w) = anbieter(&d, Some(konto(&s, true)));
    let v = a.vorschau(&format!("{w}/Archiv/foto.heic"), 200);
    assert_eq!(s.anfragen("/2/files/get_thumbnail_v2").len(), 1);
    assert!(s.anfragen("/2/files/download").is_empty(), "kein Download: {v:?}");
}

/// R-DBX-22: Endungen mit Dropbox-Vorschau – HEIC/HEIF sind dabei, PDF nicht.
#[test]
fn vorschau_endungen() {
    for e in ["jpg", "heic", "heif", "png", "webp"] {
        assert!(VORSCHAU_ENDUNGEN.contains(&e), "{e}");
    }
    assert!(!VORSCHAU_ENDUNGEN.contains(&"pdf"));
}

/// R-DBX-19
#[test]
fn pdf_seiten_laden_einmal() {
    let d = tmp("pdf");
    let s = Server::starte(standard);
    let (a, w) = anbieter(&d, Some(konto(&s, true)));
    let a = Arc::new(a);
    let faeden: Vec<_> = (1..=4)
        .map(|n| {
            let (a, p) = (a.clone(), format!("{w}/Archiv/album.pdf#seite={n}"));
            std::thread::spawn(move || a.vorschau(&p, 200))
        })
        .collect();
    for f in faeden {
        let _ = f.join().unwrap();
    }
    assert_eq!(s.anfragen("/2/files/download").len(), 1, "eine Kopie für alle Seiten");
    assert!(s.anfragen("/2/files/get_thumbnail_v2").is_empty());
    assert_eq!(std::fs::read_dir(d.join("cache").join("dropbox-tmp")).unwrap().count(), 1, "PDF bleibt kurz vorgehalten");
}

/// R-DBX-20
#[test]
fn original_bereitstellen() {
    let d = tmp("orig");
    let s = Server::starte(standard);
    let (a, w) = anbieter(&d, Some(konto(&s, true)));
    let o = a.original_bereitstellen(&format!("{w}/Archiv/online.jpg")).unwrap();
    assert_eq!(std::fs::read(&o).unwrap(), b"ORIGINAL");
    assert_eq!(o.file_name().unwrap(), "online.jpg");
    assert_eq!(a.original_bereitstellen(&format!("{w}/Archiv/lokal.jpg")).unwrap(), PathBuf::from(format!("{w}/Archiv/lokal.jpg")));
    let (a, w) = anbieter(&tmp("orig2"), Some(konto(&s, false)));
    assert!(a.original_bereitstellen(&format!("{w}/Archiv/online.jpg")).unwrap_err().contains("anmelden"));
}

/// R-DBX-23 (v0.3): Speicher neu aufbauen – gleicher App-Schlüssel behält Konto und Anmeldung,
/// anderer Schlüssel meldet das alte Konto ab, leerer Schlüssel → kein Konto, Dropbox-Ordner wie lokal.
#[test]
fn speicher_neu_aufbauen() {
    use crate::settings::Einstellungen;
    use crate::speicher::{Regel, Speicher};
    let d = tmp("aufbau");
    let s = Server::starte(standard);
    let mit_key = |k: &str| Arc::new(Konto::mit_basis(k, Box::new(ImSpeicher::default()), s.basis(), "x", warte_nicht));
    let e = |key: Option<&str>| Einstellungen {
        dropbox_app_key: key.map(String::from),
        anbieter: Some(vec![Regel { pfad: "/X".into(), art: "dropbox-klassisch".into() }]),
        ..Default::default()
    };
    let (erst, fehler) = Speicher::aufbauen(&e(Some("APPKEY")), &d, None, |_| konto(&s, true));
    assert!(fehler.is_none());
    assert_eq!(erst.verteiler.fuer("/X/a.jpg").art(), "dropbox-klassisch");
    let k1 = erst.konto.clone().unwrap();
    assert!(k1.angemeldet());

    // gleicher Schlüssel → dasselbe Konto, weiter angemeldet
    let (zweit, _) = Speicher::aufbauen(&e(Some("APPKEY")), &d, Some(&erst), |_| panic!("kein neues Konto"));
    assert!(Arc::ptr_eq(&k1, zweit.konto.as_ref().unwrap()));
    assert!(k1.angemeldet());

    // anderer Schlüssel → altes Konto abgemeldet (Token widerrufen), neues Konto mit neuem Schlüssel
    let (dritt, _) = Speicher::aufbauen(&e(Some("NEU")), &d, Some(&zweit), mit_key);
    assert!(!k1.angemeldet(), "altes Konto abgemeldet");
    assert_eq!(s.anfragen("/2/auth/token/revoke").len(), 1, "Token widerrufen");
    assert_eq!(dritt.konto.as_ref().unwrap().app_key(), "NEU");

    // ohne Schlüssel → kein Konto; Regel bleibt, Datei verhält sich wie lokal
    let (viert, _) = Speicher::aufbauen(&e(Some("")), &d, Some(&dritt), mit_key);
    assert!(viert.konto.is_none());
    assert_eq!(viert.verteiler.fuer("/X/a.jpg").art(), "dropbox-klassisch");

    // ungültige Regeln → Fehler, alles lokal
    let kaputt = Einstellungen { anbieter: Some(vec![Regel { pfad: "rel".into(), art: "lokal".into() }]), ..Default::default() };
    let (f, fehler) = Speicher::aufbauen(&kaputt, &d, None, mit_key);
    assert!(fehler.unwrap().contains("absolut"));
    assert_eq!(f.verteiler.fuer("/X/a.jpg").art(), "lokal");
    // Standard ohne Eintrag: mitgelieferter Schlüssel
    let (g, _) = Speicher::aufbauen(&Einstellungen::default(), &d, None, mit_key);
    assert_eq!(g.konto.unwrap().app_key(), crate::settings::STANDARD_DROPBOX_APP_KEY);
}
