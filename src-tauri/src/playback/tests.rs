use super::core::{Action, Backend, BackendEvent, Status, Transport};
use crate::library::{MusicTrack, files::tests::Fixture};
use std::fs::File;

#[derive(Default)]
struct FakeBackend {
    gain: f32,
    position: f64,
    duration: Option<f64>,
    seek_fail: bool,
    seek_unsupported: bool,
    active: bool,
    paused: bool,
    fail: bool,
    starts: usize,
    stops: usize,
    pauses: usize,
    resumes: usize,
    event: Option<BackendEvent>,
}
impl Backend for FakeBackend {
    fn start(&mut self, _file: File) -> Result<(), String> {
        if self.fail {
            return Err("Decoder or output failure".into());
        }
        if self.active {
            self.stops += 1;
        }
        self.position = 0.0;
        self.starts += 1;
        self.active = true;
        self.paused = false;
        Ok(())
    }
    fn supports_seek(&self) -> bool {
        self.duration.is_some() && !self.seek_unsupported
    }
    fn timing(&self) -> (f64, Option<f64>) {
        (self.position, self.duration)
    }
    fn seek(&mut self, seconds: f64) -> Result<(), String> {
        if self.seek_fail {
            return Err("Unsupported seek".into());
        }
        self.position = seconds;
        Ok(())
    }
    fn set_volume(&mut self, volume: f32) {
        self.gain = volume;
    }
    fn pause(&mut self) {
        self.pauses += 1;
        self.paused = true;
    }
    fn resume(&mut self) {
        self.resumes += 1;
        self.paused = false;
    }
    fn stop(&mut self) {
        if self.active {
            self.stops += 1;
        }
        self.active = false;
    }
    fn poll(&mut self) -> Option<BackendEvent> {
        self.event.take()
    }
}

fn resolve(fixture: &Fixture, id: &str) -> Result<(MusicTrack, File), String> {
    if id == "missing" {
        return Err("Track is missing or not saved".into());
    }
    let path = fixture.file(&format!("{id}.wav"), b"test data");
    Ok((
        MusicTrack {
            id: id.into(),
            path: path.to_string_lossy().into_owned(),
            title: id.into(),
            artist: "Artist".into(),
            album: "Album".into(),
            album_artist: None,
            duration_seconds: Some(1.0),
            track_number: None,
            disc_number: None,
            year: None,
            format: "WAV".into(),
            artwork: None,
            available: true,
            favorite: false,
        },
        File::open(path).unwrap(),
    ))
}

#[test]
fn startup_and_status_do_not_open_or_start_audio() {
    let mut transport = Transport::new(FakeBackend::default());
    let state = transport.apply(Action::Status, |_| panic!("Status must not read a track"));
    assert_eq!(state.status, Status::Stopped);
    assert!(state.track.is_none());
    assert_eq!(transport.backend.starts, 0);
}

#[test]
fn play_pause_resume_and_stop_follow_backend_state() {
    let f = Fixture::new();
    let mut transport = Transport::new(FakeBackend::default());
    assert_eq!(
        transport
            .apply(
                Action::Play {
                    track_id: "first".into()
                },
                |id| resolve(&f, id)
            )
            .status,
        Status::Playing
    );
    assert_eq!(
        transport.apply(Action::Pause, |_| panic!()).status,
        Status::Paused
    );
    assert!(transport.backend.paused);
    transport.apply(Action::Pause, |_| panic!());
    assert_eq!(transport.backend.pauses, 1);
    assert_eq!(
        transport.apply(Action::Resume, |_| panic!()).status,
        Status::Playing
    );
    transport.apply(Action::Resume, |_| panic!());
    assert_eq!(transport.backend.resumes, 1);
    assert_eq!(
        transport.apply(Action::Stop, |_| panic!()).status,
        Status::Stopped
    );
    assert!(!transport.backend.active);
    assert_eq!(transport.backend.stops, 1);
    transport.apply(Action::Stop, |_| panic!());
    assert_eq!(transport.backend.stops, 1);
}

#[test]
fn switching_tracks_replaces_the_previous_source() {
    let f = Fixture::new();
    let mut transport = Transport::new(FakeBackend::default());
    transport.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    let state = transport.apply(
        Action::Play {
            track_id: "second".into(),
        },
        |id| resolve(&f, id),
    );
    assert_eq!(state.track.unwrap().id, "second");
    assert_eq!(transport.backend.starts, 2);
    assert_eq!(transport.backend.stops, 1);
}

#[test]
fn failed_start_never_claims_playing_and_preserves_previous_audio() {
    let f = Fixture::new();
    let mut transport = Transport::new(FakeBackend::default());
    let state = transport.apply(
        Action::Play {
            track_id: "missing".into(),
        },
        |id| resolve(&f, id),
    );
    assert_eq!(state.status, Status::Stopped);
    assert!(state.error.is_some());
    assert!(state.track.is_none());
    transport.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    transport.backend.fail = true;
    let state = transport.apply(
        Action::Play {
            track_id: "second".into(),
        },
        |id| resolve(&f, id),
    );
    assert_eq!(state.status, Status::Playing);
    assert_eq!(state.track.unwrap().id, "first");
    assert!(state.error.is_some());
    assert!(transport.backend.active);
}

#[test]
fn end_of_file_and_output_failure_stop_without_auto_advancing() {
    let f = Fixture::new();
    let mut transport = Transport::new(FakeBackend::default());
    transport.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    transport.backend.event = Some(BackendEvent::Finished);
    transport.poll();
    assert_eq!(transport.state.status, Status::Stopped);
    assert_eq!(transport.backend.starts, 1);
    assert!(transport.state.error.is_none());
    transport.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    transport.backend.event = Some(BackendEvent::Failed("Device disconnected".into()));
    transport.poll();
    assert_eq!(transport.state.status, Status::Stopped);
    assert_eq!(
        transport.state.error.as_deref(),
        Some("Device disconnected")
    );
}

#[test]
fn resume_without_a_paused_source_reports_an_error() {
    let mut transport = Transport::new(FakeBackend::default());
    let state = transport.apply(Action::Resume, |_| panic!());
    assert_eq!(state.status, Status::Stopped);
    assert!(state.error.is_some());
    assert_eq!(transport.backend.resumes, 0);
}

#[test]
fn command_payload_uses_track_ids_and_rejects_unknown_actions() {
    let action: Action = serde_json::from_str(r#"{"type":"play","trackId":"saved-id"}"#).unwrap();
    assert!(matches!(action, Action::Play {track_id} if track_id == "saved-id"));
    let action: Action = serde_json::from_str(r#"{"type":"playPlaylist","trackId":"first","trackIds":["third","first"],"playlistId":"custom"}"#).unwrap();
    assert!(
        matches!(action, Action::PlayPlaylist {track_id,playlist_id,track_ids} if track_id=="first" && playlist_id=="custom" && track_ids==vec!["third","first"])
    );
    let action: Action = serde_json::from_str(
        r#"{"type":"playCollection","trackId":"first","trackIds":["first","third"],"liked":true}"#,
    )
    .unwrap();
    assert!(
        matches!(action, Action::PlayCollection { track_id, track_ids, liked: true } if track_id == "first" && track_ids == vec!["first", "third"])
    );
    assert!(
        serde_json::from_str::<Action>(r#"{"type":"seek","path":"C:/arbitrary.wav"}"#).is_err()
    );
}

#[test]
fn native_timing_seek_clamping_paused_seek_and_switch_reset() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend {
        duration: Some(60.0),
        ..Default::default()
    });
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    t.backend.position = 12.0;
    assert_eq!(t.apply(Action::Status, |_| panic!()).position_seconds, 12.0);
    t.apply(Action::Pause, |_| panic!());
    let revision = t.state.revision;
    assert_eq!(t.apply(Action::Status, |_| panic!()).revision, revision);
    let id = t.state.source_id;
    let state = t.apply(
        Action::Seek {
            source_id: id,
            seconds: 100.0,
        },
        |_| panic!(),
    );
    assert_eq!(state.position_seconds, 60.0);
    assert_eq!(state.status, Status::Paused);
    assert!(t.backend.paused);
    assert_eq!(
        t.apply(
            Action::Seek {
                source_id: id,
                seconds: -10.0
            },
            |_| panic!()
        )
        .position_seconds,
        0.0
    );
    t.apply(Action::Resume, |_| panic!());
    t.backend.position = 3.0;
    assert_eq!(t.apply(Action::Status, |_| panic!()).position_seconds, 3.0);
    let state = t.apply(
        Action::Play {
            track_id: "second".into(),
        },
        |id| resolve(&f, id),
    );
    assert_eq!(state.position_seconds, 0.0);
    assert!(
        t.apply(
            Action::Seek {
                source_id: id,
                seconds: 10.0
            },
            |_| panic!()
        )
        .error
        .is_some()
    );
    t.backend.event = Some(BackendEvent::Finished);
    t.poll();
    assert_eq!(t.state.position_seconds, 60.0);
    assert!(!t.state.can_seek);
    assert_eq!(t.apply(Action::Stop, |_| panic!()).position_seconds, 0.0);
}

#[test]
fn unknown_duration_and_failed_seek_do_not_fabricate_progress() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    let state = t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    assert!(state.duration_seconds.is_none());
    assert!(!state.can_seek);
    assert!(
        t.apply(
            Action::Seek {
                source_id: state.source_id,
                seconds: 10.0
            },
            |_| panic!()
        )
        .error
        .is_some()
    );
    t.backend.duration = Some(60.0);
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    t.backend.seek_fail = true;
    let state = t.apply(
        Action::Seek {
            source_id: t.state.source_id,
            seconds: 20.0,
        },
        |_| panic!(),
    );
    assert_eq!(state.position_seconds, 0.0);
    assert_eq!(state.status, Status::Playing);
    assert!(state.error.is_some());
    assert!(!state.can_seek);
}

#[test]
fn a_known_duration_does_not_imply_decoder_seek_support() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend {
        duration: Some(1.0),
        seek_unsupported: true,
        ..Default::default()
    });
    let state = t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    assert_eq!(state.duration_seconds, Some(1.0));
    assert!(!state.can_seek);
    assert!(
        t.apply(
            Action::Seek {
                source_id: state.source_id,
                seconds: 0.5
            },
            |_| panic!()
        )
        .error
        .is_some()
    );
}

#[test]
fn duration_becoming_known_enables_seeking_and_nonfinite_seeks_are_rejected() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    t.backend.duration = Some(0.5);
    let state = t.apply(Action::Status, |_| panic!());
    assert_eq!(state.duration_seconds, Some(0.5));
    assert!(state.can_seek);
    assert!(
        t.apply(
            Action::Seek {
                source_id: state.source_id,
                seconds: f64::INFINITY
            },
            |_| panic!()
        )
        .error
        .is_some()
    );
    assert_eq!(t.state.position_seconds, 0.0);
}

fn ordered(id: &str, previous: bool) -> Result<Option<String>, String> {
    let ids = ["first", "second", "third"];
    let index = ids
        .iter()
        .position(|entry| *entry == id)
        .ok_or("Not in library")?;
    Ok(if previous {
        index.checked_sub(1).map(|i| ids[i].into())
    } else {
        ids.get(index + 1).map(|id| (*id).into())
    })
}

#[test]
fn next_previous_restart_threshold_and_first_boundary() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend {
        duration: Some(60.0),
        ..Default::default()
    });
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    let source = t.state.source_id;
    let next = t.apply_with_navigation(
        Action::Next { source_id: source },
        |id| resolve(&f, id),
        ordered,
    );
    assert_eq!(next.track.unwrap().id, "second");
    assert_eq!(next.status, Status::Playing);
    assert_eq!(next.position_seconds, 0.0);
    assert_eq!(t.backend.starts, 2);
    // A second click captured from the old track cannot advance the new source.
    let stale = t.apply_with_navigation(
        Action::Next { source_id: source },
        |id| resolve(&f, id),
        ordered,
    );
    assert!(stale.error.is_some());
    assert_eq!(t.backend.starts, 2);
    t.backend.position = 3.01;
    let restarted = t.apply_with_navigation(
        Action::Previous {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        ordered,
    );
    assert_eq!(restarted.track.unwrap().id, "second");
    assert_eq!(restarted.position_seconds, 0.0);
    // At exactly three seconds Previous selects the preceding track.
    t.backend.position = 3.0;
    let previous = t.apply_with_navigation(
        Action::Previous {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        ordered,
    );
    assert_eq!(previous.track.unwrap().id, "first");
    t.backend.position = 0.2;
    let first = t.apply_with_navigation(
        Action::Previous {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        ordered,
    );
    assert_eq!(first.track.unwrap().id, "first");
    assert_eq!(first.position_seconds, 0.0);
}

#[test]
fn last_next_and_single_track_boundaries_do_not_wrap() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::Play {
            track_id: "third".into(),
        },
        |id| resolve(&f, id),
    );
    let last = t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        ordered,
    );
    assert_eq!(last.status, Status::Stopped);
    assert_eq!(last.track.unwrap().id, "third");
    assert_eq!(last.position_seconds, 0.0);
    assert!(!t.backend.active);
    t.apply(
        Action::Play {
            track_id: "only".into(),
        },
        |id| resolve(&f, id),
    );
    let first = t.apply_with_navigation(
        Action::Previous {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        |_, _| Ok(None),
    );
    assert_eq!(first.track.unwrap().id, "only");
    assert_eq!(first.status, Status::Playing);
    let end = t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        |_, _| Ok(None),
    );
    assert_eq!(end.status, Status::Stopped);
    assert!(end.error.is_none());
}

#[test]
fn completion_advances_once_and_end_of_library_retains_completed_time() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend {
        duration: Some(60.0),
        ..Default::default()
    });
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    let first_source = t.state.source_id;
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), ordered);
    assert_eq!(t.state.track.as_ref().unwrap().id, "second");
    assert!(t.state.source_id > first_source);
    assert_eq!(t.state.status, Status::Playing);
    assert_eq!(t.state.position_seconds, 0.0);
    for _ in 0..5 {
        t.tick(|_| panic!("No repeated starts"), ordered);
    }
    assert_eq!(t.backend.starts, 2);
    t.backend.event = Some(BackendEvent::Finished);
    // Frontend status requests and native timeout ticks use the same completion path.
    let state = t.apply_with_navigation(Action::Status, |id| resolve(&f, id), ordered);
    assert_eq!(state.track.unwrap().id, "third");
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|_| panic!("Must not wrap"), ordered);
    assert_eq!(t.state.status, Status::Stopped);
    assert_eq!(t.state.position_seconds, 60.0);
    assert_eq!(t.backend.starts, 3);
    assert!(t.state.error.is_none());
}

#[test]
fn pause_stop_replacement_and_output_error_do_not_auto_advance() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    t.apply(Action::Pause, |_| panic!());
    t.tick(|_| panic!(), |_, _| panic!());
    assert_eq!(t.state.status, Status::Paused);
    // A pending old completion is consumed before explicit track replacement.
    t.backend.event = Some(BackendEvent::Finished);
    t.apply_with_navigation(
        Action::Play {
            track_id: "third".into(),
        },
        |id| resolve(&f, id),
        |_, _| panic!("Old completion must not advance"),
    );
    t.tick(|_| panic!(), |_, _| panic!());
    assert_eq!(t.state.track.as_ref().unwrap().id, "third");
    assert_eq!(t.backend.starts, 2);
    t.apply(Action::Stop, |_| panic!());
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|_| panic!(), |_, _| panic!());
    assert_eq!(t.backend.starts, 2);
    t.apply(
        Action::Play {
            track_id: "second".into(),
        },
        |id| resolve(&f, id),
    );
    t.backend.event = Some(BackendEvent::Failed("Output lost".into()));
    t.tick(|_| panic!(), |_, _| panic!());
    assert_eq!(t.state.status, Status::Stopped);
    assert_eq!(t.state.error.as_deref(), Some("Output lost"));
}

#[test]
fn broken_next_file_preserves_manual_playback_but_auto_failure_stops_without_retrying() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    let failed = t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |_| Err("Missing next file".into()),
        ordered,
    );
    assert_eq!(failed.status, Status::Playing);
    assert_eq!(failed.track.unwrap().id, "first");
    assert!(failed.error.is_some());
    t.backend.fail = true;
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), ordered);
    assert_eq!(t.state.status, Status::Stopped);
    assert!(t.state.error.as_ref().unwrap().contains("Decoder"));
    for _ in 0..5 {
        t.tick(
            |_| panic!("Never retry failed auto-advance"),
            |_, _| panic!(),
        );
    }
    assert_eq!(t.backend.starts, 1);
}

#[test]
fn single_track_completion_and_library_lookup_errors_stop_once() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::Play {
            track_id: "only".into(),
        },
        |id| resolve(&f, id),
    );
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|_| panic!(), |_, _| Ok(None));
    assert_eq!(t.state.status, Status::Stopped);
    assert!(t.state.error.is_none());
    assert_eq!(t.backend.starts, 1);
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|_| panic!(), |_, _| Err("Library busy".into()));
    assert_eq!(t.state.status, Status::Stopped);
    assert_eq!(t.state.error.as_deref(), Some("Library busy"));
    t.tick(|_| panic!(), |_, _| panic!());
}

#[test]
fn previous_restarts_a_paused_seekable_source_without_resuming_or_reopening() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend {
        duration: Some(60.0),
        ..Default::default()
    });
    t.apply(
        Action::Play {
            track_id: "second".into(),
        },
        |id| resolve(&f, id),
    );
    t.backend.position = 10.0;
    t.apply(Action::Pause, |_| panic!());
    let state = t.apply_with_navigation(
        Action::Previous {
            source_id: t.state.source_id,
        },
        |_| panic!("Restart uses existing source"),
        ordered,
    );
    assert_eq!(state.status, Status::Paused);
    assert_eq!(state.position_seconds, 0.0);
    assert_eq!(state.track.unwrap().id, "second");
    assert_eq!(t.backend.starts, 1);
    assert!(t.backend.paused);
}

#[test]
fn volume_is_clamped_finite_and_safe_at_zero_without_opening_audio() {
    let mut t = Transport::new(FakeBackend::default());
    assert_eq!(t.backend.gain, 1.0);
    let high = t.apply(Action::SetVolume { volume: 5.0 }, |_| panic!());
    assert_eq!(high.volume, 1.0);
    assert_eq!(t.backend.gain, 1.0);
    let low = t.apply(Action::SetVolume { volume: -2.0 }, |_| panic!());
    assert_eq!(low.volume, 0.0);
    assert_eq!(t.backend.gain, 0.0);
    assert!(low.error.is_none());
    for volume in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let state = t.apply(Action::SetVolume { volume }, |_| panic!());
        assert!(state.error.is_some());
        assert_eq!(state.volume, 0.0);
        assert_eq!(t.backend.gain, 0.0);
    }
    assert_eq!(t.backend.starts, 0);
    assert_eq!(t.state.status, Status::Stopped);
}

#[test]
fn gain_and_mute_changes_preserve_position_source_and_paused_state() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend {
        duration: Some(60.0),
        ..Default::default()
    });
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    t.backend.position = 15.0;
    let source = t.state.source_id;
    t.apply(Action::SetVolume { volume: 0.4 }, |_| panic!());
    assert!((t.backend.gain - 0.4).abs() < f32::EPSILON);
    assert_eq!(t.state.status, Status::Playing);
    t.apply(Action::Pause, |_| panic!());
    t.apply(Action::SetMuted { muted: true }, |_| panic!());
    assert_eq!(t.backend.gain, 0.0);
    assert_eq!(t.state.volume, 0.4);
    t.apply(Action::SetMuted { muted: false }, |_| panic!());
    assert!((t.backend.gain - 0.4).abs() < f32::EPSILON);
    t.apply(Action::SetMuted { muted: true }, |_| panic!());
    t.apply(Action::SetVolume { volume: 0.7 }, |_| panic!());
    assert!(!t.state.muted);
    assert!((t.backend.gain - 0.7).abs() < f32::EPSILON);
    assert_eq!(t.state.status, Status::Paused);
    assert!(t.backend.paused);
    assert_eq!(t.state.position_seconds, 15.0);
    assert_eq!(t.state.source_id, source);
    assert_eq!(t.backend.starts, 1);
    t.apply(Action::SetVolume { volume: 0.0 }, |_| panic!());
    t.apply(Action::SetMuted { muted: true }, |_| panic!());
    t.apply(Action::SetMuted { muted: false }, |_| panic!());
    assert_eq!(t.backend.gain, 0.0);
    assert!(t.state.error.is_none());
}

#[test]
fn preferences_restore_without_playing_and_survive_switches_and_completion() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.restore_preferences(crate::library::PlaybackPreferences {
        volume: 0.35,
        muted: true,
    });
    assert_eq!(t.backend.starts, 0);
    assert_eq!(t.backend.gain, 0.0);
    assert_eq!(t.state.status, Status::Stopped);
    for id in ["first", "second"] {
        t.apply(
            Action::Play {
                track_id: id.into(),
            },
            |id| resolve(&f, id),
        );
        assert_eq!(t.backend.gain, 0.0);
        assert!(t.state.muted);
        assert_eq!(t.state.volume, 0.35);
    }
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), ordered);
    assert_eq!(t.state.track.as_ref().unwrap().id, "third");
    assert_eq!(t.backend.gain, 0.0);
    t.apply(Action::SetMuted { muted: false }, |_| panic!());
    assert!((t.backend.gain - 0.35).abs() < f32::EPSILON);
}

#[test]
fn volume_does_not_hide_playback_errors_and_successful_play_recovers() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::Play {
            track_id: "missing".into(),
        },
        |id| resolve(&f, id),
    );
    let error = t.state.error.clone();
    t.apply(Action::SetVolume { volume: 0.2 }, |_| panic!());
    assert_eq!(t.state.error, error);
    assert_eq!(t.state.status, Status::Stopped);
    let recovered = t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    assert_eq!(recovered.status, Status::Playing);
    assert!(recovered.error.is_none());
    assert!((t.backend.gain - 0.2).abs() < f32::EPSILON);
}

fn context_neighbor(id: &str, previous: bool, liked: bool) -> Result<Option<String>, String> {
    let ids = if liked {
        vec!["first", "third"]
    } else {
        vec!["first", "second", "third"]
    };
    let index = ids.iter().position(|candidate| *candidate == id).unwrap();
    Ok(if previous {
        index.checked_sub(1).map(|i| ids[i].into())
    } else {
        ids.get(index + 1).map(|id| (*id).into())
    })
}

#[test]
fn collection_context_survives_navigation_completion_boundaries_and_replay() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::PlayCollection {
            track_id: "first".into(),
            track_ids: vec!["first".into(), "third".into()],
            liked: true,
        },
        |id| resolve(&f, id),
    );
    assert!(t.is_liked_collection());
    let liked = t.is_liked_collection();
    let state = t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        |id, previous| context_neighbor(id, previous, liked),
    );
    assert_eq!(state.track.unwrap().id, "third");
    let stale = t.apply_with_navigation(
        Action::Previous { source_id: 1 },
        |id| resolve(&f, id),
        |id, previous| context_neighbor(id, previous, liked),
    );
    assert!(stale.error.is_some());
    assert_eq!(stale.track.unwrap().id, "third");
    t.apply_with_navigation(
        Action::Previous {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        |id, previous| context_neighbor(id, previous, liked),
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "first");
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(
        |id| resolve(&f, id),
        |id, previous| context_neighbor(id, previous, liked),
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "third");
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(
        |id| resolve(&f, id),
        |id, previous| context_neighbor(id, previous, liked),
    );
    assert_eq!(t.state.status, Status::Stopped);
    assert_eq!(t.state.track.as_ref().unwrap().id, "third");
    t.apply(
        Action::Play {
            track_id: "third".into(),
        },
        |id| resolve(&f, id),
    );
    assert!(t.is_liked_collection());
    t.apply(
        Action::PlayCollection {
            track_id: "missing".into(),
            track_ids: vec!["missing".into()],
            liked: false,
        },
        |id| resolve(&f, id),
    );
    assert!(
        t.is_liked_collection(),
        "Failed starts must preserve the playing context"
    );
    t.apply(
        Action::PlayCollection {
            track_id: "first".into(),
            track_ids: vec!["first".into(), "second".into(), "third".into()],
            liked: false,
        },
        |id| resolve(&f, id),
    );
    assert!(!t.is_liked_collection());
    let liked = t.is_liked_collection();
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(
        |id| resolve(&f, id),
        |id, previous| context_neighbor(id, previous, liked),
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "second");
}

#[test]
fn playlist_context_navigates_in_custom_order_completes_and_preserves_other_contexts() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    let playlist = vec!["third".to_string(), "first".to_string()];
    t.apply(
        Action::PlayPlaylist {
            track_id: "third".into(),
            playlist_id: "custom".into(),
            track_ids: playlist.clone(),
        },
        |id| resolve(&f, id),
    );
    assert_eq!(t.playlist_id(), Some("custom"));
    let neighbors = |id: &str, previous: bool| -> Result<Option<String>, String> {
        let index = playlist
            .iter()
            .position(|candidate| candidate == id)
            .unwrap();
        Ok(if previous {
            index.checked_sub(1).map(|i| playlist[i].clone())
        } else {
            playlist.get(index + 1).cloned()
        })
    };
    t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        neighbors,
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "first");
    t.apply_with_navigation(
        Action::Previous {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        neighbors,
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "third");
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), neighbors);
    assert_eq!(t.state.track.as_ref().unwrap().id, "first");
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), neighbors);
    assert_eq!(t.state.status, Status::Stopped);
    assert_eq!(t.playlist_id(), Some("custom"));
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    assert_eq!(t.playlist_id(), Some("custom"));
    t.apply(
        Action::PlayPlaylist {
            track_id: "missing".into(),
            playlist_id: "other".into(),
            track_ids: vec!["missing".into()],
        },
        |id| resolve(&f, id),
    );
    assert_eq!(t.playlist_id(), Some("custom"));
    t.apply(
        Action::PlayCollection {
            track_id: "first".into(),
            track_ids: vec!["first".into()],
            liked: true,
        },
        |id| resolve(&f, id),
    );
    assert!(t.is_liked_collection());
    assert_eq!(t.playlist_id(), None);
    t.apply(
        Action::PlayCollection {
            track_id: "first".into(),
            track_ids: vec!["first".into()],
            liked: false,
        },
        |id| resolve(&f, id),
    );
    assert!(!t.is_liked_collection());
    assert_eq!(t.playlist_id(), None);
}

fn enqueue(t: &mut Transport<FakeBackend>, f: &Fixture, id: &str, first: bool) {
    let state = t.apply(
        Action::Enqueue {
            track_id: id.into(),
            play_next: first,
        },
        |id| resolve(f, id),
    );
    assert!(state.error.is_none());
}
#[test]
fn queue_edits_are_session_only_ordered_and_revision_guarded_without_starting_audio() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    enqueue(&mut t, &f, "first", false);
    enqueue(&mut t, &f, "second", false);
    enqueue(&mut t, &f, "third", true);
    assert_eq!(t.backend.starts, 0);
    assert_eq!(
        t.state
            .queue
            .iter()
            .map(|e| e.track.id.as_str())
            .collect::<Vec<_>>(),
        vec!["third", "first", "second"]
    );
    let entry = t.state.queue[2].id;
    let revision = t.state.queue_revision;
    t.apply(
        Action::QueueMove {
            entry_id: entry,
            direction: -1,
            queue_revision: revision,
        },
        |_| panic!(),
    );
    assert_eq!(t.state.queue[1].track.id, "second");
    let stale = t.apply(
        Action::QueueRemove {
            entry_id: entry,
            queue_revision: revision,
        },
        |_| panic!(),
    );
    assert!(stale.error.is_some());
    assert_eq!(t.state.queue.len(), 3);
    t.apply(
        Action::QueueRemove {
            entry_id: entry,
            queue_revision: t.state.queue_revision,
        },
        |_| panic!(),
    );
    assert_eq!(t.state.queue.len(), 2);
    enqueue(&mut t, &f, "first", false);
    assert_ne!(
        t.state.queue[1].id, t.state.queue[2].id,
        "Repeated songs have distinct queue identities"
    );
    t.apply(
        Action::QueueClear {
            queue_revision: t.state.queue_revision,
        },
        |_| panic!(),
    );
    assert!(t.state.queue.is_empty());
    assert_eq!(t.backend.starts, 0);
}
#[test]
fn queued_tracks_override_then_resume_library_from_original_anchor() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    enqueue(&mut t, &f, "extra", false);
    enqueue(&mut t, &f, "other", false);
    let source = t.state.source_id;
    t.apply_with_navigation(
        Action::Next { source_id: source },
        |id| resolve(&f, id),
        ordered,
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "extra");
    let stale = t.apply_with_navigation(
        Action::Next { source_id: source },
        |id| resolve(&f, id),
        ordered,
    );
    assert!(stale.error.is_some());
    assert_eq!(t.state.queue.len(), 1);
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), ordered);
    assert_eq!(t.state.track.as_ref().unwrap().id, "other");
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), ordered);
    assert_eq!(t.state.track.as_ref().unwrap().id, "second");
}
#[test]
fn queue_preserves_liked_and_playlist_context_and_collection_boundaries() {
    for playlist in [false, true] {
        let f = Fixture::new();
        let mut t = Transport::new(FakeBackend::default());
        let ids = vec!["third".into(), "first".into()];
        let action = if playlist {
            Action::PlayPlaylist {
                track_id: "third".into(),
                playlist_id: "custom".into(),
                track_ids: ids,
            }
        } else {
            Action::PlayCollection {
                track_id: "third".into(),
                track_ids: ids,
                liked: true,
            }
        };
        t.apply(action, |id| resolve(&f, id));
        enqueue(&mut t, &f, "extra", false);
        let neighbor = |id: &str, previous: bool| -> Result<Option<String>, String> {
            assert!(!previous);
            assert_eq!(id, "third");
            Ok(Some("first".into()))
        };
        t.apply_with_navigation(
            Action::Next {
                source_id: t.state.source_id,
            },
            |id| resolve(&f, id),
            neighbor,
        );
        assert_eq!(
            t.playlist_id(),
            if playlist { Some("custom") } else { None }
        );
        assert_eq!(t.is_liked_collection(), !playlist);
        t.backend.event = Some(BackendEvent::Finished);
        t.tick(|id| resolve(&f, id), neighbor);
        assert_eq!(t.state.track.as_ref().unwrap().id, "first");
        t.backend.event = Some(BackendEvent::Finished);
        t.tick(|id| resolve(&f, id), |_, _| Ok(None));
        assert_eq!(t.state.status, Status::Stopped);
    }
}
#[test]
fn previous_retraces_queue_and_clear_keeps_current_playing_then_resumes_collection() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    enqueue(&mut t, &f, "extra", false);
    t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        ordered,
    );
    t.apply_with_navigation(
        Action::Previous {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        ordered,
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "first");
    assert_eq!(t.state.queue[0].track.id, "extra");
    t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        ordered,
    );
    enqueue(&mut t, &f, "other", false);
    let starts = t.backend.starts;
    t.apply_with_navigation(
        Action::QueueClear {
            queue_revision: t.state.queue_revision,
        },
        |id| resolve(&f, id),
        ordered,
    );
    assert_eq!(t.state.status, Status::Playing);
    assert_eq!(t.backend.starts, starts);
    assert_eq!(t.state.track.as_ref().unwrap().id, "extra");
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), ordered);
    assert_eq!(t.state.track.as_ref().unwrap().id, "second");
}
#[test]
fn idle_queue_starts_only_explicitly_and_stops_when_exhausted() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    enqueue(&mut t, &f, "extra", true);
    assert_eq!(t.state.status, Status::Stopped);
    t.apply_with_navigation(
        Action::Next { source_id: 0 },
        |id| resolve(&f, id),
        |_, _| panic!("No underlying context"),
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "extra");
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), |_, _| panic!("No underlying context"));
    assert_eq!(t.state.status, Status::Stopped);
}
#[test]
fn queue_edits_do_not_discard_completion_and_failed_entries_are_retained() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    t.backend.event = Some(BackendEvent::Finished);
    t.apply_with_navigation(
        Action::Enqueue {
            track_id: "extra".into(),
            play_next: false,
        },
        |id| resolve(&f, id),
        ordered,
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "extra");
    enqueue(&mut t, &f, "later-missing", false);
    let state = t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |_| Err("File disconnected".into()),
        ordered,
    );
    assert!(state.error.is_some());
    assert_eq!(state.status, Status::Playing);
    assert_eq!(state.queue.len(), 1);
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|_| Err("File disconnected".into()), ordered);
    assert_eq!(t.state.status, Status::Stopped);
    assert_eq!(t.state.queue.len(), 1);
}

#[test]
fn search_album_artist_selections_use_snapshot_order_previous_completion_and_queue_continuation() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::PlaySelection {
            track_id: "third".into(),
            track_ids: vec!["third".into(), "first".into()],
        },
        |id| resolve(&f, id),
    );
    t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        |_, _| panic!("Must not consult global library"),
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "first");
    t.apply_with_navigation(
        Action::Previous {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        |_, _| panic!(),
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "third");
    enqueue(&mut t, &f, "extra", false);
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), |_, _| panic!());
    assert_eq!(t.state.track.as_ref().unwrap().id, "extra");
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), |_, _| panic!());
    assert_eq!(t.state.track.as_ref().unwrap().id, "first");
    t.backend.event = Some(BackendEvent::Finished);
    t.tick(|id| resolve(&f, id), |_, _| panic!());
    assert_eq!(t.state.status, Status::Stopped);
    t.apply(
        Action::Play {
            track_id: "first".into(),
        },
        |id| resolve(&f, id),
    );
    t.apply_with_navigation(
        Action::Previous {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        |_, _| panic!(),
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "third");
    t.apply(
        Action::PlayCollection {
            track_id: "first".into(),
            track_ids: vec!["first".into(), "second".into()],
            liked: false,
        },
        |id| resolve(&f, id),
    );
    t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        ordered,
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "second");
}
#[test]
fn failed_selection_does_not_replace_existing_context_or_consume_upcoming_queue() {
    let f = Fixture::new();
    let mut t = Transport::new(FakeBackend::default());
    t.apply(
        Action::PlayPlaylist {
            track_id: "first".into(),
            playlist_id: "custom".into(),
            track_ids: vec!["first".into()],
        },
        |id| resolve(&f, id),
    );
    enqueue(&mut t, &f, "extra", false);
    let state = t.apply(
        Action::PlaySelection {
            track_id: "missing".into(),
            track_ids: vec!["missing".into()],
        },
        |id| resolve(&f, id),
    );
    assert!(state.error.is_some());
    assert_eq!(t.playlist_id(), Some("custom"));
    assert_eq!(state.queue.len(), 1);
    let action: Action = serde_json::from_str(
        r#"{"type":"playSelection","trackId":"third","trackIds":["third","first"]}"#,
    )
    .unwrap();
    t.apply(action, |id| resolve(&f, id));
    assert_eq!(t.playlist_id(), None);
    assert!(!t.is_liked_collection());
    assert_eq!(t.state.queue.len(), 1);
    t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        |_, _| panic!(),
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "extra");
    t.apply_with_navigation(
        Action::Next {
            source_id: t.state.source_id,
        },
        |id| resolve(&f, id),
        |_, _| panic!(),
    );
    assert_eq!(t.state.track.as_ref().unwrap().id, "first");
}
