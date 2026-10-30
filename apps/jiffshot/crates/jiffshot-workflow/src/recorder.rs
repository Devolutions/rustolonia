//! Screen recording: frames of an area (with the pointer, its highlight and
//! click rings, and the webcam in a corner) and system audio, the
//! microphone or both mixed, encoded to an H.264/AAC MP4 in the library.
//! A countdown comes first; then a toolbar below the area stops, pauses,
//! restarts or cancels, starts Screen Draw, and switches the pointer, the
//! webcam, system sound and the microphone on and off while recording.
//! Shift+F9 / Shift+F10 pause and stop from anywhere, or the next free
//! keys when another app holds those ([`key_names`]).

use crate::hud::{self, Border};
use crate::settings::Settings;
use crate::vfx::{self, Clicks, Mixer, NoiseGate};
use crate::{log, screendraw};
use jiffshot_capture::library::Library;
use jiffshot_capture::media::{self, AudioCapture, Reader, VideoSpec, Writer, HNS_PER_SECOND};
use jiffshot_capture::screen::{CursorSnapshot, Grabber};
use jiffshot_capture::{Frame, Rect};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde_json::json;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, VK_F10, VK_F9,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// A recording key: modifiers, virtual key and how it is written.
type Key = (HOT_KEY_MODIFIERS, u16, &'static str);

/// The pause and stop keys, best first: another app (often another capture
/// tool) may hold Shift+F9 or Shift+F10, and a key registered by nobody
/// does nothing when pressed.
const PAUSE_KEYS: [Key; 3] = [(MOD_SHIFT, VK_F9, "Shift+F9"), (MOD_CONTROL | MOD_SHIFT, VK_F9, "Ctrl+Shift+F9"), (MOD_CONTROL | MOD_ALT, VK_F9, "Ctrl+Alt+F9")];
const STOP_KEYS: [Key; 3] = [(MOD_SHIFT, VK_F10, "Shift+F10"), (MOD_CONTROL | MOD_SHIFT, VK_F10, "Ctrl+Shift+F10"), (MOD_CONTROL | MOD_ALT, VK_F10, "Ctrl+Alt+F10")];

/// The pause and stop keys the last check found free.
static KEYS: Mutex<Option<(Option<Key>, Option<Key>)>> = Mutex::new(None);

/// The first of `keys` no other app holds.
fn free(keys: &[Key]) -> Option<Key> {
    const PROBE: i32 = 0x53F0;
    keys.iter().copied().find(|&(modifiers, vk, _)| {
        // SAFETY: a test registration for this thread, removed at once.
        unsafe {
            let ok = RegisterHotKey(std::ptr::null_mut(), PROBE, modifiers | MOD_NOREPEAT, vk as u32) != 0;
            if ok {
                UnregisterHotKey(std::ptr::null_mut(), PROBE);
            }
            ok
        }
    })
}

/// Finds the pause and stop keys free now. Not while recording, which holds
/// them itself.
pub fn check_keys() -> (Option<Key>, Option<Key>) {
    let keys = (free(&PAUSE_KEYS), free(&STOP_KEYS));
    *KEYS.lock().unwrap_or_else(|e| e.into_inner()) = Some(keys);
    keys
}

/// How the pause and stop keys are written ("Shift+F9", "Shift+F10"), or
/// `None` for one that every candidate is taken for (the toolbar's button
/// still works).
pub fn key_names() -> (Option<&'static str>, Option<&'static str>) {
    let known = *KEYS.lock().unwrap_or_else(|e| e.into_inner());
    let (pause, stop) = known.unwrap_or_else(check_keys);
    (pause.map(|k| k.2), stop.map(|k| k.2))
}

/// "Shift+F9 pauses and resumes{sep}Shift+F10 stops", for the windows that
/// list keys.
pub fn keys_text(sep: &str) -> String {
    let (pause, stop) = key_names();
    let pause = pause.map_or("the toolbar pauses and resumes".to_owned(), |k| format!("{k} pauses and resumes"));
    let stop = stop.map_or("the toolbar stops".to_owned(), |k| format!("{k} stops"));
    format!("{pause}{sep}{stop}")
}

/// State shared between the recording thread and the toolbar. The
/// switches are read on every frame, so they change a recording while it
/// runs.
#[derive(Default)]
pub struct Control {
    pub stop: AtomicBool,
    pub paused: AtomicBool,
    pub frames: AtomicU64,
    /// Recorded time so far (pauses excluded), in milliseconds.
    pub millis: AtomicU64,
    pub error: Mutex<Option<String>>,
    /// A note about the sound or the webcam ("no microphone: …").
    pub audio: Mutex<Option<String>>,
    /// The pointer is drawn into the frames.
    pub cursor: AtomicBool,
    pub highlight: AtomicBool,
    pub clicks: AtomicBool,
    pub webcam: AtomicBool,
    pub webcam_circle: AtomicBool,
    /// "top-left", "top-right", "bottom-left" or "bottom-right".
    pub webcam_corner: Mutex<String>,
    /// The sound sources wanted; one that cannot start turns itself off.
    pub system: AtomicBool,
    pub microphone: AtomicBool,
}

impl Control {
    /// The switches as `options` sets them.
    pub fn new(options: &Options) -> Control {
        let c = Control::default();
        c.cursor.store(options.cursor, Ordering::Relaxed);
        c.highlight.store(options.cursor_highlight, Ordering::Relaxed);
        c.clicks.store(options.click_rings, Ordering::Relaxed);
        c.webcam.store(options.webcam.is_some(), Ordering::Relaxed);
        c.webcam_circle.store(options.webcam_circle, Ordering::Relaxed);
        *c.webcam_corner.lock().unwrap() = options.webcam.clone().unwrap_or_else(|| options.webcam_corner.clone());
        c.system.store(matches!(options.audio.as_str(), "system" | "both"), Ordering::Relaxed);
        c.microphone.store(matches!(options.audio.as_str(), "microphone" | "both"), Ordering::Relaxed);
        c
    }

    /// A fresh take with this one's switches (Restart).
    fn again(&self) -> Control {
        let c = Control::default();
        for (to, from) in [
            (&c.cursor, &self.cursor),
            (&c.highlight, &self.highlight),
            (&c.clicks, &self.clicks),
            (&c.webcam, &self.webcam),
            (&c.webcam_circle, &self.webcam_circle),
            (&c.system, &self.system),
            (&c.microphone, &self.microphone),
        ] {
            to.store(from.load(Ordering::Relaxed), Ordering::Relaxed);
        }
        *c.webcam_corner.lock().unwrap() = self.webcam_corner.lock().unwrap().clone();
        c
    }

    fn note(&self, text: String) {
        *self.audio.lock().unwrap() = Some(text);
    }

    fn on(&self, flag: &AtomicBool) -> bool {
        flag.load(Ordering::Relaxed)
    }
}

/// What goes into a recording besides the screen.
#[derive(Clone, Debug)]
pub struct Options {
    pub fps: u32,
    /// "system", "microphone", "both" or "none".
    pub audio: String,
    pub noise_gate: bool,
    pub cursor_highlight: bool,
    pub click_rings: bool,
    pub cursor: bool,
    pub webcam: Option<String>,
    /// Where the webcam goes when it is turned on while recording.
    pub webcam_corner: String,
    pub webcam_circle: bool,
    pub webcam_size: f32,
}

impl Options {
    pub fn from_settings(s: &Settings) -> Options {
        Options {
            fps: s.record_fps,
            audio: s.record_audio.clone(),
            noise_gate: s.noise_gate,
            cursor_highlight: s.cursor_highlight,
            click_rings: s.click_rings,
            cursor: s.record_cursor,
            webcam: s.webcam.clone(),
            webcam_corner: s.webcam.clone().unwrap_or_else(|| s.capture_window.webcam_corner.clone()),
            webcam_circle: s.webcam_shape != "rectangle",
            webcam_size: s.webcam_size,
        }
    }

    pub fn audio_only(fps: u32, audio: &str) -> Options {
        Options {
            fps,
            audio: audio.to_owned(),
            noise_gate: false,
            cursor_highlight: false,
            click_rings: false,
            cursor: true,
            webcam: None,
            webcam_corner: "bottom-right".into(),
            webcam_circle: true,
            webcam_size: 0.25,
        }
    }
}

/// The latest webcam picture, refreshed by a reader thread.
struct Webcam {
    latest: Arc<Mutex<Option<(u32, u32, Vec<u8>)>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Webcam {
    fn start() -> Result<Webcam, String> {
        let latest: Arc<Mutex<Option<(u32, u32, Vec<u8>)>>> = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let (l, s) = (latest.clone(), stop.clone());
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            let mut reader = match Reader::webcam() {
                Ok(r) => {
                    let _ = ready_tx.send(Ok(()));
                    r
                }
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                    return;
                }
            };
            while !s.load(Ordering::Acquire) {
                match reader.next_video() {
                    Ok(Some(f)) => *l.lock().unwrap() = Some((f.width, f.height, f.bgra)),
                    _ => break,
                }
            }
        });
        ready_rx.recv_timeout(Duration::from_secs(5)).map_err(|_| "the webcam did not start".to_owned())??;
        Ok(Webcam { latest, stop, thread: Some(thread) })
    }
}

impl Drop for Webcam {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn start_audio(microphone: bool, control: &Control) -> Option<AudioCapture> {
    match media::capture_audio(microphone) {
        Ok(c) => Some(c),
        Err(error) => {
            let what = if microphone { "microphone" } else { "system audio" };
            control.note(format!("no {what}: {error}"));
            None
        }
    }
}

/// The sound track's format when no source is on at the start, so one
/// turned on later can still join.
const DEFAULT_AUDIO: media::AudioSpec = media::AudioSpec { sample_rate: 48_000, channels: 2 };

/// A sound source that follows its switch: started when turned on (if its
/// format matches the track's), stopped when turned off.
struct Source {
    microphone: bool,
    capture: Option<AudioCapture>,
    /// It could not start or join; it stays off for this take.
    failed: bool,
}

impl Source {
    fn follow(&mut self, wanted: &AtomicBool, spec: media::AudioSpec, control: &Control) {
        let on = wanted.load(Ordering::Relaxed);
        if on && self.capture.is_none() && !self.failed {
            match start_audio(self.microphone, control) {
                Some(c) if c.spec == spec => self.capture = Some(c),
                Some(c) => {
                    let what = if self.microphone { "microphone" } else { "system sound" };
                    control.note(format!("{what} left out: {} Hz, the recording is {} Hz", c.spec.sample_rate, spec.sample_rate));
                    self.failed = true;
                }
                None => self.failed = true,
            }
            if self.failed {
                wanted.store(false, Ordering::Relaxed);
            }
        } else if !on && self.capture.is_some() {
            self.capture = None;
        }
    }
}

fn cursor_in(rect: Rect) -> Option<(i32, i32)> {
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: out-parameter only.
    unsafe { GetCursorPos(&mut p) };
    rect.contains(p.x, p.y).then_some((p.x - rect.x, p.y - rect.y))
}

/// Records `rect` into `path` until `control.stop` is set. Runs on the
/// calling thread.
pub fn record_to(path: &Path, rect: Rect, options: &Options, control: &Control) -> Result<VideoSpec, String> {
    media::startup()?;
    let spec = VideoSpec::for_size(rect.width as u32, rect.height as u32, options.fps.clamp(5, 60));
    let rect = Rect::new(rect.x, rect.y, spec.width as i32, spec.height as i32);
    let mut system = Source { microphone: false, capture: None, failed: false };
    let mut microphone = Source { microphone: true, capture: None, failed: false };
    // The track takes the format of the first source that starts.
    if control.on(&control.system) {
        system.capture = start_audio(false, control);
        system.failed = system.capture.is_none();
    }
    let audio_spec = system.capture.as_ref().map(|c| c.spec).unwrap_or_else(|| {
        if control.on(&control.microphone) {
            microphone.capture = start_audio(true, control);
            microphone.failed = microphone.capture.is_none();
        }
        microphone.capture.as_ref().map_or(DEFAULT_AUDIO, |c| c.spec)
    });
    let mut webcam: Option<Webcam> = None;
    let mut webcam_failed = false;
    let mut writer = Writer::create(path, spec, Some(audio_spec))?;
    let mut grabber = Grabber::new(rect).map_err(|e| e.to_string())?;
    let frame_time = HNS_PER_SECOND / spec.fps as i64;
    let rate = audio_spec.sample_rate as i64;
    let mut gate = options.noise_gate.then(|| NoiseGate::new(rate as u32));
    let mut mixer: Option<Mixer> = None;
    let mut clicks = Clicks::default();
    let started = Instant::now();
    let mut paused_for = Duration::ZERO;
    let mut paused_at: Option<Instant> = None;
    let mut written = 0i64;
    let mut audio_frames = 0i64;
    let mut frame = Frame::new(spec.width, spec.height);
    let drain = |c: &Option<AudioCapture>| {
        if let Some(c) = c {
            while c.samples.try_recv().is_ok() {}
        }
    };
    // Finer sleeps while recording (the default tick is 15.6 ms).
    // SAFETY: paired with timeEndPeriod below.
    unsafe { windows_sys::Win32::Media::timeBeginPeriod(1) };
    while !control.stop.load(Ordering::Acquire) {
        let now = Instant::now();
        // The toolbar's switches.
        system.follow(&control.system, audio_spec, control);
        microphone.follow(&control.microphone, audio_spec, control);
        match (system.capture.is_some() && microphone.capture.is_some(), mixer.is_some()) {
            (true, false) => mixer = Some(Mixer::default()),
            (false, true) => mixer = None,
            _ => {}
        }
        if control.on(&control.webcam) && webcam.is_none() && !webcam_failed {
            match Webcam::start() {
                Ok(w) => webcam = Some(w),
                Err(error) => {
                    control.note(format!("no webcam: {error}"));
                    control.webcam.store(false, Ordering::Relaxed);
                    webcam_failed = true;
                }
            }
        } else if !control.on(&control.webcam) && webcam.is_some() {
            webcam = None;
        }
        if control.paused.load(Ordering::Acquire) {
            paused_at.get_or_insert(now);
            drain(&system.capture);
            drain(&microphone.capture);
            std::thread::sleep(Duration::from_millis(15));
            continue;
        }
        if let Some(at) = paused_at.take() {
            paused_for += now - at;
        }
        let t = ((now - started - paused_for).as_nanos() / 100) as i64;
        control.millis.store((t / 10_000) as u64, Ordering::Relaxed);
        // Audio first: it arrives continuously, and silence is filled in so
        // the soundtrack keeps pace with the frames.
        {
            let mut out: Vec<Vec<i16>> = Vec::new();
            if let Some(m) = &microphone.capture {
                for mut block in m.samples.try_iter() {
                    if let Some(g) = gate.as_mut() {
                        g.process(&mut block);
                    }
                    match mixer.as_mut() {
                        Some(mx) => mx.push_b(&block),
                        None => out.push(block),
                    }
                }
            }
            if let Some(s) = &system.capture {
                for block in s.samples.try_iter() {
                    match mixer.as_mut() {
                        Some(mx) => mx.push_a(&block),
                        None => out.push(block),
                    }
                }
            }
            if let Some(mx) = mixer.as_mut() {
                out.push(mx.take((rate / 10) as usize));
            }
            for block in out {
                let n = (block.len() / 2) as i64;
                writer.write_audio(&block, audio_frames * HNS_PER_SECOND / rate, n * HNS_PER_SECOND / rate)?;
                audio_frames += n;
            }
            let expected = t * rate / HNS_PER_SECOND;
            if expected - audio_frames > rate / 5 {
                let gap = (expected - audio_frames) as usize;
                writer.write_audio(&media::silence(gap), audio_frames * HNS_PER_SECOND / rate, gap as i64 * HNS_PER_SECOND / rate)?;
                audio_frames += gap as i64;
            }
        }
        // A constant frame rate: frame n shows at n * frame_time. When a
        // grab takes longer than a frame, the frame is repeated, so the
        // video keeps the audio's pace.
        let due = t / frame_time;
        if due < written {
            std::thread::sleep(Duration::from_millis((((written * frame_time) - t) / 10_000).clamp(1, 5) as u64));
            continue;
        }
        let pixels = grabber.grab().map_err(|e| e.to_string())?;
        frame.pixels.copy_from_slice(pixels);
        // Screen Draw's window is left out of captures; its marks go in here.
        screendraw::blend_into(&mut frame);
        let pointer = cursor_in(rect);
        if let (true, Some(p)) = (control.on(&control.highlight), pointer) {
            vfx::highlight(&mut frame, p);
        }
        if let Some(cursor) = CursorSnapshot::take().filter(|_| control.on(&control.cursor)) {
            if rect.contains(cursor.x, cursor.y) {
                let _ = cursor.draw_onto(&mut frame, (rect.x, rect.y));
            }
        }
        if control.on(&control.clicks) {
            clicks.update(pointer.unwrap_or((-1000, -1000)));
            clicks.draw(&mut frame);
        }
        if let Some(cam) = &webcam {
            if let Some((w, h, bgra)) = cam.latest.lock().unwrap().as_ref() {
                let corner = control.webcam_corner.lock().unwrap().clone();
                vfx::picture_in_picture(&mut frame, bgra, (*w, *h), &corner, options.webcam_size, control.on(&control.webcam_circle));
            }
        }
        while written <= due {
            writer.write_frame(&frame.pixels, written * frame_time, frame_time)?;
            written += 1;
        }
        control.frames.fetch_add(1, Ordering::Relaxed);
    }
    // SAFETY: pairs with timeBeginPeriod above.
    unsafe { windows_sys::Win32::Media::timeEndPeriod(1) };
    writer.finish()?;
    Ok(spec)
}

fn clock(ms: u64) -> String {
    format!("{}:{:02}", ms / 60_000, ms / 1000 % 60)
}

/// Counts down next to `rect`; false when cancelled.
fn countdown(rect: Rect, seconds: u32) -> bool {
    if seconds == 0 {
        return true;
    }
    let start = Instant::now();
    let total = Duration::from_secs(seconds as u64);
    let choice = hud::run_panel(rect, &format!("Recording starts in {seconds}…"), &["Start now", "Cancel"], 100, move || {
        let left = total.saturating_sub(start.elapsed());
        if left.is_zero() {
            hud::finish_panel(Some(0));
            return None;
        }
        Some(format!("Recording starts in {}…  (Esc cancels)", left.as_secs() + 1))
    });
    choice == Some(0)
}

/// The toolbar's state, as the app draws it.
fn toolbar_state(control: &Control, settings: &Settings) -> serde_json::Value {
    let on = |f: &AtomicBool| f.load(Ordering::Relaxed);
    json!({
        "millis": control.millis.load(Ordering::Relaxed),
        "frames": control.frames.load(Ordering::Relaxed),
        "paused": on(&control.paused),
        "cursor": on(&control.cursor),
        "highlight": on(&control.highlight),
        "clicks": on(&control.clicks),
        "webcam": on(&control.webcam),
        "webcam_corner": control.webcam_corner.lock().unwrap().clone(),
        "webcam_circle": on(&control.webcam_circle),
        "system": on(&control.system),
        "microphone": on(&control.microphone),
        "draw": screendraw::active(),
        "draw_offered": settings.screen_draw,
        "note": control.audio.lock().unwrap().clone(),
        "keys": { "pause": key_names().0, "stop": key_names().1 },
    })
}

/// Handles a toolbar action that does not end the take.
fn toolbar_action(action: &str, control: &Control, rect: Rect, fade: Duration) {
    let flip = |f: &AtomicBool| {
        f.fetch_xor(true, Ordering::AcqRel);
    };
    match action.split_once(':').unwrap_or((action, "")) {
        ("pause", _) => flip(&control.paused),
        ("draw", _) if screendraw::active() => screendraw::hide(),
        ("draw", _) => screendraw::show(rect, fade),
        ("toggle", "cursor") => flip(&control.cursor),
        ("toggle", "highlight") => flip(&control.highlight),
        ("toggle", "clicks") => flip(&control.clicks),
        ("toggle", "webcam") => flip(&control.webcam),
        ("toggle", "system") => flip(&control.system),
        ("toggle", "microphone") => flip(&control.microphone),
        ("corner", c) if matches!(c, "top-left" | "top-right" | "bottom-left" | "bottom-right") => {
            *control.webcam_corner.lock().unwrap() = c.to_owned();
            control.webcam.store(true, Ordering::Relaxed);
        }
        ("shape", shape) => control.webcam_circle.store(shape != "rectangle", Ordering::Relaxed),
        _ => {}
    }
}

/// Records `rect` with the border and the toolbar; returns the file, or
/// `None` when cancelled. Restart throws the take away and starts again.
/// The switches as they were at the end are remembered for next time.
pub fn record(rect: Rect, settings: &Settings) -> Result<Option<PathBuf>, String> {
    let library = Library::new(jiffshot_capture::config::library_root());
    let _border = Border::show(rect);
    if !countdown(rect, settings.countdown) {
        return Ok(None);
    }
    let options = Options::from_settings(settings);
    let fade = Duration::from_secs(settings.draw_fade_seconds as u64);
    let mut control = Arc::new(Control::new(&options));
    loop {
        let path = library.recording_path().map_err(|e| e.to_string())?;
        let (c, p, o) = (control.clone(), path.clone(), options.clone());
        let worker = std::thread::spawn(move || {
            let result = record_to(&p, rect, &o, &c);
            if let Err(error) = &result {
                *c.error.lock().unwrap() = Some(error.clone());
                c.stop.store(true, Ordering::Release);
            }
            result
        });
        // The pause and stop keys work wherever the focus is.
        let (pause, stop) = check_keys();
        for (key, best, what) in [(pause, PAUSE_KEYS[0].2, "pauses"), (stop, STOP_KEYS[0].2, "stops")] {
            match key {
                Some((.., name)) if name != best => log(&format!("{best} is held by another app; {name} {what} the recording")),
                None => log(&format!("{best} and its alternatives are held by other apps; only the toolbar {what} the recording")),
                _ => {}
            }
        }
        let keys: Vec<(u32, u16, &str)> =
            [(pause, "pause"), (stop, "stop")].into_iter().filter_map(|(key, action)| key.map(|(modifiers, vk, _)| (modifiers, vk, action))).collect();
        let ended = hud::run_toolbar(
            rect,
            &keys,
            100,
            || {
                if control.stop.load(Ordering::Acquire) {
                    // The recorder stopped by itself (an error).
                    hud::finish_panel(Some(0));
                }
                toolbar_state(&control, settings)
            },
            |action| match action {
                "stop" | "cancel" | "restart" => true,
                other => {
                    toolbar_action(other, &control, rect, fade);
                    false
                }
            },
        );
        screendraw::hide();
        control.stop.store(true, Ordering::Release);
        let result = worker.join().map_err(|_| "the recorder crashed".to_owned())?;
        crate::tray::remember_recording(&control);
        match ended.as_str() {
            "restart" if result.is_ok() => {
                let _ = std::fs::remove_file(&path);
                log("recording restarted");
                control = Arc::new(control.again());
                continue;
            }
            "cancel" => {
                let _ = std::fs::remove_file(&path);
                return Ok(None);
            }
            _ => {}
        }
        let spec = result?;
        log(&format!(
            "recorded {}x{} at {} fps: {} frames, {} ({})",
            spec.width,
            spec.height,
            spec.fps,
            control.frames.load(Ordering::Relaxed),
            clock(control.millis.load(Ordering::Relaxed)),
            path.display()
        ));
        return Ok(Some(path));
    }
}
