//! Zero-allocation rolling performance counters for water v4 stages.
//!
//! The profiler is owned entirely by the caller: it contains no globals, no
//! `Mutex`, no `unsafe`, and never allocates. A `Counter` records timing marks
//! (`begin` / `end`, nanoseconds relative to its own origin) plus a rolling
//! average and rolling maximum duration in milliseconds.
//!
//! `Profiler` groups the three hot stages (sim, render, particle). Both types
//! can render a short textual `report()` into a fixed inline byte buffer; the
//! returned `&str` borrows that buffer and stays valid until the next call.
//!
//! Usage:
//! ```ignore
//! let mut p = Profiler::new();
//! p.sim.begin_at(t0_ns);
//! p.sim.end_at(t1_ns);
//! // or, with the built-in clock:
//! p.render.start();
//! // ... render work ...
//! p.render.stop();
//! let text = p.report(); // borrows p.report_buf
//! ```
//!
//! For maximum control, `push_ns` / `push_ms` record a sample that the caller
//! measured itself, which is handy in tests and in code that already owns a
//! clock.

use std::fmt;
use std::fmt::Write as _;
use std::time::Instant;

/// Number of samples kept in each rolling window.
pub const WINDOW: usize = 32;

/// Capacity of a single counter's report buffer, in bytes.
pub const COUNTER_REPORT_CAP: usize = 72;

/// Capacity of the combined profiler report buffer, in bytes.
pub const PROFILER_REPORT_CAP: usize = 256;

/// The three stages measured by [`Profiler`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Sim,
    Render,
    Particle,
}

impl Stage {
    /// All stages in report order.
    pub const ALL: [Stage; 3] = [Stage::Sim, Stage::Render, Stage::Particle];

    /// Stable short name used in reports.
    #[inline]
    pub fn name(self) -> &'static str {
        match self {
            Stage::Sim => "sim",
            Stage::Render => "render",
            Stage::Particle => "particle",
        }
    }

    /// Index into [`Profiler::counter`] style arrays.
    #[inline]
    pub fn index(self) -> usize {
        match self {
            Stage::Sim => 0,
            Stage::Render => 1,
            Stage::Particle => 2,
        }
    }
}

/// A fixed-capacity, allocation-free formatter target.
struct Buf<'a> {
    bytes: &'a mut [u8],
    len: usize,
}

impl<'a> Buf<'a> {
    #[inline]
    fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, len: 0 }
    }

    /// Consume the buffer and return the ASCII text written so far.
    #[inline]
    fn finish(self) -> &'a str {
        let Self { bytes, len } = self;
        // Only ASCII is written by the formatters below, so this cannot fail.
        core::str::from_utf8(&bytes[..len]).unwrap_or("")
    }
}

impl fmt::Write for Buf<'_> {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let free = self.bytes.len() - self.len;
        let take = s.len().min(free);
        self.bytes[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

/// Format a single counter's summary into `out` without borrowing the counter.
#[inline]
fn format_counter(out: &mut [u8], avg: f32, max: f32, n: usize) -> &str {
    let mut w = Buf::new(out);
    let _ = write!(w, "avg={avg:.3}ms max={max:.3}ms n={n}");
    w.finish()
}

/// Rolling timing counter for one pipeline stage.
///
/// `begin` and `end` are nanosecond marks from `origin` (set at construction),
/// so they are comparable for a given counter. `avg_ms` and `max_ms` describe
/// the most recent [`WINDOW`] completed samples.
pub struct Counter {
    /// Nanosecond mark of the most recent `start` / `begin_at` call.
    pub begin: u64,
    /// Nanosecond mark of the most recent `stop` / `end_at` call.
    pub end: u64,
    /// Rolling average sample duration in milliseconds.
    pub avg_ms: f32,
    /// Rolling maximum sample duration in milliseconds.
    pub max_ms: f32,

    origin: Instant,
    samples: [u32; WINDOW],
    head: usize,
    count: usize,
    window: usize,
    sum_ns: u64,
    report_buf: [u8; COUNTER_REPORT_CAP],
}

impl Default for Counter {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Counter {
    /// Create a counter with the default [`WINDOW`]-sample window.
    #[inline]
    pub fn new() -> Self {
        Self::with_window(WINDOW)
    }

    /// Create a counter with a custom window, clamped to `1..=WINDOW`.
    #[inline]
    pub fn with_window(window: usize) -> Self {
        Self {
            begin: 0,
            end: 0,
            avg_ms: 0.0,
            max_ms: 0.0,
            origin: Instant::now(),
            samples: [0; WINDOW],
            head: 0,
            count: 0,
            window: window.clamp(1, WINDOW),
            sum_ns: 0,
            report_buf: [0; COUNTER_REPORT_CAP],
        }
    }

    /// Nanoseconds elapsed since this counter's origin.
    #[inline]
    pub fn now_ns(&self) -> u64 {
        self.origin.elapsed().as_nanos() as u64
    }

    /// Open a timing interval using the built-in clock.
    #[inline]
    pub fn start(&mut self) {
        self.begin = self.now_ns();
    }

    /// Close the interval and record its duration.
    #[inline]
    pub fn stop(&mut self) {
        let now = self.now_ns();
        self.end_at(now);
    }

    /// Open a timing interval using a caller-supplied nanosecond mark.
    #[inline]
    pub fn begin_at(&mut self, now_ns: u64) {
        self.begin = now_ns;
    }

    /// Close the interval using a caller-supplied mark and record its duration.
    #[inline]
    pub fn end_at(&mut self, now_ns: u64) {
        self.end = now_ns;
        self.push_ns(now_ns.saturating_sub(self.begin));
    }

    /// Record a duration measured by the caller, in nanoseconds.
    #[inline]
    pub fn push_ns(&mut self, elapsed_ns: u64) {
        let ns = elapsed_ns.min(u32::MAX as u64) as u32;
        self.push_sample(ns);
    }

    /// Record a duration measured by the caller, in milliseconds.
    ///
    /// Non-finite or negative values are ignored so a bad sample cannot poison
    /// the rolling statistics.
    #[inline]
    pub fn push_ms(&mut self, ms: f32) {
        if !ms.is_finite() || ms <= 0.0 {
            return;
        }
        let ns = (ms * 1_000_000.0).min(u32::MAX as f32) as u64;
        self.push_sample(ns.min(u32::MAX as u64) as u32);
    }

    #[inline]
    fn push_sample(&mut self, ns: u32) {
        if self.count == self.window {
            let old = self.samples[self.head] as u64;
            self.sum_ns -= old;
        } else {
            self.count += 1;
        }
        self.samples[self.head] = ns;
        self.head = (self.head + 1) % self.window;
        self.sum_ns += ns as u64;

        let count = self.count;
        self.avg_ms = (self.sum_ns as f64 / count as f64 / 1_000_000.0) as f32;
        // The window is at most 32 entries; a linear scan is cheap and keeps
        // the maximum exact even after the oldest sample is evicted.
        let mut max = 0u32;
        for &s in &self.samples[..count] {
            if s > max {
                max = s;
            }
        }
        self.max_ms = (max as f64 / 1_000_000.0) as f32;
    }

    /// Number of samples currently in the rolling window.
    #[inline]
    pub fn samples(&self) -> usize {
        self.count
    }

    /// Clear all timing marks and statistics (keeps the configured window).
    #[inline]
    pub fn reset(&mut self) {
        self.begin = 0;
        self.end = 0;
        self.avg_ms = 0.0;
        self.max_ms = 0.0;
        self.samples = [0; WINDOW];
        self.head = 0;
        self.count = 0;
        self.sum_ns = 0;
    }

    /// Render this counter's summary into a caller-provided buffer.
    #[inline]
    pub fn report_into<'a>(&self, out: &'a mut [u8]) -> &'a str {
        format_counter(out, self.avg_ms, self.max_ms, self.count)
    }

    /// Render this counter's summary into its own fixed buffer.
    #[inline]
    pub fn report(&mut self) -> &str {
        let (avg, max, n) = (self.avg_ms, self.max_ms, self.count);
        format_counter(&mut self.report_buf, avg, max, n)
    }
}

/// Format all three stages into `out` without borrowing the profiler.
#[inline]
fn format_profiler(
    out: &mut [u8],
    s: (f32, f32, usize),
    r: (f32, f32, usize),
    p: (f32, f32, usize),
) -> &str {
    let mut w = Buf::new(out);
    let _ = write!(
        w,
        "water3d sim {:.3}/{:.3}ms({}) render {:.3}/{:.3}ms({}) particle {:.3}/{:.3}ms({})",
        s.0, s.1, s.2, r.0, r.1, r.2, p.0, p.1, p.2
    );
    w.finish()
}

/// Caller-owned set of stage counters, with a fixed inline report buffer.
pub struct Profiler {
    pub sim: Counter,
    pub render: Counter,
    pub particle: Counter,
    report_buf: [u8; PROFILER_REPORT_CAP],
}

impl Default for Profiler {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Profiler {
    /// Create a profiler with three default counters.
    #[inline]
    pub fn new() -> Self {
        Self {
            sim: Counter::new(),
            render: Counter::new(),
            particle: Counter::new(),
            report_buf: [0; PROFILER_REPORT_CAP],
        }
    }

    /// Create a profiler whose counters all use a custom window.
    #[inline]
    pub fn with_window(window: usize) -> Self {
        Self {
            sim: Counter::with_window(window),
            render: Counter::with_window(window),
            particle: Counter::with_window(window),
            report_buf: [0; PROFILER_REPORT_CAP],
        }
    }

    /// Shared access to a stage counter.
    #[inline]
    pub fn counter(&self, stage: Stage) -> &Counter {
        match stage {
            Stage::Sim => &self.sim,
            Stage::Render => &self.render,
            Stage::Particle => &self.particle,
        }
    }

    /// Mutable access to a stage counter, used to record samples.
    #[inline]
    pub fn counter_mut(&mut self, stage: Stage) -> &mut Counter {
        match stage {
            Stage::Sim => &mut self.sim,
            Stage::Render => &mut self.render,
            Stage::Particle => &mut self.particle,
        }
    }

    /// Reset all three counters.
    #[inline]
    pub fn reset(&mut self) {
        self.sim.reset();
        self.render.reset();
        self.particle.reset();
    }

    /// Render all three stages into a caller-provided buffer.
    #[inline]
    pub fn report_into<'a>(&self, out: &'a mut [u8]) -> &'a str {
        let s = (self.sim.avg_ms, self.sim.max_ms, self.sim.samples());
        let r = (self.render.avg_ms, self.render.max_ms, self.render.samples());
        let p = (self.particle.avg_ms, self.particle.max_ms, self.particle.samples());
        format_profiler(out, s, r, p)
    }

    /// Render all three stages into the profiler's fixed buffer.
    ///
    /// The returned string borrows `report_buf` and is overwritten on the next
    /// call, so callers should consume or copy it before profiling more work.
    #[inline]
    pub fn report(&mut self) -> &str {
        let s = (self.sim.avg_ms, self.sim.max_ms, self.sim.samples());
        let r = (self.render.avg_ms, self.render.max_ms, self.render.samples());
        let p = (self.particle.avg_ms, self.particle.max_ms, self.particle.samples());
        format_profiler(&mut self.report_buf, s, r, p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1.0e-4
    }

    #[test]
    fn rolling_average_and_max_evict_old_samples() {
        let mut c = Counter::with_window(4);
        for ms in [1.0f32, 2.0, 3.0, 4.0] {
            c.push_ms(ms);
        }
        assert_eq!(c.samples(), 4);
        assert!(close(c.avg_ms, 2.5), "avg {}", c.avg_ms);
        assert!(close(c.max_ms, 4.0), "max {}", c.max_ms);

        // Oldest sample (1ms) leaves; the maximum must be recalculated too.
        c.push_ms(8.0);
        assert_eq!(c.samples(), 4);
        assert!(close(c.avg_ms, 4.25), "rolling avg {}", c.avg_ms);
        assert!(close(c.max_ms, 8.0), "rolling max {}", c.max_ms);

        // Feed the window entirely with small samples; the 8ms max decays.
        for _ in 0..4 {
            c.push_ms(0.25);
        }
        assert!(close(c.avg_ms, 0.25), "decayed avg {}", c.avg_ms);
        assert!(close(c.max_ms, 0.25), "decayed max {}", c.max_ms);

        // Invalid samples cannot poison the statistics.
        c.push_ms(f32::NAN);
        c.push_ms(-1.0);
        assert_eq!(c.samples(), 4);
        assert!(close(c.avg_ms, 0.25));
        assert!(close(c.max_ms, 0.25));

        c.reset();
        assert_eq!(c.samples(), 0);
        assert_eq!(c.avg_ms, 0.0);
        assert_eq!(c.max_ms, 0.0);
    }

    #[test]
    fn begin_end_marks_and_fixed_buffer_report() {
        let mut c = Counter::new();
        c.begin_at(1_000);
        c.end_at(3_500_000);
        assert_eq!(c.begin, 1_000);
        assert_eq!(c.end, 3_500_000);
        assert!(close(c.avg_ms, 3.499), "avg {}", c.avg_ms);
        assert!(close(c.max_ms, 3.499), "max {}", c.max_ms);

        let first = c.report().to_owned();
        assert!(first.contains("avg="));
        assert!(first.contains("max="));
        assert!(first.contains("n=1"));
        // Reuse of the fixed buffer yields the same text without allocation.
        assert_eq!(c.report(), first.as_str());

        // A tiny caller buffer truncates safely and remains valid UTF-8.
        let mut tiny = [0u8; 4];
        assert_eq!(c.report_into(&mut tiny), "avg=");
    }

    #[test]
    fn profiler_routes_all_three_stages() {
        let mut p = Profiler::with_window(2);
        p.counter_mut(Stage::Sim).push_ms(1.0);
        p.counter_mut(Stage::Render).push_ms(5.0);
        p.counter_mut(Stage::Particle).push_ms(0.5);

        assert_eq!(p.sim.samples(), 1);
        assert_eq!(p.render.samples(), 1);
        assert_eq!(p.particle.samples(), 1);
        assert!(close(p.counter(Stage::Render).avg_ms, 5.0));
        assert_eq!(Stage::Sim.name(), "sim");
        assert_eq!(Stage::Particle.index(), 2);

        let report = p.report().to_owned();
        assert!(report.contains("sim 1.000/1.000ms"));
        assert!(report.contains("render 5.000/5.000ms"));
        assert!(report.contains("particle 0.500/0.500ms"));
        assert_eq!(p.report(), report.as_str());

        p.reset();
        assert_eq!(p.render.samples(), 0);
    }
}
