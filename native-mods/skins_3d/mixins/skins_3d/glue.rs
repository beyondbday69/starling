use super::Game;
use horizon_client::mobs::render::umbx_skins3d_cache as voxels;
use horizon_client::mesher::Vertex;
use horizon_client::mobs::{Mob, MobType, Part, render_model};
use std::collections::HashSet;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

pub(super) const ID: &str = "skins_3d";
const VERSION: &str = "1.1.1";
const DESCRIPTION: &str = "The outer layer of every skin as real 3D pixels: hat, jacket, sleeves and pants.";
const REACH: &str = "reach";
const REACHES: [u32; 4] = [8, 16, 32, 64];
const DEFAULT_REACH: u32 = 32;
const SKIN_EDGE: usize = 64;
const MAX_MISSING: usize = 512;

const OFFLINE_SKIN: &str = "horizon/offline_skin";
const OWN_SKIN: &str = "horizon/own_skin";
const ELYTRA: &str = "minecraft:elytra";

static ACTIVE: AtomicBool = AtomicBool::new(false);
static MISSING: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static LOGGED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

fn reach(game: &Game<'_>) -> u32 {
    game.native_setting(ID, REACH).and_then(|v| v.parse().ok()).filter(|r| REACHES.contains(r)).unwrap_or(DEFAULT_REACH)
}

fn size(bytes: usize) -> Option<(usize, usize)> {
    let pixels = bytes / 4;
    let side = (pixels as f64).sqrt() as usize;
    if bytes % 4 != 0 || pixels == 0 {
        None
    } else if side * side == pixels && side % SKIN_EDGE == 0 {
        Some((side, side))
    } else if pixels == SKIN_EDGE * SKIN_EDGE / 2 {
        Some((SKIN_EDGE, SKIN_EDGE / 2))
    } else {
        None
    }
}

fn texture_file(resources: &Path, location: &str) -> Option<PathBuf> {
    let safe = !location.is_empty()
        && !location.contains("..")
        && !location.starts_with('/')
        && location.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_/.-".contains(&b));
    safe.then(|| resources.join("assets").join("minecraft").join("textures").join(format!("{location}.png")))
}

fn bundled(game: &Game<'_>, location: &str) -> Option<(usize, usize, Vec<u8>)> {
    let file = texture_file(&game.options.resources, location)?;
    let (width, height, rgba) = horizon_client::hud::decode(&file).ok()?;
    Some((width as usize, height as usize, rgba))
}

fn from_atlas(game: &Game<'_>, location: &str) -> Option<(usize, usize, Vec<u8>)> {
    let loc = location
        .strip_prefix("minecraft:")
        .unwrap_or(location)
        .strip_suffix(".png")
        .unwrap_or(location);
    let loc = loc.strip_prefix("textures/").unwrap_or(loc);
    let rect = game
        .gui
        .atlas
        .sprites
        .get(loc)
        .or_else(|| game.gui.atlas.sprites.get(location))?;
    let aw = game.gui.atlas.width as usize;
    let ah = game.gui.atlas.height as usize;
    if aw == 0 || ah == 0 {
        return None;
    }
    let (mut tw, mut th) = rect.texels((aw as u32, ah as u32));
    if size((tw * th * 4) as usize).is_none() && size((rect.w * rect.h * 4) as usize).is_some() {
        tw = rect.w;
        th = rect.h;
    }
    let (w, h) = size((tw * th * 4) as usize)?;
    let u0 = (rect.u0 * aw as f32).round() as usize;
    let v0 = (rect.v0 * ah as f32).round() as usize;
    if u0 >= aw || v0 >= ah {
        return None;
    }
    let pixels = game.gui.atlas.pixels();
    if pixels.len() < aw * ah * 4 {
        return None;
    }
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        let src_y = (v0 + y).min(ah - 1);
        let src_start = (src_y * aw + u0) * 4;
        let dst_start = (y * w) * 4;
        let row_len = (w * 4).min(aw.saturating_sub(u0) * 4);
        rgba[dst_start..dst_start + row_len].copy_from_slice(&pixels[src_start..src_start + row_len]);
    }
    Some((w, h, rgba))
}

fn ensure_skin(game: &Game<'_>, location: &str) {
    let mut missing = MISSING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if missing.contains(location) {
        return;
    }
    let downloads = game.skin_downloads.borrow();
    if let Some(rgba) = downloads.image(location)
        && let Some((width, height)) = size(rgba.len())
    {
        log::info!("3D Skins: resolved '{location}' ({}x{}) from skin_downloads", width, height);
        voxels::offer(location, width, height, rgba);
        return;
    }
    if (location == OWN_SKIN || location == OFFLINE_SKIN)
        && let Some(rgba) = game.own_skin.image.as_ref()
        && let Some((width, height)) = size(rgba.len())
    {
        log::info!("3D Skins: resolved '{location}' ({}x{}) from own_skin", width, height);
        voxels::offer(location, width, height, rgba);
        return;
    }
    if location == OFFLINE_SKIN
        && let Some((rgba, _)) = horizon_client::skins::offline_skin()
        && let Some((width, height)) = size(rgba.len())
    {
        log::info!("3D Skins: resolved '{location}' ({}x{}) from offline_skin", width, height);
        voxels::offer(location, width, height, rgba);
        return;
    }
    if let Some((width, height, rgba)) = from_atlas(game, location) {
        log::info!("3D Skins: resolved '{location}' ({}x{}) from GUI atlas", width, height);
        voxels::offer(location, width, height, &rgba);
        return;
    }
    if let Some((width, height, rgba)) = bundled(game, location)
        && size(rgba.len()) == Some((width, height))
    {
        log::info!("3D Skins: resolved '{location}' ({}x{}) from bundled texture file", width, height);
        voxels::offer(location, width, height, &rgba);
        return;
    }
    if location.starts_with("skins/") {
        let mut logged = LOGGED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if logged.len() < MAX_MISSING && logged.insert(location.to_owned()) {
            log::info!("3D Skins: waiting for download of skin '{location}'");
        }
        return;
    }
    if location == OWN_SKIN || location == OFFLINE_SKIN {
        let mut logged = LOGGED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if logged.len() < MAX_MISSING && logged.insert(location.to_owned()) {
            log::info!("3D Skins: waiting for profile/own_skin image for '{location}'");
        }
        return;
    }
    log::warn!("3D Skins: no pixels found for '{location}', keeping flat layer");
    if missing.len() < MAX_MISSING {
        missing.insert(location.to_owned());
    }
}

fn shown(head: bool, chest: Option<&str>, legs: bool) -> u8 {
    let mut shown = voxels::ALL;
    if head {
        shown &= !voxels::HAT;
    }
    if chest.is_some_and(|item| item != ELYTRA) {
        shown &= !voxels::JACKET_AND_SLEEVES;
    }
    if legs {
        shown &= !voxels::PANTS;
    }
    shown
}

fn swap(m: &mut Mob, near: bool, shown: u8) {
    let location = m.skin.as_deref().unwrap_or(&m.kind.texture);
    let model = voxels::model(&m.kind, location, m.slim, if near { shown } else { 0 });
    let same = match (&model, &m.skin_3d) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    };
    if !same {
        m.skin_3d = model;
    }
}

fn restore(game: &mut Game<'_>) {
    for m in game.mobs.iter_mut().chain(game.player_model.iter_mut()) {
        if &*m.kind.id == voxels::PLAYER {
            m.skin_3d = None;
        }
    }
    game.update_remote_skins();
}

pub(super) fn model(game: &Game<'_>, kind: &Arc<MobType>, skin: &str, slim: bool) -> ControlFlow<Option<Arc<Part>>> {
    if !game.native_on(ID) {
        return ControlFlow::Continue(());
    }
    ensure_skin(game, skin);
    ControlFlow::Break(voxels::model(kind, skin, slim, voxels::ALL))
}

fn find_part<'a>(part: &'a Part, name: &str) -> Option<&'a Part> {
    if part.name == name {
        return Some(part);
    }
    for child in &part.children {
        if let Some(found) = find_part(child, name) {
            return Some(found);
        }
    }
    None
}

pub(super) fn arm(game: &Game<'_>, left: bool, matrix: glam::Mat4, light: u32, out: &mut Vec<Vertex>) -> ControlFlow<()> {
    if !game.native_on(ID) {
        return ControlFlow::Continue(());
    }
    let player = game.player_model.as_ref();
    let name = if left { "left_arm" } else { "right_arm" };
    let skin_name = player
        .and_then(|m| m.skin.as_deref())
        .unwrap_or("entity/player/wide/steve");
    ensure_skin(game, skin_name);
    let model = player
        .and_then(|m| m.skin_3d.as_ref())
        .cloned()
        .or_else(|| {
            let m = player?;
            voxels::model(&m.kind, skin_name, m.slim, voxels::ALL)
        });
    let part = model.as_ref().and_then(|m| find_part(m, name));
    let sprite = game
        .gui
        .atlas
        .sprites
        .get(skin_name)
        .or_else(|| game.gui.atlas.sprites.get("entity/player/wide/steve"))
        .copied();
    let (Some(part), Some(sprite)) = (part, sprite) else {
        return ControlFlow::Continue(());
    };
    let z_rot = if left { -0.1 } else { 0.1 };
    let pose = |p: &Part| {
        let mut pose = p.pose;
        pose[3] = 0.0;
        pose[4] = 0.0;
        pose[5] = if p.name == name { z_rot } else { 0.0 };
        pose
    };
    render_model(part, matrix, &pose, [sprite.u0, sprite.v0, sprite.u1, sprite.v1], light, 0, out);
    ControlFlow::Break(())
}

pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_on(ID) {
        if ACTIVE.swap(false, Ordering::Relaxed) {
            restore(game);
        }
        return;
    }
    if !ACTIVE.swap(true, Ordering::Relaxed) {
        log::info!("3D Skins: initialized, active=true, reach={}, native_on={}", reach(game), game.native_on(ID));
    }
    let reach = f64::from(reach(game));
    let camera = game.native_camera().map(|c| c.position);
    let near = |x: f64, y: f64, z: f64| camera.is_none_or(|c| (x - c[0]).powi(2) + (y - c[1]).powi(2) + (z - c[2]).powi(2) <= reach * reach);
    let [head, chest, legs, _] = game.armor.each_ref().map(|s| s.as_ref().map(|s| &*s.item));
    let own = shown(head.is_some(), chest, legs.is_some());
    if let Some(m) = game.player_model.as_mut() {
        let close = near(m.position.x, m.position.y, m.position.z);
        swap(m, close, own);
    }
    for m in game.mobs.iter_mut().filter(|m| &*m.kind.id == voxels::PLAYER) {
        let worn = shown(m.extra.head.is_some(), m.extra.chest.as_deref(), m.extra.legs.is_some());
        let close = near(m.position.x, m.position.y, m.position.z);
        swap(m, close, worn);
    }
    let wanted = voxels::wanted();
    if wanted.is_empty() {
        return;
    }
    for location in &wanted {
        ensure_skin(game, location);
    }
    if let Some(m) = game.player_model.as_mut() {
        let close = near(m.position.x, m.position.y, m.position.z);
        swap(m, close, own);
    }
    for m in game.mobs.iter_mut().filter(|m| &*m.kind.id == voxels::PLAYER) {
        let worn = shown(m.extra.head.is_some(), m.extra.chest.as_deref(), m.extra.legs.is_some());
        let close = near(m.position.x, m.position.y, m.position.z);
        swap(m, close, worn);
    }
}

pub(super) fn listed(mods: &mut Vec<horizon_client::mod_api::NativeMod>) {
    mods.push(horizon_client::mod_api::NativeMod {
        id: ID.into(),
        title: format!("3D Skins {VERSION}"),
        description: DESCRIPTION.into(),
        keys: Vec::new(),
    });
}

pub(super) fn page(game: &Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id == ID {
        buttons.push(format!("3D layers within: {} blocks", reach(game)));
    }
}

pub(super) fn click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID || button != 0 {
        return;
    }
    let now = reach(game);
    let next = REACHES[(REACHES.iter().position(|r| *r == now).unwrap_or(0) + 1) % REACHES.len()];
    game.native_set_setting(ID, REACH, &next.to_string());
}
