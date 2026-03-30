use crate::utils::protobuf;
use rusqlite::Connection;
use std::path::PathBuf;

use super::version::AntigravityVersion;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ResolvedAntigravityTarget {
    pub executable_path: Option<PathBuf>,
    pub startup_args: Option<Vec<String>>,
    pub user_data_dir: Option<PathBuf>,
    pub storage_path: PathBuf,
    pub db_path: PathBuf,
    pub version: Option<AntigravityVersion>,
}

fn build_auth_status_json(access_token: &str, email: &str) -> String {
    let display_name = protobuf::derive_display_name(email);
    let initials = protobuf::derive_display_initials(&display_name);

    serde_json::json!({
        "name": initials,
        "apiKey": access_token,
        "email": email,
    })
    .to_string()
}

fn get_antigravity_path() -> Option<PathBuf> {
    if let Ok(config) = crate::modules::config::load_app_config() {
        if let Some(path_str) = config.antigravity_executable {
            let path = PathBuf::from(path_str);
            if path.exists() {
                return Some(path);
            }
        }
    }
    crate::modules::process::get_antigravity_executable_path()
}

fn get_default_storage_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or("failed_to_get_home_dir")?;
        return Ok(home.join("Library/Application Support/Antigravity/User/globalStorage/storage.json"));
    }

    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").map_err(|_| "failed_to_get_appdata_env".to_string())?;
        return Ok(PathBuf::from(appdata).join("Antigravity\\User\\globalStorage\\storage.json"));
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().ok_or("failed_to_get_home_dir")?;
        return Ok(home.join(".config/Antigravity/User/globalStorage/storage.json"));
    }
}

fn build_profile_paths(user_data_dir: Option<&PathBuf>, executable_path: Option<&PathBuf>) -> Result<(PathBuf, PathBuf), String> {
    if let Some(user_data_dir) = user_data_dir {
        let storage_path = user_data_dir.join("User").join("globalStorage").join("storage.json");
        let db_path = user_data_dir.join("User").join("globalStorage").join("state.vscdb");
        return Ok((storage_path, db_path));
    }

    if let Some(executable_path) = executable_path {
        if let Some(parent_dir) = executable_path.parent() {
            let storage_path = parent_dir
                .join("data")
                .join("user-data")
                .join("User")
                .join("globalStorage")
                .join("storage.json");
            let db_path = parent_dir
                .join("data")
                .join("user-data")
                .join("User")
                .join("globalStorage")
                .join("state.vscdb");

            if storage_path.exists() || db_path.exists() {
                return Ok((storage_path, db_path));
            }
        }
    }

    let storage_path = get_default_storage_path()?;
    let db_path = storage_path
        .parent()
        .ok_or_else(|| "failed_to_get_storage_parent_dir".to_string())?
        .join("state.vscdb");
    Ok((storage_path, db_path))
}

pub fn resolve_antigravity_target() -> Result<ResolvedAntigravityTarget, String> {
    let startup_args = crate::modules::process::get_configured_antigravity_args()
        .or_else(crate::modules::process::get_args_from_running_process);
    let user_data_dir = startup_args
        .as_ref()
        .and_then(|args| crate::modules::process::get_user_data_dir_from_args(args));
    let executable_path = crate::modules::process::get_configured_antigravity_executable_path()
        .or_else(crate::modules::process::get_path_from_running_process)
        .or_else(get_antigravity_path);
    let (storage_path, db_path) = build_profile_paths(user_data_dir.as_ref(), executable_path.as_ref())?;
    let version = executable_path
        .as_ref()
        .and_then(|path| crate::modules::version::get_antigravity_version_for_path(path).ok());

    Ok(ResolvedAntigravityTarget {
        executable_path,
        startup_args,
        user_data_dir,
        storage_path,
        db_path,
        version,
    })
}

/// Get Antigravity database path (cross-platform)
pub fn get_db_path() -> Result<PathBuf, String> {
    // Prefer path specified by --user-data-dir argument
    if let Some(user_data_dir) = crate::modules::process::get_user_data_dir_from_process() {
        let custom_db_path = user_data_dir.join("User").join("globalStorage").join("state.vscdb");
        if custom_db_path.exists() {
            return Ok(custom_db_path);
        }
    }

    // Check if in portable mode
    if let Some(antigravity_path) = get_antigravity_path() {
        if let Some(parent_dir) = antigravity_path.parent() {
            let portable_db_path = PathBuf::from(parent_dir)
                .join("data")
                .join("user-data")
                .join("User")
                .join("globalStorage")
                .join("state.vscdb");

            if portable_db_path.exists() {
                return Ok(portable_db_path);
            }
        }
    }

    // Standard mode: use system default path
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or("Failed to get home directory")?;
        Ok(home.join("Library/Application Support/Antigravity/User/globalStorage/state.vscdb"))
    }

    #[cfg(target_os = "windows")]
    {
        let appdata =
            std::env::var("APPDATA").map_err(|_| "Failed to get APPDATA environment variable".to_string())?;
        Ok(PathBuf::from(appdata).join("Antigravity\\User\\globalStorage\\state.vscdb"))
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().ok_or("Failed to get home directory")?;
        Ok(home.join(".config/Antigravity/User/globalStorage/state.vscdb"))
    }
}

/// Inject Token and Email into database
#[allow(dead_code)]
pub fn inject_token(
    db_path: &PathBuf,
    access_token: &str,
    refresh_token: &str,
    expiry: i64,
    email: &str,
    is_gcp_tos: bool,
    project_id: Option<&str>,
) -> Result<String, String> {
    inject_token_with_version(
        db_path,
        access_token,
        refresh_token,
        expiry,
        email,
        is_gcp_tos,
        project_id,
        None,
    )
}

pub fn inject_token_with_version(
    db_path: &PathBuf,
    access_token: &str,
    refresh_token: &str,
    expiry: i64,
    email: &str,
    is_gcp_tos: bool,
    project_id: Option<&str>,
    version_override: Option<&AntigravityVersion>,
) -> Result<String, String> {
    crate::modules::logger::log_info("Starting Token injection...");

    let version_result = version_override
        .cloned()
        .map(Ok)
        .unwrap_or_else(crate::modules::version::get_antigravity_version);

    match version_result {
        Ok(ver) => {
            crate::modules::logger::log_info(&format!(
                "Detected Antigravity version: {}",
                ver.short_version
            ));
            
            // 2. Choose injection strategy based on version
            if crate::modules::version::is_new_version(&ver) {
                // >= 1.16.5: Use new format only
                crate::modules::logger::log_info(
                    "Using new format injection (antigravityUnifiedStateSync.oauthToken)",
                );
                inject_new_format(
                    db_path,
                    access_token,
                    refresh_token,
                    expiry,
                    email,
                    is_gcp_tos,
                    project_id,
                )
            } else {
                // < 1.16.5: Use old format only
                crate::modules::logger::log_info(
                    "Using old format injection (jetskiStateSync.agentManagerInitState)",
                );
                inject_old_format(db_path, access_token, refresh_token, expiry, email)
            }
        }
        Err(e) => {
            // Cannot detect version: Try both formats (fallback)
            crate::modules::logger::log_warn(&format!(
                "Version detection failed, trying both formats for compatibility: {}",
                e
            ));
            
            // Try new format first
            let new_result = inject_new_format(
                db_path,
                access_token,
                refresh_token,
                expiry,
                email,
                is_gcp_tos,
                project_id,
            );
            
            // Try old format
            let old_result = inject_old_format(db_path, access_token, refresh_token, expiry, email);
            
            // Return success if either format succeeded
            if new_result.is_ok() || old_result.is_ok() {
                Ok("Token injection successful (dual format fallback)".to_string())
            } else {
                Err(format!(
                    "Both formats failed - New: {:?}, Old: {:?}",
                    new_result.err(),
                    old_result.err()
                ))
            }
        }
    }
}

/// Ensure ItemTable schema exists (for brand-new DB files created when IDE has never run)
fn ensure_item_table_exists(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ItemTable (
            key TEXT UNIQUE ON CONFLICT REPLACE,
            value BLOB
        );"
    ).map_err(|e| format!("Failed to ensure ItemTable schema: {}", e))
}

/// New format injection (>= 1.16.5)
fn inject_new_format(
    db_path: &PathBuf,
    access_token: &str,
    refresh_token: &str,
    expiry: i64,
    email: &str,
    is_gcp_tos: bool,
    project_id: Option<&str>,
) -> Result<String, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open database: {}", e))?;
    
    // [FIX TH1] Ensure ItemTable exists - for brand-new accounts that have never
    // signed into the IDE, state.vscdb won't have any tables yet.
    ensure_item_table_exists(&conn)?;
    
    // Create OAuthTokenInfo (binary)
    let oauth_info = protobuf::create_oauth_info(access_token, refresh_token, expiry, is_gcp_tos);
    let outer_b64 = protobuf::create_unified_state_entry("oauthTokenInfoSentinelKey", &oauth_info);
    
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)",
        ["antigravityUnifiedStateSync.oauthToken", &outer_b64],
    )
    .map_err(|e| format!("Failed to write new format: {}", e))?;
    
    inject_user_status(&conn, email)?;

    // [FIX] Inject antigravityAuthStatus — this plain-JSON key is written
    // by the IDE after its own OAuth flow. Without it, the IDE cannot recognize the
    // injected token for accounts that have never logged into the IDE directly, causing
    // the Login screen to appear broken (no pre-filled account / auth fails after login).
    inject_auth_status(&conn, access_token, email)?;

    if let Some(project_id) = project_id.map(str::trim).filter(|pid| !pid.is_empty()) {
        inject_enterprise_project_preference(&conn, project_id)?;
    } else {
        clear_enterprise_project_preference(&conn)?;
    }

    // Inject Onboarding flag
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)",
        ["antigravityOnboarding", "true"],
    )
    .map_err(|e| format!("Failed to write onboarding flag: {}", e))?;
    
    Ok("Token injection successful (new format)".to_string())
}

fn inject_user_status(conn: &Connection, email: &str) -> Result<(), String> {
    let payload = protobuf::create_minimal_user_status_payload(email);
    let entry_b64 = protobuf::create_unified_state_entry("userStatusSentinelKey", &payload);

    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)",
        ["antigravityUnifiedStateSync.userStatus", &entry_b64],
    )
    .map_err(|e| format!("Failed to write user status: {}", e))?;

    Ok(())
}

/// Inject `antigravityAuthStatus` — plain-JSON key that the IDE writes after its own
/// OAuth flow. Format from live IDE: `{"name":"<initials>","apiKey":"<access_token>"}`.
///
/// Without this key, accounts that have never logged into the IDE directly cannot be
/// recognized by the IDE after a token-injection switch: the Login screen either shows
/// no pre-filled account or fails authentication silently (TH1 bug).
fn inject_auth_status(conn: &Connection, access_token: &str, email: &str) -> Result<(), String> {
    let display_name = protobuf::derive_display_name(email);
    let name = protobuf::derive_display_initials(&display_name);
    let json = build_auth_status_json(access_token, email);

    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)",
        ["antigravityAuthStatus", &json],
    )
    .map_err(|e| format!("Failed to write auth status: {}", e))?;

    crate::modules::logger::log_info(&format!(
        "Injected antigravityAuthStatus for {} (name={})",
        email, name
    ));

    Ok(())
}

fn inject_enterprise_project_preference(conn: &Connection, project_id: &str) -> Result<(), String> {
    let payload = protobuf::create_string_value_payload(project_id);
    let entry_b64 = protobuf::create_unified_state_entry("enterpriseGcpProjectId", &payload);

    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)",
        [
            "antigravityUnifiedStateSync.enterprisePreferences",
            &entry_b64,
        ],
    )
    .map_err(|e| format!("Failed to write enterprise preferences: {}", e))?;

    Ok(())
}

fn clear_enterprise_project_preference(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "DELETE FROM ItemTable WHERE key = ?",
        ["antigravityUnifiedStateSync.enterprisePreferences"],
    )
    .map_err(|e| format!("Failed to clear enterprise preferences: {}", e))?;

    Ok(())
}

/// Old format injection (< 1.16.5)
fn inject_old_format(
    db_path: &PathBuf,
    access_token: &str,
    refresh_token: &str,
    expiry: i64,
    email: &str,
) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    use rusqlite::Error as SqliteError;

    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    // [FIX TH1] Ensure ItemTable exists — for brand-new accounts whose state.vscdb
    // was just auto-created by SQLite (empty file, no tables yet).
    // Without this, SELECT below would error with "no such table: ItemTable"
    // instead of QueryReturnedNoRows, causing the switch to fail entirely.
    ensure_item_table_exists(&conn)?;

    // Read current data — if the key doesn't exist yet (brand-new account that has never
    // signed into the IDE before), treat it as an empty protobuf blob instead of failing.
    // This is the exact scenario that caused the "switch account works only after first
    // direct IDE login" bug: the key is absent for new accounts so UPDATE silently did
    // nothing and the IDE restarted showing the "needs login" state.
    let blob: Vec<u8> = match conn.query_row(
        "SELECT value FROM ItemTable WHERE key = ?",
        ["jetskiStateSync.agentManagerInitState"],
        |row| row.get::<_, String>(0),
    ) {
        Ok(current_data) => {
            general_purpose::STANDARD
                .decode(&current_data)
                .map_err(|e| format!("Base64 decoding failed: {}", e))?
        }
        Err(SqliteError::QueryReturnedNoRows) => {
            // [FIX] Account has never authenticated with the IDE directly.
            // The key does not exist yet — start from an empty protobuf blob.
            crate::modules::logger::log_info(
                "Old format key absent (new account) — creating fresh entry for account switch."
            );
            Vec::new()
        }
        Err(e) => return Err(format!("Failed to read data: {}", e)),
    };

    // Remove old fields
    let mut clean_data = protobuf::remove_field(&blob, 1)?; // UserID
    clean_data = protobuf::remove_field(&clean_data, 2)?;   // Email
    clean_data = protobuf::remove_field(&clean_data, 6)?;   // OAuthTokenInfo

    // Create new fields
    let new_email_field = protobuf::create_email_field(email);
    let new_oauth_field = protobuf::create_oauth_field(access_token, refresh_token, expiry);

    // Merge data
    // We intentionally do NOT re-inject Field 1 (UserID) to force the client
    // to re-authenticate the session with the new token.
    let final_data = [clean_data, new_email_field, new_oauth_field].concat();
    let final_b64 = general_purpose::STANDARD.encode(&final_data);

    // [FIX] Use INSERT OR REPLACE instead of UPDATE so that brand-new accounts
    // (where the key row does not yet exist) also get written correctly.
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)",
        ["jetskiStateSync.agentManagerInitState", &final_b64],
    )
    .map_err(|e| format!("Failed to write data: {}", e))?;

    inject_auth_status(&conn, access_token, email)?;
    inject_user_status(&conn, email)?;

    // Inject Onboarding flag
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)",
        ["antigravityOnboarding", "true"],
    )
    .map_err(|e| format!("Failed to write onboarding flag: {}", e))?;

    Ok("Token injection successful (old format)".to_string())
}

#[cfg(test)]
mod tests {
    use super::{build_auth_status_json, inject_new_format};
    use super::resolve_antigravity_target;
    use crate::utils::protobuf;
    use rusqlite::Connection;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("antigravity-db-test-{}-{}.vscdb", name, unique))
    }

    #[test]
    fn auth_status_json_contains_email_and_initials() {
        let json = build_auth_status_json("token-123", "john.doe@example.com");
        let value: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["name"], "JD");
        assert_eq!(value["apiKey"], "token-123");
        assert_eq!(value["email"], "john.doe@example.com");
    }

    #[test]
    fn inject_new_format_bootstraps_empty_db_for_first_time_account() {
        let db_path = temp_db_path("first-time");

        let result = inject_new_format(
            &db_path,
            "access-token",
            "refresh-token",
            1_700_000_000,
            "john.doe@example.com",
            true,
            Some("project-123"),
        );

        assert!(result.is_ok(), "expected injection success, got: {result:?}");

        let conn = Connection::open(&db_path).unwrap();

        let keys = [
            "antigravityUnifiedStateSync.oauthToken",
            "antigravityUnifiedStateSync.userStatus",
            "antigravityAuthStatus",
            "antigravityUnifiedStateSync.enterprisePreferences",
            "antigravityOnboarding",
        ];

        for key in keys {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM ItemTable WHERE key = ?",
                    [key],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "missing expected key: {key}");
        }

        let auth_status: String = conn
            .query_row(
                "SELECT value FROM ItemTable WHERE key = ?",
                ["antigravityAuthStatus"],
                |row| row.get(0),
            )
            .unwrap();
        let auth_status: Value = serde_json::from_str(&auth_status).unwrap();
        assert_eq!(auth_status["name"], "JD");
        assert_eq!(auth_status["apiKey"], "access-token");
        assert_eq!(auth_status["email"], "john.doe@example.com");

        let user_status_entry: String = conn
            .query_row(
                "SELECT value FROM ItemTable WHERE key = ?",
                ["antigravityUnifiedStateSync.userStatus"],
                |row| row.get(0),
            )
            .unwrap();
        let (sentinel, payload) = protobuf::decode_unified_state_entry(&user_status_entry).unwrap();
        assert_eq!(sentinel, "userStatusSentinelKey");
        assert_eq!(protobuf::find_field(&payload, 1).unwrap().unwrap(), b"John Doe");
        assert_eq!(
            protobuf::find_field(&payload, 3).unwrap().unwrap(),
            b"john.doe@example.com"
        );
        assert_eq!(
            protobuf::find_field(&payload, 7).unwrap().unwrap(),
            b"john.doe@example.com"
        );

        let onboarding: String = conn
            .query_row(
                "SELECT value FROM ItemTable WHERE key = ?",
                ["antigravityOnboarding"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(onboarding, "true");

        drop(conn);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn resolved_target_keeps_storage_and_db_in_same_profile() {
        let target = resolve_antigravity_target().unwrap();
        assert_eq!(
            target.db_path.parent().unwrap(),
            target.storage_path.parent().unwrap()
        );
    }
}
