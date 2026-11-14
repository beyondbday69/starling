//! Mods built as shared libraries: the client's side of `horizon-mod-abi`.
//!
//! A mod installed this way is a `.so` in `horizon-native-mods/<id>/`, not source compiled into
//! this client. `horizon-mod-loader` does the opening and the pointer work; all this file does is
//! implement [`Host`] for the game and pass its four hooks on.
//!
//! Nothing here runs while no library mod is installed: the folder is read once, and finding
//! nothing costs one `read_dir`.

use super::Game;
use horizon_client::mod_api::NativeMod;
use horizon_mod_loader::{self as loader, Host, ModInfo};

/// The folders a player drops a mod's library in: the one a `.hnmod` goes in, and the content
/// mods' folder beside it, because a player who unzips a mod there should still see it listed.
fn folders() -> Vec<std::path::PathBuf> {
    let game = Game::game_dir();
    vec![
        game.join(horizon_client::mod_list::NATIVE_DIR),
        game.join(horizon_client::mod_list::DIR),
    ]
}

/// How tall a standing player's eyes are, in blocks.
const EYE_HEIGHT: f64 = 1.62;

/// The game, as a mod may see it: a log line, its own settings, and the world it stands in.
struct GameHost<'a, 'g> {
    game: &'a mut Game<'g>,
}

/// The block a ray meets, walked one block at a time (a mod asks often enough that the walk must
/// be cheap and predictable).
fn walk(
    from: [f64; 3],
    to: [f64; 3],
    solid: impl Fn([i32; 3]) -> bool,
) -> Option<horizon_mod_abi::RayHitOut> {
    let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    if length < 1e-6 {
        return None;
    }
    let steps = (length * 4.0).ceil() as i32;
    let mut last = [
        from[0].floor() as i32,
        from[1].floor() as i32,
        from[2].floor() as i32,
    ];
    for step in 1..=steps {
        let f = f64::from(step) / f64::from(steps);
        let at = [
            from[0] + delta[0] * f,
            from[1] + delta[1] * f,
            from[2] + delta[2] * f,
        ];
        let block = [
            at[0].floor() as i32,
            at[1].floor() as i32,
            at[2].floor() as i32,
        ];
        if block != last {
            if solid(block) {
                let mut normal = [0.0; 3];
                let axis = (0..3).max_by(|&a, &b| {
                    let (da, db) = ((delta[a] / length).abs(), (delta[b] / length).abs());
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })?;
                normal[axis] = -delta[axis].signum();
                return Some(horizon_mod_abi::RayHitOut { at, block, normal });
            }
            last = block;
        }
    }
    None
}

impl Host for GameHost<'_, '_> {
    fn log(&self, level: u32, text: &str) {
        match level {
            0 => log::error!("mod: {text}"),
            1 => log::warn!("mod: {text}"),
            3 => log::debug!("mod: {text}"),
            _ => log::info!("mod: {text}"),
        }
    }

    fn setting(&self, id: &str, key: &str) -> Option<String> {
        self.game.native_setting(id, key)
    }

    fn set_setting(&mut self, id: &str, key: &str, value: &str) {
        self.game.native_set_setting(id, key, value);
    }

    fn listener(&self) -> Option<[f64; 3]> {
        let player = self.game.native_player()?;
        Some([player.x, player.y + EYE_HEIGHT, player.z])
    }

    fn block_name(&self, pos: [i32; 3]) -> Option<String> {
        self.game.native_player()?;
        Some(self.game.native_block(pos))
    }

    fn face_is_sturdy(&self, pos: [i32; 3], side: usize) -> bool {
        // The ABI counts sides x-, x+, y-, y+, z-, z+; vanilla's `Direction` counts down, up,
        // north, south, west, east.
        const SIDE: [u32; 6] = [4, 5, 0, 1, 2, 3];
        let Some(&direction) = SIDE.get(side) else {
            return false;
        };
        let state = self.game.world.block(pos[0], pos[1], pos[2]);
        self.game.res.facts().get(state).sturdy_full & (1 << direction) != 0
    }

    fn is_water(&self, pos: [i32; 3]) -> bool {
        self.game.native_player().is_some() && self.game.native_block(pos) == "minecraft:water"
    }

    fn sounds_started(&mut self) -> Vec<loader::StartedInfo> {
        self.game
            .audio
            .started_since()
            .into_iter()
            .map(|(instance, id, at)| loader::StartedInfo { instance, id, at })
            .collect()
    }

    fn set_sound(&mut self, instance: u64, gain: f32, _pitch: f32) -> bool {
        self.game.audio.set_sound_gain(instance, gain)
    }

    fn raycast(&self, from: [f64; 3], to: [f64; 3]) -> Option<horizon_mod_abi::RayHitOut> {
        self.game.native_player()?;
        walk(from, to, |pos| {
            let name = self.game.native_block(pos);
            name != "minecraft:air" && name != "minecraft:cave_air" && name != "minecraft:void_air"
        })
    }
}

/// Reads the folder once and adds what it holds to Options > Mods.
pub(super) fn list(game: &mut Game<'_>, mods: &mut Vec<NativeMod>) {
    let mut host = GameHost { game };
    loader::scan(&folders(), &mut host);
    for ModInfo {
        id,
        title,
        description,
    } in loader::list()
    {
        mods.push(NativeMod {
            id,
            title,
            description,
            keys: Vec::new(),
        });
    }
}

/// Once a frame: starts any mod that has not started, then runs them all.
pub(super) fn frame(game: &mut Game<'_>, partial: f32) {
    let mut host = GameHost { game };
    loader::start(&mut host);
    loader::frame(&mut host, partial);
}

/// Once a tick.
pub(super) fn tick(game: &mut Game<'_>) {
    let mut host = GameHost { game };
    loader::tick(&mut host);
}

/// The rows of `mod_id`'s page in Options > Mods.
pub(super) fn settings(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    let mut host = GameHost { game };
    loader::settings(&mut host, mod_id, buttons);
}

/// A tap on row `button` of `mod_id`'s page.
pub(super) fn settings_click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    let mut host = GameHost { game };
    loader::settings_click(&mut host, mod_id, button);
}
