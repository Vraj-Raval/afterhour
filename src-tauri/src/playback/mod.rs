mod core;
mod device;
#[cfg(test)]
mod tests;

use self::core::Transport;
pub use self::core::{Action, PlaybackSnapshot};
use crate::library::{self, LibraryState};
use device::DeviceBackend;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, SyncSender, TrySendError},
};
use std::time::Duration;
use tauri::State;

struct Request {
    action: Action,
    reply: mpsc::Sender<PlaybackSnapshot>,
}
struct Worker {
    sender: SyncSender<Request>,
    shutdown: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct PlaybackManager {
    worker: Mutex<Option<Worker>>,
}

impl PlaybackManager {
    fn request(
        &self,
        app: tauri::AppHandle,
        library: Arc<LibraryState>,
        action: Action,
    ) -> Result<PlaybackSnapshot, String> {
        let sender = {
            let mut worker = self
                .worker
                .lock()
                .map_err(|_| "Playback worker state is unavailable.")?;
            if worker.is_none() {
                let (sender, receiver) = mpsc::sync_channel::<Request>(32);
                let shutdown = Arc::new(AtomicBool::new(false));
                let closing = shutdown.clone();
                std::thread::Builder::new()
                    .name("afterhours-audio".into())
                    .spawn(move || {
                        let mut transport = Transport::new(DeviceBackend::default());
                        match library::load_playback_preferences(&app, &library) {
                            Ok(preferences) => transport.restore_preferences(preferences),
                            Err(error) => {
                                transport.state.preferences_error = Some(format!("Could not restore volume preferences: {error}"));
                                transport.state.revision += 1;
                            }
                        }
                        while !closing.load(Ordering::SeqCst) {
                            match receiver.recv_timeout(Duration::from_millis(100)) {
                                Ok(request) => {
                                    let preference_action = matches!(request.action, Action::SetVolume { .. } | Action::SetMuted { .. });
                                    let before = transport.preferences();
                                    let liked = transport.is_liked_collection();
                                    let playlist_id = transport.playlist_id().map(str::to_owned);
                                    let starting_playlist = match &request.action {
                                        Action::PlayPlaylist { playlist_id, .. } => Some(playlist_id.clone()),
                                        _ => None,
                                    };
                                    transport.apply_with_navigation(
                                        request.action,
                                        |id| match starting_playlist.as_deref() {
                                            Some(playlist) => library::playlist_start_track(&library, playlist, id),
                                            None => library::playback_track(&library, id),
                                        },
                                        |id, previous| {
                                            library::playback_neighbor(&library, id, previous, liked, playlist_id.as_deref())
                                        },
                                    );
                                    if preference_action && (before != transport.preferences() || transport.state.preferences_error.is_some()) {
                                        transport.state.preferences_error = library::save_playback_preferences(&library, transport.preferences())
                                            .err().map(|error| format!("Volume applied, but preferences could not be saved: {error}"));
                                        transport.state.revision += 1;
                                    }
                                    let _ = request.reply.send(transport.state.clone());
                                }
                                Err(mpsc::RecvTimeoutError::Timeout) => {
                                    let liked = transport.is_liked_collection();
                                    let playlist_id = transport.playlist_id().map(str::to_owned);
                                    transport.tick(
                                    |id| library::playback_track(&library, id),
                                    |id, previous| {
                                        library::playback_neighbor(&library, id, previous, liked, playlist_id.as_deref())
                                    },
                                );
                                }
                                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                            }
                        }
                        // DeviceBackend is dropped here, releasing the player and OS stream.
                    })
                    .map_err(|e| format!("Cannot start audio worker: {e}"))?;
                *worker = Some(Worker { sender, shutdown });
            }
            worker
                .as_ref()
                .ok_or("Playback worker could not start.")?
                .sender
                .clone()
        };
        let (reply, receiver) = mpsc::channel();
        sender
            .try_send(Request { action, reply })
            .map_err(|error| match error {
                TrySendError::Full(_) => "Playback is busy. Please try again.".to_string(),
                TrySendError::Disconnected(_) => {
                    "Playback worker stopped unexpectedly. Restart Afterhours.".to_string()
                }
            })?;
        receiver
            .recv()
            .map_err(|_| "Playback worker stopped before completing the request.".into())
    }

    pub fn shutdown(&self) {
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                worker.shutdown.store(true, Ordering::SeqCst);
            }
        }
    }
}

#[tauri::command]
pub async fn playback_control(
    app: tauri::AppHandle,
    action: Action,
    manager: State<'_, Arc<PlaybackManager>>,
    library: State<'_, Arc<LibraryState>>,
) -> Result<PlaybackSnapshot, String> {
    let manager = Arc::clone(manager.inner());
    let library = Arc::clone(library.inner());
    tauri::async_runtime::spawn_blocking(move || manager.request(app, library, action))
        .await
        .map_err(|e| format!("Playback request failed: {e}"))?
}
