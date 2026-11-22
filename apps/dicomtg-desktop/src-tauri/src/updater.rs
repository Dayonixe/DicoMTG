use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::Serialize;
use tauri::{AppHandle, State, ipc::Channel};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::commands::AppState;

#[derive(Default)]
pub(crate) struct UpdateGate(Arc<Mutex<Activity>>);

#[derive(Default)]
struct Activity {
    operations: usize,
    installing: bool,
}

pub(crate) struct ActivityGuard {
    activity: Arc<Mutex<Activity>>,
    installing: bool,
}

impl UpdateGate {
    pub(crate) fn start_activity(&self) -> Result<ActivityGuard, String> {
        self.enter(false)
    }

    fn start_installation(&self) -> Result<ActivityGuard, String> {
        self.enter(true)
    }

    fn enter(&self, installing: bool) -> Result<ActivityGuard, String> {
        let mut activity = self.0.lock().map_err(|error| error.to_string())?;
        if activity.installing {
            return Err("Une mise à jour de l’application est en cours.".into());
        }
        if installing {
            if activity.operations != 0 {
                return Err("Attendez la fin des opérations en cours, puis réessayez.".into());
            }
            activity.installing = true;
        } else {
            activity.operations += 1;
        }
        Ok(ActivityGuard {
            activity: Arc::clone(&self.0),
            installing,
        })
    }
}

impl Drop for ActivityGuard {
    fn drop(&mut self) {
        if let Ok(mut activity) = self.activity.lock() {
            if self.installing {
                activity.installing = false;
            } else {
                activity.operations -= 1;
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct PendingUpdate(Mutex<Option<Update>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateStatus {
    current_version: String,
    configured: bool,
    can_install: bool,
}

#[derive(Serialize)]
pub(crate) struct UpdateInfo {
    version: String,
    notes: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub(crate) enum UpdateProgress {
    Downloading { received: u64, total: Option<u64> },
    Installing,
}

#[tauri::command]
pub(crate) fn app_update_status(app: AppHandle) -> UpdateStatus {
    UpdateStatus {
        current_version: app.package_info().version.to_string(),
        configured: app
            .config()
            .plugins
            .0
            .get("updater")
            .and_then(|config| config.get("pubkey"))
            .and_then(|key| key.as_str())
            .is_some_and(|key| !key.trim().is_empty()),
        can_install: is_nsis_installation(),
    }
}

fn is_nsis_installation() -> bool {
    use tauri::utils::{config::BundleType, platform::bundle_type};

    cfg!(windows)
        && bundle_type() == Some(BundleType::Nsis)
        && std::env::current_exe()
            .ok()
            .and_then(|path| {
                path.parent()
                    .map(|directory| directory.join("uninstall.exe"))
            })
            .is_some_and(|uninstaller| uninstaller.is_file())
}

#[tauri::command]
pub(crate) async fn check_for_app_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Result<Option<UpdateInfo>, String> {
    if !app_update_status(app.clone()).configured {
        return Err("Cette version ne contient pas de clé publique de mise à jour.".into());
    }
    let update = app
        .updater_builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| format!("Impossible de vérifier les mises à jour : {error}"))?;
    let info = update.as_ref().map(|update| UpdateInfo {
        version: update.version.clone(),
        notes: update.body.clone(),
    });
    *pending.0.lock().map_err(|error| error.to_string())? = update;
    Ok(info)
}

#[tauri::command]
pub(crate) async fn install_app_update(
    state: State<'_, AppState>,
    pending: State<'_, PendingUpdate>,
    version: String,
    progress: Channel<UpdateProgress>,
) -> Result<(), String> {
    if !is_nsis_installation() {
        return Err(
            "L’installation intégrée est réservée à la version installée de DicoMTG.".into(),
        );
    }
    let installation = state.update_gate.start_installation()?;
    let Some(mut update) = pending
        .0
        .lock()
        .map_err(|error| error.to_string())?
        .as_ref()
        .filter(|update| update.version == version)
        .cloned()
    else {
        return Err("Recherchez à nouveau les mises à jour avant d’installer.".into());
    };
    let download_url = &update.download_url;
    if download_url.scheme() != "https"
        || download_url.host_str() != Some("github.com")
        || !download_url
            .path()
            .starts_with("/Dayonixe/DicoMTG/releases/download/")
    {
        return Err("Le manifeste indique une adresse de téléchargement non autorisée.".into());
    }

    update.timeout = Some(Duration::from_secs(180));
    let mut received = 0;
    let mut last_progress = None;
    let package = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                let percent = total
                    .filter(|size| *size > 0)
                    .map(|size| received * 100 / size);
                if last_progress != Some(percent) {
                    last_progress = Some(percent);
                    let _ = progress.send(UpdateProgress::Downloading { received, total });
                }
            },
            || {},
        )
        .await
        .map_err(|error| {
            format!("Téléchargement ou vérification de la signature impossible : {error}")
        })?;

    let _ = progress.send(UpdateProgress::Installing);
    tauri::async_runtime::spawn_blocking(move || {
        let _installation = installation;
        update
            .install(package)
            .map_err(|error| format!("Impossible de lancer l’installeur : {error}"))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updater_waits_for_all_operations_and_rejects_new_ones_during_installation() {
        let gate = UpdateGate::default();
        let import = gate.start_activity().unwrap();
        let refresh = gate.start_activity().unwrap();
        assert!(gate.start_installation().is_err());

        std::thread::spawn(move || drop(import)).join().unwrap();
        assert!(gate.start_installation().is_err());
        drop(refresh);

        let installation = gate.start_installation().unwrap();
        assert!(gate.start_activity().is_err());
        assert!(gate.start_installation().is_err());
        drop(installation);
        assert!(gate.start_activity().is_ok());
    }
}
