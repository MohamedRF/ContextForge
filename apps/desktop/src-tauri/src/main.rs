#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use contextforge_service::{
    ContextForgeService, ServiceError, core::Repository, graph::Direction, index, integrations,
    parser, registry::Registry,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

struct State {
    registry: Registry,
    workers: Arc<tokio::sync::Semaphore>,
    indexing: Mutex<HashMap<i64, Arc<AtomicBool>>>,
    integration_plans: Mutex<HashMap<i64, (String, integrations::ChangePlan)>>,
}

#[derive(Clone, Serialize)]
struct Progress {
    project_id: i64,
    operation_id: String,
    phase: String,
    processed: usize,
    total: usize,
    elapsed_ms: u128,
}

async fn work<T: Send + 'static>(
    state: &State,
    operation: impl FnOnce(Registry) -> Result<T, ServiceError> + Send + 'static,
) -> Result<T, String> {
    let permit = state
        .workers
        .clone()
        .try_acquire_owned()
        .map_err(|_| "Application is busy; retry when an operation finishes.")?;
    let registry = state.registry.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        operation(registry).map_err(|_| {
            "Operation failed. Check repository access, index availability and input limits."
                .to_string()
        })
    })
    .await
    .map_err(|_| "Native worker failed; retry the operation.".to_string())?
}

#[tauri::command]
fn application_status() -> Value {
    ContextForgeService::diagnostics()
}

#[tauri::command]
async fn list_projects(state: tauri::State<'_, State>) -> Result<Value, String> {
    work(&state, |registry| {
        Ok(serde_json::to_value(registry.list()?)?)
    })
    .await
}

#[tauri::command]
async fn add_project(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
) -> Result<Option<i64>, String> {
    work(&state, move |registry| {
        let Some(path) = app
            .dialog()
            .file()
            .set_title("Select a repository")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        Ok(Some(registry.add(
            &path.into_path().map_err(|_| "invalid directory")?,
        )?))
    })
    .await
}

#[tauri::command]
async fn remove_project(id: i64, state: tauri::State<'_, State>) -> Result<(), String> {
    if state
        .indexing
        .lock()
        .map_err(|_| "Index state unavailable")?
        .contains_key(&id)
    {
        return Err(
            "Cancel indexing and wait for it to finish before removing the project.".into(),
        );
    }
    work(&state, move |registry| registry.remove(id)).await
}

#[tauri::command]
async fn project_status(id: i64, state: tauri::State<'_, State>) -> Result<Value, String> {
    work(&state, move |registry| {
        let repository = registry.repository(id)?;
        Ok(json!({"repository": repository.status(), "index": index::status(&repository)?, "version": env!("CARGO_PKG_VERSION")}))
    }).await
}

#[tauri::command]
async fn index_project(
    id: i64,
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
) -> Result<Value, String> {
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut tasks = state
            .indexing
            .lock()
            .map_err(|_| "Index state unavailable")?;
        if tasks.contains_key(&id) {
            return Err("This project is already indexing.".into());
        }
        tasks.insert(id, cancel.clone());
    }
    let result = work(&state, move |registry| {
        let repository = registry.repository(id)?;
        if !repository.initialized {
            repository.init()?;
        }
        let repository = registry.repository(id)?;
        let start = Instant::now();
        let operation_id = format!(
            "{id}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let mut last = start.checked_sub(Duration::from_secs(1)).unwrap_or(start);
        let result = index::index_controlled(&repository, false, &mut |phase, processed, total| {
            if last.elapsed() >= Duration::from_millis(100) || phase == "committing" {
                let _ = app.emit(
                    "index-progress",
                    Progress {
                        project_id: id,
                        operation_id: operation_id.clone(),
                        phase: phase.into(),
                        processed,
                        total,
                        elapsed_ms: start.elapsed().as_millis(),
                    },
                );
                last = Instant::now();
            }
            !cancel.load(Ordering::Relaxed)
        });
        let phase = match &result {
            Ok(_) => "ready",
            Err(index::Error::Cancelled) => "cancelled",
            Err(_) => "failed",
        };
        let _ = app.emit(
            "index-progress",
            Progress {
                project_id: id,
                operation_id,
                phase: phase.into(),
                processed: 0,
                total: 0,
                elapsed_ms: start.elapsed().as_millis(),
            },
        );
        Ok(serde_json::to_value(result?)?)
    })
    .await;
    state
        .indexing
        .lock()
        .map_err(|_| "Index state unavailable")?
        .remove(&id);
    result
}

#[tauri::command]
fn cancel_indexing(id: i64, state: tauri::State<'_, State>) -> Result<(), String> {
    if let Some(cancel) = state
        .indexing
        .lock()
        .map_err(|_| "Index state unavailable")?
        .get(&id)
    {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
async fn search_code(
    id: i64,
    query: String,
    offset: usize,
    state: tauri::State<'_, State>,
) -> Result<Value, String> {
    work(&state, move |registry| {
        Ok(serde_json::to_value(index::search::search(
            &registry.repository(id)?,
            &query,
            30,
            offset,
        )?)?)
    })
    .await
}

#[tauri::command]
async fn project_files(id: i64, state: tauri::State<'_, State>) -> Result<Value, String> {
    work(&state, move |registry| {
        Ok(serde_json::to_value(
            registry.repository(id)?.project_map(1000)?,
        )?)
    })
    .await
}

fn preview(repository: &Repository, path: &str, offset: usize) -> Result<Value, ServiceError> {
    if path.len() > 1024
        || offset > 1_000_000
        || !repository.source_path_allowed(Path::new(path))?
    {
        return Err("ineligible source path".into());
    }
    let source = repository.read_source(Path::new(path))?;
    let lines: Vec<_> = source.lines().skip(offset).take(201).collect();
    // Bound both lines and bytes returned to the webview.
    let mut text = String::new();
    let mut count = 0;
    for line in lines.iter().take(200) {
        if text.len() + line.len() + 1 > 65_536 {
            break;
        }
        text.push_str(line);
        text.push('\n');
        count += 1;
    }
    Ok(
        json!({"path": path, "offset": offset, "text": text, "lines": count, "has_more": lines.len() > count, "outline": parser::analyze(repository, Path::new(path), 500)?}),
    )
}

#[tauri::command]
async fn file_preview(
    id: i64,
    path: String,
    offset: usize,
    state: tauri::State<'_, State>,
) -> Result<Value, String> {
    work(&state, move |registry| {
        preview(&registry.repository(id)?, &path, offset)
    })
    .await
}

#[tauri::command]
async fn dependencies(
    id: i64,
    selector: String,
    incoming: bool,
    depth: usize,
    state: tauri::State<'_, State>,
) -> Result<Value, String> {
    work(&state, move |registry| {
        let graph = ContextForgeService::load_graph(&registry.repository(id)?)?;
        Ok(serde_json::to_value(graph.traverse(
            &selector,
            if incoming {
                Direction::Incoming
            } else {
                Direction::Outgoing
            },
            depth,
            100,
            200,
        )?)?)
    })
    .await
}

#[tauri::command]
async fn git_impact(
    id: i64,
    mode: String,
    base: String,
    state: tauri::State<'_, State>,
) -> Result<Value, String> {
    work(&state, move |registry| {
        use contextforge_service::git;
        let mode = match mode.as_str() {
            "unstaged" => git::Mode::Unstaged,
            "staged" => git::Mode::Staged,
            "base" => git::Mode::Base {
                base,
                head: "HEAD".into(),
            },
            _ => return Err("invalid Git mode".into()),
        };
        Ok(serde_json::to_value(git::analyze(
            &registry.repository(id)?,
            &mode,
            3,
            100,
        )?)?)
    })
    .await
}

#[tauri::command]
async fn pack_context(
    id: i64,
    query: String,
    budget: usize,
    include_git: bool,
    state: tauri::State<'_, State>,
) -> Result<Value, String> {
    work(&state, move |registry| {
        Ok(serde_json::to_value(ContextForgeService::pack_context(
            &registry.repository(id)?,
            &query,
            budget,
            include_git,
        )?)?)
    })
    .await
}

#[tauri::command]
async fn integration_preview(
    id: i64,
    harness: integrations::Harness,
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
) -> Result<Value, String> {
    let plan = work(&state, move |registry| {
        // Explicit native executable selection avoids assuming the GUI's PATH.
        let path = app
            .dialog()
            .file()
            .set_title("Select the ContextForge CLI executable")
            .blocking_pick_file()
            .ok_or("executable selection cancelled")?
            .into_path()
            .map_err(|_| "invalid executable path")?;
        if !path.is_file() {
            return Err("executable is unavailable".into());
        }
        integrations::plan(registry.repository(id)?.root(), harness, Some(&path))
    })
    .await?;
    let operation_id = format!(
        "{id}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "Clock unavailable")?
            .as_nanos()
    );
    let mut value =
        serde_json::to_value(&plan.preview).map_err(|_| "Unable to serialize preview")?;
    value["operation_id"] = json!(operation_id);
    state
        .integration_plans
        .lock()
        .map_err(|_| "Preview state unavailable")?
        .insert(id, (operation_id, plan));
    Ok(value)
}

#[tauri::command]
async fn integration_remove_preview(
    id: i64,
    harness: integrations::Harness,
    state: tauri::State<'_, State>,
) -> Result<Value, String> {
    let plan = work(&state, move |registry| {
        integrations::plan(registry.repository(id)?.root(), harness, None)
    })
    .await?;
    let operation_id = format!(
        "{id}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "Clock unavailable")?
            .as_nanos()
    );
    let mut value =
        serde_json::to_value(&plan.preview).map_err(|_| "Unable to serialize preview")?;
    value["operation_id"] = json!(operation_id);
    state
        .integration_plans
        .lock()
        .map_err(|_| "Preview state unavailable")?
        .insert(id, (operation_id, plan));
    Ok(value)
}

#[tauri::command]
async fn integration_apply(
    id: i64,
    operation_id: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
) -> Result<Value, String> {
    let plan = {
        let mut plans = state
            .integration_plans
            .lock()
            .map_err(|_| "Preview state unavailable")?;
        if !plans
            .get(&id)
            .is_some_and(|(current, _)| *current == operation_id)
        {
            return Err("Configuration preview is stale; preview again before applying.".into());
        }
        plans.remove(&id).ok_or("Preview state unavailable")?.1
    };
    work(&state, move |_| {
        let confirmed = app.dialog().message(format!("Apply the reviewed ContextForge entry to {}? Entry: \n{}\nExisting configuration will be backed up. Other values are preserved; formatting/comments may change.", plan.preview.destination, plan.preview.configuration)).title("Confirm MCP configuration change").buttons(tauri_plugin_dialog::MessageDialogButtons::OkCancel).blocking_show();
        if !confirmed { return Ok(json!({"applied": false})); }
        let backup = plan.apply()?;
        Ok(json!({"applied": true, "backup": backup, "connectivity": "not_tested"}))
    }).await
}

#[tauri::command]
async fn integration_status(
    id: i64,
    harness: integrations::Harness,
    state: tauri::State<'_, State>,
) -> Result<Value, String> {
    work(&state, move |registry| {
        integrations::configuration_status(registry.repository(id)?.root(), harness)
    })
    .await
}

#[tauri::command]
async fn get_theme(state: tauri::State<'_, State>) -> Result<String, String> {
    work(&state, |registry| registry.theme()).await
}

#[tauri::command]
async fn set_theme(theme: String, state: tauri::State<'_, State>) -> Result<(), String> {
    work(&state, move |registry| registry.set_theme(&theme)).await
}

#[tauri::command]
async fn selected_project(state: tauri::State<'_, State>) -> Result<Option<i64>, String> {
    work(&state, |registry| registry.selected_project()).await
}

#[tauri::command]
async fn select_project(id: i64, state: tauri::State<'_, State>) -> Result<(), String> {
    work(&state, move |registry| registry.select_project(id)).await
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let registry = Registry::user().map_err(|_| "unable to open project registry")?;
            app.manage(State {
                registry,
                workers: Arc::new(tokio::sync::Semaphore::new(4)),
                indexing: Mutex::new(HashMap::new()),
                integration_plans: Mutex::new(HashMap::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            application_status,
            list_projects,
            add_project,
            remove_project,
            project_status,
            index_project,
            cancel_indexing,
            search_code,
            project_files,
            file_preview,
            dependencies,
            git_impact,
            pack_context,
            integration_preview,
            integration_remove_preview,
            integration_apply,
            integration_status,
            get_theme,
            set_theme,
            selected_project,
            select_project
        ])
        .build(tauri::generate_context!())
        .expect("unable to initialize ContextForge Desktop")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let state = app.state::<State>();
                if let Ok(tasks) = state.indexing.lock() {
                    for cancel in tasks.values() {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_traversal_and_invalid_offsets() {
        let repository =
            Repository::open(Some(Path::new(".")), Path::new(".")).expect("repository");
        assert!(preview(&repository, "../Cargo.toml", 0).is_err());
        assert!(preview(&repository, "src/main.rs", 1_000_001).is_err());
    }
}
