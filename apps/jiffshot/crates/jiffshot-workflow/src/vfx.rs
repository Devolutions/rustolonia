//! What the recorder adds to frames and sound: a highlight around the
//! pointer, rings where it clicks, the webcam in a corner, a microphone
//! noise gate and the mix of system audio with the microphone.

use jiffshot_capture::Frame;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};

/// Blends `color` (BGR) at `alpha` (0..1) into a pixel.
fn blend(px: &mut [u8], color: [u8; 3], alpha: f32) {
    for c in 0..3 {
        px[c] = (px[c] as f32 * (1.0 - alpha) + color[c] as f32 * alpha).round() as u8;
    }
}

/// Paints the pixels within `outer` of `center` (and beyond `inner`), with
/// a soft one-pixel edge.
fn disc(frame: &mut Frame, center: (f32, f32), inner: f32, outer: f32, color: [u8; 3], alpha: f32) {
    let (w, h) = (frame.width as i32, frame.height as i32);
    let stride = frame.stride();
    let x0 = ((center.0 - outer - 1.0).floor() as i32).max(0);
    let x1 = ((center.0 + outer + 1.0).ceil() as i32).min(w);
    let y0 = ((center.1 - outer - 1.0).floor() as i32).max(0);
    let y1 = ((center.1 + outer + 1.0).ceil() as i32).min(h);
    for y in y0..y1 {
        for x in x0..x1 {
            let d = ((x as f32 + 0.5 - center.0).powi(2) + (y as f32 + 0.5 - center.1).powi(2)).sqrt();
            let cover = (outer + 0.5 - d).clamp(0.0, 1.0) * (d - inner + 0.5).clamp(0.0, 1.0);
            if cover > 0.0 {
                let i = y as usize * stride + x as usize * 4;
                blend(&mut frame.pixels[i..i + 4], color, alpha * cover);
            }
        }
    }
}

/// A translucent yellow disc under the pointer (`at` in frame pixels).
pub fn highlight(frame: &mut Frame, at: (i32, i32)) {
    disc(frame, (at.0 as f32, at.1 as f32), 0.0, 26.0, [0x2E, 0xE6, 0xFF], 0.35);
}

/// Watches the mouse buttons and draws an expanding ring for each click.
#[derive(Default)]
pub struct Clicks {
    down: (bool, bool),
    rings: Vec<((i32, i32), bool, Instant)>,
}

const RING_TIME: Duration = Duration::from_millis(450);

impl Clicks {
    /// Call once per frame; `at` is the pointer in frame pixels.
    pub fn update(&mut self, at: (i32, i32)) {
        // SAFETY: plain key-state queries.
        let (left, right) = unsafe { (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000 != 0, GetAsyncKeyState(VK_RBUTTON as i32) as u16 & 0x8000 != 0) };
        if left && !self.down.0 {
            self.rings.push((at, false, Instant::now()));
        }
        if right && !self.down.1 {
            self.rings.push((at, true, Instant::now()));
        }
        self.down = (left, right);
        self.rings.retain(|r| r.2.elapsed() < RING_TIME);
    }

    pub fn draw(&self, frame: &mut Frame) {
        for (at, right, when) in &self.rings {
            let t = when.elapsed().as_secs_f32() / RING_TIME.as_secs_f32();
            let r = 10.0 + 26.0 * t;
            // Left clicks blue, right clicks orange.
            let color = if *right { [0x0B, 0x9E, 0xF5] } else { [0xD9, 0x6F, 0x1E] };
            disc(frame, (at.0 as f32, at.1 as f32), r - 3.0, r, color, 0.9 * (1.0 - t));
        }
    }
}

/// Puts `cam` (BGRA, `cw` x `ch`) into a corner of `frame`: `size` is the
/// share of the frame's height, `corner` is "top-left", "top-right",
/// "bottom-left" or "bottom-right", and `circle` crops it round.
pub fn picture_in_picture(frame: &mut Frame, cam: &[u8], (cw, ch): (u32, u32), corner: &str, size: f32, circle: bool) {
    if cw == 0 || ch == 0 {
        return;
    }
    let (fw, fh) = (frame.width as i32, frame.height as i32);
    let h = ((fh as f32 * size.clamp(0.1, 0.6)) as i32).max(32);
    let w = if circle { h } else { (h as f32 * cw as f32 / ch as f32) as i32 };
    let margin = 16;
    let x0 = if corner.ends_with("left") { margin } else { fw - w - margin };
    let y0 = if corner.starts_with("top") { margin } else { fh - h - margin };
    // Circles show the middle square of the camera picture.
    let (sx0, sw) = if circle { ((cw.saturating_sub(ch)) / 2, cw.min(ch)) } else { (0, cw) };
    let (cx, cy, radius) = (w as f32 / 2.0, h as f32 / 2.0, w.min(h) as f32 / 2.0);
    let stride = frame.stride();
    for y in 0..h {
        let fy = y0 + y;
        if fy < 0 || fy >= fh {
            continue;
        }
        let sy = (y as u32 * ch / h as u32).min(ch - 1);
        for x in 0..w {
            let fx = x0 + x;
            if fx < 0 || fx >= fw {
                continue;
            }
            let mut cover = 1.0;
            let mut border = 0.0;
            if circle {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                cover = (radius + 0.5 - d).clamp(0.0, 1.0);
                border = (d - (radius - 3.0)).clamp(0.0, 1.0);
            } else if x < 2 || y < 2 || x >= w - 2 || y >= h - 2 {
                border = 1.0;
            }
            if cover <= 0.0 {
                continue;
            }
            let sx = sx0 + (x as u32 * sw / w as u32).min(sw - 1);
            let s = (sy * cw + sx) as usize * 4;
            let i = fy as usize * stride + fx as usize * 4;
            let src = [cam[s], cam[s + 1], cam[s + 2]];
            let color = if border > 0.0 { [0xFF, 0xFF, 0xFF] } else { src };
            blend(&mut frame.pixels[i..i + 4], color, cover);
        }
    }
}

/// Silences the microphone between words: below the threshold for longer
/// than the hold time, the sound fades out; above it, it comes straight
/// back.
pub struct NoiseGate {
    gain: f32,
    quiet_for: usize,
    rate: usize,
}

impl NoiseGate {
    pub fn new(rate: u32) -> NoiseGate {
        NoiseGate { gain: 1.0, quiet_for: 0, rate: rate as usize }
    }

    /// Processes interleaved stereo in place.
    pub fn process(&mut self, pcm: &mut [i16]) {
        const THRESHOLD: f32 = 0.012;
        let hold = self.rate / 5;
        let release = 1.0 / (self.rate as f32 * 0.08);
        // Decide per 10 ms window, apply per sample.
        let window = (self.rate / 100).max(1) * 2;
        for chunk in pcm.chunks_mut(window) {
            let rms = (chunk.iter().map(|s| (*s as f32 / 32768.0).powi(2)).sum::<f32>() / chunk.len() as f32).sqrt();
            if rms >= THRESHOLD {
                self.quiet_for = 0;
            } else {
                self.quiet_for += chunk.len() / 2;
            }
            let open = self.quiet_for < hold;
            for frame in chunk.chunks_mut(2) {
                self.gain = if open { 1.0 } else { (self.gain - release).max(0.0) };
                for s in frame {
                    *s = (*s as f32 * self.gain) as i16;
                }
            }
        }
    }
}

/// Mixes two stereo streams that arrive in blocks of any size; a stream
/// that falls behind (loopback sends nothing during silence) is padded
/// with silence.
#[derive(Default)]
pub struct Mixer {
    a: VecDeque<i16>,
    b: VecDeque<i16>,
}

impl Mixer {
    pub fn push_a(&mut self, pcm: &[i16]) {
        self.a.extend(pcm);
    }

    pub fn push_b(&mut self, pcm: &[i16]) {
        self.b.extend(pcm);
    }

    /// Mixed samples ready now. `behind` (in stereo frames): pad the
    /// shorter stream when the other is ahead by more than that.
    pub fn take(&mut self, behind: usize) -> Vec<i16> {
        let (la, lb) = (self.a.len(), self.b.len());
        if la > lb + behind * 2 {
            self.b.extend(std::iter::repeat_n(0, la - lb));
        } else if lb > la + behind * 2 {
            self.a.extend(std::iter::repeat_n(0, lb - la));
        }
        let n = self.a.len().min(self.b.len()) & !1;
        self.a.drain(..n).zip(self.b.drain(..n)).map(|(x, y)| (x as i32 + y as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_pads_the_silent_stream() {
        let mut m = Mixer::default();
        m.push_a(&[100; 2000]);
        assert!(m.take(2000).is_empty(), "waits while within the allowance");
        let out = m.take(100);
        assert_eq!(out.len(), 2000);
        assert!(out.iter().all(|s| *s == 100));
        m.push_a(&[20_000; 4]);
        m.push_b(&[20_000; 4]);
        assert_eq!(m.take(100), vec![i16::MAX; 4], "clips instead of wrapping");
    }

    #[test]
    fn gate_closes_on_hiss_and_opens_on_speech() {
        let mut g = NoiseGate::new(48_000);
        let mut hiss: Vec<i16> = (0..48_000 * 2).map(|i| if i % 2 == 0 { 60 } else { -60 }).collect();
        g.process(&mut hiss);
        assert!(hiss[hiss.len() - 2..].iter().all(|s| *s == 0), "hiss is gated after the hold");
        let mut speech: Vec<i16> = (0..4800).map(|i| ((i as f32 * 0.05).sin() * 8000.0) as i16).collect();
        let before = speech.clone();
        g.process(&mut speech);
        assert_eq!(speech[100], before[100], "speech passes at once");
    }

    #[test]
    fn webcam_circle_lands_in_the_corner() {
        let mut frame = Frame::new(400, 300);
        let cam = vec![0x80u8; 64 * 48 * 4];
        picture_in_picture(&mut frame, &cam, (64, 48), "bottom-right", 0.3, true);
        let px = |x: usize, y: usize| frame.pixels[(y * 400 + x) * 4];
        // 90 px circle, 16 px from the bottom-right corner: center (339, 239).
        assert_eq!(px(339, 239), 0x80);
        assert_eq!(px(295, 195), 0, "outside the circle stays");
        assert_eq!(px(10, 10), 0);
    }
}
