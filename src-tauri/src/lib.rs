mod db;
mod crypto;
mod registry;
mod vault;
mod ssh;
mod sftp;
mod tunnel;

use db::DbPool;
use vault::VaultState;
use db::profiles::{self, CreateProfileInput, UpdateProfileInput, ProfileSummary, Group};
use db::snippets::{self, Snippet, CreateSnippetInput, UpdateSnippetInput};
use db::custom_recipes::{self, CustomRecipe, CreateRecipeInput, UpdateRecipeInput};
use ssh::session::SessionManager;
use ssh::types::{ConnectionConfig, AuthMethod, SessionInfo, OsInfo};
use ssh::keys::SshKeyInfo;
use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Shello.", name)
}

// ── SSH Commands (Sprint 2–3) ────────────────────────────────────────

#[tauri::command]
async fn ssh_connect(
    state: tauri::State<'_, SessionManager>,
    config: ConnectionConfig,
) -> Result<String, String> {
    state.connect(config).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ssh_exec(
    state: tauri::State<'_, SessionManager>,
    session_id: String,
    command: String,
) -> Result<String, String> {
    state.exec(&session_id, &command).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ssh_disconnect(
    state: tauri::State<'_, SessionManager>,
    tunnels: tauri::State<'_, tunnel::TunnelManager>,
    session_id: String,
) -> Result<(), String> {
    tunnels.stop_all(&session_id).await;
    state.disconnect(&session_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ssh_list_sessions(
    state: tauri::State<'_, SessionManager>,
) -> Result<Vec<SessionInfo>, String> {
    Ok(state.list().await)
}

#[tauri::command]
async fn ssh_open_pty(
    state: tauri::State<'_, SessionManager>,
    app_handle: tauri::AppHandle,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    state.open_pty(&session_id, cols, rows, app_handle).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ssh_write(
    state: tauri::State<'_, SessionManager>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    state.write(&session_id, data).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ssh_resize(
    state: tauri::State<'_, SessionManager>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    state.resize(&session_id, cols, rows).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ssh_detect_os(
    state: tauri::State<'_, SessionManager>,
    pool: tauri::State<'_, DbPool>,
    session_id: String,
    profile_id: Option<String>,
) -> Result<OsInfo, String> {
    let os_info = state.detect_os(&session_id).await.map_err(|e| e.to_string())?;

    // Persist to profile if provided
    if let Some(pid) = profile_id {
        let _ = profiles::update_detected_os(&pool, &pid, &os_info.distro).await;
    }

    Ok(os_info)
}

#[tauri::command]
async fn ssh_has_running_process(
    state: tauri::State<'_, SessionManager>,
    session_id: String,
) -> Result<bool, String> {
    // Check if there are foreground processes beyond the shell itself
    // `ps -t $(tty) -o stat=` lists process states on the current TTY
    // A process with 'S+' or 'R+' in the stat column means a foreground process
    let output = state
        .exec(&session_id, "ps -o stat= -t $(tty) 2>/dev/null | grep -c '+' || echo 0")
        .await
        .unwrap_or_else(|_| "0".to_string());
    let count: i32 = output.trim().parse().unwrap_or(0);
    // count > 1 means there's something besides the shell's own ps command
    Ok(count > 1)
}

// ── Profile Commands (Sprint 4) ─────────────────────────────────────

#[tauri::command]
async fn profile_list(
    pool: tauri::State<'_, DbPool>,
) -> Result<Vec<ProfileSummary>, String> {
    profiles::list_profiles(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn profile_create(
    pool: tauri::State<'_, DbPool>,
    vault: tauri::State<'_, VaultState>,
    input: CreateProfileInput,
) -> Result<ProfileSummary, String> {
    profiles::create_profile(&pool, input, &vault).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn profile_update(
    pool: tauri::State<'_, DbPool>,
    vault: tauri::State<'_, VaultState>,
    id: String,
    input: UpdateProfileInput,
) -> Result<ProfileSummary, String> {
    profiles::update_profile(&pool, &id, input, &vault).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn profile_delete(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<(), String> {
    profiles::delete_profile(&pool, &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn profile_search(
    pool: tauri::State<'_, DbPool>,
    query: String,
) -> Result<Vec<ProfileSummary>, String> {
    profiles::search_profiles(&pool, &query).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn profile_connect(
    pool: tauri::State<'_, DbPool>,
    vault: tauri::State<'_, VaultState>,
    session_manager: tauri::State<'_, SessionManager>,
    app_handle: tauri::AppHandle,
    profile_id: String,
    cols: u32,
    rows: u32,
    timeout_secs: Option<u64>,
) -> Result<String, String> {
    let profile = profiles::get_profile(&pool, &profile_id)
        .await
        .map_err(|e| e.to_string())?;

    let (password, key_path, passphrase) = profiles::get_profile_decrypted(&pool, &profile_id, &vault)
        .await
        .map_err(|e| e.to_string())?;

    let auth = match profile.auth_type.as_str() {
        "password" => AuthMethod::Password { password },
        "key" => AuthMethod::Key {
            private_key_path: key_path,
            passphrase,
        },
        other => return Err(format!("Unknown auth type: {}", other)),
    };

    let config = ConnectionConfig {
        host: profile.host,
        port: profile.port as u16,
        username: profile.username,
        auth,
        timeout_secs: timeout_secs.unwrap_or(10),
    };

    let session_id = session_manager
        .connect(config)
        .await
        .map_err(|e| e.to_string())?;

    session_manager
        .open_pty(&session_id, cols, rows, app_handle)
        .await
        .map_err(|e| e.to_string())?;

    Ok(session_id)
}

// ── Group Commands (Sprint 4) ───────────────────────────────────────

#[tauri::command]
async fn group_list(
    pool: tauri::State<'_, DbPool>,
) -> Result<Vec<Group>, String> {
    profiles::list_groups(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn group_create(
    pool: tauri::State<'_, DbPool>,
    name: String,
    color: String,
) -> Result<Group, String> {
    profiles::create_group(&pool, &name, &color).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn group_update(
    pool: tauri::State<'_, DbPool>,
    id: String,
    name: String,
    color: String,
) -> Result<Group, String> {
    profiles::update_group(&pool, &id, &name, &color).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn group_delete(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<(), String> {
    profiles::delete_group(&pool, &id).await.map_err(|e| e.to_string())
}

/// Get decrypted password for a profile (used for auto-fill features).
#[tauri::command]
async fn profile_get_password(
    pool: tauri::State<'_, DbPool>,
    vault: tauri::State<'_, VaultState>,
    profile_id: String,
) -> Result<String, String> {
    let (password, _, _) = profiles::get_profile_decrypted(&pool, &profile_id, &vault)
        .await
        .map_err(|e| e.to_string())?;
    Ok(password)
}

// ── Snippet Commands ─────────────────────────────────────────────────

#[tauri::command]
async fn snippet_list(pool: tauri::State<'_, DbPool>) -> Result<Vec<Snippet>, String> {
    snippets::list_snippets(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn snippet_create(pool: tauri::State<'_, DbPool>, input: CreateSnippetInput) -> Result<Snippet, String> {
    snippets::create_snippet(&pool, input).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn snippet_update(pool: tauri::State<'_, DbPool>, id: String, input: UpdateSnippetInput) -> Result<Snippet, String> {
    snippets::update_snippet(&pool, &id, input).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn snippet_delete(pool: tauri::State<'_, DbPool>, id: String) -> Result<(), String> {
    snippets::delete_snippet(&pool, &id).await.map_err(|e| e.to_string())
}

// ── SSH Key Commands ────────────────────────────────────────────────

#[tauri::command]
async fn ssh_list_keys() -> Result<Vec<SshKeyInfo>, String> {
    ssh::keys::list_keys().map_err(|e| e.to_string())
}

#[tauri::command]
async fn ssh_generate_key(
    name: String,
    key_type: String,
    passphrase: Option<String>,
) -> Result<SshKeyInfo, String> {
    ssh::keys::generate_key(&name, &key_type, passphrase.as_deref())
        .await
        .map_err(|e| e.to_string())
}

// ── Session Logging Commands ────────────────────────────────────────

#[tauri::command]
async fn ssh_start_logging(
    state: tauri::State<'_, SessionManager>,
    session_id: String,
    path: String,
) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    state.start_logging(&session_id, &path).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ssh_stop_logging(
    state: tauri::State<'_, SessionManager>,
    session_id: String,
) -> Result<(), String> {
    state.stop_logging(&session_id).await.map_err(|e| e.to_string())
}

// ── Custom Recipe Commands ────────────────────────────────────────────

#[tauri::command]
async fn custom_recipe_list(pool: tauri::State<'_, DbPool>) -> Result<Vec<CustomRecipe>, String> {
    custom_recipes::list_recipes(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn custom_recipe_create(pool: tauri::State<'_, DbPool>, input: CreateRecipeInput) -> Result<CustomRecipe, String> {
    custom_recipes::create_recipe(&pool, input).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn custom_recipe_update(pool: tauri::State<'_, DbPool>, id: String, input: UpdateRecipeInput) -> Result<CustomRecipe, String> {
    custom_recipes::update_recipe(&pool, &id, input).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn custom_recipe_delete(pool: tauri::State<'_, DbPool>, id: String) -> Result<(), String> {
    custom_recipes::delete_recipe(&pool, &id).await.map_err(|e| e.to_string())
}

// ── Registry Commands ─────────────────────────────────────────────────

#[tauri::command]
async fn registry_sync(
    pool: tauri::State<'_, DbPool>,
) -> Result<std::collections::HashMap<String, String>, String> {
    registry::sync_all(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn registry_get(
    pool: tauri::State<'_, DbPool>,
    key: String,
) -> Result<Option<String>, String> {
    registry::get_cached(&pool, &key).await.map_err(|e| e.to_string())
}

// ── App Builder ─────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let pool = tauri::async_runtime::block_on(db::init(&app_data_dir))?;

            let vault_state = VaultState::new();
            tauri::async_runtime::block_on(vault::try_auto_unlock(&pool, &vault_state));

            app.manage(pool);
            app.manage(vault_state);
            Ok(())
        })
        .manage(SessionManager::new())
        .manage(sftp::SftpManager::new())
        .manage(tunnel::TunnelManager::new())
        .invoke_handler(tauri::generate_handler![
            greet,
            ssh_connect,
            ssh_exec,
            ssh_disconnect,
            ssh_list_sessions,
            ssh_open_pty,
            ssh_write,
            ssh_resize,
            ssh_detect_os,
            ssh_has_running_process,
            profile_list,
            profile_create,
            profile_update,
            profile_delete,
            profile_search,
            profile_connect,
            profile_get_password,
            group_list,
            group_create,
            group_update,
            group_delete,
            snippet_list,
            snippet_create,
            snippet_update,
            snippet_delete,
            ssh_list_keys,
            ssh_generate_key,
            ssh_start_logging,
            ssh_stop_logging,
            custom_recipe_list,
            custom_recipe_create,
            custom_recipe_update,
            custom_recipe_delete,
            registry_sync,
            registry_get,
            vault::vault_status,
            vault::vault_setup,
            vault::vault_unlock,
            vault::vault_lock,
            vault::vault_change_password,
            vault::vault_forget_device,
            sftp::sftp_open,
            sftp::sftp_list,
            sftp::sftp_local_list,
            sftp::sftp_local_home,
            sftp::sftp_mkdir,
            sftp::sftp_delete,
            sftp::sftp_rename,
            sftp::sftp_create_file,
            sftp::sftp_chmod,
            sftp::sftp_download,
            sftp::sftp_upload,
            sftp::sftp_cancel,
            sftp::sftp_close,
            ssh::config_import::ssh_config_parse,
            tunnel::tunnel_list,
            tunnel::tunnel_create,
            tunnel::tunnel_update,
            tunnel::tunnel_delete,
            tunnel::tunnel_start,
            tunnel::tunnel_stop,
            tunnel::tunnel_active,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
