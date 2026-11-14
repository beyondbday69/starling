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

/// The game, as a mod may see it: a log line and its own settings, nothing else yet.
struct GameHost<'a, 'g> {
    game: &'a Game<'g>,
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

    fn set_setting(&self, id: &str, key: &str, value: &str) {
        self.game.native_set_setting(id, key, value);
    }
}

/// Reads the folder once and adds what it holds to Options > Mods.
pub(super) fn list(game: &Game<'_>, mods: &mut Vec<NativeMod>) {
    let host = GameHost { game };
    loader::scan(&folders(), &host);
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
pub(super) fn frame(game: &Game<'_>, partial: f32) {
    let host = GameHost { game };
    loader::start(&host);
    loader::frame(&host, partial);
}

/// Once a tick.
pub(super) fn tick(game: &Game<'_>) {
    let host = GameHost { game };
    loader::tick(&host);
}

/// The rows of `mod_id`'s page in Options > Mods.
pub(super) fn settings(game: &Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    let host = GameHost { game };
    loader::settings(&host, mod_id, buttons);
}

/// A tap on row `button` of `mod_id`'s page.
pub(super) fn settings_click(game: &Game<'_>, mod_id: &str, button: usize) {
    let host = GameHost { game };
    loader::settings_click(&host, mod_id, button);
}
