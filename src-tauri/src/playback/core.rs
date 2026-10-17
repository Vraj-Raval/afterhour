use crate::library::{MusicTrack, PlaybackPreferences};
use serde::{Deserialize, Serialize};
use std::fs::File;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSnapshot {
    pub revision: u64,
    pub volume: f64,
    pub muted: bool,
    pub preferences_error: Option<String>,
    pub source_id: u64,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub can_seek: bool,
    pub status: Status,
    pub track: Option<MusicTrack>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Action {
    Play {
        track_id: String,
    },
    PlayCollection {
        track_id: String,
        track_ids: Vec<String>,
        liked: bool,
    },
    Seek {
        source_id: u64,
        seconds: f64,
    },
    Next {
        source_id: u64,
    },
    Previous {
        source_id: u64,
    },
    SetVolume {
        volume: f64,
    },
    SetMuted {
        muted: bool,
    },
    Pause,
    Resume,
    Stop,
    Status,
}

pub enum BackendEvent {
    Finished,
    Failed(String),
}

pub trait Backend {
    // A failed start must leave the previous source unchanged.
    fn start(&mut self, file: File) -> Result<(), String>;
    fn supports_seek(&self) -> bool {
        false
    }
    fn timing(&self) -> (f64, Option<f64>) {
        (0.0, None)
    }
    fn seek(&mut self, _seconds: f64) -> Result<(), String> {
        Err("Seeking is unsupported for this source.".into())
    }
    fn set_volume(&mut self, volume: f32);
    fn pause(&mut self);
    fn resume(&mut self);
    fn stop(&mut self);
    fn poll(&mut self) -> Option<BackendEvent>;
}

pub struct Transport<B: Backend> {
    pub backend: B,
    liked_collection: bool,
    pub state: PlaybackSnapshot,
}

impl<B: Backend> Transport<B> {
    pub fn new(mut backend: B) -> Self {
        backend.set_volume(1.0);
        Self {
            backend,
            liked_collection: false,
            state: PlaybackSnapshot {
                revision: 0,
                volume: 1.0,
                muted: false,
                preferences_error: None,
                source_id: 0,
                position_seconds: 0.0,
                duration_seconds: None,
                can_seek: false,
                status: Status::Stopped,
                track: None,
                error: None,
            },
        }
    }

    pub fn is_liked_collection(&self) -> bool {
        self.liked_collection
    }

    pub fn preferences(&self) -> PlaybackPreferences {
        PlaybackPreferences {
            volume: self.state.volume,
            muted: self.state.muted,
        }
    }

    pub fn restore_preferences(&mut self, preferences: PlaybackPreferences) {
        self.state.volume = if preferences.volume.is_finite() {
            preferences.volume.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.state.muted = preferences.muted;
        self.apply_gain();
        self.state.revision += 1;
    }

    fn apply_gain(&mut self) {
        self.backend.set_volume(if self.state.muted {
            0.0
        } else {
            self.state.volume as f32
        });
    }

    pub fn poll(&mut self) -> bool {
        if let Some(event) = self.backend.poll() {
            // Only the currently playing source can complete naturally.
            let completed =
                matches!(event, BackendEvent::Finished) && self.state.status == Status::Playing;
            if matches!(event, BackendEvent::Finished) {
                self.state.position_seconds = self
                    .state
                    .duration_seconds
                    .unwrap_or(self.state.position_seconds);
            }
            self.state.can_seek = false;
            self.backend.stop();
            self.state.status = Status::Stopped;
            self.state.error = match event {
                BackendEvent::Finished => None,
                BackendEvent::Failed(error) => Some(error),
            };
            self.state.revision += 1;
            return completed;
        }
        false
    }

    fn update_timing(&mut self) {
        if self.state.status == Status::Stopped {
            return;
        }
        let (position, duration) = self.backend.timing();
        let duration = duration.filter(|d| d.is_finite() && *d > 0.0);
        let position = if position.is_finite() {
            position.max(0.0).min(duration.unwrap_or(f64::MAX))
        } else {
            0.0
        };
        if self.state.duration_seconds.is_none() && duration.is_some() {
            self.state.can_seek = self.backend.supports_seek();
        }
        if self.state.position_seconds != position || self.state.duration_seconds != duration {
            self.state.position_seconds = position;
            self.state.duration_seconds = duration;
            self.state.revision += 1;
        }
    }

    fn start_track(&mut self, track: MusicTrack, file: File) -> Result<(), String> {
        self.backend.start(file)?;
        self.state.source_id += 1;
        self.state.position_seconds = 0.0;
        self.state.duration_seconds = self.backend.timing().1;
        self.state.can_seek = self
            .state
            .duration_seconds
            .is_some_and(|d| d.is_finite() && d > 0.0)
            && self.backend.supports_seek();
        self.state.track = Some(track);
        self.state.status = Status::Playing;
        Ok(())
    }

    fn navigate(
        &mut self,
        previous: bool,
        resolve: &mut impl FnMut(&str) -> Result<(MusicTrack, File), String>,
        neighbor: &mut impl FnMut(&str, bool) -> Result<Option<String>, String>,
    ) -> Result<(), String> {
        let current = self
            .state
            .track
            .as_ref()
            .ok_or("Select a library track first.")?
            .id
            .clone();
        let target = if previous && self.state.position_seconds > 3.0 {
            Some(current.clone())
        } else {
            neighbor(&current, previous)?.or_else(|| previous.then(|| current.clone()))
        };
        if let Some(target) = target {
            if previous
                && target == current
                && self.state.status != Status::Stopped
                && self.state.can_seek
            {
                if let Err(error) = self.backend.seek(0.0) {
                    self.state.can_seek = false;
                    return Err(error);
                }
                // Invalidate repeated clicks captured before this restart, without replacing audio.
                self.state.source_id += 1;
                self.state.position_seconds = 0.0;
                return Ok(());
            }
            let restarting_paused =
                previous && target == current && self.state.status == Status::Paused;
            let (track, file) = resolve(&target)?;
            self.start_track(track, file)?;
            if restarting_paused {
                self.backend.pause();
                self.state.status = Status::Paused;
            }
        } else {
            self.backend.stop();
            self.state.status = Status::Stopped;
            self.state.position_seconds = 0.0;
            self.state.can_seek = false;
        }
        Ok(())
    }

    pub fn tick(
        &mut self,
        mut resolve: impl FnMut(&str) -> Result<(MusicTrack, File), String>,
        mut neighbor: impl FnMut(&str, bool) -> Result<Option<String>, String>,
    ) {
        if self.poll() {
            // One attempt only. A failed next file must not trigger an advance loop.
            let completed_position = self.state.position_seconds;
            self.state.error = self.navigate(false, &mut resolve, &mut neighbor).err();
            if self.state.status == Status::Stopped {
                self.state.position_seconds = completed_position;
            }
            self.state.revision += 1;
        }
    }

    #[cfg(test)]
    pub fn apply(
        &mut self,
        action: Action,
        resolve: impl FnMut(&str) -> Result<(MusicTrack, File), String>,
    ) -> PlaybackSnapshot {
        self.apply_with_navigation(action, resolve, |_, _| Ok(None))
    }

    pub fn apply_with_navigation(
        &mut self,
        action: Action,
        mut resolve: impl FnMut(&str) -> Result<(MusicTrack, File), String>,
        mut neighbor: impl FnMut(&str, bool) -> Result<Option<String>, String>,
    ) -> PlaybackSnapshot {
        let preference_action =
            matches!(action, Action::SetVolume { .. } | Action::SetMuted { .. });
        if matches!(action, Action::Status) || preference_action {
            self.tick(&mut resolve, &mut neighbor);
        } else {
            self.poll();
        }
        self.update_timing();
        if matches!(action, Action::Status) {
            return self.state.clone();
        }
        let seeking = matches!(action, Action::Seek { .. } | Action::Previous { .. });
        let result = match action {
            Action::Play { track_id } => {
                let replaying = self
                    .state
                    .track
                    .as_ref()
                    .is_some_and(|track| track.id == track_id);
                resolve(&track_id)
                    .and_then(|(track, file)| self.start_track(track, file))
                    .map(|()| {
                        if !replaying {
                            self.liked_collection = false;
                        }
                    })
            }
            Action::PlayCollection {
                track_id,
                track_ids,
                liked,
            } => {
                // The view sends the complete collection, before search/pagination.
                // SQLite refreshes its membership/order for each subsequent navigation.
                if !track_ids.contains(&track_id) {
                    Err(
                        "The selected track is no longer in this collection. Select it again."
                            .into(),
                    )
                } else {
                    resolve(&track_id)
                        .and_then(|(track, file)| self.start_track(track, file))
                        .map(|()| {
                            self.liked_collection = liked;
                        })
                }
            }
            Action::Next { source_id } | Action::Previous { source_id } => {
                if source_id != self.state.source_id {
                    Err("The active track changed. Try track navigation again.".into())
                } else {
                    self.navigate(
                        matches!(action, Action::Previous { .. }),
                        &mut resolve,
                        &mut neighbor,
                    )
                }
            }
            Action::Seek { source_id, seconds } => {
                if source_id != self.state.source_id || self.state.status == Status::Stopped {
                    Err("The active track changed. Try seeking again.".into())
                } else if !seconds.is_finite() || !self.state.can_seek {
                    Err("Seeking is unavailable for this track.".into())
                } else if let Some(duration) = self.state.duration_seconds {
                    let result = self.backend.seek(seconds.clamp(0.0, duration));
                    if result.is_err() {
                        self.state.can_seek = false;
                    }
                    result
                } else {
                    Err("Track duration is unknown.".into())
                }
            }
            Action::SetVolume { volume } => {
                if !volume.is_finite() {
                    Err("Volume must be a finite number.".into())
                } else {
                    self.state.volume = volume.clamp(0.0, 1.0);
                    self.state.muted = false;
                    self.apply_gain();
                    Ok(())
                }
            }
            Action::SetMuted { muted } => {
                self.state.muted = muted;
                self.apply_gain();
                Ok(())
            }
            Action::Pause => {
                if self.state.status == Status::Playing {
                    self.backend.pause();
                    self.state.status = Status::Paused;
                }
                Ok(())
            }
            Action::Resume => {
                if self.state.status == Status::Paused {
                    self.backend.resume();
                    self.state.status = Status::Playing;
                    Ok(())
                } else if self.state.status == Status::Playing {
                    Ok(())
                } else {
                    Err("No paused track to resume. Select a library track to play.".into())
                }
            }
            Action::Stop => {
                self.backend.stop();
                self.state.status = Status::Stopped;
                self.state.position_seconds = 0.0;
                self.state.can_seek = false;
                Ok(())
            }
            Action::Status => unreachable!(),
        };
        // Rodio can transiently report the requested offset after a failed seek.
        // Preserve the last measurement until its audio callback refreshes position.
        if !seeking || result.is_ok() {
            self.update_timing();
        }
        if !preference_action || result.is_err() {
            self.state.error = result.err();
        }
        self.state.revision += 1;
        self.state.clone()
    }
}
