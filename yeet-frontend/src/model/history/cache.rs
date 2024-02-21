use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

use crate::error::AppError;

use super::{History, HistoryNode, HistoryState};

pub fn load(history: &mut History) -> Result<(), AppError> {
    let history_path = get_history_path()?;
    if !Path::new(&history_path).exists() {
        return Err(AppError::LoadHistoryFailed);
    }

    let history_file = File::open(history_path)?;
    let mut history_csv_reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(history_file);

    for result in history_csv_reader.records() {
        let record = match result {
            Ok(record) => record,
            Err(_) => return Err(AppError::LoadHistoryFailed),
        };

        let changed_at = match record.get(0) {
            Some(val) => {
                if let Ok(changed_at) = val.parse::<u64>() {
                    changed_at
                } else {
                    continue;
                }
            }
            None => continue,
        };

        let path = match record.get(1) {
            Some(path) => path,
            None => continue,
        };

        let mut iter = Path::new(path).components();
        if let Some(component) = iter.next() {
            if let Some(component_name) = component.as_os_str().to_str() {
                super::add_entry(
                    &mut history.entries,
                    changed_at,
                    HistoryState::Loaded,
                    component_name,
                    iter,
                );
            }
        }
    }

    Ok(())
}

pub fn optimize() -> Result<(), AppError> {
    let mut history = History::default();
    load(&mut history)?;
    save_filtered(&history, HistoryState::Loaded, true)?;

    Ok(())
}

pub fn save(history: &History) -> Result<(), AppError> {
    save_filtered(history, HistoryState::Added, false)
}

fn get_history_path() -> Result<String, AppError> {
    let cache_dir = match dirs::cache_dir() {
        Some(cache_dir) => match cache_dir.to_str() {
            Some(cache_dir_string) => cache_dir_string.to_string(),
            None => return Err(AppError::LoadHistoryFailed),
        },
        None => return Err(AppError::LoadHistoryFailed),
    };

    Ok(format!("{}{}", cache_dir, "/yeet/history"))
}

fn save_filtered(
    history: &History,
    state_filter: HistoryState,
    overwrite: bool,
) -> Result<(), AppError> {
    let entries = get_paths(PathBuf::new(), &history.entries);

    let history_path = get_history_path()?;
    let history_dictionary = match Path::new(&history_path).parent() {
        Some(path) => path,
        None => return Err(AppError::LoadHistoryFailed),
    };

    fs::create_dir_all(history_dictionary)?;

    let history_writer = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(overwrite)
        .append(!overwrite)
        .open(history_path)?;

    let mut history_csv_writer = csv::Writer::from_writer(history_writer);
    for (changed_at, state, path) in entries {
        if state != state_filter {
            continue;
        }

        if !path.exists() {
            continue;
        }

        if let Some(path) = path.to_str() {
            if history_csv_writer
                .write_record([changed_at.to_string().as_str(), path])
                .is_err()
            {
                // TODO: log error
            }
        }
    }

    history_csv_writer.flush()?;

    Ok(())
}

fn get_paths(
    current_path: PathBuf,
    nodes: &HashMap<String, HistoryNode>,
) -> Vec<(u64, HistoryState, PathBuf)> {
    let mut result = Vec::new();
    for node in nodes.values() {
        let mut path = current_path.clone();
        path.push(&node.component);

        if node.nodes.is_empty() {
            result.push((node.changed_at, node.state.clone(), path));
        } else {
            result.append(&mut get_paths(path, &node.nodes));
        }
    }

    result
}
