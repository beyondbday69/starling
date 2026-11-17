//! HoldMyItems: First-person visible arms, hands, and two-handed grips for held items.

use horizon_mod_abi::{
    ABI, ButtonSink, HostVTable, HoldMyItems, ModDesc, guard, text,
};
use std::ffi::c_char;
use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, Ordering};

static HOST: AtomicPtr<HostVTable> = AtomicPtr::new(std::ptr::null_mut());

fn host() -> Option<&'static HostVTable> {
    let ptr = HOST.load(Ordering::Relaxed);
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { &*ptr })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum GripStyle {
    Default,
    Raised,
    Lowered,
    Forward,
}

impl GripStyle {
    fn from_index(idx: usize) -> Self {
        match idx {
            1 => Self::Raised,
            2 => Self::Lowered,
            3 => Self::Forward,
            _ => Self::Default,
        }
    }

    fn to_index(self) -> usize {
        match self {
            Self::Default => 0,
            Self::Raised => 1,
            Self::Lowered => 2,
            Self::Forward => 3,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Raised => "Raised (+0.05)",
            Self::Lowered => "Lowered (-0.05)",
            Self::Forward => "Forward (+0.08)",
        }
    }

    fn offsets(self) -> (f32, f32, f32) {
        match self {
            Self::Default => (0.0, 0.0, 0.0),
            Self::Raised => (0.0, 0.05, 0.0),
            Self::Lowered => (0.0, -0.05, 0.0),
            Self::Forward => (0.0, 0.0, 0.08),
        }
    }
}

struct State {
    enabled: bool,
    two_handed: bool,
    offhand_arm: bool,
    grip_style: GripStyle,
    arm_bobbing: bool,
}

impl State {
    fn new() -> Self {
        Self {
            enabled: true,
            two_handed: true,
            offhand_arm: true,
            grip_style: GripStyle::Default,
            arm_bobbing: true,
        }
    }

    fn load(&mut self) {
        let Some(host) = host() else { return };
        let mut buf = [0; horizon_mod_abi::TEXT];

        if unsafe { (host.setting)(c"enabled".as_ptr(), buf.as_mut_ptr(), buf.len()) } > 0 {
            if let Some(val) = unsafe { text(buf.as_ptr()) } {
                self.enabled = val != "0";
            }
        }
        if unsafe { (host.setting)(c"two_handed".as_ptr(), buf.as_mut_ptr(), buf.len()) } > 0 {
            if let Some(val) = unsafe { text(buf.as_ptr()) } {
                self.two_handed = val != "0";
            }
        }
        if unsafe { (host.setting)(c"offhand_arm".as_ptr(), buf.as_mut_ptr(), buf.len()) } > 0 {
            if let Some(val) = unsafe { text(buf.as_ptr()) } {
                self.offhand_arm = val != "0";
            }
        }
        if unsafe { (host.setting)(c"grip_style".as_ptr(), buf.as_mut_ptr(), buf.len()) } > 0 {
            if let Some(val) = unsafe { text(buf.as_ptr()) } {
                if let Ok(idx) = val.parse::<usize>() {
                    self.grip_style = GripStyle::from_index(idx);
                }
            }
        }
        if unsafe { (host.setting)(c"arm_bobbing".as_ptr(), buf.as_mut_ptr(), buf.len()) } > 0 {
            if let Some(val) = unsafe { text(buf.as_ptr()) } {
                self.arm_bobbing = val != "0";
            }
        }
    }

    fn save(&self) {
        let Some(host) = host() else { return };
        let s_enabled = if self.enabled { c"1" } else { c"0" };
        let s_two_handed = if self.two_handed { c"1" } else { c"0" };
        let s_offhand = if self.offhand_arm { c"1" } else { c"0" };
        let s_grip = match self.grip_style {
            GripStyle::Default => c"0",
            GripStyle::Raised => c"1",
            GripStyle::Lowered => c"2",
            GripStyle::Forward => c"3",
        };
        let s_bob = if self.arm_bobbing { c"1" } else { c"0" };

        unsafe {
            (host.set_setting)(c"enabled".as_ptr(), s_enabled.as_ptr());
            (host.set_setting)(c"two_handed".as_ptr(), s_two_handed.as_ptr());
            (host.set_setting)(c"offhand_arm".as_ptr(), s_offhand.as_ptr());
            (host.set_setting)(c"grip_style".as_ptr(), s_grip.as_ptr());
            (host.set_setting)(c"arm_bobbing".as_ptr(), s_bob.as_ptr());
        }
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    let mut lock = STATE.lock().ok()?;
    let state = lock.get_or_insert_with(State::new);
    Some(f(state))
}

unsafe extern "C" fn init(table: *const HostVTable) -> bool {
    let _ = guard(|| {
        if !table.is_null() {
            HOST.store(table.cast_mut(), Ordering::Relaxed);
        }
        with_state(|s| s.load());
    });
    true
}

unsafe extern "C" fn hold_my_items(_partial: f32, out: *mut HoldMyItems) {
    let _ = guard(|| {
        if out.is_null() {
            return;
        }
        let config = unsafe { &mut *out };
        with_state(|state| {
            if !state.enabled {
                config.active = 0;
                return;
            }
            config.active = 1;
            config.show_main_arm = 1;
            config.show_off_arm = if state.offhand_arm { 1 } else { 0 };
            config.two_handed = if state.two_handed { 1 } else { 0 };

            let (ox, oy, oz) = state.grip_style.offsets();
            config.offset_x = ox;
            config.offset_y = oy;
            config.offset_z = oz;
            config.pitch = 0.0;
            config.yaw = 0.0;
            config.roll = 0.0;
        });
    });
}

unsafe extern "C" fn settings_rows(sink: *const ButtonSink) {
    let _ = guard(|| {
        if sink.is_null() {
            return;
        }
        let sink = unsafe { &*sink };

        with_state(|state| {
            let row0 = format!(
                "Hold My Items: {}",
                if state.enabled { "ON" } else { "OFF" }
            );
            let row1 = format!(
                "Two-Handed Stance: {}",
                if state.two_handed { "ON" } else { "OFF" }
            );
            let row2 = format!(
                "Offhand Arm: {}",
                if state.offhand_arm { "ON" } else { "OFF" }
            );
            let row3 = format!("Grip Placement: {}", state.grip_style.name());
            let row4 = format!(
                "Arm Bobbing: {}",
                if state.arm_bobbing { "ON" } else { "OFF" }
            );

            for row in &[row0, row1, row2, row3, row4] {
                let mut c_str = row.as_bytes().to_vec();
                c_str.push(0);
                unsafe { (sink.push)(sink.ctx, c_str.as_ptr() as *const c_char) };
            }
        });
    });
}

unsafe extern "C" fn settings_click(button: u32) {
    let _ = guard(|| {
        with_state(|state| {
            match button {
                0 => state.enabled = !state.enabled,
                1 => state.two_handed = !state.two_handed,
                2 => state.offhand_arm = !state.offhand_arm,
                3 => {
                    let next = (state.grip_style.to_index() + 1) % 4;
                    state.grip_style = GripStyle::from_index(next);
                }
                4 => state.arm_bobbing = !state.arm_bobbing,
                _ => {}
            }
            state.save();
        });
    });
}

static ID: &[u8] = b"holdmyitems\0";
static TITLE: &[u8] = b"Hold My Items 1.0\0";
static DESCRIPTION: &[u8] =
    b"Visible first-person arms, hands, and two-handed grips for held items and tools.\0";

static DESC: ModDesc = ModDesc {
    abi: ABI,
    api_level: 7,
    id: ID.as_ptr() as *const c_char,
    title: TITLE.as_ptr() as *const c_char,
    description: DESCRIPTION.as_ptr() as *const c_char,
    init: Some(init),
    tick: None,
    frame: None,
    settings: Some(settings_rows),
    settings_click: Some(settings_click),
    camera_offset: None,
    player_eyes: None,
    hold_my_items: Some(hold_my_items),
};

/// The entry point looked up by `horizon-mod-loader`.
///
/// # Safety
/// `table` is the client's table, valid for the process's life.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn horizon_mod_entry(table: *const HostVTable) -> *const ModDesc {
    if !table.is_null() {
        HOST.store(table.cast_mut(), Ordering::Relaxed);
    }
    with_state(|s| s.load());
    &DESC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grip_style_cycling() {
        for i in 0..4 {
            let style = GripStyle::from_index(i);
            assert_eq!(style.to_index(), i);
            assert!(!style.name().is_empty());
        }
        assert_eq!(GripStyle::from_index(99), GripStyle::Default);
    }

    #[test]
    fn test_hold_my_items_callback() {
        let mut config = HoldMyItems::default();
        unsafe { hold_my_items(0.5, &mut config) };
        assert_eq!(config.active, 1);
        assert_eq!(config.show_main_arm, 1);
    }
}
