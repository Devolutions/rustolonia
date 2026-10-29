//! Video with Media Foundation and WASAPI: an MP4 writer (H.264 video, AAC
//! audio), an MP4 reader that decodes to top-down BGRA frames and 16-bit
//! PCM, and capture of system audio (loopback) or the microphone.
//!
//! Times are in 100-nanosecond units ("hns"), as Media Foundation uses.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use windows::core::{GUID, HSTRING};
use windows::Win32::Media::Audio::{
    eCapture, eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT,
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED};

pub type Result<T> = std::result::Result<T, String>;

fn e(error: windows::core::Error) -> String {
    format!("{} (0x{:08X})", error.message(), error.code().0 as u32)
}

pub const HNS_PER_SECOND: i64 = 10_000_000;

/// `CODECAPI_AVEncMPVGOPSize`: frames between keyframes.
const CODECAPI_AVENC_MPV_GOP_SIZE: GUID = GUID::from_u128(0x95f31b26_95a4_41aa_9303_246a7fc6eef1);

/// Starts COM (multithreaded) on this thread and Media Foundation once.
pub fn startup() -> Result<()> {
    // SAFETY: per-thread COM initialization; "already initialized" results
    // are fine.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    static STARTED: OnceLock<std::result::Result<(), String>> = OnceLock::new();
    // SAFETY: MFStartup once per process; never shut down while running.
    STARTED.get_or_init(|| unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL).map_err(e) }).clone()
}

fn pack(hi: u32, lo: u32) -> u64 {
    (hi as u64) << 32 | lo as u64
}

fn unpack(v: u64) -> (u32, u32) {
    ((v >> 32) as u32, v as u32)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VideoSpec {
    /// Even width and height (H.264 needs them).
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate: u32,
}

impl VideoSpec {
    /// A spec for `width` x `height` (rounded down to even) with a bitrate
    /// that keeps screen text sharp.
    pub fn for_size(width: u32, height: u32, fps: u32) -> VideoSpec {
        let (w, h) = (width & !1, height & !1);
        let bitrate = ((w as u64 * h as u64 * fps as u64) / 6).clamp(1_500_000, 40_000_000) as u32;
        VideoSpec { width: w, height: h, fps, bitrate }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioSpec {
    pub sample_rate: u32,
    pub channels: u32,
}

/// Writes an MP4 file. Frames are top-down BGRA (the fourth byte unused).
pub struct Writer {
    writer: IMFSinkWriter,
    video: u32,
    audio: Option<u32>,
    spec: VideoSpec,
}

// SAFETY: the sink writer is free-threaded; the writer is used from one
// thread at a time.
unsafe impl Send for Writer {}

fn media_type() -> Result<IMFMediaType> {
    // SAFETY: plain factory call.
    unsafe { MFCreateMediaType().map_err(e) }
}

impl Writer {
    pub fn create(path: &Path, spec: VideoSpec, audio: Option<AudioSpec>) -> Result<Writer> {
        startup()?;
        // SAFETY: Media Foundation calls with valid arguments; every object is
        // reference counted by the `windows` crate.
        unsafe {
            let mut attrs: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attrs, 2).map_err(e)?;
            let attrs = attrs.unwrap();
            attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, std::env::var_os("JIFFSHOT_HW_ENCODE").is_some() as u32).map_err(e)?;
            attrs.SetUINT32(&MF_SINK_WRITER_DISABLE_THROTTLING, 1).map_err(e)?;
            let writer = MFCreateSinkWriterFromURL(&HSTRING::from(path.as_os_str()), None, &attrs).map_err(e)?;

            let out = media_type()?;
            out.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(e)?;
            out.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264).map_err(e)?;
            out.SetUINT32(&MF_MT_AVG_BITRATE, spec.bitrate).map_err(e)?;
            out.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32).map_err(e)?;
            out.SetUINT64(&MF_MT_FRAME_SIZE, pack(spec.width, spec.height)).map_err(e)?;
            out.SetUINT64(&MF_MT_FRAME_RATE, pack(spec.fps, 1)).map_err(e)?;
            out.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1)).map_err(e)?;
            let video = writer.AddStream(&out).map_err(e)?;

            let input = media_type()?;
            input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(e)?;
            input.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).map_err(e)?;
            input.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32).map_err(e)?;
            input.SetUINT64(&MF_MT_FRAME_SIZE, pack(spec.width, spec.height)).map_err(e)?;
            input.SetUINT64(&MF_MT_FRAME_RATE, pack(spec.fps, 1)).map_err(e)?;
            input.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1)).map_err(e)?;
            // A positive stride says the rows are top-down.
            input.SetUINT32(&MF_MT_DEFAULT_STRIDE, spec.width * 4).map_err(e)?;
            // A keyframe every two seconds, so the editor can seek quickly
            // (hardware encoders otherwise may write just one).
            let mut params: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut params, 1).map_err(e)?;
            let params = params.unwrap();
            params.SetUINT32(&CODECAPI_AVENC_MPV_GOP_SIZE, spec.fps * 2).map_err(e)?;
            writer.SetInputMediaType(video, &input, &params).map_err(e)?;

            let audio = match audio {
                Some(a) => {
                    let out = media_type()?;
                    out.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).map_err(e)?;
                    out.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_AAC).map_err(e)?;
                    out.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).map_err(e)?;
                    out.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, a.sample_rate).map_err(e)?;
                    out.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, a.channels).map_err(e)?;
                    out.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 24_000).map_err(e)?;
                    let index = writer.AddStream(&out).map_err(e)?;
                    let input = media_type()?;
                    input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).map_err(e)?;
                    input.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM).map_err(e)?;
                    input.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).map_err(e)?;
                    input.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, a.sample_rate).map_err(e)?;
                    input.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, a.channels).map_err(e)?;
                    input.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, a.channels * 2).map_err(e)?;
                    input.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, a.sample_rate * a.channels * 2).map_err(e)?;
                    writer.SetInputMediaType(index, &input, None).map_err(e)?;
                    Some(index)
                }
                None => None,
            };
            writer.BeginWriting().map_err(e)?;
            Ok(Writer { writer, video, audio, spec })
        }
    }

    pub fn spec(&self) -> VideoSpec {
        self.spec
    }

    fn write(&self, stream: u32, bytes: &[u8], time: i64, duration: i64) -> Result<()> {
        // SAFETY: the buffer is locked, filled within its length and
        // unlocked before it is handed to the sample.
        unsafe {
            let buffer = MFCreateMemoryBuffer(bytes.len() as u32).map_err(e)?;
            let mut p = std::ptr::null_mut();
            buffer.Lock(&mut p, None, None).map_err(e)?;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
            buffer.Unlock().map_err(e)?;
            buffer.SetCurrentLength(bytes.len() as u32).map_err(e)?;
            let sample = MFCreateSample().map_err(e)?;
            sample.AddBuffer(&buffer).map_err(e)?;
            sample.SetSampleTime(time).map_err(e)?;
            sample.SetSampleDuration(duration).map_err(e)?;
            self.writer.WriteSample(stream, &sample).map_err(e)
        }
    }

    /// Writes a frame of exactly `width * height * 4` bytes.
    pub fn write_frame(&mut self, bgra: &[u8], time: i64, duration: i64) -> Result<()> {
        let len = (self.spec.width * self.spec.height * 4) as usize;
        if bgra.len() < len {
            return Err("frame too small".into());
        }
        self.write(self.video, &bgra[..len], time, duration)
    }

    /// Writes interleaved 16-bit samples.
    pub fn write_audio(&mut self, pcm: &[i16], time: i64, duration: i64) -> Result<()> {
        let Some(stream) = self.audio else { return Ok(()) };
        if pcm.is_empty() {
            return Ok(());
        }
        // SAFETY: i16 slices are plain bytes.
        let bytes = unsafe { std::slice::from_raw_parts(pcm.as_ptr() as *const u8, pcm.len() * 2) };
        self.write(stream, bytes, time, duration)
    }

    pub fn has_audio(&self) -> bool {
        self.audio.is_some()
    }

    pub fn finish(self) -> Result<()> {
        // SAFETY: completes the file; the writer is dropped afterwards.
        unsafe { self.writer.Finalize().map_err(e) }
    }
}

/// A decoded video frame.
pub struct VideoFrame {
    pub time: i64,
    pub width: u32,
    pub height: u32,
    /// Top-down BGRA, `width * 4` bytes per row.
    pub bgra: Vec<u8>,
}

/// Reads an MP4 (or anything Media Foundation can open).
pub struct Reader {
    reader: IMFSourceReader,
    pub width: u32,
    pub height: u32,
    stride: i32,
    pub duration: i64,
    pub fps: f64,
    pub audio: Option<AudioSpec>,
}

// SAFETY: as for `Writer`.
unsafe impl Send for Reader {}

const VIDEO: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
const AUDIO: u32 = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;

impl Reader {
    pub fn open(path: &Path) -> Result<Reader> {
        startup()?;
        // SAFETY: Media Foundation calls with valid arguments.
        unsafe {
            let mut attrs: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attrs, 1).map_err(e)?;
            let attrs = attrs.unwrap();
            attrs.SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1).map_err(e)?;
            let reader = MFCreateSourceReaderFromURL(&HSTRING::from(path.as_os_str()), &attrs).map_err(e)?;
            Self::with_reader(reader)
        }
    }

    /// The first webcam, decoded to BGRA frames as they arrive (`duration`
    /// is 0; read with `next_video`).
    pub fn webcam() -> Result<Reader> {
        startup()?;
        // SAFETY: Media Foundation calls with valid arguments; the activate
        // array is freed with CoTaskMemFree after its first entry is taken.
        unsafe {
            let mut attrs: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attrs, 1).map_err(e)?;
            let attrs = attrs.unwrap();
            attrs.SetGUID(&MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE, &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID).map_err(e)?;
            let mut devices: *mut Option<IMFActivate> = std::ptr::null_mut();
            let mut count = 0u32;
            MFEnumDeviceSources(&attrs, &mut devices, &mut count).map_err(e)?;
            let list = std::slice::from_raw_parts_mut(devices, count as usize);
            let first = list.first().cloned().flatten();
            for d in list.iter_mut() {
                drop(d.take());
            }
            CoTaskMemFree(Some(devices as *const _));
            let activate = first.ok_or("no webcam")?;
            let source: IMFMediaSource = activate.ActivateObject().map_err(e)?;
            let mut rattrs: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut rattrs, 1).map_err(e)?;
            let rattrs = rattrs.unwrap();
            rattrs.SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1).map_err(e)?;
            let reader = MFCreateSourceReaderFromMediaSource(&source, &rattrs).map_err(e)?;
            Self::with_reader(reader)
        }
    }

    fn with_reader(reader: IMFSourceReader) -> Result<Reader> {
        // SAFETY: Media Foundation calls on a live source reader.
        unsafe {
            let want = media_type()?;
            want.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(e)?;
            want.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).map_err(e)?;
            reader.SetCurrentMediaType(VIDEO, None, &want).map_err(e)?;
            let current = reader.GetCurrentMediaType(VIDEO).map_err(e)?;
            let (width, height) = unpack(current.GetUINT64(&MF_MT_FRAME_SIZE).map_err(e)?);
            let stride = current.GetUINT32(&MF_MT_DEFAULT_STRIDE).map(|s| s as i32).unwrap_or(width as i32 * 4);
            let fps = current.GetUINT64(&MF_MT_FRAME_RATE).map(unpack).map(|(n, d)| n as f64 / d.max(1) as f64).unwrap_or(30.0);
            let duration = reader
                .GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE.0 as u32, &MF_PD_DURATION)
                .ok()
                .and_then(|v| u64::try_from(&v).ok())
                .unwrap_or(0) as i64;
            let pcm = media_type()?;
            pcm.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).map_err(e)?;
            pcm.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM).map_err(e)?;
            pcm.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).map_err(e)?;
            let audio = reader.SetCurrentMediaType(AUDIO, None, &pcm).ok().and_then(|_| {
                let t = reader.GetCurrentMediaType(AUDIO).ok()?;
                Some(AudioSpec {
                    sample_rate: t.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND).ok()?,
                    channels: t.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS).ok()?,
                })
            });
            let mut r = Reader { reader, width, height, stride, duration, fps, audio };
            r.select_audio(false)?;
            Ok(r)
        }
    }

    /// Whether audio samples are read too (off by default, for frame
    /// decoding).
    pub fn select_audio(&mut self, on: bool) -> Result<()> {
        if self.audio.is_none() {
            return Ok(());
        }
        // SAFETY: valid stream index.
        unsafe { self.reader.SetStreamSelection(AUDIO, on).map_err(e) }
    }

    /// Whether video frames are decoded (turn off to read only audio).
    pub fn select_video(&mut self, on: bool) -> Result<()> {
        // SAFETY: valid stream index.
        unsafe { self.reader.SetStreamSelection(VIDEO, on).map_err(e) }
    }

    pub fn seek(&mut self, time: i64) -> Result<()> {
        let position = PROPVARIANT::from(time.max(0));
        // SAFETY: a PROPVARIANT holding a VT_I8 position, as required.
        unsafe { self.reader.SetCurrentPosition(&GUID::zeroed(), &position).map_err(e) }
    }

    fn read(&mut self, stream: u32) -> Result<Option<(i64, IMFSample)>> {
        loop {
            let (mut flags, mut time) = (0u32, 0i64);
            let mut sample: Option<IMFSample> = None;
            // SAFETY: out-parameters only.
            unsafe { self.reader.ReadSample(stream, 0, None, Some(&mut flags), Some(&mut time), Some(&mut sample)).map_err(e)? };
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                return Ok(None);
            }
            if stream == VIDEO && flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0 as u32 != 0 {
                // The decoder pads frames to a multiple of 16 and says so
                // here; the picture stays at its original size, top left.
                // SAFETY: reads the reader's current output type.
                if let Ok(t) = unsafe { self.reader.GetCurrentMediaType(VIDEO) } {
                    let coded = unsafe { t.GetUINT64(&MF_MT_FRAME_SIZE) }.map(unpack).map(|(w, _)| w).unwrap_or(self.width);
                    self.stride = unsafe { t.GetUINT32(&MF_MT_DEFAULT_STRIDE) }.map(|s| s as i32).unwrap_or(coded as i32 * 4);
                }
            }
            if let Some(sample) = sample {
                return Ok(Some((time, sample)));
            }
        }
    }

    fn bytes(sample: &IMFSample) -> Result<Vec<u8>> {
        // SAFETY: the buffer is locked while it is copied.
        unsafe {
            let buffer = sample.ConvertToContiguousBuffer().map_err(e)?;
            let (mut p, mut len) = (std::ptr::null_mut(), 0u32);
            buffer.Lock(&mut p, None, Some(&mut len)).map_err(e)?;
            let out = std::slice::from_raw_parts(p, len as usize).to_vec();
            buffer.Unlock().map_err(e)?;
            Ok(out)
        }
    }

    /// The next frame in decode order.
    pub fn next_video(&mut self) -> Result<Option<VideoFrame>> {
        let Some((time, sample)) = self.read(VIDEO)? else { return Ok(None) };
        let (w, h) = (self.width as usize, self.height as usize);
        let row = w * 4;
        let mut bgra = vec![0u8; row * h];
        let raw = Self::bytes(&sample)?;
        let stride = self.stride.unsigned_abs() as usize;
        for y in 0..h {
            // A negative stride means the buffer is bottom-up.
            let src_y = if self.stride < 0 { h - 1 - y } else { y };
            let start = src_y * stride;
            if start + row <= raw.len() {
                bgra[y * row..(y + 1) * row].copy_from_slice(&raw[start..start + row]);
            }
        }
        Ok(Some(VideoFrame { time, width: self.width, height: self.height, bgra }))
    }

    /// The next block of interleaved 16-bit audio.
    pub fn next_audio(&mut self) -> Result<Option<(i64, Vec<i16>)>> {
        let Some((time, sample)) = self.read(AUDIO)? else { return Ok(None) };
        let raw = Self::bytes(&sample)?;
        Ok(Some((time, raw.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect())))
    }

    /// The frame showing at `time`.
    pub fn frame_at(&mut self, time: i64) -> Result<Option<VideoFrame>> {
        self.seek(time)?;
        let half = (HNS_PER_SECOND as f64 / self.fps.max(1.0) / 2.0) as i64;
        let mut last = None;
        while let Some(frame) = self.next_video()? {
            let done = frame.time + half >= time;
            last = Some(frame);
            if done {
                break;
            }
        }
        Ok(last)
    }
}

/// Audio being captured: interleaved 16-bit stereo blocks arrive on
/// `samples`; dropping the handle stops capture.
pub struct AudioCapture {
    pub spec: AudioSpec,
    pub samples: Receiver<Vec<i16>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
const SUBTYPE_IEEE_FLOAT: GUID = GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);

/// Starts capturing what the speakers play (`microphone` false) or the
/// default microphone, converted to 16-bit stereo at the device rate.
pub fn capture_audio(microphone: bool) -> Result<AudioCapture> {
    let (tx, rx) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<AudioSpec>>();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let thread = std::thread::Builder::new()
        .name("audio-capture".into())
        .spawn(move || {
            // SAFETY: WASAPI calls on this thread with COM initialized; the mix
            // format is freed with CoTaskMemFree as documented.
            let run = || unsafe {
                CoInitializeEx(None, COINIT_MULTITHREADED).ok().map_err(e)?;
                let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(e)?;
                let device = devices.GetDefaultAudioEndpoint(if microphone { eCapture } else { eRender }, eConsole).map_err(e)?;
                let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(e)?;
                let format = client.GetMixFormat().map_err(e)?;
                let f: WAVEFORMATEX = *format;
                let float = f.wFormatTag == WAVE_FORMAT_IEEE_FLOAT
                    || (f.wFormatTag == WAVE_FORMAT_EXTENSIBLE && { (*(format as *const WAVEFORMATEXTENSIBLE)).SubFormat } == SUBTYPE_IEEE_FLOAT);
                let (channels, bits, rate) = (f.nChannels as usize, f.wBitsPerSample, f.nSamplesPerSec);
                let flags = if microphone { 0 } else { AUDCLNT_STREAMFLAGS_LOOPBACK };
                let init = client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 2_000_000, 0, format, None);
                CoTaskMemFree(Some(format as *const _));
                init.map_err(e)?;
                if !(rate == 44_100 || rate == 48_000) || !(float && bits == 32 || !float && bits == 16) {
                    return Err(format!("unsupported audio format: {rate} Hz, {bits} bit{}", if float { " float" } else { "" }));
                }
                let capture: IAudioCaptureClient = client.GetService().map_err(e)?;
                client.Start().map_err(e)?;
                let _ = ready_tx.send(Ok(AudioSpec { sample_rate: rate, channels: 2 }));
                while !flag.load(Ordering::Acquire) {
                    std::thread::sleep(Duration::from_millis(10));
                    while capture.GetNextPacketSize().map_err(e)? > 0 {
                        let (mut data, mut frames, mut bflags) = (std::ptr::null_mut(), 0u32, 0u32);
                        capture.GetBuffer(&mut data, &mut frames, &mut bflags, None, None).map_err(e)?;
                        let n = frames as usize;
                        let mut out = vec![0i16; n * 2];
                        if bflags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 == 0 {
                            for i in 0..n {
                                for c in 0..2 {
                                    let src = (i * channels + c.min(channels - 1)) as usize;
                                    out[i * 2 + c] = if float {
                                        let v = *(data as *const f32).add(src);
                                        (v.clamp(-1.0, 1.0) * 32767.0) as i16
                                    } else {
                                        *(data as *const i16).add(src)
                                    };
                                }
                            }
                        }
                        capture.ReleaseBuffer(frames).map_err(e)?;
                        if tx.send(out).is_err() {
                            break;
                        }
                    }
                }
                let _ = client.Stop();
                Ok(())
            };
            if let Err(error) = run() {
                let _ = ready_tx.send(Err(error));
            }
        })
        .map_err(|e| e.to_string())?;
    let spec = ready_rx.recv_timeout(Duration::from_secs(5)).map_err(|_| "audio device did not start".to_owned())??;
    Ok(AudioCapture { spec, samples: rx, stop, thread: Some(thread) })
}

/// Silence for `frames` stereo frames.
pub fn silence(frames: usize) -> Vec<i16> {
    vec![0; frames * 2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_reads_back_an_mp4() {
        let dir = std::env::temp_dir().join(format!("jiffshot-media-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.mp4");
        let spec = VideoSpec::for_size(320, 240, 10);
        let audio = AudioSpec { sample_rate: 48_000, channels: 2 };
        let mut w = Writer::create(&path, spec, Some(audio)).unwrap();
        let step = HNS_PER_SECOND / 10;
        for i in 0..20i64 {
            // A bar that moves right, on gray; top half red, bottom half blue.
            let mut frame = vec![0u8; 320 * 240 * 4];
            for y in 0..240usize {
                for x in 0..320usize {
                    let p = &mut frame[(y * 320 + x) * 4..(y * 320 + x) * 4 + 4];
                    let bar = (x as i64 / 16) == i;
                    let c = if bar { [255, 255, 255, 0] } else if y < 120 { [40, 40, 220, 0] } else { [220, 60, 40, 0] };
                    p.copy_from_slice(&c);
                }
            }
            w.write_frame(&frame, i * step, step).unwrap();
            let tone: Vec<i16> = (0..4800).flat_map(|n| {
                let v = ((n as f32 / 48_000.0 * 440.0 * std::f32::consts::TAU).sin() * 8000.0) as i16;
                [v, v]
            }).collect();
            w.write_audio(&tone, i * step, step).unwrap();
        }
        w.finish().unwrap();

        let mut r = Reader::open(&path).unwrap();
        assert_eq!((r.width, r.height), (320, 240));
        assert!((r.duration - 2 * HNS_PER_SECOND).abs() < HNS_PER_SECOND / 5, "duration {}", r.duration);
        assert_eq!(r.audio, Some(audio));
        let f = r.frame_at(HNS_PER_SECOND).unwrap().unwrap();
        // Orientation and colors survive: red on top, blue at the bottom
        // (BGRA), and the bar of frame 10 at x = 160..176.
        let px = |x: usize, y: usize| &f.bgra[(y * 320 + x) * 4..(y * 320 + x) * 4 + 3];
        assert!(px(40, 30)[2] > 150 && px(40, 30)[0] < 100, "top is red: {:?}", px(40, 30));
        assert!(px(40, 200)[0] > 150 && px(40, 200)[2] < 100, "bottom is blue: {:?}", px(40, 200));
        assert!(px(168, 60).iter().all(|c| *c > 200), "bar at frame 10: {:?}", px(168, 60));
        r.select_audio(true).unwrap();
        r.seek(0).unwrap();
        let (_, pcm) = r.next_audio().unwrap().unwrap();
        assert!(pcm.iter().any(|s| s.abs() > 2000), "audio came back");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rows_stay_aligned_at_sizes_not_a_multiple_of_16() {
        // A region recording's size is whatever was dragged; the encoder
        // pads to 16, and the padding must not shear the picture.
        let dir = std::env::temp_dir().join(format!("jiffshot-media-odd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("odd.mp4");
        let spec = VideoSpec::for_size(1430, 718, 10);
        let (w, h) = (spec.width as usize, spec.height as usize);
        let mut writer = Writer::create(&path, spec, None).unwrap();
        // A white column at x = 700..740 on dark gray, every row.
        let frame: Vec<u8> = (0..w * h).flat_map(|i| if (700..740).contains(&(i % w)) { [255u8; 4] } else { [40, 40, 40, 0] }).collect();
        let step = HNS_PER_SECOND / 10;
        for i in 0..10i64 {
            writer.write_frame(&frame, i * step, step).unwrap();
        }
        writer.finish().unwrap();

        let mut r = Reader::open(&path).unwrap();
        assert_eq!((r.width as usize, r.height as usize), (w, h));
        let f = r.frame_at(HNS_PER_SECOND / 2).unwrap().unwrap();
        for y in [5, h / 2, h - 5] {
            let px = |x: usize| f.bgra[(y * w + x) * 4];
            assert!(px(720) > 200 && px(650) < 90 && px(790) < 90, "column at row {y}: {} {} {}", px(650), px(720), px(790));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod probe {
    /// `JIFFSHOT_PROBE=<file.mp4> cargo test probe -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn probe_file() {
        let path = std::env::var("JIFFSHOT_PROBE").unwrap();
        let mut r = super::Reader::open(std::path::Path::new(&path)).unwrap();
        println!("{}x{} {:.2}s fps {} audio {:?}", r.width, r.height, r.duration as f64 / 1e7, r.fps, r.audio);
        let mut n = 0;
        loop {
            match r.next_video() {
                Ok(Some(f)) => {
                    n += 1;
                    if n % 20 == 0 {
                        println!("  frame {n} at {}", f.time);
                    }
                }
                Ok(None) => {
                    println!("end after {n} frames");
                    break;
                }
                Err(err) => {
                    println!("error after {n} frames: {err}");
                    break;
                }
            }
        }
        for t in [0i64, 1, 5_000_000, 25_000_000] {
            let mut r = super::Reader::open(std::path::Path::new(&path)).unwrap();
            println!("seek {t}: {:?}", r.seek(t));
            match r.next_video() {
                Ok(Some(f)) => println!("  frame at {}", f.time),
                other => println!("  {:?}", other.map(|f| f.map(|f| f.time))),
            }
        }
    }
}
