//! End-to-End-Test der HTTP-API.
//!
//! Der Test startet den echten Server auf einem freien Port und spricht ihn
//! ueber eine einfache HTTP/1.1-Verbindung an. Damit wird der Weg geprueft,
//! den auch die Weboberflaeche nimmt: Erstinbetriebnahme, TOTP-Enrollment,
//! Anmeldung, Objektanlage, Kompilierung und Aktivierung.
//!
//! Abgedeckte Punkte aus Kapitel 24 (Definition of Done):
//! 3 (First Admin ohne Default-Credentials), 4 (TOTP), 5 (KMS-Initialisierung),
//! 10 (Config-Kompilierung), 12 (Rollback-Faehigkeit), 20 (Audit/Health).

use std::net::SocketAddr;

use extelio::application::{http, state::AppState};
use extelio::infrastructure::config::{Options, Paths};
use extelio::infrastructure::db::Db;
use extelio::infrastructure::migrations;
use extelio::kms::Kms;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Response {
    fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).unwrap_or(serde_json::Value::Null)
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

async fn call(
    addr: SocketAddr,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    body: Option<serde_json::Value>,
) -> Response {
    let payload = body.map(|b| b.to_string()).unwrap_or_default();
    let mut request =
        format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n");
    if let Some(c) = cookie {
        request.push_str(&format!("Cookie: {c}\r\n"));
    }
    if !payload.is_empty() {
        request.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            payload.len()
        ));
    } else if method != "GET" {
        request.push_str("Content-Type: application/json\r\nContent-Length: 2\r\n");
    }
    request.push_str("\r\n");
    if !payload.is_empty() {
        request.push_str(&payload);
    } else if method != "GET" {
        request.push_str("{}");
    }

    let mut stream = TcpStream::connect(addr).await.expect("Verbindung");
    stream.write_all(request.as_bytes()).await.expect("Senden");
    stream.flush().await.unwrap();

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.expect("Lesen");
    let text = String::from_utf8_lossy(&raw).to_string();

    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((text.as_str(), ""));
    let mut lines = head.lines();
    let status = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();

    Response {
        status,
        headers,
        body: body.to_string(),
    }
}

async fn start_server() -> (SocketAddr, tempdir::Guard) {
    let dir = tempdir::create();
    let paths = Paths {
        data: dir.path.clone(),
        run: dir.path.join("run"),
        web_root: dir.path.join("web"),
    };
    paths.ensure().unwrap();
    std::fs::create_dir_all(&paths.web_root).unwrap();
    std::fs::write(
        paths.web_root.join("index.html"),
        "<!doctype html><title>EXTELIO</title>",
    )
    .unwrap();

    let db = Db::open(&paths.db_file()).unwrap();
    migrations::run(&db).unwrap();
    let kms = Kms::open_or_init(&paths.kms_dir()).unwrap();

    let state = AppState::new(db, kms, paths, Options::default());
    let app = http::router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    // Kurz warten, bis der Server Verbindungen annimmt.
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;
    (addr, dir)
}

mod tempdir {
    use std::path::PathBuf;

    pub struct Guard {
        pub path: PathBuf,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    pub fn create() -> Guard {
        let path =
            std::env::temp_dir().join(format!("extelio-it-{}-{}", std::process::id(), uuid_like()));
        std::fs::create_dir_all(&path).unwrap();
        Guard { path }
    }

    fn uuid_like() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos().to_string())
            .unwrap_or_else(|_| "0".into())
    }
}

#[tokio::test]
async fn full_first_boot_and_configuration_flow() {
    let (addr, _dir) = start_server().await;

    // --- Erstinbetriebnahme steht offen -------------------------------
    let status = call(addr, "GET", "/api/setup/status", None, None).await;
    assert_eq!(status.status, 200);
    assert_eq!(status.json()["needs_setup"], true);

    // Sicherheitsheader liegen auf jeder Antwort (Kapitel 12).
    let csp = status.header("content-security-policy").expect("CSP fehlt");
    assert!(csp.contains("default-src 'self'"));
    assert!(
        !csp.contains("unsafe-inline"),
        "CSP darf kein unsafe-inline erlauben"
    );
    assert_eq!(status.header("x-frame-options"), Some("DENY"));

    // --- Ohne Anmeldung kein Zugriff ----------------------------------
    let denied = call(addr, "GET", "/api/extensions", None, None).await;
    assert_eq!(denied.status, 401, "Kapitel 20: kein offener Zugang");

    // --- Schwaches Passwort wird abgelehnt (Kapitel 6.2) --------------
    let weak = call(
        addr,
        "POST",
        "/api/setup/admin",
        None,
        Some(
            serde_json::json!({ "username": "admin", "display_name": "Admin", "password": "kurz" }),
        ),
    )
    .await;
    assert_eq!(weak.status, 400);

    // --- Ersten Administrator anlegen ---------------------------------
    let created = call(
        addr,
        "POST",
        "/api/setup/admin",
        None,
        Some(serde_json::json!({
            "username": "admin",
            "display_name": "Chriss",
            "password": "Extelio-Test-2026!"
        })),
    )
    .await;
    assert_eq!(created.status, 200, "{}", created.body);
    let payload = created.json();
    let user_id = payload["user_id"].as_str().unwrap().to_string();
    let secret = payload["totp_secret"].as_str().unwrap().to_string();
    assert!(payload["provisioning_uri"]
        .as_str()
        .unwrap()
        .starts_with("otpauth://totp/EXTELIO:admin"));

    // Ein zweiter Aufruf ist nicht mehr moeglich.
    let again = call(
        addr,
        "POST",
        "/api/setup/admin",
        None,
        Some(serde_json::json!({
            "username": "zweiter", "display_name": "X", "password": "Extelio-Test-2026!"
        })),
    )
    .await;
    assert_eq!(again.status, 409);

    // --- TOTP bestaetigen ---------------------------------------------
    let now = extelio::infrastructure::time::unix_seconds();
    let code = extelio::auth::totp::code(&secret, now).unwrap();
    let confirmed = call(
        addr,
        "POST",
        "/api/setup/totp/confirm",
        None,
        Some(serde_json::json!({ "user_id": user_id, "code": code })),
    )
    .await;
    assert_eq!(confirmed.status, 200, "{}", confirmed.body);

    // --- Anmeldung ohne zweiten Faktor scheitert ----------------------
    let no_totp = call(
        addr,
        "POST",
        "/api/auth/login",
        None,
        Some(serde_json::json!({ "username": "admin", "password": "Extelio-Test-2026!" })),
    )
    .await;
    assert_eq!(no_totp.status, 401);

    // --- Anmeldung mit zweitem Faktor ---------------------------------
    let code =
        extelio::auth::totp::code(&secret, extelio::infrastructure::time::unix_seconds()).unwrap();
    let login = call(
        addr,
        "POST",
        "/api/auth/login",
        None,
        Some(serde_json::json!({
            "username": "admin", "password": "Extelio-Test-2026!", "totp": code
        })),
    )
    .await;
    assert_eq!(login.status, 200, "{}", login.body);

    let set_cookie = login
        .header("set-cookie")
        .expect("Set-Cookie fehlt")
        .to_string();
    assert!(set_cookie.contains("HttpOnly"), "Kapitel 6.4");
    assert!(set_cookie.contains("SameSite=Strict"), "Kapitel 6.4");
    let cookie = set_cookie.split(';').next().unwrap().to_string();

    // --- Sitzung ist gueltig ------------------------------------------
    let session = call(addr, "GET", "/api/auth/session", Some(&cookie), None).await;
    assert_eq!(session.status, 200);
    assert_eq!(session.json()["role"], "systemadministrator");

    // --- Nebenstelle anlegen ------------------------------------------
    let ext = call(
        addr,
        "POST",
        "/api/extensions",
        Some(&cookie),
        Some(serde_json::json!({ "number": "201", "name": "Empfang" })),
    )
    .await;
    assert_eq!(ext.status, 200, "{}", ext.body);
    let sip_password = ext.json()["sip_password"].as_str().unwrap().to_string();
    assert!(sip_password.len() >= 20, "Kapitel 5.4: starke Credentials");

    // Doppelte Nummer wird abgelehnt.
    let duplicate = call(
        addr,
        "POST",
        "/api/extensions",
        Some(&cookie),
        Some(serde_json::json!({ "number": "201", "name": "Zweitbelegung" })),
    )
    .await;
    assert_eq!(duplicate.status, 409);

    // --- Rufnummer normalisieren --------------------------------------
    let number = call(
        addr,
        "POST",
        "/api/numbers",
        Some(&cookie),
        Some(serde_json::json!({ "number": "0511 123456", "country": "DE" })),
    )
    .await;
    assert_eq!(number.status, 200, "{}", number.body);
    assert_eq!(number.json()["canonical"], "+49511123456");

    // --- Konfiguration kompilieren und aktivieren ----------------------
    let compiled = call(addr, "POST", "/api/config/compile", Some(&cookie), None).await;
    assert_eq!(compiled.status, 200, "{}", compiled.body);
    let generation = compiled.json()["generation"].as_i64().unwrap();
    assert_eq!(generation, 1);
    assert!(compiled.json()["files"].as_i64().unwrap() >= 9);

    let activated = call(
        addr,
        "POST",
        "/api/config/activate",
        Some(&cookie),
        Some(serde_json::json!({ "generation": generation })),
    )
    .await;
    assert_eq!(activated.status, 200, "{}", activated.body);

    let generations = call(addr, "GET", "/api/config/generations", Some(&cookie), None).await;
    assert_eq!(generations.json()["active"], 1);

    // --- Health und Dashboard -----------------------------------------
    let health = call(addr, "GET", "/api/system/health", Some(&cookie), None).await;
    assert_eq!(health.status, 200);
    let checks = health.json()["checks"].as_array().unwrap().len();
    assert!(
        checks >= 13,
        "alle Pflichtchecks laufen, gefunden: {checks}"
    );

    let dashboard = call(addr, "GET", "/api/dashboard", Some(&cookie), None).await;
    assert_eq!(dashboard.json()["counters"]["extensions"], 1);
    assert_eq!(dashboard.json()["counters"]["numbers"], 1);

    // --- Audit-Kette ist intakt ---------------------------------------
    let verify = call(addr, "GET", "/api/audit/verify", Some(&cookie), None).await;
    assert_eq!(verify.json()["valid"], true);
    assert!(verify.json()["entries"].as_i64().unwrap() >= 5);

    let audit = call(addr, "GET", "/api/audit?limit=50", Some(&cookie), None).await;
    let actions: Vec<String> = audit.json()["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["action"].as_str().unwrap_or_default().to_string())
        .collect();
    for expected in [
        "auth.login",
        "extension.create",
        "config.compile",
        "config.activate",
    ] {
        assert!(
            actions.iter().any(|a| a == expected),
            "Audit-Eintrag '{expected}' fehlt"
        );
    }
    // Kapitel 14.5: Es darf kein Secret im Audit stehen.
    assert!(
        !audit.body.contains(&sip_password),
        "Secret im Audit-Protokoll"
    );
    assert!(
        !audit.body.contains("Extelio-Test-2026!"),
        "Passwort im Audit-Protokoll"
    );

    // --- Sicherung erstellen -------------------------------------------
    let backup = call(addr, "POST", "/api/backup", Some(&cookie), None).await;
    assert_eq!(backup.status, 200, "{}", backup.body);
    assert!(backup.json()["size_bytes"].as_i64().unwrap() > 0);

    // --- Abmelden beendet die Sitzung ----------------------------------
    let logout = call(addr, "POST", "/api/auth/logout", Some(&cookie), None).await;
    assert_eq!(logout.status, 200);
    let after = call(addr, "GET", "/api/auth/session", Some(&cookie), None).await;
    assert_eq!(after.status, 401, "Kapitel 6.4: Sitzung ist widerrufen");

    // --- Weboberflaeche wird ausgeliefert -------------------------------
    let index = call(addr, "GET", "/", None, None).await;
    assert_eq!(index.status, 200);
    assert!(index.body.contains("EXTELIO"));
    let traversal = call(addr, "GET", "/../../etc/passwd", None, None).await;
    assert!(
        !traversal.body.contains("root:"),
        "Pfad-Traversal muss scheitern"
    );
}

#[tokio::test]
async fn brute_force_protection_locks_the_account() {
    let (addr, _dir) = start_server().await;

    call(
        addr,
        "POST",
        "/api/setup/admin",
        None,
        Some(serde_json::json!({
            "username": "admin", "display_name": "Chriss", "password": "Extelio-Test-2026!"
        })),
    )
    .await;

    let mut throttled = false;
    for _ in 0..8 {
        let res = call(
            addr,
            "POST",
            "/api/auth/login",
            None,
            Some(serde_json::json!({
                "username": "admin", "password": "falsch-falsch-falsch", "totp": "000000"
            })),
        )
        .await;
        if res.status == 429 {
            throttled = true;
            break;
        }
        assert_eq!(res.status, 401);
    }
    assert!(
        throttled,
        "Kapitel 6.8: der Zaehler darf nicht zurueckgerollt werden"
    );
}
