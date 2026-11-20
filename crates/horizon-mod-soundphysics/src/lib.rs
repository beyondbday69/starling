//! Sound Physics as a mod the client loads at runtime: a block between you and a sound makes it
//! quieter, and a room gives it something to answer back with.
//!
//! The model is Sound Physics Remastered's (Sonic Ether, henkelmax, GPL-3.0), ported to Rust in
//! [`engine`]: the ray directions, the block material tables, the occlusion and reflection maths.
//! What this file adds is the world it runs on: every question the model asks about a block
//! (`block_name`, `face_is_sturdy`, `is_water`, `raycast`) is answered by the client through the
//! host table, and the number it arrives at is handed back with `set_sound`.
//!
//! Only the direct path is applied today: the client can change a playing sound's volume, not yet
//! its reverb sends or its low-pass, so `direct_gain` is the whole of what a player hears. The
//! sends are computed and logged all the same, so the model can be read before it is wired in.

mod engine;

use engine::{
    Config, Environment, RateLimiter, RayHit, SoundWorld, blocks as materials, evaluate,
    is_ambient_sound,
};
use horizon_mod_abi::{ABI, ButtonSink, HostVTable, ModDesc, guard};
use std::ffi::{CStr, c_char};
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// The host, once the client has handed it over. Every call out goes through this.
static HOST: AtomicPtr<HostVTable> = AtomicPtr::new(std::ptr::null_mut());
/// How many sounds have been shaped, for the settings rows.
static SHAPED: AtomicUsize = AtomicUsize::new(0);
/// The last gain applied, in thousandths, so a row can show it without a lock.
static LAST: AtomicU32 = AtomicU32::new(1000);
/// Sounds the client no longer had when the gain was set, so a row can show why nothing changed.
static MISSED: AtomicUsize = AtomicUsize::new(0);
/// Sounds seen but skipped before any maths (relative, too far, ambient).
static SKIPPED: AtomicUsize = AtomicUsize::new(0);

/// What the player chose.
struct Settings {
    on: bool,
    /// How much of the model's occlusion to apply, as a percentage.
    strength: u32,
    /// Occlusion exaggerated, so a difference is unmistakable.
    loud_test: bool,
    /// A summary line in the log every this many sounds (0: never).
    log_every: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            on: true,
            strength: 100,
            loud_test: false,
            log_every: 32,
        }
    }
}

fn settings() -> &'static Mutex<Settings> {
    static SETTINGS: OnceLock<Mutex<Settings>> = OnceLock::new();
    SETTINGS.get_or_init(|| Mutex::new(Settings::default()))
}

/// The client, as this mod sees it.
struct Client {
    host: *const HostVTable,
}

impl Client {
    fn table(&self) -> &HostVTable {
        // SAFETY: the table outlives every callback: the client builds it once and never moves it.
        unsafe { &*self.host }
    }
}

/// The world, as the model reads it: each answer a call to the client.
struct World<'a> {
    client: &'a Client,
    config: &'a Config,
}

impl World<'_> {
    fn name(&self, pos: [i32; 3]) -> Option<String> {
        let table = self.client.table();
        let mut buffer = [0 as c_char; horizon_mod_abi::TEXT];
        // SAFETY: the buffer is `TEXT` bytes and the client is told its length.
        let len = unsafe {
            (table.block_name)(pos[0], pos[1], pos[2], buffer.as_mut_ptr(), buffer.len())
        };
        if len == 0 {
            return None;
        }
        // SAFETY: the client wrote `len` bytes and a NUL.
        Some(
            unsafe { CStr::from_ptr(buffer.as_ptr()) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

impl SoundWorld for World<'_> {
    fn block_occlusion(&self, pos: [i32; 3]) -> f64 {
        self.name(pos)
            .map_or(1.0, |name| materials::occlusion(&name, self.config))
    }

    fn block_reflectivity(&self, pos: [i32; 3]) -> f64 {
        self.name(pos)
            .map_or(0.0, |name| materials::reflectivity(&name, self.config))
    }

    fn face_is_sturdy(&self, pos: [i32; 3], side: usize) -> bool {
        let table = self.client.table();
        // SAFETY: a plain query; the client keeps no pointer.
        unsafe { (table.face_is_sturdy)(pos[0], pos[1], pos[2], side as u32) == 1 }
    }

    fn is_water(&self, pos: [i32; 3]) -> bool {
        let table = self.client.table();
        // SAFETY: as above.
        unsafe { (table.is_water)(pos[0], pos[1], pos[2]) == 1 }
    }

    fn raycast(&self, from: [f64; 3], to: [f64; 3], _ignore: Option<[i32; 3]>) -> RayHit {
        let table = self.client.table();
        let mut out = horizon_mod_abi::RayHitOut::default();
        // SAFETY: both points and the out-parameter are this frame's own values.
        let hit = unsafe { (table.raycast)(&from, &to, &mut out) } == 1;
        if hit {
            RayHit {
                hit: true,
                at: out.at,
                block: out.block,
                normal: out.normal,
            }
        } else {
            RayHit::miss(to)
        }
    }
}

/// A sound the client just started, as this mod was told about it.
struct Started {
    instance: u64,
    id: String,
    at: [f64; 3],
}

fn started(client: &Client) -> Vec<Started> {
    let table = client.table();
    let mut out = [horizon_mod_abi::StartedSound {
        instance: 0,
        id: std::ptr::null(),
        has_at: 0,
        at: [0.0; 3],
    }; 64];
    // SAFETY: the array is 64 entries and the client is told so.
    let count = unsafe { (table.sounds_started)(out.as_mut_ptr(), out.len()) };
    out[..count.min(out.len())]
        .iter()
        .filter(|sound| sound.has_at == 1 && !sound.id.is_null())
        .map(|sound| Started {
            instance: sound.instance,
            // SAFETY: the client keeps each id alive for the length of this call.
            id: unsafe { CStr::from_ptr(sound.id) }
                .to_string_lossy()
                .into_owned(),
            at: sound.at,
        })
        .collect()
}

/// Shapes the sounds that started since the last frame, as `SoundPhysics.onSoundPlay` does.
fn shape(client: &Client) {
    let (on, strength, loud_test, log_every) = {
        let settings = settings().lock().expect("settings");
        (
            settings.on,
            settings.strength,
            settings.loud_test,
            settings.log_every,
        )
    };
    if !on {
        return;
    }
    let table = client.table();
    let mut eye = [0.0; 3];
    // SAFETY: one out-parameter, this frame's own value.
    if unsafe { (table.listener)(&mut eye) } == 0 {
        return;
    }
    let config = Config::default();
    let world = World {
        client,
        config: &config,
    };
    let listener_underwater = world.is_water([
        eye[0].floor() as i32,
        eye[1].floor() as i32,
        eye[2].floor() as i32,
    ]);
    let mut limiter = RateLimiter::default();
    for sound in started(client) {
        if is_ambient_sound(&sound.id) && !config.evaluate_ambient_sounds {
            continue;
        }
        let distance = (0..3)
            .map(|i| (sound.at[i] - eye[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        if distance > config.max_sound_processing_distance {
            SKIPPED.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        if limiter.over_limit(&sound.id, &config) {
            SKIPPED.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        let environment: Environment = evaluate(
            &world,
            sound.at,
            eye,
            listener_underwater,
            &config,
            1.0,
            16.0,
            false,
        );
        // The model's occlusion is 1 for a clear path; a test run exaggerates it so the
        // difference cannot be missed.
        let model = f64::from(environment.direct_gain).clamp(0.0, 1.0);
        let occluded = 1.0 - model;
        let scaled = occluded * f64::from(strength) / 100.0 * if loud_test { 3.0 } else { 1.0 };
        // Never quite silence: a sound a player cannot hear at all reads as a bug, not as a wall.
        let gain = (1.0 - scaled).clamp(0.15, 1.0);
        // A wall takes the high frequencies first; a room answers through the sends.
        let cutoff = f64::from(environment.direct_cutoff).clamp(0.0, 1.0);
        let send = environment
            .sends
            .iter()
            .map(|s| f64::from(s.gain))
            .fold(0.0, f64::max)
            .clamp(0.0, 1.0);
        // SAFETY: a plain call; the client owns the sound it names.
        let applied =
            unsafe { (table.set_sound)(sound.instance, gain as f32, cutoff as f32, send as f32) }
                == 1;
        let shaped = SHAPED.fetch_add(1, Ordering::Relaxed) + 1;
        LAST.store((gain * 1000.0) as u32, Ordering::Relaxed);
        if !applied {
            MISSED.fetch_add(1, Ordering::Relaxed);
        }
        let plain = shaped <= 12 || (log_every > 0 && shaped % log_every as usize == 0);
        if plain {
            log(
                client,
                2,
                &format!(
                    "{} at {:.2}x (occluded {:.0}%, cutoff {:.2}, reverb {:.2}, {}): {}",
                    sound.id,
                    gain,
                    occluded * 100.0,
                    cutoff,
                    send,
                    if applied { "applied" } else { "missed" },
                    format_args!("{shaped} shaped"),
                ),
            );
        }
    }
}

fn log(client: &Client, level: u32, text: &str) {
    let table = client.table();
    // SAFETY: the text is NUL-free (Rust strings are) and outlives the call.
    let text = std::ffi::CString::new(text).unwrap_or_default();
    unsafe { (table.log)(level, text.as_ptr()) };
}

/// The rows shown in Options > Mods.
unsafe extern "C" fn settings_rows(sink: *const ButtonSink) {
    let (on, strength, loud_test, log_every) = {
        let settings = settings().lock().expect("settings");
        (
            settings.on,
            settings.strength,
            settings.loud_test,
            settings.log_every,
        )
    };
    let shaped = SHAPED.load(Ordering::Relaxed);
    let missed = MISSED.load(Ordering::Relaxed);
    let skipped = SKIPPED.load(Ordering::Relaxed);
    let last = f32::from(LAST.load(Ordering::Relaxed) as u16) / 1000.0;
    let rows = [
        format!("Sound Physics: {}", if on { "On" } else { "Off" }),
        format!("Occlusion Strength: {strength}%"),
        format!("Test Mode (3x): {}", if loud_test { "On" } else { "Off" }),
        format!("Log every: {log_every} sounds"),
        format!("Sounds processed: {shaped}"),
        format!(
            "Applied: {} missed: {}",
            shaped.saturating_sub(missed),
            missed
        ),
        format!("Skipped (relative/far): {skipped}"),
        format!("Last sound: {last:.2}x"),
    ];
    for row in rows {
        // SAFETY: the sink belongs to the client and is valid for this call.
        unsafe { push(sink, &row) };
    }
}

unsafe extern "C" fn settings_click(button: u32) {
    let mut settings = settings().lock().expect("settings");
    match button {
        0 => settings.on = !settings.on,
        1 => {
            settings.strength = match settings.strength {
                100 => 150,
                150 => 200,
                _ => 100,
            }
        }
        2 => settings.loud_test = !settings.loud_test,
        3 => {
            settings.log_every = match settings.log_every {
                32 => 8,
                8 => 0,
                _ => 32,
            }
        }
        _ => {}
    }
}

unsafe extern "C" fn init(_host: *const HostVTable) -> bool {
    let host = HOST.load(Ordering::Relaxed);
    // SAFETY: the client built this table once and keeps it for the process's life.
    let Some(table) = (unsafe { host.as_ref() }) else {
        return false;
    };
    if table.abi != ABI {
        return false;
    }
    let client = Client { host };
    log(&client, 2, "sound physics: started, abi agrees");
    true
}

unsafe extern "C" fn frame(_partial: f32) {
    let host = HOST.load(Ordering::Relaxed);
    if host.is_null() {
        return;
    }
    let client = Client { host };
    let _ = guard(|| shape(&client));
}

/// Writes one row into the client's list. Not `extern "C"`: it never crosses the boundary.
unsafe fn push(sink: *const ButtonSink, text: &str) {
    let Some(sink) = (unsafe { sink.as_ref() }) else {
        return;
    };
    let text = std::ffi::CString::new(text).unwrap_or_default();
    // SAFETY: the client's sink writes the text into its own list during this call.
    unsafe { (sink.push)(sink.ctx, text.as_ptr()) };
}

static DESC: ModDesc = ModDesc {
    abi: ABI,
    api_level: 7,
    id: c"soundphysics".as_ptr(),
    title: c"Sound Physics".as_ptr(),
    description: c"Blocks swallow sound and rooms answer it, as Sound Physics Remastered does."
        .as_ptr(),
    init: Some(init),
    tick: None,
    frame: Some(frame),
    settings: Some(settings_rows),
    settings_click: Some(settings_click),
    camera_offset: None,
    player_eyes: None,
};

/// The one symbol the client looks up.
///
/// # Safety
/// `host` is the client's table, valid and unmoved for the process's life; the pointer handed back
/// points at a `static` table that outlives every callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn horizon_mod_entry(host: *const HostVTable) -> *const ModDesc {
    HOST.store(host.cast_mut(), Ordering::Relaxed);
    &DESC
}

// The mixer answers through the client: this file only decides what to ask for.
