//! Reproduction chronométrée du cycle Stop→Start des apps (bug "mouline").
//!
//! Imite exactement TomNodeService.swift : create → start → stop (teardown
//! détaché) → create+start IMMÉDIAT avec la MÊME identité et le MÊME data_dir,
//! pendant que l'ancienne instance agonise encore en arrière-plan.
//!
//! Lancer avec : cargo test --locked --test restart_timing -- --nocapture --test-threads=1

use std::ffi::CString;
use std::time::Instant;

use tom_protocol_ffi::{tom_node_create, tom_node_start, tom_node_stop};

fn c(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Un cycle create+start, renvoie (handle, durée_ms). Panique si start échoue.
unsafe fn create_and_start(
    create_cfg: &str,
    start_cfg: &str,
) -> (*mut tom_protocol_ffi::TomNodeHandle, u128) {
    let t0 = Instant::now();
    let h = unsafe { tom_node_create(c(create_cfg).as_ptr()) };
    assert!(!h.is_null(), "tom_node_create a renvoyé NULL");
    let rc = unsafe { tom_node_start(h, c(start_cfg).as_ptr()) };
    let elapsed = t0.elapsed().as_millis();
    assert_eq!(rc, 0, "tom_node_start a échoué (rc={rc}) après {elapsed}ms");
    (h, elapsed)
}

fn scenario(label: &str, enable_dht: bool) {
    let dir = std::env::temp_dir().join(format!("tom-restart-timing-{label}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let identity = dir.join("id.key");
    let data = dir.join("data");

    let create_cfg = r#"{"n0_discovery": false}"#.to_string();
    let start_cfg = format!(
        concat!(
            r#"{{"username":"timing","encryption":true,"enable_dht":{dht},"#,
            r#""n0_discovery":false,"local_discovery":false,"#,
            r#""identity_path":{id:?},"data_dir":{data:?}}}"#
        ),
        dht = enable_dht,
        id = identity.display().to_string(),
        data = data.display().to_string(),
    );

    // Démarrage 1 (à froid)
    let (h1, d1) = unsafe { create_and_start(&create_cfg, &start_cfg) };
    eprintln!("[{label}] start#1 (froid)        : {d1} ms");

    std::thread::sleep(std::time::Duration::from_secs(2));

    // Stop — doit rendre la main immédiatement (teardown détaché)
    let t = Instant::now();
    unsafe { tom_node_stop(h1) };
    eprintln!("[{label}] stop (retour UI)       : {} ms", t.elapsed().as_millis());

    // Redémarrage IMMÉDIAT — même identité + même data_dir, ancienne instance
    // encore en cours de teardown : c'est le geste utilisateur "Stop puis Start".
    let (h2, d2) = unsafe { create_and_start(&create_cfg, &start_cfg) };
    eprintln!("[{label}] start#2 (restart chaud): {d2} ms");

    // Redémarrage après accalmie (teardown fini) pour comparaison
    std::thread::sleep(std::time::Duration::from_secs(8));
    unsafe { tom_node_stop(h2) };
    std::thread::sleep(std::time::Duration::from_secs(8));
    let (h3, d3) = unsafe { create_and_start(&create_cfg, &start_cfg) };
    eprintln!("[{label}] start#3 (après repos)  : {d3} ms");
    unsafe { tom_node_stop(h3) };
    std::thread::sleep(std::time::Duration::from_secs(4));
}

#[test]
fn restart_timing_sans_dht() {
    scenario("sans-dht", false);
}

#[test]
fn restart_timing_avec_dht() {
    scenario("avec-dht", true);
}

/// Conditions RÉELLES des apps : mDNS actif, n0_discovery actif, relais
/// configuré MORT (le NAS éteint) — le cas où l'utilisateur voit "Starting…"
/// mouliner. Mesure le temps que tom_node_start retient le spinner.
#[test]
fn restart_timing_conditions_apps() {
    let dir = std::env::temp_dir().join("tom-restart-timing-apps");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let identity = dir.join("id.key");
    let data = dir.join("data");

    let create_cfg = r#"{"n0_discovery": true, "relay_url": "http://192.168.0.21:3340"}"#;
    let start_cfg = format!(
        concat!(
            r#"{{"username":"timing","encryption":true,"enable_dht":true,"#,
            r#""n0_discovery":true,"local_discovery":true,"#,
            r#""relay_url":"http://192.168.0.21:3340","#,
            r#""identity_path":{id:?},"data_dir":{data:?}}}"#
        ),
        id = identity.display().to_string(),
        data = data.display().to_string(),
    );

    let (h1, d1) = unsafe { create_and_start(create_cfg, &start_cfg) };
    eprintln!("[apps-réel] start#1 (froid)        : {d1} ms");
    std::thread::sleep(std::time::Duration::from_secs(2));
    let t = Instant::now();
    unsafe { tom_node_stop(h1) };
    eprintln!("[apps-réel] stop (retour UI)       : {} ms", t.elapsed().as_millis());
    let (h2, d2) = unsafe { create_and_start(create_cfg, &start_cfg) };
    eprintln!("[apps-réel] start#2 (restart chaud): {d2} ms");
    unsafe { tom_node_stop(h2) };
    std::thread::sleep(std::time::Duration::from_secs(4));
}
