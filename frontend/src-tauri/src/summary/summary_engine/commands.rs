// Tauri commands for built-in AI model management
// Exposes model download, status, and management functionality to frontend

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::sync::Mutex;

use super::model_manager::{DownloadProgress, ModelInfo, ModelManager};

const QWEN35_4B_RECOMMENDED_RAM_GB: u64 = 14;

pub(crate) fn summary_model_priority(model_name: &str) -> u8 {
    match model_name {
        "qwen3.5:4b" => 4,
        "qwen3.5:2b" => 3,
        "gemma3:4b" => 2,
        "gemma3:1b" => 1,
        _ => 0,
    }
}

pub(crate) fn recommend_summary_model(_is_macos: bool, _system_ram_gb: u64) -> &'static str {
    "gemma-4"
}

pub(crate) fn get_recommended_summary_model_for_current_system() -> Result<&'static str, String> {
    let system_ram_gb = get_system_ram_gb()?;
    let is_macos = cfg!(target_os = "macos");

    log::info!(
        "System RAM detected: {} GB, Platform: {}",
        system_ram_gb,
        if is_macos { "macOS" } else { "other" }
    );

    Ok(recommend_summary_model(is_macos, system_ram_gb))
}

// ============================================================================
// Global State
// ============================================================================

/// Global model manager instance
pub struct ModelManagerState(pub Arc<Mutex<Option<Arc<ModelManager>>>>);

/// Initialize the model manager
pub async fn init_model_manager<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<()> {
    let models_dir = app.path().app_data_dir()?.join("models").join("summary");

    let manager = ModelManager::new_with_models_dir(Some(models_dir))?;
    manager.init().await?;

    let state: State<ModelManagerState> = app.state();
    let mut manager_lock = state.0.lock().await;
    *manager_lock = Some(Arc::new(manager));

    log::info!("Built-in AI model manager initialized");
    Ok(())
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// List all available built-in AI models with their status
#[tauri::command]
pub async fn builtin_ai_list_models<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
) -> Result<Vec<ModelInfo>, String> {
    let manager = {
        // Ensure manager is initialized
        {
            let manager_lock = state.0.lock().await;
            if manager_lock.is_none() {
                drop(manager_lock);
                init_model_manager(&app)
                    .await
                    .map_err(|e| format!("Failed to initialize model manager: {}", e))?;
            }
        }

        let manager_lock = state.0.lock().await;
        manager_lock
            .as_ref()
            .ok_or_else(|| "Model manager not initialized".to_string())?
            .clone()
    };

    let models = manager.list_models().await;
    Ok(models)
}

/// Get information about a specific model
#[tauri::command]
pub async fn builtin_ai_get_model_info<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<Option<ModelInfo>, String> {
    let manager = {
        // Ensure manager is initialized
        {
            let manager_lock = state.0.lock().await;
            if manager_lock.is_none() {
                drop(manager_lock);
                init_model_manager(&app)
                    .await
                    .map_err(|e| format!("Failed to initialize model manager: {}", e))?;
            }
        }

        let manager_lock = state.0.lock().await;
        manager_lock
            .as_ref()
            .ok_or_else(|| "Model manager not initialized".to_string())?
            .clone()
    };

    let info = manager.get_model_info(&model_name).await;
    Ok(info)
}

/// Download a built-in AI model with progress updates (bypassed in hosted mode)
#[tauri::command]
pub async fn builtin_ai_download_model<R: Runtime>(
    app: AppHandle<R>,
    _state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<(), String> {
    log::info!("Hosted mode: skipping local download for {}", model_name);
    let _ = app.emit(
        "builtin-ai-download-progress",
        serde_json::json!({
            "model": model_name,
            "progress": 100,
            "downloaded_mb": 0,
            "total_mb": 0,
            "speed_mbps": 0,
            "status": "completed"
        }),
    );
    Ok(())
}

/// Cancel an ongoing model download
#[tauri::command]
pub async fn builtin_ai_cancel_download<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<(), String> {
    let manager = {
        let manager_lock = state.0.lock().await;
        manager_lock
            .as_ref()
            .ok_or_else(|| "Model manager not initialized".to_string())?
            .clone()
    };

    manager
        .cancel_download(&model_name)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "builtin-ai-download-progress",
        serde_json::json!({
            "model": model_name,
            "progress": 0,
            "status": "cancelled"
        }),
    );

    Ok(())
}

/// Delete a corrupted or available model file
#[tauri::command]
pub async fn builtin_ai_delete_model(
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<(), String> {
    let manager = {
        let manager_lock = state.0.lock().await;
        manager_lock
            .as_ref()
            .ok_or_else(|| "Model manager not initialized".to_string())?
            .clone()
    };

    manager
        .delete_model(&model_name)
        .await
        .map_err(|e| e.to_string())
}

/// Check if a model is ready to use
#[tauri::command]
pub async fn builtin_ai_is_model_ready<R: Runtime>(
    _app: AppHandle<R>,
    _state: State<'_, ModelManagerState>,
    model_name: String,
    _refresh: Option<bool>,
) -> Result<bool, String> {
    log::info!("Hosted mode: model '{}' ready check -> true", model_name);
    Ok(true)
}

/// Check if any summary model is available (for onboarding)
/// Returns the first available model name by priority, or None if no models exist
#[tauri::command]
pub async fn builtin_ai_get_available_summary_model<R: Runtime>(
    _app: AppHandle<R>,
    _state: State<'_, ModelManagerState>,
) -> Result<Option<String>, String> {
    Ok(Some("gemma-4".to_string()))
}

// ============================================================================
// Startup Initialization & Utility Commands
// ============================================================================

pub async fn init_model_manager_at_startup<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<(), String> {
    let models_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("models")
        .join("summary");

    let manager = ModelManager::new_with_models_dir(Some(models_dir))
        .map_err(|e| format!("Failed to create ModelManager: {}", e))?;

    manager
        .init()
        .await
        .map_err(|e| format!("Failed to initialize ModelManager: {}", e))?;

    let state: State<ModelManagerState> = app.state();
    let mut manager_lock = state.0.lock().await;
    *manager_lock = Some(Arc::new(manager));

    log::info!("ModelManager initialized at startup");
    Ok(())
}


/// Get recommended summary model based on platform and system RAM.
/// macOS → qwen3.5:4b
/// non-macOS + <8GB RAM → qwen3.5:2b
/// non-macOS + >=8GB RAM → qwen3.5:4b
#[tauri::command]
pub async fn builtin_ai_get_recommended_model() -> Result<String, String> {
    let recommended = get_recommended_summary_model_for_current_system()?;

    log::info!("Recommended summary model: {}", recommended);
    Ok(recommended.to_string())
}

/// Get total system RAM in gigabytes
fn get_system_ram_gb() -> Result<u64, String> {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_memory();

    let total_memory_bytes = sys.total_memory();
    let total_memory_gb = total_memory_bytes / (1024 * 1024 * 1024);

    Ok(total_memory_gb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommended_summary_model_uses_gemma4() {
        assert_eq!(recommend_summary_model(true, 13), "gemma-4");
        assert_eq!(recommend_summary_model(false, 16), "gemma-4");
    }
}
