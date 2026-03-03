use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::freebox::{FreeboxClient, LanHost};

/// NAT port forwarding rule (Freebox response).
/// Uses `serde(default)` because the creation response may omit some fields.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
#[allow(dead_code)]
pub struct NatRule {
    pub id: u64,
    pub enabled: bool,
    pub comment: Option<String>,
    pub ip_proto: String,
    pub wan_port_start: u16,
    pub wan_port_end: u16,
    pub lan_ip: String,
    pub lan_port: u16,
    pub src_ip: Option<String>,
}

/// NAT rule creation request.
#[derive(Debug, Serialize)]
pub struct NatRuleRequest {
    pub enabled: bool,
    pub ip_proto: String,
    pub wan_port_start: u16,
    pub wan_port_end: u16,
    pub lan_ip: String,
    pub lan_port: u16,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub src_ip: String,
    pub comment: String,
}

/// Create NAT rules for the relay (UDP + TCP).
/// If `force` is true, conflicting rules are deleted and recreated.
pub async fn setup(
    client: &FreeboxClient,
    port: u16,
    lan_ip: &str,
    comment: &str,
    force: bool,
) -> Result<()> {
    let existing = client.list_nat_rules().await?;

    for proto in &["udp", "tcp"] {
        // Check existing rules
        let found = existing.iter().find(|r| {
            r.wan_port_start == port && r.ip_proto == *proto
        });

        if let Some(rule) = found {
            if rule.lan_ip == lan_ip && rule.lan_port == port {
                println!("  {}: regle existante (id={}, {}:{} -> {}:{})",
                    proto.to_uppercase(), rule.id,
                    proto, port, rule.lan_ip, rule.lan_port);
                continue;
            }
            if force {
                println!("  {}: suppression regle existante id={} ({} -> {})",
                    proto.to_uppercase(), rule.id, rule.lan_ip, lan_ip);
                client.delete_nat_rule(rule.id).await?;
            } else {
                println!("  ATTENTION: regle existante {} port {} pointe vers {} (pas {})",
                    proto.to_uppercase(), port, rule.lan_ip, lan_ip);
                println!("  Relance avec --force pour la remplacer.");
                continue;
            }
        }

        let req = NatRuleRequest {
            enabled: true,
            ip_proto: proto.to_string(),
            wan_port_start: port,
            wan_port_end: port,
            lan_ip: lan_ip.to_string(),
            lan_port: port,
            src_ip: String::new(),
            comment: format!("{comment} ({proto})"),
        };

        let created = client.create_nat_rule(&req).await?;
        println!("  {}: regle creee (id={})", proto.to_uppercase(), created.id);
    }

    // Show public IP
    match client.connection_info().await {
        Ok(conn) => {
            let ipv4 = conn.ipv4.unwrap_or_else(|| "?".into());
            println!();
            println!("NAT: {}:{} (UDP+TCP) -> {}:{}", ipv4, port, lan_ip, port);
            println!("Relay URL: http://{}:{}", ipv4, port);
        }
        Err(e) => {
            tracing::warn!("impossible de recuperer l'IP publique: {e}");
        }
    }

    Ok(())
}

/// Display current NAT rules and connection info.
pub async fn status(client: &FreeboxClient, relay_port: u16) -> Result<()> {
    // Connection info
    let conn = client.connection_info().await?;
    let ipv4 = conn.ipv4.as_deref().unwrap_or("?");
    let ipv6 = conn.ipv6.as_deref().unwrap_or("?");
    let state = conn.state.as_deref().unwrap_or("?");

    println!("Connexion: {state}");
    println!("  IPv4: {ipv4}");
    println!("  IPv6: {ipv6}");
    println!();

    // NAT rules
    let rules = client.list_nat_rules().await?;
    if rules.is_empty() {
        println!("Aucune regle NAT configuree.");
    } else {
        println!("Regles NAT ({} total):", rules.len());
        for rule in &rules {
            let status = if rule.enabled { "ON " } else { "OFF" };
            let comment = rule.comment.as_deref().unwrap_or("");
            println!(
                "  [{}] id={:<3} {} {:<5} :{} -> {}:{}  {}",
                status,
                rule.id,
                rule.ip_proto.to_uppercase(),
                rule.wan_port_start,
                rule.wan_port_end,
                rule.lan_ip,
                rule.lan_port,
                comment,
            );
        }
    }
    println!();

    // Check relay rule
    let has_udp = rules.iter().any(|r| {
        r.enabled && r.ip_proto == "udp" && r.wan_port_start == relay_port
    });
    let has_tcp = rules.iter().any(|r| {
        r.enabled && r.ip_proto == "tcp" && r.wan_port_start == relay_port
    });

    if has_udp && has_tcp {
        println!("Relay NAT port {relay_port}: OK (UDP + TCP)");
    } else if has_udp {
        println!("Relay NAT port {relay_port}: UDP OK, TCP MANQUANT");
    } else if has_tcp {
        println!("Relay NAT port {relay_port}: TCP OK, UDP MANQUANT");
    } else {
        println!("Relay NAT port {relay_port}: ABSENT — lance 'tom-gateway setup'");
    }

    Ok(())
}

/// Try to auto-detect the NAS IP from the LAN browser.
pub async fn detect_nas_ip(client: &FreeboxClient, known_ip: Option<&str>) -> Result<String> {
    if let Some(ip) = known_ip {
        return Ok(ip.to_string());
    }

    let hosts = client.lan_browser().await?;
    let keywords = ["debian", "nas", "vm", "freebox-server"];

    let mut candidates: Vec<(String, String)> = Vec::new();

    for host in &hosts {
        let name = host
            .primary_name
            .as_deref()
            .unwrap_or("")
            .to_lowercase();

        let is_match = keywords.iter().any(|kw| name.contains(kw));
        if !is_match {
            continue;
        }

        if let Some(ref l3) = host.l3connectivities {
            for conn in l3 {
                if conn.af == "ipv4" && conn.active.unwrap_or(false) {
                    candidates.push((
                        host.primary_name.clone().unwrap_or_default(),
                        conn.addr.clone(),
                    ));
                }
            }
        }
    }

    match candidates.len() {
        0 => {
            println!("Devices LAN detectes:");
            print_lan_hosts(&hosts);
            bail!(
                "Impossible de detecter le NAS automatiquement. \
                 Utilise --lan-ip pour specifier l'adresse."
            );
        }
        1 => {
            let (name, ip) = &candidates[0];
            println!("NAS detecte: {name} ({ip})");
            Ok(ip.clone())
        }
        _ => {
            println!("Plusieurs candidats NAS:");
            for (name, ip) in &candidates {
                println!("  - {name}: {ip}");
            }
            bail!(
                "Plusieurs NAS detectes. Utilise --lan-ip pour specifier lequel."
            );
        }
    }
}

/// Print LAN hosts as a simple table.
pub fn print_lan_hosts(hosts: &[LanHost]) {
    for host in hosts {
        let name = host.primary_name.as_deref().unwrap_or("?");
        let mac = host
            .l2ident
            .as_ref()
            .map(|l| l.id.as_str())
            .unwrap_or("?");
        let active = if host.active.unwrap_or(false) { "active" } else { "      " };
        let reach = if host.reachable.unwrap_or(false) { "reachable" } else { "         " };

        let ips: Vec<String> = host
            .l3connectivities
            .as_ref()
            .map(|l3| l3.iter().map(|c| c.addr.clone()).collect())
            .unwrap_or_default();

        println!("  {:<20} {:<18} {:<7} {:<10} {}",
            name, mac, active, reach, ips.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nat_rule_deserialize() {
        let json = r#"{
            "id": 42,
            "enabled": true,
            "comment": "ToM relay",
            "ip_proto": "udp",
            "wan_port_start": 3340,
            "wan_port_end": 3340,
            "lan_ip": "192.168.0.83",
            "lan_port": 3340,
            "src_ip": ""
        }"#;
        let rule: NatRule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.id, 42);
        assert_eq!(rule.lan_ip, "192.168.0.83");
        assert_eq!(rule.ip_proto, "udp");
        assert!(rule.enabled);
    }

    #[test]
    fn nat_rule_request_serialize() {
        let req = NatRuleRequest {
            enabled: true,
            ip_proto: "udp".into(),
            wan_port_start: 3340,
            wan_port_end: 3340,
            lan_ip: "192.168.0.83".into(),
            lan_port: 3340,
            src_ip: String::new(),
            comment: "ToM relay (udp)".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["wan_port_start"], 3340);
        assert_eq!(json["ip_proto"], "udp");
        assert_eq!(json["lan_ip"], "192.168.0.83");
    }
}
