//! Video in the editor: scrubbing and playback (frames shown on the
//! canvas), trimming, export to MP4 or animated GIF, and taking a frame
//! into the image editor. Decoding and encoding use Media Foundation
//! through `jiffshot_capture::media`.

use crate::app::{self, App};
use crate::canvas::with_editor;
use jiffshot_capture::media::{Reader, VideoFrame, VideoSpec, Writer, HNS_PER_SECOND};
use jiffshot_doc::{raster, Document, Pixmap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn hns(seconds: f64) -> i64 {
    (seconds * HNS_PER_SECOND as f64) as i64
}

fn seconds(hns: i64) -> f64 {
    hns as f64 / HNS_PER_SECOND as f64
}

/// A decoded frame as an opaque pixmap.
pub fn to_pixmap(frame: &VideoFrame) -> Option<Pixmap> {
    let mut rgba = Vec::with_capacity(frame.bgra.len());
    for p in frame.bgra.chunks_exact(4) {
        rgba.extend_from_slice(&[p[2], p[1], p[0], 255]);
    }
    Pixmap::from_vec(rgba, tiny_skia::IntSize::from_wh(frame.width, frame.height)?)
}

struct Player {
    stop: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<()>,
    sound: Option<std::thread::JoinHandle<()>>,
}

pub struct Session {
    path: PathBuf,
    reader: Reader,
    position: f64,
    trim: (f64, f64),
    player: Option<Player>,
    current: Option<Pixmap>,
    /// Playback volume as 0..=0xFFFF, read by the sound thread.
    volume: Arc<AtomicU32>,
}

impl Session {
    pub fn open(path: &Path) -> Result<Session, String> {
        let reader = Reader::open(path)?;
        let duration = seconds(reader.duration);
        Ok(Session { path: path.to_path_buf(), reader, position: 0.0, trim: (0.0, duration), player: None, current: None, volume: Arc::new(AtomicU32::new(0xFFFF)) })
    }

    pub fn duration(&self) -> f64 {
        seconds(self.reader.duration)
    }

    pub fn info(&self) -> String {
        format!(
            "{} × {} · {:.0} fps · {}",
            self.reader.width,
            self.reader.height,
            self.reader.fps,
            if self.reader.audio.is_some() { "with audio" } else { "no audio" }
        )
    }

    pub fn frame_at(&mut self, at: f64) -> Option<Pixmap> {
        let frame = self.reader.frame_at(hns(at)).ok().flatten()?;
        let pm = to_pixmap(&frame);
        self.current = pm.clone();
        pm
    }

    fn stop_playing(&mut self) -> bool {
        match self.player.take() {
            Some(p) => {
                p.stop.store(true, Ordering::Release);
                let _ = p.thread.join();
                if let Some(s) = p.sound {
                    let _ = s.join();
                }
                true
            }
            None => false,
        }
    }

    pub fn stop(mut self) {
        self.stop_playing();
    }
}

/// Puts a frame on the canvas, keeping the view.
fn show(app: &mut App, pm: Pixmap) {
    with_editor(|e| e.load(Document::new(pm)));
    app.bump_frame();
}

fn publish(app: &mut App) {
    let Some(v) = &app.video else { return };
    let (position, trim, playing) = (v.position, v.trim, v.player.is_some());
    if let Some(sink) = &app.sink {
        let _ = sink.set_video_position(position);
        let _ = sink.set_trim_start(trim.0);
        let _ = sink.set_trim_end(trim.1);
        let _ = sink.set_playing(playing);
    }
}

/// The slider moved.
pub fn seek(app: &mut App, at: f64) {
    let Some(v) = app.video.as_mut() else { return };
    if (v.position - at).abs() < 1e-3 {
        return;
    }
    v.stop_playing();
    v.position = at.clamp(0.0, v.duration());
    let at = v.position;
    if let Some(pm) = v.frame_at(at) {
        show(app, pm);
    }
    publish(app);
}

fn play(app: &mut App) {
    let Some(v) = app.video.as_mut() else { return };
    if v.stop_playing() {
        return publish(app);
    }
    let (path, mut from, end) = (v.path.clone(), v.position, v.trim.1);
    if from >= end - 0.05 {
        from = v.trim.0;
    }
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let sound = v.reader.audio.is_some().then(|| {
        let (path, stop, volume) = (path.clone(), stop.clone(), v.volume.clone());
        std::thread::spawn(move || sound::play(&path, from, end, &stop, &volume))
    });
    // Frames go to the UI thread one at a time; if it is still showing the
    // last one, the next is dropped rather than queued.
    let showing = Arc::new(AtomicBool::new(false));
    let thread = std::thread::spawn(move || {
        let Ok(mut reader) = Reader::open(&path) else { return };
        if reader.seek(hns(from)).is_err() {
            return;
        }
        let started = Instant::now();
        while !flag.load(Ordering::Acquire) {
            let Ok(Some(frame)) = reader.next_video() else { break };
            let t = seconds(frame.time);
            if t < from - 0.02 {
                continue;
            }
            if t > end {
                break;
            }
            let wait = (t - from) - started.elapsed().as_secs_f64();
            if wait > 0.0 {
                std::thread::sleep(Duration::from_secs_f64(wait.min(0.5)));
            }
            if showing.swap(true, Ordering::AcqRel) {
                continue;
            }
            let Some(pm) = to_pixmap(&frame) else { break };
            let done = showing.clone();
            app::post(move |app| {
                if let Some(v) = app.video.as_mut() {
                    v.position = t;
                    v.current = Some(pm.clone());
                    show(app, pm);
                    if let Some(sink) = &app.sink {
                        let _ = sink.set_video_position(t);
                    }
                }
                done.store(false, Ordering::Release);
            });
        }
        app::post(|app| {
            if let Some(v) = app.video.as_mut() {
                if let Some(p) = v.player.take() {
                    p.stop.store(true, Ordering::Release);
                }
            }
            publish(app);
        });
    });
    v.player = Some(Player { stop, thread, sound });
    publish(app);
}

/// Fits a frame into `w` x `h` (BGRA), scaled to fit and centered on
/// black; a frame of that size is copied as it is.
fn fit_frame(frame: &VideoFrame, w: usize, h: usize) -> Vec<u8> {
    let mut out = vec![0u8; w * h * 4];
    let (fw, fh) = (frame.width as usize, frame.height as usize);
    if fw >= w && fh >= h && fw - w < 2 && fh - h < 2 {
        // Even sizes for H.264: drop an odd last row or column.
        for y in 0..h {
            let src = y * fw * 4;
            out[y * w * 4..(y + 1) * w * 4].copy_from_slice(&frame.bgra[src..src + w * 4]);
        }
        return out;
    }
    if let Some(pm) = to_pixmap(frame) {
        paste_fitted(&pm, &mut out, w, h);
    }
    out
}

/// Draws `pm` scaled to fit and centered into the BGRA buffer `out`.
fn paste_fitted(pm: &Pixmap, out: &mut [u8], w: usize, h: usize) {
    let scale = (w as f64 / pm.width() as f64).min(h as f64 / pm.height() as f64);
    let sw = ((pm.width() as f64 * scale).round() as u32).clamp(1, w as u32);
    let sh = ((pm.height() as f64 * scale).round() as u32).clamp(1, h as u32);
    let small = if (sw, sh) == (pm.width(), pm.height()) { Some(pm.clone()) } else { raster::resample(pm, sw, sh) };
    let Some(small) = small else { return };
    let (x0, y0) = ((w - sw as usize) / 2, (h - sh as usize) / 2);
    for (i, p) in small.pixels().iter().enumerate() {
        let (x, y) = (i % sw as usize, i / sw as usize);
        // Premultiplied, so this is the picture over black.
        let d = ((y0 + y) * w + x0 + x) * 4;
        out[d..d + 4].copy_from_slice(&[p.blue(), p.green(), p.red(), 0]);
    }
}

/// One piece of a new video: `(file, from, to)` in seconds.
pub type Segment = (PathBuf, f64, f64);

/// Writes the segments one after another to a new MP4 (re-encoded) at the
/// first one's size; others are fitted in. Audio is kept when `audio` is
/// set and every source has the same sound format.
fn export_segments(segments: &[Segment], out: &Path, audio: bool, progress: impl Fn(f64)) -> Result<(), String> {
    let first = Reader::open(&segments.first().ok_or("nothing to export")?.0)?;
    let spec = VideoSpec::for_size(first.width, first.height, first.fps.round().max(1.0) as u32);
    let usable = |a: Option<jiffshot_capture::media::AudioSpec>| a.filter(|a| (a.sample_rate == 44_100 || a.sample_rate == 48_000) && a.channels <= 2);
    let mut audio_spec = if audio { usable(first.audio) } else { None };
    drop(first);
    for (path, ..) in &segments[1..] {
        if audio_spec.is_some() && usable(Reader::open(path)?.audio) != audio_spec {
            audio_spec = None;
        }
    }
    let mut writer = Writer::create(out, spec, audio_spec)?;
    let frame_time = HNS_PER_SECOND / spec.fps as i64;
    let rate = audio_spec.map_or(48_000, |a| a.sample_rate) as i64;
    let channels = audio_spec.map_or(2, |a| a.channels) as i64;
    let (w, h) = (spec.width as usize, spec.height as usize);
    let total = segments.iter().map(|(_, a, b)| (b - a).max(0.0)).sum::<f64>().max(0.01);
    let mut base = 0i64;
    let mut done = 0.0;
    for (source, from, to) in segments {
        let mut video = Reader::open(source)?;
        let mut sound = match audio_spec {
            Some(_) => {
                let mut r = Reader::open(source)?;
                r.select_video(false)?;
                r.select_audio(true)?;
                r.seek(hns(*from))?;
                Some(r)
            }
            None => None,
        };
        let (start, end) = (hns(*from), hns(*to));
        video.seek(start)?;
        let mut pending_audio: Option<(i64, Vec<i16>)> = None;
        let mut audio_done = sound.is_none();
        while let Some(frame) = video.next_video()? {
            if frame.time + frame_time / 2 < start {
                continue;
            }
            if frame.time >= end {
                break;
            }
            writer.write_frame(&fit_frame(&frame, w, h), base + (frame.time - start).max(0), frame_time)?;
            // Audio up to this frame's time, from the second reader.
            while !audio_done {
                let block = match pending_audio.take() {
                    Some(b) => Some(b),
                    None => sound.as_mut().unwrap().next_audio()?,
                };
                let Some((at, pcm)) = block else {
                    audio_done = true;
                    break;
                };
                if at > frame.time + frame_time {
                    pending_audio = Some((at, pcm));
                    break;
                }
                if at >= start && at < end {
                    let n = pcm.len() as i64 / channels;
                    writer.write_audio(&pcm, base + at - start, n * HNS_PER_SECOND / rate)?;
                }
            }
            progress((done + seconds(frame.time - start)) / total);
        }
        base += end - start;
        done += to - from;
    }
    writer.finish()
}

/// Copies the trimmed range to a new MP4 (re-encoded, audio included).
fn export_mp4(source: &Path, out: &Path, (from, to): (f64, f64), progress: impl Fn(f64)) -> Result<(), String> {
    export_segments(&[(source.to_path_buf(), from, to)], out, true, progress)
}

/// GIF choices: frames a second, largest width, loop, fade the end to
/// black, dither.
#[derive(Clone, Copy, Debug)]
pub struct GifOptions {
    pub fps: f64,
    pub max_width: u32,
    pub looped: bool,
    pub fade: bool,
    pub dither: bool,
}

/// A 4x4 ordered-dither offset, so smooth gradients survive the 256-color
/// palette as fine patterns instead of bands.
fn dither(rgba: &mut image::RgbaImage) {
    const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    for (x, y, p) in rgba.enumerate_pixels_mut() {
        let d = (BAYER[(y % 4) as usize][(x % 4) as usize] - 8) * 2;
        for c in 0..3 {
            p.0[c] = (p.0[c] as i32 + d).clamp(0, 255) as u8;
        }
    }
}

/// Makes a video from pictures, each shown for its time in `seconds` (the
/// last time repeats for the rest) with a short cross-fade, at the size of
/// the largest (at most 1920 x 1080), with `sound` (16-bit PCM) as its
/// soundtrack.
fn slideshow(images: &[PathBuf], out: &Path, seconds: &[f64], sound: Option<(jiffshot_capture::media::AudioSpec, Vec<i16>)>, progress: impl Fn(f64)) -> Result<(), String> {
    let pictures: Vec<Pixmap> = images.iter().filter_map(|p| jiffshot_doc::io::load_image(p).ok()).collect();
    if pictures.is_empty() {
        return Err("none of the files is a picture".into());
    }
    let (w, h) = pictures.iter().fold((0u32, 0u32), |(w, h), p| (w.max(p.width()), h.max(p.height())));
    let fit = (1920.0 / w as f64).min(1080.0 / h as f64).min(1.0);
    let spec = VideoSpec::for_size(((w as f64 * fit) as u32).max(16), ((h as f64 * fit) as u32).max(16), 30);
    let (w, h) = (spec.width as usize, spec.height as usize);
    let slides: Vec<Vec<u8>> = pictures
        .iter()
        .map(|pm| {
            let mut buf = vec![0u8; w * h * 4];
            paste_fitted(pm, &mut buf, w, h);
            buf
        })
        .collect();
    let mut writer = Writer::create(out, spec, sound.as_ref().map(|s| s.0))?;
    let frames_for = |i: usize| (seconds.get(i).or(seconds.last()).copied().unwrap_or(3.0).max(0.5) * spec.fps as f64).round() as usize;
    let frame_time = HNS_PER_SECOND / spec.fps as i64;
    let total: usize = (0..slides.len()).map(frames_for).sum();
    if let Some((a, pcm)) = &sound {
        // The soundtrack, in tenth-of-a-second blocks, up to the video's end.
        let ch = a.channels.max(1) as usize;
        let block = a.sample_rate as usize / 10 * ch;
        let limit = (total as f64 / spec.fps as f64 * a.sample_rate as f64) as usize * ch;
        for (n, chunk) in pcm[..pcm.len().min(limit)].chunks(block).enumerate() {
            let at = n as i64 * HNS_PER_SECOND / 10;
            writer.write_audio(chunk, at, (chunk.len() / ch) as i64 * HNS_PER_SECOND / a.sample_rate as i64)?;
        }
    }
    let mut mixed = vec![0u8; w * h * 4];
    let (mut i, mut start) = (0usize, 0usize);
    for n in 0..total {
        while n >= start + frames_for(i) {
            start += frames_for(i);
            i += 1;
        }
        let (per, k) = (frames_for(i), n - start);
        let fade = (spec.fps as usize / 2).min(per / 2);
        let frame = match slides.get(i + 1) {
            Some(next) if k >= per - fade => {
                let t = (k + fade + 1 - per) as u32 * 256 / (fade as u32 + 1);
                for ((m, a), b) in mixed.iter_mut().zip(&slides[i]).zip(next) {
                    *m = ((*a as u32 * (256 - t) + *b as u32 * t) >> 8) as u8;
                }
                &mixed
            }
            _ => &slides[i],
        };
        writer.write_frame(frame, n as i64 * frame_time, frame_time)?;
        progress(n as f64 / total as f64);
    }
    writer.finish()
}

/// Writes the trimmed range as an animated GIF.
fn export_gif(source: &Path, out: &Path, (from, to): (f64, f64), options: GifOptions, progress: impl Fn(f64)) -> Result<(), String> {
    use image::codecs::gif::{GifEncoder, Repeat};
    let GifOptions { fps, max_width, .. } = options;
    let mut reader = Reader::open(source)?;
    let scale = (max_width as f64 / reader.width as f64).min(1.0);
    let (w, h) = (((reader.width as f64 * scale).round() as u32).max(1), ((reader.height as f64 * scale).round() as u32).max(1));
    let file = std::fs::File::create(out).map_err(|e| e.to_string())?;
    let mut encoder = GifEncoder::new_with_speed(std::io::BufWriter::new(file), 10);
    encoder.set_repeat(if options.looped { Repeat::Infinite } else { Repeat::Finite(0) }).map_err(|e| e.to_string())?;
    reader.seek(hns(from))?;
    let step = 1.0 / fps;
    let mut next = from;
    let delay = image::Delay::from_numer_denom_ms((1000.0 / fps).round() as u32, 1);
    while let Some(frame) = reader.next_video()? {
        let t = seconds(frame.time);
        if t > to {
            break;
        }
        if t + 1e-3 < next {
            continue;
        }
        next += step;
        let Some(pm) = to_pixmap(&frame) else { continue };
        let small = if scale < 1.0 { raster::resample(&pm, w, h).ok_or("resize failed")? } else { pm };
        let mut rgba = jiffshot_doc::io::to_rgba_image(&small);
        // The last half second fades to black.
        let left = to - t;
        if options.fade && left < 0.5 {
            let k = (left / 0.5).clamp(0.0, 1.0);
            for p in rgba.pixels_mut() {
                for c in 0..3 {
                    p.0[c] = (p.0[c] as f64 * k) as u8;
                }
            }
        }
        if options.dither {
            dither(&mut rgba);
        }
        encoder.encode_frame(image::Frame::from_parts(rgba, 0, 0, delay)).map_err(|e| e.to_string())?;
        progress((t - from) / (to - from).max(0.01));
    }
    Ok(())
}

/// Runs `work` (given a progress callback) off the UI thread, reporting
/// progress in the status bar; opens the result when `open` is set.
fn in_background(out: PathBuf, open: bool, work: impl FnOnce(&Path, &dyn Fn(f64)) -> Result<(), String> + Send + 'static) {
    std::thread::spawn(move || {
        let started = Instant::now();
        let last = std::sync::Mutex::new(Instant::now());
        let progress = |f: f64| {
            let mut l = last.lock().unwrap();
            if l.elapsed() > Duration::from_millis(400) {
                *l = Instant::now();
                let pct = (f * 100.0).clamp(0.0, 100.0);
                app::post(move |app| app.set_status(format!("Exporting… {pct:.0}%")));
            }
        };
        let result = work(&out, &progress);
        let secs = started.elapsed().as_secs_f64();
        app::post(move |app| match result {
            Ok(()) => {
                if open {
                    app.open_path(&out);
                }
                app.set_status(format!("Exported {} in {secs:.1} s", out.display()));
            }
            Err(error) => app.set_status(format!("Export failed: {error}")),
        });
    });
}

fn stem_of(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Recording".into())
}

fn run_export(app: &mut App, gif: bool) {
    let Some(v) = app.video.as_mut() else { return };
    v.stop_playing();
    let (source, trim) = (v.path.clone(), v.trim);
    let (kind, ext) = if gif { ("Animated GIF", "gif") } else { ("MP4 video", "mp4") };
    let name = format!("{} (selection).{ext}", stem_of(&source));
    let prefs = app.settings.video.clone();
    app.pick_save(if gif { "Export GIF" } else { "Save selection as" }, &name, &[(kind, ext)], move |app, out| {
        if out == source {
            return app.set_status("Choose a different file than the one being edited.");
        }
        app.set_status(format!("Exporting {}…", out.display()));
        let (fps, max_width) = prefs.gif();
        let options = GifOptions { fps, max_width, looped: prefs.gif_loop, fade: prefs.gif_fade, dither: prefs.gif_dither };
        in_background(out, false, move |out, progress| if gif { export_gif(&source, out, trim, options, progress) } else { export_mp4(&source, out, trim, progress) });
    });
}

/// Saves the video without the part between the trim marks.
fn cut_selection(app: &mut App) {
    let Some(v) = app.video.as_mut() else { return };
    v.stop_playing();
    let (source, (a, b), duration) = (v.path.clone(), v.trim, v.duration());
    if a <= 0.05 && b >= duration - 0.05 {
        return app.set_status("Set the start and end of the part to cut first.");
    }
    let segments: Vec<Segment> = [(source.clone(), 0.0, a), (source.clone(), b, duration)].into_iter().filter(|(_, x, y)| y - x > 0.05).collect();
    let name = format!("{} (cut).mp4", stem_of(&source));
    app.pick_save("Save the video without the selection", &name, &[("MP4 video", "mp4")], move |app, out| {
        if out == source {
            return app.set_status("Choose a different file than the one being edited.");
        }
        app.set_status(format!("Cutting {a:.1} s to {b:.1} s…"));
        in_background(out, true, move |out, progress| export_segments(&segments, out, true, progress));
    });
}

fn silent_copy(app: &mut App) {
    let Some(v) = app.video.as_mut() else { return };
    v.stop_playing();
    let (source, duration) = (v.path.clone(), v.duration());
    let name = format!("{} (silent).mp4", stem_of(&source));
    app.pick_save("Save a silent copy", &name, &[("MP4 video", "mp4")], move |app, out| {
        if out == source {
            return app.set_status("Choose a different file than the one being edited.");
        }
        in_background(out, false, move |out, progress| export_segments(&[(source, 0.0, duration)], out, false, progress));
    });
}

/// Joins `files` end to end, asking where to save.
pub fn combine_files(app: &mut App, files: Vec<PathBuf>) {
    let mut segments = Vec::new();
    for f in &files {
        match Reader::open(f) {
            Ok(r) => segments.push((f.clone(), 0.0, seconds(r.duration))),
            Err(error) => return app.set_status(format!("{}: {error}", f.display())),
        }
    }
    let name = format!("{} (combined).mp4", stem_of(&files[0]));
    app.pick_save("Save the combined video", &name, &[("MP4 video", "mp4")], move |app, out| {
        app.set_status(format!("Combining {} videos…", segments.len()));
        in_background(out, true, move |out, progress| export_segments(&segments, out, true, progress));
    });
}

/// Joins videos end to end (the open one first).
fn combine(app: &mut App) {
    let current = app.video.as_mut().map(|v| {
        v.stop_playing();
        v.path.clone()
    });
    app.pick_files("Videos to add at the end", true, "MP4 videos", &["mp4"], move |app, picked| {
        let mut files: Vec<PathBuf> = current.into_iter().collect();
        files.extend(picked);
        if files.len() < 2 {
            return app.set_status("Pick at least two videos to combine.");
        }
        combine_files(app, files);
    });
}

fn from_images(app: &mut App) {
    let seconds_each = app.settings.video.slide_seconds;
    app.pick_files("Pictures for the video", true, "Images", &["png", "jpg", "jpeg", "bmp", "gif", "webp"], move |app, images| {
        if images.is_empty() {
            return;
        }
        let name = format!("{} (slideshow).mp4", stem_of(&images[0]));
        app.pick_save("Save the video", &name, &[("MP4 video", "mp4")], move |app, out| {
            app.set_status(format!("Making a video of {} pictures…", images.len()));
            in_background(out, true, move |out, progress| slideshow(&images, out, &[seconds_each], None, progress));
        });
    });
}

/// A narration: the pictures shown one at a time in the editor while
/// the microphone records; Next moves on and marks the time.
pub struct Narration {
    images: Vec<PathBuf>,
    index: usize,
    started: Instant,
    marks: Vec<f64>,
    spec: jiffshot_capture::media::AudioSpec,
    sound: Arc<std::sync::Mutex<Vec<i16>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

fn show_slide(app: &mut App) {
    let Some(n) = &app.narration else { return };
    let (i, count) = (n.index, n.images.len());
    let path = n.images[i].clone();
    if let Ok(pm) = jiffshot_doc::io::load_image(&path) {
        with_editor(|e| e.load(Document::new(pm)));
        app.zoom("fit");
        app.bump_frame();
    }
    let last = i + 1 == count;
    app.set_status(format!("● Narrating picture {} of {count}: talk, then {}", i + 1, if last { "Finish" } else { "Next picture" }));
    app.publish_narration();
}

/// Starts a narrated video: pick pictures, then talk over each.
fn narrate(app: &mut App) {
    app.pick_files("Pictures to narrate", true, "Images", &["png", "jpg", "jpeg", "bmp", "gif", "webp"], move |app, images| {
        if images.is_empty() {
            return;
        }
        let capture = match jiffshot_capture::media::capture_audio(true) {
            Ok(c) => c,
            Err(error) => return app.set_status(format!("No microphone: {error}")),
        };
        let spec = capture.spec;
        let sound: Arc<std::sync::Mutex<Vec<i16>>> = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let (s, f) = (sound.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            while !f.load(Ordering::Acquire) {
                while let Ok(block) = capture.samples.try_recv() {
                    s.lock().unwrap().extend_from_slice(&block);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        app.stop_video();
        app.narration = Some(Narration { images, index: 0, started: Instant::now(), marks: Vec::new(), spec, sound, stop, thread: Some(thread) });
        show_slide(app);
    });
}

fn narrate_next(app: &mut App) {
    let Some(n) = app.narration.as_mut() else { return };
    n.marks.push(n.started.elapsed().as_secs_f64() - n.marks.iter().sum::<f64>());
    if n.index + 1 < n.images.len() {
        n.index += 1;
        show_slide(app);
    } else {
        narrate_finish(app);
    }
}

/// Stops recording and saves the narrated video.
fn narrate_finish(app: &mut App) {
    let Some(mut n) = app.narration.take() else { return };
    if n.marks.len() < n.images.len() {
        n.marks.push(n.started.elapsed().as_secs_f64() - n.marks.iter().sum::<f64>());
    }
    n.stop.store(true, Ordering::Release);
    if let Some(t) = n.thread.take() {
        let _ = t.join();
    }
    let pcm = std::mem::take(&mut *n.sound.lock().unwrap());
    // Pictures not reached are left out.
    let images: Vec<PathBuf> = n.images.into_iter().take(n.marks.len()).collect();
    let (marks, spec) = (n.marks, n.spec);
    app.publish_narration();
    let name = format!("{} (narrated).mp4", stem_of(&images[0]));
    app.pick_save("Save the narrated video", &name, &[("MP4 video", "mp4")], move |app, out| {
        app.set_status(format!("Making a narrated video of {} pictures…", images.len()));
        in_background(out, true, move |out, progress| slideshow(&images, out, &marks, Some((spec, pcm)), progress));
    });
}

/// Moves the playhead by `frames` frames.
fn step(app: &mut App, frames: f64) {
    let Some(v) = app.video.as_ref() else { return };
    let at = v.position + frames / v.reader.fps.max(1.0);
    seek(app, at);
}

/// Applies the saved volume to the open video.
pub fn apply_volume(app: &mut App) {
    let prefs = &app.settings.video;
    let level = if prefs.muted { 0.0 } else { prefs.volume.clamp(0.0, 1.0) };
    if let Some(v) = &app.video {
        v.volume.store((level * 65535.0) as u32, Ordering::Release);
    }
}

pub fn action(app: &mut App, what: &str) {
    match what {
        "record" => match crate::winutil::start_recording() {
            Ok(()) => app.set_status("Select the area to record…"),
            Err(error) => app.set_status(error),
        },
        "play" => play(app),
        "start" | "end" => {
            if let Some(v) = app.video.as_mut() {
                let p = v.position;
                if what == "start" {
                    v.trim = (p.min(v.trim.1 - 0.1).max(0.0), v.trim.1);
                } else {
                    v.trim = (v.trim.0, p.max(v.trim.0 + 0.1));
                }
                let (a, b) = v.trim;
                app.set_status(format!("Keeping {a:.1} s to {b:.1} s"));
                publish(app);
            }
        }
        "export-mp4" => run_export(app, false),
        "export-gif" => run_export(app, true),
        "cut" => cut_selection(app),
        "silent" => silent_copy(app),
        "combine" => combine(app),
        "slideshow" => from_images(app),
        "narrate" => narrate(app),
        "narrate-next" => narrate_next(app),
        "narrate-finish" => narrate_finish(app),
        "prev" => step(app, -1.0),
        "next" => step(app, 1.0),
        "mute" => {
            app.settings.video.muted = !app.settings.video.muted;
            app.settings.save();
            apply_volume(app);
            let state = if app.settings.video.muted { "Sound off" } else { "Sound on" };
            app.set_status(state);
        }
        "frame" => {
            let frame = app.video.as_mut().and_then(|v| {
                v.stop_playing();
                v.current.clone().or_else(|| {
                    let p = v.position;
                    v.frame_at(p)
                })
            });
            if let Some(pm) = frame {
                app.stop_video();
                app.load(Document::new(pm), None, None);
                app.set_status("The video frame is now an image; annotate and save it as usual.");
            }
        }
        _ => {}
    }
}

/// A library thumbnail: the frame half a second in.
pub fn thumbnail(path: &Path) -> Option<Pixmap> {
    let mut reader = Reader::open(path).ok()?;
    let at = (reader.duration / 2).min(HNS_PER_SECOND / 2);
    reader.frame_at(at).ok().flatten().as_ref().and_then(to_pixmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a short clip, then exports a trimmed MP4 and a GIF of it.
    #[test]
    fn trims_and_exports() {
        let dir = std::env::temp_dir().join(format!("jiffshot-video-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("clip.mp4");
        let spec = VideoSpec::for_size(160, 120, 10);
        let mut w = Writer::create(&src, spec, None).unwrap();
        for i in 0..30i64 {
            let shade = (i * 8) as u8;
            let frame: Vec<u8> = (0..160 * 120).flat_map(|_| [shade, 128, 255 - shade, 0]).collect();
            w.write_frame(&frame, i * HNS_PER_SECOND / 10, HNS_PER_SECOND / 10).unwrap();
        }
        w.finish().unwrap();
        let mp4 = dir.join("trimmed.mp4");
        export_mp4(&src, &mp4, (1.0, 2.0), |_| {}).unwrap();
        let r = Reader::open(&mp4).unwrap();
        assert!((seconds(r.duration) - 1.0).abs() < 0.25, "trimmed to about a second: {}", seconds(r.duration));
        let gif = dir.join("clip.gif");
        let options = GifOptions { fps: 5.0, max_width: 80, looped: true, fade: true, dither: true };
        export_gif(&src, &gif, (0.0, 3.0), options, |_| {}).unwrap();
        let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(std::fs::File::open(&gif).unwrap())).unwrap();
        let frames = image::AnimationDecoder::into_frames(decoder).collect_frames().unwrap();
        assert!((13..=16).contains(&frames.len()), "{} frames", frames.len());
        assert_eq!(frames[0].buffer().width(), 80);

        // Cut 1..2 s out of the 3 s clip, then join the result with the
        // original: about 2 s, then 5 s.
        let cut = dir.join("cut.mp4");
        export_segments(&[(src.clone(), 0.0, 1.0), (src.clone(), 2.0, 3.0)], &cut, true, |_| {}).unwrap();
        let mut r = Reader::open(&cut).unwrap();
        assert!((seconds(r.duration) - 2.0).abs() < 0.25, "cut: {}", seconds(r.duration));
        // Source frame 22 (shade 176, blue channel first) follows the join.
        let after = r.frame_at(hns(1.2)).unwrap().unwrap();
        assert!((after.bgra[0] as i32 - 176).abs() < 24, "the frame after the cut is from 2.2 s: {}", after.bgra[0]);
        let joined = dir.join("joined.mp4");
        export_segments(&[(cut.clone(), 0.0, 2.0), (src.clone(), 0.0, 3.0)], &joined, true, |_| {}).unwrap();
        let d = seconds(Reader::open(&joined).unwrap().duration);
        assert!((d - 5.0).abs() < 0.3, "joined: {d}");

        // Two pictures of different sizes, a second each.
        let (a, b) = (dir.join("a.png"), dir.join("b.png"));
        let mut pa = Pixmap::new(200, 100).unwrap();
        pa.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        pa.save_png(&a).unwrap();
        let mut pb = Pixmap::new(100, 100).unwrap();
        pb.fill(tiny_skia::Color::from_rgba8(0, 0, 255, 255));
        pb.save_png(&b).unwrap();
        let show = dir.join("show.mp4");
        slideshow(&[a.clone(), b.clone()], &show, &[1.0], None, |_| {}).unwrap();
        let mut r = Reader::open(&show).unwrap();
        assert_eq!((r.width, r.height), (200, 100));
        assert!((seconds(r.duration) - 2.0).abs() < 0.2, "{}", seconds(r.duration));
        let first = r.frame_at(hns(0.2)).unwrap().unwrap();
        assert!(first.bgra[2] > 200 && first.bgra[0] < 60, "red first: {:?}", &first.bgra[..4]);
        let second = r.frame_at(hns(1.6)).unwrap().unwrap();
        let mid = (50 * 200 + 100) * 4;
        assert!(second.bgra[mid] > 200 && second.bgra[mid + 2] < 60, "blue second: {:?}", &second.bgra[mid..mid + 4]);
        assert!(second.bgra[(50 * 200 + 10) * 4] < 30, "black bars beside the square picture");

        // Narrated: the first picture for 0.5 s, the second for 1.5 s,
        // with a soundtrack.
        let narrated = dir.join("narrated.mp4");
        let spec = jiffshot_capture::media::AudioSpec { sample_rate: 48_000, channels: 2 };
        let tone: Vec<i16> = (0..96_000).flat_map(|n| {
            let v = ((n as f32 / 48_000.0 * 440.0 * std::f32::consts::TAU).sin() * 8000.0) as i16;
            [v, v]
        }).collect();
        slideshow(&[a, b], &narrated, &[0.5, 1.5], Some((spec, tone)), |_| {}).unwrap();
        let mut r = Reader::open(&narrated).unwrap();
        assert!((seconds(r.duration) - 2.0).abs() < 0.2, "{}", seconds(r.duration));
        assert_eq!(r.audio.map(|a| a.sample_rate), Some(48_000));
        let at_one = r.frame_at(hns(1.0)).unwrap().unwrap();
        assert!(at_one.bgra[mid] > 200, "the second picture shows from 0.5 s");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Playback sound through the wave-out API.
mod sound {
    use super::{hns, seconds};
    use jiffshot_capture::media::Reader;
    use std::collections::VecDeque;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::time::Duration;
    use windows_sys::Win32::Media::Audio::{
        waveOutClose, waveOutOpen, waveOutPrepareHeader, waveOutReset, waveOutSetVolume, waveOutUnprepareHeader, waveOutWrite, CALLBACK_NULL, HWAVEOUT,
        WAVEFORMATEX, WAVEHDR, WAVE_MAPPER,
    };

    const WHDR_DONE: u32 = 1;

    /// A queued block: the header and the samples it points at, kept in
    /// place until the device is done with them.
    type Block = (Box<WAVEHDR>, Vec<i16>);

    /// Plays `path`'s sound from `from` to `end` seconds until `stop`.
    pub fn play(path: &Path, from: f64, end: f64, stop: &AtomicBool, volume: &AtomicU32) {
        let Ok(mut reader) = Reader::open(path) else { return };
        let Some(spec) = reader.audio else { return };
        if reader.select_video(false).is_err() || reader.select_audio(true).is_err() || reader.seek(hns(from)).is_err() {
            return;
        }
        let channels = spec.channels as u16;
        let format = WAVEFORMATEX {
            wFormatTag: 1,
            nChannels: channels,
            nSamplesPerSec: spec.sample_rate,
            nAvgBytesPerSec: spec.sample_rate * channels as u32 * 2,
            nBlockAlign: channels * 2,
            wBitsPerSample: 16,
            cbSize: 0,
        };
        let mut out: HWAVEOUT = std::ptr::null_mut();
        // SAFETY: the format outlives the call; no callback.
        if unsafe { waveOutOpen(&mut out, WAVE_MAPPER, &format, 0, 0, CALLBACK_NULL) } != 0 {
            return;
        }
        let header_size = std::mem::size_of::<WAVEHDR>() as u32;
        let mut queued: VecDeque<Block> = VecDeque::new();
        let release = |q: &mut VecDeque<Block>, all: bool| {
            while let Some((h, _)) = q.front_mut() {
                if !all && h.dwFlags & WHDR_DONE == 0 {
                    break;
                }
                // SAFETY: a header prepared on this device.
                unsafe { waveOutUnprepareHeader(out, &mut **h, header_size) };
                q.pop_front();
            }
        };
        let mut level = u32::MAX;
        while !stop.load(Ordering::Acquire) {
            let v = volume.load(Ordering::Acquire).min(0xFFFF);
            if v != level {
                level = v;
                // SAFETY: an open device.
                unsafe { waveOutSetVolume(out, v | (v << 16)) };
            }
            release(&mut queued, false);
            if queued.len() >= 8 {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            let Ok(Some((at, pcm))) = reader.next_audio() else { break };
            if seconds(at) < from - 0.05 {
                continue;
            }
            if seconds(at) > end {
                break;
            }
            // SAFETY: a zeroed header is valid before it is filled in.
            let mut header: Box<WAVEHDR> = Box::new(unsafe { std::mem::zeroed() });
            header.lpData = pcm.as_ptr() as *mut u8;
            header.dwBufferLength = (pcm.len() * 2) as u32;
            // SAFETY: the header and its samples stay in `queued` until
            // they are unprepared.
            let ok = unsafe { waveOutPrepareHeader(out, &mut *header, header_size) == 0 && waveOutWrite(out, &mut *header, header_size) == 0 };
            queued.push_back((header, pcm));
            if !ok {
                break;
            }
        }
        // Let what is queued finish playing, unless stopped.
        while !stop.load(Ordering::Acquire) && queued.iter().any(|(h, _)| h.dwFlags & WHDR_DONE == 0) {
            std::thread::sleep(Duration::from_millis(15));
        }
        // SAFETY: stops the device, frees what it held, then closes it.
        unsafe { waveOutReset(out) };
        release(&mut queued, true);
        unsafe { waveOutClose(out) };
    }
}
