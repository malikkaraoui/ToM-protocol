use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Persisted Freebox auth token.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredToken {
    pub app_id: String,
    pub app_token: String,
    pub freebox_url: String,
}

/// Default token file path: ~/.tom/freebox_token.json
pub fn default_token_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME env var not set")?;
    Ok(PathBuf::from(home).join(".tom").join("freebox_token.json"))
}

/// Save token to disk. Creates ~/.tom/ directory if needed.
pub fn save(path: &PathBuf, token: &StoredToken) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(token)?;
    std::fs::write(path, &json)
        .with_context(|| format!("failed to write {}", path.display()))?;

    // Set file permissions to 0600 (user-only read/write)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }

    tracing::info!("token saved to {}", path.display());
    Ok(())
}

/// Load token from disk.
pub fn load(path: &PathBuf) -> Result<StoredToken> {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("No token found at {}. Run 'tom-gateway auth' first.", path.display());
        }
        Err(e) => {
            return Err(e).with_context(|| format!("failed to read {}", path.display()));
        }
    };

    serde_json::from_str(&data).with_context(|| {
        format!(
            "Corrupted token file at {}. Delete it and re-run 'tom-gateway auth'.",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("tom-gateway-test-token");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("token.json");

        let token = StoredToken {
            app_id: "tom.gateway".into(),
            app_token: "secret123".into(),
            freebox_url: "http://mafreebox.freebox.fr".into(),
        };

        save(&path, &token).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.app_id, "tom.gateway");
        assert_eq!(loaded.app_token, "secret123");
        assert_eq!(loaded.freebox_url, "http://mafreebox.freebox.fr");

        // Check permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&path).unwrap().permissions();
            assert_eq!(perms.mode() & 0o777, 0o600);
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_missing_file() {
        let path = PathBuf::from("/tmp/tom-gateway-nonexistent-xyz/token.json");
        let result = load(&path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No token found"), "got: {err}");
    }

    #[test]
    fn load_corrupted_file() {
        let dir = std::env::temp_dir().join("tom-gateway-test-corrupt");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("token.json");
        std::fs::write(&path, "not json").unwrap();

        let result = load(&path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Corrupted"), "got: {err}");

        std::fs::remove_dir_all(&dir).ok();
    }
}
