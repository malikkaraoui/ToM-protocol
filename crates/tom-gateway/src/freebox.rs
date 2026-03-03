use anyhow::{bail, Context, Result};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

// ── API Types ────────────────────────────────────────────────────────

/// Freebox API discovery response.
#[derive(Debug, Deserialize)]
pub struct ApiVersion {
    pub api_base_url: String,
    pub api_version: String,
    pub device_name: String,
}

/// Standard Freebox API response wrapper.
#[derive(Debug, Deserialize)]
pub struct FreeboxResponse<T> {
    pub success: bool,
    pub result: Option<T>,
    pub msg: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthResult {
    pub app_token: String,
    pub track_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct AuthTrackResult {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginResult {
    pub challenge: String,
}

#[derive(Debug, Deserialize)]
pub struct SessionResult {
    pub session_token: String,
}

/// Connection info from Freebox.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ConnectionResult {
    pub state: Option<String>,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub bandwidth_down: Option<u64>,
    pub bandwidth_up: Option<u64>,
}

/// LAN host entry.
#[derive(Debug, Deserialize)]
pub struct LanHost {
    pub primary_name: Option<String>,
    pub l2ident: Option<L2Ident>,
    pub l3connectivities: Option<Vec<L3Connectivity>>,
    pub active: Option<bool>,
    pub reachable: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct L2Ident {
    pub id: String,
    #[serde(rename = "type")]
    pub ident_type: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct L3Connectivity {
    pub addr: String,
    pub af: String,
    pub active: Option<bool>,
    pub reachable: Option<bool>,
}

#[derive(Debug, Serialize)]
struct AuthRequest {
    app_id: String,
    app_name: String,
    app_version: String,
    device_name: String,
}

#[derive(Debug, Serialize)]
struct SessionRequest {
    app_id: String,
    password: String,
}

// ── Freebox Client ───────────────────────────────────────────────────

/// Authenticated Freebox API client.
pub struct FreeboxClient {
    client: Client,
    base_url: String,
    api_base: String,
    session_token: String,
}

const APP_ID: &str = "tom.gateway";

impl FreeboxClient {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}{}", self.base_url, self.api_base, path);
        let resp = self
            .client
            .get(&url)
            .header("X-Fbx-App-Auth", &self.session_token)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        let body: FreeboxResponse<T> = resp.json().await?;
        unwrap_response(body)
    }

    async fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        let url = format!("{}{}{}", self.base_url, self.api_base, path);
        let resp = self
            .client
            .post(&url)
            .header("X-Fbx-App-Auth", &self.session_token)
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;

        let body: FreeboxResponse<T> = resp.json().await?;
        unwrap_response(body)
    }

    async fn delete_req(&self, path: &str) -> Result<()> {
        let url = format!("{}{}{}", self.base_url, self.api_base, path);
        let resp = self
            .client
            .delete(&url)
            .header("X-Fbx-App-Auth", &self.session_token)
            .send()
            .await
            .with_context(|| format!("DELETE {url}"))?;

        let body: FreeboxResponse<serde_json::Value> = resp.json().await?;
        if !body.success {
            let code = body.error_code.unwrap_or_default();
            let msg = body.msg.unwrap_or_default();
            bail!("Freebox API error: {code}: {msg}");
        }
        Ok(())
    }

    // ── Public API methods ───────────────────────────────────────────

    pub async fn connection_info(&self) -> Result<ConnectionResult> {
        self.get("/connection/").await
    }

    pub async fn lan_browser(&self) -> Result<Vec<LanHost>> {
        self.get("/lan/browser/pub/").await
    }

    pub async fn list_nat_rules(&self) -> Result<Vec<super::nat::NatRule>> {
        self.get("/fw/redir/").await
    }

    pub async fn create_nat_rule(
        &self,
        rule: &super::nat::NatRuleRequest,
    ) -> Result<super::nat::NatRule> {
        self.post("/fw/redir/", rule).await
    }

    pub async fn delete_nat_rule(&self, id: u64) -> Result<()> {
        self.delete_req(&format!("/fw/redir/{id}")).await
    }
}

// ── Free functions ───────────────────────────────────────────────────

fn unwrap_response<T>(resp: FreeboxResponse<T>) -> Result<T> {
    if !resp.success {
        let code = resp.error_code.unwrap_or_default();
        let msg = resp.msg.unwrap_or_default();
        if code == "auth_required" {
            bail!("Session expired or revoked. Run 'tom-gateway auth' to re-authenticate.\n  API: {msg}");
        }
        bail!("Freebox API error [{code}]: {msg}");
    }
    resp.result.context("Freebox API returned success but no result")
}

/// Compute HMAC-SHA1(app_token, challenge) as lowercase hex string.
pub fn compute_password(app_token: &str, challenge: &str) -> String {
    let mut mac =
        HmacSha1::new_from_slice(app_token.as_bytes()).expect("HMAC accepts any key length");
    mac.update(challenge.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Discover Freebox API endpoint.
pub async fn discover(url: Option<&str>) -> Result<(String, String)> {
    let base = url.unwrap_or("http://mafreebox.freebox.fr");
    let discovery_url = format!("{}/api_version", base.trim_end_matches('/'));

    let client = Client::new();
    let resp: ApiVersion = client
        .get(&discovery_url)
        .send()
        .await
        .with_context(|| format!("Failed to reach Freebox at {discovery_url}"))?
        .json()
        .await
        .context("Failed to parse Freebox discovery response")?;

    // Extract major version for API path (e.g. "14.0" -> "/api/v14")
    let major = resp
        .api_version
        .split('.')
        .next()
        .unwrap_or("4");
    let api_base = format!("{}v{}", resp.api_base_url, major);

    tracing::info!(
        "Discovered {} ({}), API {}",
        resp.device_name,
        base,
        api_base
    );
    Ok((base.to_string(), api_base))
}

/// One-time authorization flow.
/// Returns (app_id, app_token) on success.
pub async fn authorize(base_url: &str, api_base: &str, app_name: &str) -> Result<(String, String)> {
    let client = Client::new();
    let url = format!("{}{}/login/authorize/", base_url, api_base);

    let body = AuthRequest {
        app_id: APP_ID.to_string(),
        app_name: app_name.to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        device_name: hostname(),
    };

    let resp: FreeboxResponse<AuthResult> = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("Failed to request authorization")?
        .json()
        .await?;

    let auth = unwrap_response(resp)?;
    let app_token = auth.app_token;
    let track_id = auth.track_id;

    println!();
    println!(">>> Appuie sur le bouton \u{2714} sur l'ecran LCD de la Freebox...");
    println!();

    // Poll for approval (every 2s, max 60s)
    let track_url = format!("{}{}/login/authorize/{}", base_url, api_base, track_id);
    for i in 0..30 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let resp: FreeboxResponse<AuthTrackResult> =
            client.get(&track_url).send().await?.json().await?;

        if let Some(ref result) = resp.result {
            match result.status.as_str() {
                "granted" => {
                    println!("Authentification reussie !");
                    return Ok((APP_ID.to_string(), app_token));
                }
                "denied" => bail!("Autorisation refusee sur la Freebox."),
                "timeout" => bail!("Timeout — pas de reponse sur la Freebox LCD."),
                "pending" => {
                    if i % 5 == 0 {
                        println!("  En attente... ({}/60s)", (i + 1) * 2);
                    }
                }
                other => bail!("Statut inattendu: {other}"),
            }
        }
    }

    bail!("Timeout apres 60s. Reessaye 'tom-gateway auth'.")
}

/// Open a session using stored app_token.
pub async fn open_session(
    base_url: &str,
    api_base: &str,
    app_id: &str,
    app_token: &str,
) -> Result<FreeboxClient> {
    let client = Client::new();

    // 1. Get challenge
    let login_url = format!("{}{}/login/", base_url, api_base);
    let resp: FreeboxResponse<LoginResult> =
        client.get(&login_url).send().await?.json().await?;
    let login = unwrap_response(resp)?;

    // 2. Compute HMAC-SHA1 password
    let password = compute_password(app_token, &login.challenge);

    // 3. Open session
    let session_url = format!("{}{}/login/session/", base_url, api_base);
    let body = SessionRequest {
        app_id: app_id.to_string(),
        password,
    };
    let resp: FreeboxResponse<SessionResult> = client
        .post(&session_url)
        .json(&body)
        .send()
        .await
        .context("Failed to open session")?
        .json()
        .await?;
    let session = unwrap_response(resp)?;

    tracing::info!("session opened");
    Ok(FreeboxClient {
        client,
        base_url: base_url.to_string(),
        api_base: api_base.to_string(),
        session_token: session.session_token,
    })
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("USER").map(|u| format!("{u}-device")))
        .unwrap_or_else(|_| "tom-device".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sha1_produces_40_hex_chars() {
        let password = compute_password("mytoken", "mychallenge");
        assert_eq!(password.len(), 40);
        assert!(password.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hmac_sha1_deterministic() {
        let a = compute_password("token123", "challenge456");
        let b = compute_password("token123", "challenge456");
        assert_eq!(a, b);
    }

    #[test]
    fn hmac_sha1_different_inputs() {
        let a = compute_password("token1", "challenge");
        let b = compute_password("token2", "challenge");
        assert_ne!(a, b);
    }

    #[test]
    fn hmac_sha1_empty_inputs() {
        let password = compute_password("", "");
        assert_eq!(password.len(), 40);
    }

    #[test]
    fn api_version_deserialize() {
        let json = r#"{
            "uid": "abcdef",
            "device_name": "Freebox Server",
            "device_type": "FreeboxServerMini",
            "api_version": "14.0",
            "api_base_url": "/api/"
        }"#;
        let v: ApiVersion = serde_json::from_str(json).unwrap();
        assert_eq!(v.api_base_url, "/api/");
        assert_eq!(v.api_version, "14.0");
    }

    #[test]
    fn freebox_response_success() {
        let json = r#"{"success": true, "result": {"challenge": "abc123"}}"#;
        let resp: FreeboxResponse<LoginResult> = serde_json::from_str(json).unwrap();
        assert!(resp.success);
        assert_eq!(resp.result.unwrap().challenge, "abc123");
    }

    #[test]
    fn freebox_response_error() {
        let json =
            r#"{"success": false, "msg": "Invalid token", "error_code": "auth_required"}"#;
        let resp: FreeboxResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(!resp.success);
        assert_eq!(resp.error_code.unwrap(), "auth_required");
    }

    #[test]
    fn lan_host_deserialize() {
        let json = r#"{
            "primary_name": "nas-debian",
            "l2ident": {"id": "A0:78:17:AD:92:6F", "type": "mac_address"},
            "l3connectivities": [
                {"addr": "192.168.0.83", "af": "ipv4", "active": true, "reachable": true}
            ],
            "active": true,
            "reachable": true
        }"#;
        let host: LanHost = serde_json::from_str(json).unwrap();
        assert_eq!(host.primary_name.unwrap(), "nas-debian");
        let l3 = host.l3connectivities.unwrap();
        assert_eq!(l3[0].addr, "192.168.0.83");
    }
}
