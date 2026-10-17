use super::core::{Backend, BackendEvent};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::{
    fs::File,
    io::{Seek, SeekFrom},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

/// Lives on the playback worker. The output stream survives pause, stop, and track changes.
#[derive(Default)]
pub struct DeviceBackend {
    player: Option<Player>,
    gain: Option<f32>,
    duration: Option<Duration>,
    seekable: bool,
    output: Option<MixerDeviceSink>,
    errors: Option<Receiver<String>>,
}

impl Backend for DeviceBackend {
    fn start(&mut self, file: File) -> Result<(), String> {
        // File clones share a cursor: drop the probe and rewind before decoding the active source.
        let mut file = file;
        let mut probe = Decoder::try_from(file.try_clone().map_err(|e| e.to_string())?)
            .map_err(|e| format!("Cannot decode this audio file: {e}"))?;
        if probe.next().is_none() {
            return Err("This file contains no decodable audio samples.".into());
        }
        let seekable = probe.try_seek(Duration::ZERO).is_ok();
        drop(probe);
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        let source =
            Decoder::try_from(file).map_err(|e| format!("Cannot decode this audio file: {e}"))?;
        let duration = source.total_duration();
        if self.output.is_none() {
            let (sender, receiver) = mpsc::sync_channel(1);
            let output = DeviceSinkBuilder::from_default_device()
                .map_err(|e| format!("Cannot open the Windows default audio output: {e}"))?
                .with_error_callback(move |error| { let _ = sender.try_send(format!("Audio output failed: {error}. Check your Windows audio device, then play the track again.")); })
                .open_sink_or_fallback().map_err(|e| format!("Cannot start the Windows default audio output: {e}"))?;
            self.output = Some(output);
            self.errors = Some(receiver);
        }
        let output = self.output.as_ref().ok_or("Audio output is unavailable.")?;
        let player = Player::connect_new(output.mixer());
        player.pause();
        player.set_volume(self.gain.unwrap_or(1.0));
        self.stop(); // Drop the old source; never append a second song to a queue.
        self.duration = duration;
        self.seekable = seekable;
        player.append(source);
        player.play();
        self.player = Some(player);
        Ok(())
    }
    fn supports_seek(&self) -> bool {
        self.seekable
    }
    fn timing(&self) -> (f64, Option<f64>) {
        (
            self.player
                .as_ref()
                .map_or(0.0, |p| p.get_pos().as_secs_f64()),
            self.duration.map(|d| d.as_secs_f64()),
        )
    }
    fn seek(&mut self, seconds: f64) -> Result<(), String> {
        self.player
            .as_ref()
            .ok_or("No active audio source.")?
            .try_seek(Duration::from_secs_f64(seconds))
            .map_err(|e| format!("Cannot seek this track: {e}"))
    }
    fn set_volume(&mut self, volume: f32) {
        self.gain = Some(volume);
        if let Some(player) = &self.player {
            player.set_volume(volume);
        }
    }
    fn pause(&mut self) {
        if let Some(player) = &self.player {
            player.pause();
        }
    }
    fn resume(&mut self) {
        if let Some(player) = &self.player {
            player.play();
        }
    }
    fn stop(&mut self) {
        if let Some(player) = self.player.take() {
            player.stop();
        }
    }
    fn poll(&mut self) -> Option<BackendEvent> {
        if let Some(error) = self
            .errors
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok())
        {
            self.stop();
            self.output = None;
            self.errors = None;
            return Some(BackendEvent::Failed(error));
        }
        self.player
            .as_ref()
            .filter(|player| player.empty())
            .map(|_| BackendEvent::Finished)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::files::tests::Fixture;

    #[test]
    fn native_gain_changes_real_samples_without_a_device_or_source_restart() {
        let (player, mut output) = Player::new();
        player.append(rodio::source::SineWave::new(440.0));
        let mut backend = DeviceBackend {
            player: Some(player),
            ..Default::default()
        };
        backend.set_volume(0.0);
        assert!(output.by_ref().take(1000).all(|sample| sample == 0.0));
        backend.set_volume(0.25);
        // Rodio applies gain at its next control callback (~5ms of samples).
        output.by_ref().take(1000).for_each(drop);
        let samples: Vec<_> = output.by_ref().take(1000).collect();
        assert!(samples.iter().any(|sample| sample.abs() > 0.01));
        assert!(samples.iter().all(|sample| sample.abs() <= 0.251));
        let player = backend.player.as_ref().unwrap();
        assert!(!player.empty());
        assert!(!player.is_paused());
        let position = player.get_pos();
        backend.pause();
        backend.set_volume(0.75);
        assert!(backend.player.as_ref().unwrap().is_paused());
        assert_eq!(backend.player.as_ref().unwrap().get_pos(), position);
        assert_eq!(backend.player.as_ref().unwrap().volume(), 0.75);
    }

    #[test]
    fn malformed_audio_fails_before_opening_an_output_device() {
        let f = Fixture::new();
        let path = f.file("invalid.flac", b"not audio");
        let mut backend = DeviceBackend::default();
        assert!(backend.start(File::open(path).unwrap()).is_err());
        assert!(backend.output.is_none());
        assert!(backend.player.is_none());
    }

    #[test]
    fn decodes_real_wav_samples_without_speakers() {
        let f = Fixture::new();
        let mut bytes = b"RIFF".to_vec();
        bytes.extend(52u32.to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(8000u32.to_le_bytes());
        bytes.extend(16000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend([0; 16]);
        let path = f.file("fixture.wav", &bytes);
        let decoder = Decoder::try_from(File::open(path).unwrap()).unwrap();
        assert_eq!(decoder.count(), 8);
    }

    #[test]
    fn real_decoder_seeks_while_paused_without_an_output_device() {
        let fixture = Fixture::new();
        let data_len = 16000u32;
        let mut bytes = b"RIFF".to_vec();
        bytes.extend((36 + data_len).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(8000u32.to_le_bytes());
        bytes.extend(16000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(data_len.to_le_bytes());
        bytes.resize(bytes.len() + data_len as usize, 0);
        let path = fixture.file("seek.wav", &bytes);
        let decoder = Decoder::try_from(File::open(path).unwrap()).unwrap();
        let duration = decoder.total_duration();
        let (player, mut output) = Player::new();
        player.pause();
        player.append(decoder);
        // Consume Rodio's mixer output in memory, including its control processing.
        let closing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = closing.clone();
        let consumer = std::thread::spawn(move || {
            while !flag.load(std::sync::atomic::Ordering::SeqCst) {
                for _ in 0..48 {
                    output.next();
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        });
        let mut backend = DeviceBackend {
            player: Some(player),
            duration,
            seekable: true,
            ..Default::default()
        };
        let seek_result = backend.seek(0.5);
        std::thread::sleep(Duration::from_millis(20));
        let paused_position = backend.timing().0;
        std::thread::sleep(Duration::from_millis(20));
        let frozen_position = backend.timing().0;
        let still_paused = backend.player.as_ref().unwrap().is_paused();
        backend.resume();
        std::thread::sleep(Duration::from_millis(20));
        let resumed_position = backend.timing().0;
        closing.store(true, std::sync::atomic::Ordering::SeqCst);
        consumer.join().unwrap();
        assert!(seek_result.is_ok());
        assert_eq!(duration, Some(Duration::from_secs(1)));
        assert!((paused_position - 0.5).abs() < 0.01);
        assert_eq!(paused_position, frozen_position);
        assert!(still_paused);
        assert!(resumed_position > frozen_position);
    }
}
