//! Harnais de repro du gel « start après arrêt propre » (iPad/ATV, 2026-07-17).
//!
//! Rejoue le cycle de vie EXACT de l'app : create → start → vie courte →
//! stop (teardown détaché) → recréation IMMÉDIATE dans le même process.
//! Aucun test ne couvrait ce chemin — les deux gels terrain le suivent.
//!
//! Lancer : cargo run --locked --example cycle_harness -- <dir> [cycles]
//! Chaque phase imprime sa durée ; un gel se voit par l'absence de sortie
//! (superviser de l'extérieur et `sample <pid>` pour la stack).
//!
//! ⚠️ Découverte publique OFF par défaut (DHT + n0 + mDNS) : les cycles
//! publiaient l'identité dans le VRAI rendezvous DHT partagé (fantômes flotte).
//! Le gel historique visé impliquait le DHT (getaddrinfo synchrone, SharedDht) :
//! pour rejouer CE scénario fidèlement, opt-in TOM_HARNESS_APP_PARITY=1.

use std::ffi::CString;
use std::time::{Duration, Instant};

use tom_protocol_ffi::{tom_node_create, tom_node_start, tom_node_stop};

fn c(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/tom-ffi-cycle".to_string());
    let cycles: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    std::fs::create_dir_all(&dir).unwrap();
    let identity = format!("{dir}/id.key");
    let data = format!("{dir}/data");

    // Découverte publique OFF par défaut (anti-fantômes) ; TOM_HARNESS_APP_PARITY=1
    // restaure la config exacte de l'app (DHT + n0 + mDNS + data_dir persistant).
    let app_parity = std::env::var("TOM_HARNESS_APP_PARITY").is_ok_and(|v| v == "1");
    if app_parity {
        eprintln!("APP_PARITY: découverte publique ACTIVE (DHT + n0 + mDNS)");
    }
    let create_cfg = c(&format!(r#"{{"n0_discovery": {app_parity}}}"#));
    let start_cfg = c(&format!(
        concat!(
            r#"{{"username":"cycleharness","encryption":true,"enable_dht":{dht},"#,
            r#""n0_discovery":{n0},"local_discovery":{mdns},"#,
            r#""identity_path":{id:?},"data_dir":{data:?},"app_build":9999}}"#
        ),
        dht = app_parity,
        n0 = app_parity,
        mdns = app_parity,
        id = identity,
        data = data,
    ));

    for i in 1..=cycles {
        let t = Instant::now();
        let h = unsafe { tom_node_create(create_cfg.as_ptr()) };
        assert!(!h.is_null(), "cycle {i}: create NULL");
        eprintln!("[cycle {i}] create: {:?}", t.elapsed());

        let t = Instant::now();
        let rc = unsafe { tom_node_start(h, start_cfg.as_ptr()) };
        eprintln!("[cycle {i}] start rc={rc}: {:?}", t.elapsed());
        assert_eq!(rc, 0, "cycle {i}: start a échoué");

        // Vie courte (le nœud rejoint le réseau réel, comme l'app).
        std::thread::sleep(Duration::from_secs(2));

        let t = Instant::now();
        unsafe { tom_node_stop(h) };
        eprintln!("[cycle {i}] stop (détaché): {:?}", t.elapsed());

        // L'app redémarre vite après un arrêt propre (iPad : 45 s, ATV BG
        // wake : immédiat à l'échelle du teardown). On serre encore plus.
        std::thread::sleep(Duration::from_millis(300));
    }
    eprintln!("FIN {cycles} cycles sans gel");
}
