use parking_lot::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use xcloud_media::{AudioOutput, FrameMailbox, MailboxReader};
use xcloud_proto::{streaming_config::StreamingConfig, streaming_types::StreamingStatsSnapshot};
use xcloud_stream::{decode_thread::DecodeThread, pipeline::NativeMediaPipeline};
use xcloud_webrtc::{ConfigurableMediaSink, MediaSink, Transport};

#[derive(Clone, Default)]
pub struct MediaHealth {
    pub assembled: u32,
    pub incomplete_drops: u32,
    pub decoded: u32,
    pub queue_drops: u32,
    pub decoder_errors: u32,
    pub gated: u32,
    pub waiting_keyframe: bool,
}
pub struct Pipeline {
    inner: NativeMediaPipeline,
    health: Arc<Mutex<MediaHealth>>,
    last_report: u64,
}
impl MediaSink for Pipeline {
    fn on_video_rtp(&mut self, packet: &[u8], now: u64) {
        self.inner.push_video_rtp(packet, now);
    }
    fn on_audio_rtp(&mut self, packet: &[u8], now: u64) {
        self.inner.push_audio_rtp(packet, now);
    }
    fn tick(&mut self, now: u64) {
        self.inner.tick(now);
        if now.saturating_sub(self.last_report) >= 1000 {
            self.last_report = now;
            let decode = self.inner.decode_stats();
            *self.health.lock() = MediaHealth {
                assembled: self.inner.video_stats().frames,
                incomplete_drops: self.inner.video_stats().dropped,
                decoded: decode.frames_decoded,
                queue_drops: decode.dropped_full,
                decoder_errors: decode.decode_errors,
                gated: decode.gated,
                waiting_keyframe: self.inner.waiting_keyframe(),
            };
        }
    }
    fn reset(&mut self) {
        self.inner.reset();
        *self.health.lock() = MediaHealth::default();
        self.last_report = 0;
    }
    fn stats(&self) -> StreamingStatsSnapshot {
        self.inner.stats()
    }
}
impl ConfigurableMediaSink for Pipeline {
    fn configure(&mut self, c: &StreamingConfig) {
        self.inner.configure(c);
    }
}

pub struct Media {
    pub reader: Arc<Mutex<MailboxReader>>,
    pub audio: Arc<Mutex<AudioOutput>>,
    pub muted: Arc<AtomicBool>,
    pub decoded: Arc<AtomicU64>,
    pub health: Arc<Mutex<MediaHealth>>,
}
impl Media {
    pub fn attach(
        transport: &xcloud_webrtc::native::WebRtcTransport,
        ctx: crate::Notify,
        start_muted: bool,
    ) -> (
        Self,
        crossbeam_channel::Receiver<xcloud_stream::pipeline::PipelineEvent>,
    ) {
        let (writer, reader) = FrameMailbox::open();
        let decoded = Arc::new(AtomicU64::new(0));
        let counter = decoded.clone();
        let on_frame: xcloud_stream::decode_thread::FrameCallback = Box::new(move || {
            counter.fetch_add(1, Ordering::Relaxed);
            ctx.notify();
        });
        #[cfg(target_os = "macos")]
        let decode = DecodeThread::spawn(writer, Some(on_frame), |sink| {
            xcloud_media::vt::VtDecoder::new(
                xcloud_media::vt::VtConfig {
                    hardware: xcloud_media::vt::HardwarePolicy::Require,
                    ..Default::default()
                },
                sink,
            )
        });
        #[cfg(not(target_os = "macos"))]
        let decode = DecodeThread::spawn(writer, Some(on_frame), |sink| {
            xcloud_media::ffmpeg::FfmpegDecoder::new(Default::default(), sink)
        });
        let audio = Arc::new(Mutex::new(AudioOutput::new()));
        let output = audio.clone();
        let muted = Arc::new(AtomicBool::new(start_muted));
        let mute_gate = muted.clone();
        let (tx, rx) = crossbeam_channel::bounded(64);
        let pipeline = NativeMediaPipeline::new(
            transport.rtcp_sender(),
            decode,
            Box::new(move |pcm, rate, channels| {
                push_audio(&output, &mute_gate, pcm, rate, channels as u16);
            }),
            tx,
        );
        let health = Arc::new(Mutex::new(MediaHealth::default()));
        transport.set_configurable_media_sink(Box::new(Pipeline {
            inner: pipeline,
            health: health.clone(),
            last_report: 0,
        }));
        (
            Self {
                reader: Arc::new(Mutex::new(reader)),
                audio,
                muted,
                decoded,
                health,
            },
            rx,
        )
    }
}

fn push_audio(
    output: &Mutex<AudioOutput>,
    muted: &AtomicBool,
    pcm: &[i16],
    rate: u32,
    channels: u16,
) {
    let mut output = output.lock();
    if !muted.load(Ordering::Relaxed) {
        output.push(pcm, rate, channels);
    }
}

#[cfg(test)]
mod audio_tests {
    use super::*;
    struct SilentBackend;
    impl xcloud_media::AudioBackend for SilentBackend {
        fn open(
            &mut self,
            _: xcloud_media::PcmFormat,
            _: xcloud_media::PcmSource,
        ) -> xcloud_media::Result<Box<dyn xcloud_media::OutputStream>> {
            Err(xcloud_media::Error::Unavailable("synthetic backend".into()))
        }
    }
    #[test]
    fn mute_discards_audio_without_opening_a_device_and_unmute_restarts_input() {
        let output = Mutex::new(AudioOutput::with_backend(Box::new(SilentBackend)));
        let muted = AtomicBool::new(true);
        push_audio(&output, &muted, &[10, 20], 48000, 2);
        assert!(output.lock().format().is_none());
        muted.store(false, Ordering::Relaxed);
        push_audio(&output, &muted, &[10, 20], 48000, 2);
        assert_eq!(
            output.lock().format(),
            Some(xcloud_media::PcmFormat::new(48000, 2))
        );
        muted.store(true, Ordering::Relaxed);
        output.lock().stop();
        push_audio(&output, &muted, &[10, 20], 48000, 2);
        assert!(output.lock().format().is_none());
    }
}
