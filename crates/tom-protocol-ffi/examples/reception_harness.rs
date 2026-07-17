//! Harnais de repro du bug de réception FFI (messages_recus=0 sur les apps).
//!
//! Appelle l'API C exactement comme TomNodeService.swift : create → start
//! (même config que l'app : DHT + n0 + mDNS + data_dir) → poll
//! tom_node_receive_messages toutes les secondes. Permet de discriminer
//! « bug reproductible dans le crate FFI » vs « bug environnement app »
//! sans toucher à Xcode.
//!
//! Lancer : cargo run --locked --example reception_harness -- <dir_travail>
//! Puis depuis le NAS : POST /send?to=<node_id affiché> et surveiller RECUS.
//!
//! ⚠️ Découverte publique OFF par défaut (DHT + n0 + mDNS) : chaque lancement
//! semait une identité jetable dans le VRAI rendezvous DHT partagé — fantômes
//! durables pour toute la flotte (pollution topologie du 17/07). Pour une repro
//! fidèle à l'app (réseau réel), opt-in explicite : TOM_HARNESS_APP_PARITY=1.

use std::ffi::{CStr, CString};
use std::time::{Duration, Instant};

use tom_protocol_ffi::{
    tom_node_create, tom_node_free_string, tom_node_receive_messages, tom_node_start,
    tom_node_status, tom_node_stop,
};

fn c(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Récupère une chaîne C renvoyée par le FFI et la libère.
unsafe fn take_string(ptr: *mut std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
    unsafe { tom_node_free_string(ptr) };
    Some(s)
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/tom-ffi-harness".to_string());
    std::fs::create_dir_all(&dir).unwrap();
    // Le répertoire contiendra une clé privée Ed25519 (id.key) : accès propriétaire seul.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    let identity = format!("{dir}/id.key");
    let data = format!("{dir}/data");

    // Découverte publique OFF par défaut (anti-fantômes) ; TOM_HARNESS_APP_PARITY=1
    // restaure la config exacte de l'app (TomNodeWrapper.start) : DHT, n0, mDNS.
    let app_parity = std::env::var("TOM_HARNESS_APP_PARITY").is_ok_and(|v| v == "1");
    if app_parity {
        eprintln!("APP_PARITY: découverte publique ACTIVE (DHT + n0 + mDNS)");
    }
    let create_cfg = format!(r#"{{"n0_discovery": {app_parity}}}"#);
    let start_cfg = format!(
        concat!(
            r#"{{"username":"ffiharness","encryption":true,"enable_dht":{dht},"#,
            r#""n0_discovery":{n0},"local_discovery":{mdns},"#,
            r#""identity_path":{id:?},"data_dir":{data:?},"app_build":9999}}"#
        ),
        dht = app_parity,
        n0 = app_parity,
        mdns = app_parity,
        id = identity,
        data = data,
    );

    let h = unsafe { tom_node_create(c(&create_cfg).as_ptr()) };
    assert!(!h.is_null(), "tom_node_create a renvoyé NULL");
    let rc = unsafe { tom_node_start(h, c(&start_cfg).as_ptr()) };
    assert_eq!(rc, 0, "tom_node_start a échoué (rc={rc})");

    if let Some(status) = unsafe { take_string(tom_node_status(h)) } {
        eprintln!("STATUS_INITIAL: {status}");
    }

    let t0 = Instant::now();
    let mut total = 0usize;
    let mut last_status = Instant::now();
    while t0.elapsed() < Duration::from_secs(600) {
        std::thread::sleep(Duration::from_secs(1));

        if let Some(json) = unsafe { take_string(tom_node_receive_messages(h)) } {
            if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str(&json) {
                if !arr.is_empty() {
                    total += arr.len();
                    eprintln!("[t+{}s] RECUS +{} → total={}", t0.elapsed().as_secs(), arr.len(), total);
                }
            }
        }

        if last_status.elapsed() >= Duration::from_secs(15) {
            last_status = Instant::now();
            if let Some(status) = unsafe { take_string(tom_node_status(h)) } {
                eprintln!("[t+{}s] total_recus={} STATUS: {status}", t0.elapsed().as_secs(), total, );
            }
        }
    }

    eprintln!("FIN total_recus={total}");
    // tom_node_stop CONSOMME le handle (Box::from_raw + teardown détaché, cf.
    // contrat d'ownership lib.rs::detached_teardown) — ne PAS appeler
    // tom_node_free ensuite : ce serait un double-free.
    unsafe { tom_node_stop(h) };
    std::thread::sleep(Duration::from_secs(2));
}
