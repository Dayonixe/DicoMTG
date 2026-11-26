mod commands;
mod updater;

use std::{fs, io, path::Path};

use dicomtg_core::CollectionStore;
use tauri::Manager;

pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let data_directory = app.path().app_local_data_dir()?;
            migrate_legacy_data_directory(&app.path().local_data_dir()?, &data_directory)?;
            fs::create_dir_all(&data_directory)?;
            let database_path = data_directory.join("dicomtg.sqlite");
            let store = CollectionStore::open(&database_path)?;
            let collection_writer = store.open_peer()?;
            app.manage(commands::AppState::new(
                store,
                collection_writer,
                data_directory.join("updates"),
                data_directory.join("card-images"),
            ));
            app.manage(updater::PendingUpdate::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_dashboard,
            commands::load_type_analysis,
            commands::import_collection,
            commands::list_collections,
            commands::create_collection,
            commands::update_collection,
            commands::rename_collection,
            commands::delete_collection,
            commands::set_collection_selected,
            commands::download_collection_csv,
            commands::update_reference_data,
            commands::load_card_image_bytes,
            commands::load_card_localization,
            commands::load_card_french_text,
            commands::load_card_edhrec_stats,
            commands::load_image_cache_stats,
            commands::clear_image_cache,
            updater::app_update_status,
            updater::check_for_app_update,
            updater::install_app_update
        ])
        .run(tauri::generate_context!())
}

fn migrate_legacy_data_directory(
    local_data_directory: &Path,
    current: &Path,
) -> Result<(), io::Error> {
    if current.join("dicomtg.sqlite").exists() {
        return Ok(());
    }

    let mut candidates = Vec::new();
    for entry in fs::read_dir(local_data_directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if entry.file_type()?.is_dir()
            && entry.path() != current
            && name.starts_with("fr.")
            && name.ends_with(".dicomtg")
            && entry.path().join("dicomtg.sqlite").is_file()
        {
            candidates.push(entry.path());
        }
    }

    let legacy = if let Some(preferred) = candidates.iter().find(|path| {
        path.file_name().is_some_and(|name| {
            name.to_string_lossy()
                .eq_ignore_ascii_case("fr.prawse.dicomtg")
        })
    }) {
        preferred
    } else {
        match candidates.as_slice() {
            [] => return Ok(()),
            [legacy] => legacy,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "multiple legacy DicoMTG data directories found",
                ));
            }
        }
    };
    fs::create_dir_all(current)?;
    for name in ["dicomtg.sqlite", "card-images", "updates"] {
        let source = legacy.join(name);
        let target = current.join(name);
        if source.exists() && !target.exists() {
            fs::rename(source, target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::migrate_legacy_data_directory;

    #[test]
    fn app_002_moves_only_owned_legacy_data() {
        let root = temporary_directory(line!());
        let legacy = root.join("fr.legacy.dicomtg");
        let current = root.join("DicoMTG");
        fs::create_dir_all(legacy.join("card-images")).unwrap();
        fs::create_dir_all(legacy.join("updates")).unwrap();
        fs::create_dir_all(legacy.join("EBWebView")).unwrap();
        fs::write(legacy.join("dicomtg.sqlite"), b"collection").unwrap();
        fs::write(legacy.join("card-images/card.jpg"), b"image").unwrap();
        fs::write(legacy.join("updates/catalog.json"), b"catalog").unwrap();
        fs::write(legacy.join("EBWebView/lock"), b"webview").unwrap();

        migrate_legacy_data_directory(&root, &current).unwrap();

        assert_eq!(
            fs::read(current.join("dicomtg.sqlite")).unwrap(),
            b"collection"
        );
        assert_eq!(
            fs::read(current.join("card-images/card.jpg")).unwrap(),
            b"image"
        );
        assert_eq!(
            fs::read(current.join("updates/catalog.json")).unwrap(),
            b"catalog"
        );
        assert_eq!(fs::read(legacy.join("EBWebView/lock")).unwrap(), b"webview");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn app_002_preserves_an_existing_new_database() {
        let root = temporary_directory(line!());
        let legacy = root.join("fr.legacy.dicomtg");
        let current = root.join("DicoMTG");
        fs::create_dir_all(&legacy).unwrap();
        fs::create_dir_all(&current).unwrap();
        fs::write(legacy.join("dicomtg.sqlite"), b"legacy").unwrap();
        fs::write(current.join("dicomtg.sqlite"), b"current").unwrap();

        migrate_legacy_data_directory(&root, &current).unwrap();

        assert_eq!(fs::read(legacy.join("dicomtg.sqlite")).unwrap(), b"legacy");
        assert_eq!(
            fs::read(current.join("dicomtg.sqlite")).unwrap(),
            b"current"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn app_002_prefers_the_prawse_data_directory() {
        let root = temporary_directory(line!());
        let preferred = root.join("fr.prawse.dicomtg");
        let older = root.join("fr.theopirouelle.dicomtg");
        let current = root.join("DicoMTG");
        fs::create_dir_all(&preferred).unwrap();
        fs::create_dir_all(&older).unwrap();
        fs::write(preferred.join("dicomtg.sqlite"), b"preferred").unwrap();
        fs::write(older.join("dicomtg.sqlite"), b"older").unwrap();

        migrate_legacy_data_directory(&root, &current).unwrap();

        assert_eq!(
            fs::read(current.join("dicomtg.sqlite")).unwrap(),
            b"preferred"
        );
        assert_eq!(fs::read(older.join("dicomtg.sqlite")).unwrap(), b"older");
        fs::remove_dir_all(root).unwrap();
    }

    fn temporary_directory(line: u32) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../target/test-data")
            .join(format!(
                "dicomtg-brand-migration-{}-{line}",
                std::process::id()
            ))
    }
}
