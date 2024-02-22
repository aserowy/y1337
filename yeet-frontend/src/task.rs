use std::path::{Path, PathBuf};

use tokio::{
    fs,
    sync::mpsc::Sender,
    task::{AbortHandle, JoinSet},
};
use yeet_keymap::message::{ContentKind, Message};

use crate::{
    error::AppError,
    model::{
        history::{self, History},
        register::{self, RegisterEntry},
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Task {
    AddPath(PathBuf),
    DeletePath(PathBuf),
    DeleteRegisterEntry(RegisterEntry),
    EmitMessages(Vec<Message>),
    EnumerateDirectory(PathBuf),
    LoadPreview(PathBuf),
    OptimizeHistory,
    RenamePath(PathBuf, PathBuf),
    RestorePath(RegisterEntry, PathBuf),
    SaveHistory(History),
    TrashPath(RegisterEntry),
    YankPath(RegisterEntry),
}

pub struct TaskManager {
    abort_handles: Vec<(Task, AbortHandle)>,
    sender: Sender<Vec<Message>>,
    tasks: JoinSet<Result<(), AppError>>,
}

impl TaskManager {
    pub fn new(sender: Sender<Vec<Message>>) -> Self {
        Self {
            abort_handles: Vec::new(),
            sender,
            tasks: JoinSet::new(),
        }
    }

    pub fn abort(&mut self, task: &Task) {
        if let Some(index) = self.abort_handles.iter().position(|(t, _)| t == task) {
            let (_, abort_handle) = self.abort_handles.remove(index);
            abort_handle.abort();
        }
    }

    pub async fn finishing(&mut self) -> Result<(), AppError> {
        let mut errors = Vec::new();
        for (task, abort_handle) in self.abort_handles.drain(..) {
            if should_abort_on_finish(task) {
                abort_handle.abort();
            }
        }

        while let Some(task) = self.tasks.join_next().await {
            match task {
                Ok(Ok(())) => (),
                // TODO: log error
                Ok(Err(error)) => errors.push(error),
                // TODO: log error
                Err(_) => (),
            };
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(AppError::Aggregate(errors))
        }
    }

    // TODO: if error occurs, enable handling in model with RenderAction + sender
    pub fn run(&mut self, task: Task) {
        let abort_handle = match task.clone() {
            Task::AddPath(path) => self.tasks.spawn(async move {
                if path.exists() {
                    return Err(AppError::InvalidTargetPath);
                }

                if let Some(path_str) = path.to_str() {
                    if path_str.ends_with('/') {
                        fs::create_dir_all(path).await?;
                    } else {
                        let parent = match Path::new(&path).parent() {
                            Some(path) => path,
                            None => return Err(AppError::InvalidTargetPath),
                        };

                        fs::create_dir_all(parent).await?;
                        fs::write(path, "").await?;
                    }
                }

                Ok(())
            }),
            Task::DeletePath(path) => self.tasks.spawn(async move {
                if !path.exists() {
                    return Err(AppError::InvalidTargetPath);
                }

                if path.is_file() {
                    fs::remove_file(&path).await?;
                } else if path.is_dir() {
                    fs::remove_dir_all(&path).await?;
                };

                Ok(())
            }),
            Task::DeleteRegisterEntry(entry) => self.tasks.spawn(async move {
                if let Err(_error) = register::delete(entry).await {
                    // TODO: log error
                    println!("Error: {:?}", _error);
                }
                Ok(())
            }),
            Task::EmitMessages(messages) => {
                let sender = self.sender.clone();
                self.tasks.spawn(async move {
                    if let Err(_error) = sender.send(messages).await {
                        // TODO: log error
                        println!("Error: {:?}", _error);
                    }
                    Ok(())
                })
            }
            Task::EnumerateDirectory(path) => {
                let internal_sender = self.sender.clone();
                self.tasks.spawn(async move {
                    if !path.exists() {
                        return Err(AppError::InvalidTargetPath);
                    }

                    let read_dir = tokio::fs::read_dir(path.clone()).await;
                    let mut cache = Vec::new();
                    match read_dir {
                        Ok(mut rd) => {
                            let mut cache_size = 100;

                            while let Some(entry) = rd.next_entry().await? {
                                let kind = if entry.path().is_dir() {
                                    ContentKind::Directory
                                } else {
                                    ContentKind::File
                                };

                                let content = match entry.path().file_name() {
                                    Some(content) => content.to_str().unwrap_or("").to_string(),
                                    None => "".to_string(),
                                };

                                cache.push((kind, content));

                                if cache.len() >= cache_size {
                                    let _ = internal_sender
                                        .send(vec![Message::PathEnumerationContentChanged(
                                            path.clone(),
                                            cache.clone(),
                                        )])
                                        .await;

                                    cache_size *= 2;
                                }
                            }

                            let _ = internal_sender
                                .send(vec![
                                    Message::PathEnumerationContentChanged(
                                        path.clone(),
                                        cache.clone(),
                                    ),
                                    Message::PathEnumerationFinished(path),
                                ])
                                .await;

                            Ok(())
                        }
                        Err(error) => Err(AppError::FileOperationFailed(error)),
                    }
                })
            }
            Task::LoadPreview(path) => {
                let internal_sender = self.sender.clone();
                self.tasks.spawn(async move {
                    if let Some(kind) = infer::get_from_path(path.clone())? {
                        // TODO: add preview for images here
                        // TODO: add preview for archives here
                        if !kind.mime_type().starts_with("text") {
                            return Ok(());
                        }
                    }

                    let content = fs::read_to_string(path.clone()).await?;
                    let _ = internal_sender
                        .send(vec![Message::PreviewLoaded(
                            path.clone(),
                            content.lines().map(|s| s.to_string()).collect(),
                        )])
                        .await;

                    Ok(())
                })
            }
            Task::OptimizeHistory => self.tasks.spawn(async move {
                history::cache::optimize()?;

                Ok(())
            }),
            Task::RenamePath(old, new) => self.tasks.spawn(async move {
                if !old.exists() {
                    return Err(AppError::InvalidTargetPath);
                }

                fs::rename(old, new).await?;

                Ok(())
            }),
            Task::RestorePath(entry, path) => self.tasks.spawn(async move {
                register::restore(entry, path)?;
                Ok(())
            }),
            Task::SaveHistory(history) => self.tasks.spawn(async move {
                if let Err(_error) = history::cache::save(&history) {
                    // TODO: log error
                    println!("Error: {:?}", _error);
                }
                history::cache::optimize()?;

                Ok(())
            }),
            Task::TrashPath(entry) => self.tasks.spawn(async move {
                if let Err(_error) = register::cache_and_compress(entry).await {
                    // TODO: log error
                    println!("Error: {:?}", _error);
                }

                Ok(())
            }),
            Task::YankPath(entry) => self.tasks.spawn(async move {
                if let Err(_error) = register::compress(entry).await {
                    // TODO: log error
                    println!("Error: {:?}", _error);
                }

                Ok(())
            }),
        };

        if let Some(index) = self.abort_handles.iter().position(|(t, _)| t == &task) {
            let (_, abort_handle) = self.abort_handles.remove(index);
            abort_handle.abort();
        }

        self.abort_handles.push((task, abort_handle));
    }
}

fn should_abort_on_finish(task: Task) -> bool {
    match task {
        Task::AddPath(_) => false,
        Task::DeletePath(_) => false,
        Task::DeleteRegisterEntry(_) => false,
        Task::EmitMessages(_) => true,
        Task::EnumerateDirectory(_) => true,
        Task::LoadPreview(_) => true,
        Task::OptimizeHistory => false,
        Task::RenamePath(_, _) => false,
        Task::RestorePath(_, _) => false,
        Task::SaveHistory(_) => false,
        Task::TrashPath(_) => false,
        Task::YankPath(_) => false,
    }
}
