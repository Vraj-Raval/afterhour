# Native playback: Task 10A

Rodio 0.23.0 owns the decoder and Windows output through its current `Player` /
`DeviceSinkBuilder` APIs. Defaults are disabled; enabled features are `playback`,
`flac`, `mp3`, `mp4`, `vorbis`, `wav`, and `symphonia-alac`. No microphone/recording
feature is enabled.

Configured formats: FLAC, MP3, PCM WAV, AAC (ADTS), M4A/MP4 with AAC or ALAC, and
Ogg Vorbis. Container-specific variants still depend on decoder support. Opus
and WMA playback are not supported; scanner format support is broader than
playback support. No files are modified or converted.

## Ownership and commands

The application manages one `PlaybackManager`. Its lazily created background
worker owns the Rodio output and processes bounded, serialized requests. Status
requests start only the control worker, not the audio device. The default output
opens on the first successful play request and survives pause/resume, stop, and
track changes. A device error releases it so a later play request can retry.
Application exit signals worker shutdown and releases the source/output.

`playback_control` accepts only play-by-saved-track-ID, pause, resume, stop, and
status actions. The main local window receives its generated command permission.
There are no new filesystem permissions, arbitrary path arguments, database
schema changes, or SQL commands exposed to React.

The library resolves a track ID with a parameterized SQLite query. It checks
availability, canonical location, saved-root containment, and file access before
opening it. A busy scan transaction returns a retryable error instead of blocking
the transport worker behind the database lock.

## State and behavior

Native snapshots contain current metadata, stopped/playing/paused status, error,
and a monotonically increasing revision. React updates only after native replies
and rejects stale responses. Status polling every 750 ms detects source completion
and output-device errors; no playback position is requested or displayed. A
communication failure disables transport controls and shows unknown availability
rather than asserting that audio continues playing.

A row click starts/restarts that saved track. The player button pauses or resumes
the existing source. When stopped or naturally finished, Play reopens the current
saved track from the beginning. Stop drops the source while retaining metadata.
There is no auto-play on startup and no next-track or queue behavior.

The decoder is opened and its first buffered sample checked before replacing a
currently playing source. Startup/decode failures therefore preserve the old
track and its real status, alongside a useful error. Output errors stop playback.
Rodio's iterator-based decoder does not expose every mid-file decode failure as
a separate error; some malformed streams can terminate early as an end-of-source.

## Tests and manual verification

Transport tests use a fake backend and temporary files, with no output devices.
Decoder tests read a generated WAV and malformed audio without sending samples
to speakers. Frontend tests mock native IPC and verify acknowledgment, play/pause
commands, duplicate suppression, errors, and out-of-order responses.

Restart the native app after command/dependency changes. Manual Windows testing
is still required for actual FLAC/MP3 output, pause/resume continuity, switching
tracks, stop/exit cleanup, end-of-file status, missing files, unsupported formats,
and disconnected/default-output-device behavior. Automated tests never access a
personal music library or require speakers.
