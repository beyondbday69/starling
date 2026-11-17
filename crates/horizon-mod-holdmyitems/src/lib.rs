//! HoldMyItems 5.2.0: Ported from beyondbday69/Horizon-HoldMyItems
//! First-person hand & item physics, 3D block/item poses, spring-damper inertia,
//! per-weapon combat slashes, inspection, and hand lighting.

pub mod runtime;

use horizon_mod_abi::{
    ABI, ButtonSink, HoldMyItems, HostVTable, ItemInfo, ModDesc, PlayerInfo, TEXT, guard, text,
};
use runtime::{Config, HmiState, ItemFlags, LoadedConfig, PlayerFrameInput, UseKind};
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

fn classify_item(id: &str, count: u32) -> ItemFlags {
    if id.is_empty() || id == "minecraft:air" || count == 0 {
        return ItemFlags::empty();
    }
    let short = id.strip_prefix("minecraft:").unwrap_or(id);
    let is_block = !matches!(
        short,
        "diamond_sword"
            | "iron_sword"
            | "golden_sword"
            | "stone_sword"
            | "wooden_sword"
            | "netherite_sword"
            | "diamond_pickaxe"
            | "iron_pickaxe"
            | "golden_pickaxe"
            | "stone_pickaxe"
            | "wooden_pickaxe"
            | "netherite_pickaxe"
            | "diamond_axe"
            | "iron_axe"
            | "golden_axe"
            | "stone_axe"
            | "wooden_axe"
            | "netherite_axe"
            | "diamond_shovel"
            | "iron_shovel"
            | "golden_shovel"
            | "stone_shovel"
            | "wooden_shovel"
            | "netherite_shovel"
            | "diamond_hoe"
            | "iron_hoe"
            | "golden_hoe"
            | "stone_hoe"
            | "wooden_hoe"
            | "netherite_hoe"
            | "bow"
            | "crossbow"
            | "trident"
            | "shield"
            | "fishing_rod"
            | "flint_and_steel"
            | "shears"
            | "spyglass"
            | "brush"
            | "mace"
            | "apple"
            | "bread"
            | "cooked_beef"
            | "cooked_porkchop"
            | "golden_apple"
            | "potion"
            | "splash_potion"
            | "lingering_potion"
            | "milk_bucket"
            | "water_bucket"
            | "lava_bucket"
            | "honey_bottle"
            | "stick"
            | "feather"
            | "bone"
            | "arrow"
    ) && !short.ends_with("_bucket")
        && !short.ends_with("_sword")
        && !short.ends_with("_axe")
        && !short.ends_with("_pickaxe")
        && !short.ends_with("_shovel")
        && !short.ends_with("_hoe")
        && !short.ends_with("_spear");

    let use_kind = if short == "bow" {
        UseKind::Bow
    } else if short == "crossbow" {
        UseKind::Crossbow
    } else if short == "trident" {
        UseKind::Trident
    } else if short == "shield" {
        UseKind::Block
    } else if short == "spyglass" {
        UseKind::Spyglass
    } else if short == "brush" {
        UseKind::Brush
    } else if short == "goat_horn" {
        UseKind::TootHorn
    } else if short.ends_with("_spear") || short == "spear" {
        UseKind::Spear
    } else if short.contains("potion") || short == "milk_bucket" || short == "honey_bottle" {
        UseKind::Drink
    } else if matches!(
        short,
        "apple"
            | "golden_apple"
            | "bread"
            | "beef"
            | "cooked_beef"
            | "porkchop"
            | "cooked_porkchop"
            | "mutton"
            | "cooked_mutton"
            | "chicken"
            | "cooked_chicken"
            | "cod"
            | "cooked_cod"
            | "salmon"
            | "cooked_salmon"
            | "carrot"
            | "golden_carrot"
            | "baked_potato"
            | "sweet_berries"
            | "cookie"
            | "melon_slice"
    ) {
        UseKind::Eat
    } else {
        UseKind::None
    };

    ItemFlags {
        id: id.to_owned(),
        is_empty: false,
        is_block,
        use_kind,
        is_charged_crossbow: false,
        is_cooling_down: false,
        spear_can_damage: false,
        spear_can_dismount: false,
        spear_can_knockback: false,
        spear_hit_impact: false,
    }
}

struct ModState {
    config: Config,
    hmi: HmiState,
    enabled: bool,
    attack_down: bool,
    inspect_down: bool,
}

impl ModState {
    fn new() -> Self {
        Self {
            config: Config::default(),
            hmi: HmiState::default(),
            enabled: true,
            attack_down: false,
            inspect_down: false,
        }
    }

    fn load(&mut self) {
        let Some(host) = host() else { return };
        let mut buf = [0; TEXT];

        let mut read_setting = |key: &core::ffi::CStr| -> Option<&str> {
            if unsafe { (host.setting)(key.as_ptr(), buf.as_mut_ptr(), buf.len()) } > 0 {
                unsafe { text(buf.as_ptr()) }
            } else {
                None
            }
        };

        if let Some(Ok(cfg)) = read_setting(c"config.json").map(Config::from_json) {
            self.config = cfg;
        }
        if let Some(val) = read_setting(c"enabled") {
            self.enabled = val != "0";
        }
    }

    fn save(&self) {
        let Some(host) = host() else { return };
        let json = self.config.to_json();
        let c_json = std::ffi::CString::new(json).unwrap_or_default();
        unsafe {
            (host.set_setting)(c"config.json".as_ptr(), c_json.as_ptr());
            let en_str = if self.enabled { c"1" } else { c"0" };
            (host.set_setting)(c"enabled".as_ptr(), en_str.as_ptr());
        }
    }
}

static STATE: Mutex<Option<ModState>> = Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut ModState) -> R) -> R {
    let mut lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let state = lock.get_or_insert_with(ModState::new);
    f(state)
}

fn query_items(host: &HostVTable) -> (ItemFlags, ItemFlags) {
    let mut buf = [0; TEXT];
    let mut info = ItemInfo::default();

    let main = if unsafe { (host.item)(0, buf.as_mut_ptr(), buf.len(), &mut info) } != 0 {
        let id = unsafe { text(buf.as_ptr()) }.unwrap_or("");
        classify_item(id, info.count)
    } else {
        ItemFlags::empty()
    };

    let off = if unsafe { (host.item)(1, buf.as_mut_ptr(), buf.len(), &mut info) } != 0 {
        let id = unsafe { text(buf.as_ptr()) }.unwrap_or("");
        classify_item(id, info.count)
    } else {
        ItemFlags::empty()
    };

    (main, off)
}

fn query_player_input(host: &HostVTable, inspect_down: bool) -> PlayerFrameInput {
    let mut player = PlayerInfo {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        vx: 0.0,
        vy: 0.0,
        vz: 0.0,
        yaw: 0.0,
        pitch: 0.0,
        health: 20.0,
        mode: 0,
        flags: 2,
    };
    let has_player = unsafe { (host.player)(&mut player) } != 0;
    if !has_player {
        return PlayerFrameInput::default();
    }

    let use_key = unsafe { (host.key)(c"use".as_ptr()) };
    let using = (use_key & 2) != 0;

    PlayerFrameInput {
        pitch: player.pitch,
        yaw: player.yaw,
        speed: ((player.vx * player.vx + player.vz * player.vz) as f32).sqrt(),
        y_speed: player.vy as f32,
        on_ground: player.on_ground(),
        sneaking: player.sneaking(),
        swimming: player.in_water() && player.sprinting(),
        crawling: false,
        climbing: false,
        touching_water: player.in_water(),
        using_riptide: false,
        using_item: using,
        active_main_hand: true,
        inspect_held: inspect_down,
        block_breaking: false,
        age_ticks: 0.0,
        main_equip_progress: 0.0,
        off_equip_progress: 0.0,
    }
}

// ---------------------------------------------------------------------------
// Callbacks exported to Starling mod-loader
// ---------------------------------------------------------------------------

unsafe extern "C" fn init(host: *const HostVTable) -> bool {
    let _ = guard(|| {
        if !host.is_null() {
            HOST.store(host.cast_mut(), Ordering::Relaxed);
        }
        with_state(|s| s.load());
    });
    true
}

unsafe extern "C" fn tick() {
    let _ = guard(|| {
        let Some(host) = host() else { return };
        let (main_item, off_item) = query_items(host);

        with_state(|s| {
            s.hmi
                .tick_swings(&main_item, &off_item, None, None, &s.config);
        });
    });
}

unsafe extern "C" fn frame(partial: f32) {
    let _ = guard(|| {
        let Some(host) = host() else { return };
        let (main_item, off_item) = query_items(host);

        let attack_key = unsafe { (host.key)(c"attack".as_ptr()) };
        let attack_now = (attack_key & 2) != 0;

        with_state(|s| {
            if attack_now {
                let was_down = s.attack_down;
                s.hmi.on_swing_ex(true, &main_item, &s.config, was_down);
            }
            s.attack_down = attack_now;

            let input = query_player_input(host, s.inspect_down);
            let _sounds = s
                .hmi
                .step_frame(0.016, partial, &main_item, &off_item, &input, &s.config);
        });
    });
}

unsafe extern "C" fn settings(sink: *const ButtonSink) {
    let _ = guard(|| {
        if sink.is_null() {
            return;
        }
        with_state(|s| {
            let loaded: LoadedConfig = Ok(s.config.clone());
            let labels = runtime::settings_labels(&loaded);
            for label in labels {
                if let Ok(c_label) = std::ffi::CString::new(label) {
                    unsafe { ((*sink).push)((*sink).ctx, c_label.as_ptr()) };
                }
            }
        });
    });
}

unsafe extern "C" fn settings_click(button: u32) {
    let _ = guard(|| {
        with_state(|s| {
            let loaded: LoadedConfig = Ok(s.config.clone());
            if let Some(effect) = runtime::settings_click(&loaded, button as usize) {
                match effect {
                    runtime::SettingsEffect::Save(new_cfg) => {
                        s.config = new_cfg;
                        s.save();
                    }
                    runtime::SettingsEffect::Reload => {
                        s.load();
                    }
                }
            }
        });
    });
}

unsafe extern "C" fn hold_my_items(partial: f32, out: *mut HoldMyItems) {
    let _ = guard(|| {
        if out.is_null() {
            return;
        }
        let Some(host) = host() else { return };

        with_state(|s| {
            if !s.enabled {
                unsafe { (*out).active = 0 };
                return;
            }

            let (main_item, off_item) = query_items(host);
            let input = query_player_input(host, s.inspect_down);

            let eval_main = s.hmi.evaluate_hand(
                true, true, &main_item, &off_item, partial, &input, &s.config,
            );
            let eval_off = s.hmi.evaluate_hand(
                false, false, &off_item, &main_item, partial, &input, &s.config,
            );

            let res = unsafe { &mut *out };
            res.active = 1;
            res.show_main_arm = 1;
            res.show_off_arm = if off_item.is_empty { 0 } else { 1 };
            res.two_handed = if s.config.sword_counter_attack { 1 } else { 0 };
            res.offset_x = s.config.x_offset;
            res.offset_y = s.config.y_offset;
            res.offset_z = s.config.z_offset;
            res.pitch = 0.0;
            res.yaw = 0.0;
            res.roll = 0.0;
            res.has_matrices = 1;
            res.main_arm_matrix = eval_main.arm_pose.0;
            res.main_item_matrix = eval_main.item_pose.0;
            res.off_arm_matrix = eval_off.arm_pose.0;
            res.off_item_matrix = eval_off.item_pose.0;
        });
    });
}

// ---------------------------------------------------------------------------
// Mod Descriptor & Symbol Export
// ---------------------------------------------------------------------------

static ID: &[u8] = b"holdmyitems\0";
static TITLE: &[u8] = b"Hold My Items 5.2.0\0";
static DESCRIPTION: &[u8] =
    b"First-person hand & item physics, 3D block/item poses, spring-damper inertia, and per-weapon combat slashes.\0";

static DESC: ModDesc = ModDesc {
    abi: ABI,
    api_level: 7,
    id: ID.as_ptr() as *const c_char,
    title: TITLE.as_ptr() as *const c_char,
    description: DESCRIPTION.as_ptr() as *const c_char,
    init: Some(init),
    tick: Some(tick),
    frame: Some(frame),
    settings: Some(settings),
    settings_click: Some(settings_click),
    camera_offset: None,
    player_eyes: None,
    hold_my_items: Some(hold_my_items),
};

/// The one entry point looked up by `horizon-mod-loader`.
///
/// # Safety
/// `table` is the client's vtable, valid for the life of the process.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn horizon_mod_entry(table: *const HostVTable) -> *const ModDesc {
    let _ = guard(|| {
        if !table.is_null() {
            HOST.store(table.cast_mut(), Ordering::Relaxed);
        }
        with_state(|s| s.load());
    });
    &DESC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_items() {
        let sword = classify_item("minecraft:diamond_sword", 1);
        assert!(!sword.is_block);
        assert!(!sword.is_empty);

        let shield = classify_item("minecraft:shield", 1);
        assert_eq!(shield.use_kind, UseKind::Block);

        let apple = classify_item("minecraft:apple", 1);
        assert_eq!(apple.use_kind, UseKind::Eat);

        let stone = classify_item("minecraft:stone", 64);
        assert!(stone.is_block);

        let empty = classify_item("minecraft:air", 0);
        assert!(empty.is_empty);
    }

    #[test]
    fn test_mod_state_default() {
        let state = ModState::new();
        assert!(state.enabled);
        assert_eq!(state.config.swing_speed, 10);
    }
}
