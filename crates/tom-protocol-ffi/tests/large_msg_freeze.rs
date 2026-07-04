//! Reproduction déterministe du gel "gros message" (DoS constaté en campagne).
//!
//! Hypothèse : `tom_node_send_message` fait `runtime.block_on(send_message())`,
//! et `send_message` écrit dans un canal de commandes borné (16). Sous rafale de
//! gros payloads, le canal sature → `send_message().await` bloque → `block_on`
//! gèle le thread appelant. Dans les apps, le wrapper Swift est un `actor` :
//! TOUS les appels FFI (status, réception) se sérialisent derrière ce thread
//! gelé → l'app entière fige.
//!
//! Ce test imite l'actor Swift : un thread "UI" qui appelle `tom_node_status`
//! en boucle et mesure sa latence, pendant qu'un thread "campagne" matraque
//! l'envoi de gros messages. Si la latence UI explose, le gel est reproduit.
//!
//! Lancer : cargo test --locked --test large_msg_freeze -- --nocapture --test-threads=1

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tom_protocol_ffi::{
    tom_node_create, tom_node_receive_messages, tom_node_send_message, tom_node_start,
    tom_node_status,
};

/// Compte les messages actuellement livrés à un nœud (vide la file FFI).
unsafe fn recv_count(h: *mut tom_protocol_ffi::TomNodeHandle) -> usize {
    let p = unsafe { tom_node_receive_messages(h) };
    if p.is_null() {
        return 0;
    }
    let json = unsafe { std::ffi::CStr::from_ptr(p) }
        .to_string_lossy()
        .into_owned();
    unsafe { tom_protocol_ffi::tom_node_free_string(p) };
    // batch JSON : compte les occurrences de "from" (un par message livré)
    json.matches("\"from\"").count()
}

fn c(s: &str) -> CString {
    CString::new(s).unwrap()
}

unsafe fn boot(label: &str) -> (*mut tom_protocol_ffi::TomNodeHandle, String) {
    let dir = std::env::temp_dir().join(format!("tom-freeze-{label}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let id = dir.join("id.key");
    let data = dir.join("data");
    let create = r#"{"n0_discovery": false}"#;
    let start = format!(
        concat!(
            r#"{{"username":"{label}","encryption":true,"enable_dht":false,"#,
            r#""n0_discovery":false,"local_discovery":true,"#,
            r#""identity_path":{id:?},"data_dir":{data:?}}}"#
        ),
        label = label,
        id = id.display().to_string(),
        data = data.display().to_string(),
    );
    let h = unsafe { tom_node_create(c(create).as_ptr()) };
    assert!(!h.is_null());
    let rc = unsafe { tom_node_start(h, c(&start).as_ptr()) };
    assert_eq!(rc, 0, "start {label}");
    // récupère le node_id via status JSON
    let sptr = unsafe { tom_node_status(h) };
    let json = unsafe { std::ffi::CStr::from_ptr(sptr) }
        .to_string_lossy()
        .into_owned();
    let nid = json
        .split("\"node_id\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap_or("")
        .to_string();
    (h as usize as *mut _, nid)
}

/// Envoie `n` messages de `size` octets de A vers B, laisse le temps de livrer,
/// et renvoie combien B en a effectivement reçus.
unsafe fn essai_taille(
    ha: *mut tom_protocol_ffi::TomNodeHandle,
    hb: *mut tom_protocol_ffi::TomNodeHandle,
    idb: &str,
    size: usize,
    n: usize,
) -> usize {
    let _ = unsafe { recv_count(hb) }; // vider la file avant l'essai
    let target = c(idb);
    let payload = vec![b'A'; size];
    for _ in 0..n {
        unsafe {
            tom_node_send_message(ha, target.as_ptr(), payload.as_ptr(), payload.len());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    // laisser le temps de livrer + drainer
    let mut total = 0;
    for _ in 0..15 {
        std::thread::sleep(std::time::Duration::from_millis(400));
        total += unsafe { recv_count(hb) };
        if total >= n {
            break;
        }
    }
    total
}

/// Non-régression du DoS "inflation ×8" (campagne 2026-07-04) : un payload
/// de 150/250 Ko gonflait au-delà du plafond wire 1 MiB (double sérialisation
/// MessagePack en tableaux d'entiers) → rejeté → jamais livré. Corrigé par
/// `serde_bytes` (encodage `bin` compact). Ces paliers = ceux de la campagne.
#[test]
fn livraison_paliers_campagne() {
    let (ha, _ida) = unsafe { boot("A") };
    let (hb, idb) = unsafe { boot("B") };
    std::thread::sleep(std::time::Duration::from_secs(3));

    let paliers = [1_000usize, 10_000, 50_000, 100_000, 150_000, 250_000];
    for &size in &paliers {
        let recus = unsafe { essai_taille(ha, hb, &idb, size, 5) };
        eprintln!("[campagne] {size:>8} o : {recus}/5 reçus");
        assert!(
            recus >= 4,
            "RÉGRESSION inflation : {size} o livré {recus}/5 (le fix serde_bytes doit garder ≤250 Ko à 100 %)"
        );
    }
}

/// Documente le PLAFOND TRANSPORT résiduel (chantier séparé) : au-delà de
/// ~256 Ko, un message est écrit sur le stream QUIC sans erreur mais n'est
/// jamais livré (probablement la fenêtre de flux QUIC par défaut, PAS
/// l'inflation de sérialisation qui est corrigée). Hors périmètre campagne.
#[test]
#[ignore] // informatif — cliff transport connu, à traiter dans un second temps
fn plafond_transport_gros_messages() {
    let (ha, _ida) = unsafe { boot("A") };
    let (hb, idb) = unsafe { boot("B") };
    std::thread::sleep(std::time::Duration::from_secs(3));

    for &size in &[250_000usize, 300_000, 350_000, 500_000, 900_000] {
        let recus = unsafe { essai_taille(ha, hb, &idb, size, 3) };
        eprintln!("[transport] {size:>8} o : {recus}/3 reçus");
    }
}

#[test]
#[ignore] // gardé pour diag latence — l'actor Swift n'est pas reproduit en Rust pur
fn gros_message_ne_doit_pas_geler_l_actor() {
    // handles bruts partagés entre threads via usize (pointeurs FFI opaques).
    let (ha, _ida) = unsafe { boot("A") };
    let (hb, idb) = unsafe { boot("B") };
    let ha_us = ha as usize;
    let hb_us = hb as usize;

    // Laisser mDNS établir la connexion A<->B.
    std::thread::sleep(std::time::Duration::from_secs(3));

    let stop = Arc::new(AtomicBool::new(false));
    let max_ui_latency_us = Arc::new(AtomicU64::new(0));

    // Thread "campagne" : matraque A -> B avec des gros messages (150 Ko).
    let sender = {
        let stop = stop.clone();
        let idb = idb.clone();
        std::thread::spawn(move || {
            let ha = ha_us as *mut tom_protocol_ffi::TomNodeHandle;
            let target = c(&idb);
            let payload = vec![b'A'; 150_000];
            let mut n = 0u64;
            while !stop.load(Ordering::Relaxed) {
                unsafe {
                    tom_node_send_message(ha, target.as_ptr(), payload.as_ptr(), payload.len());
                }
                n += 1;
                if n >= 200 {
                    break;
                }
            }
            n
        })
    };

    // Thread "UI" (imite l'actor Swift) : status en boucle, mesure la latence max.
    let ui = {
        let stop = stop.clone();
        let max_lat = max_ui_latency_us.clone();
        std::thread::spawn(move || {
            let hb = hb_us as *mut tom_protocol_ffi::TomNodeHandle;
            while !stop.load(Ordering::Relaxed) {
                let t = Instant::now();
                let p = unsafe { tom_node_status(hb) };
                if !p.is_null() {
                    unsafe { tom_protocol_ffi::tom_node_free_string(p) };
                }
                let us = t.elapsed().as_micros() as u64;
                max_lat.fetch_max(us, Ordering::Relaxed);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        })
    };

    let sent = sender.join().unwrap();
    stop.store(true, Ordering::Relaxed);
    ui.join().unwrap();

    let worst_ms = max_ui_latency_us.load(Ordering::Relaxed) as f64 / 1000.0;
    eprintln!("[freeze] messages 150Ko envoyés (A→B): {sent}");
    eprintln!("[freeze] pire latence status UI (B): {worst_ms:.1} ms");

    // Un status doit rester quasi instantané (< 500 ms même sous charge).
    // Avant fix : la latence explose (plusieurs secondes) car l'actor gèle.
    assert!(
        worst_ms < 500.0,
        "GEL REPRODUIT : status UI a bloqué {worst_ms:.1} ms sous rafale de gros messages"
    );
}
