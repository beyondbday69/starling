pub(crate) mod core;
pub(crate) mod net;
pub(crate) mod page;
pub(crate) mod trail;

use crate::game::Game;
use crate::game::native_hooks::NativeMod;

impl Game<'_> {
    pub(crate) fn elytratrails_listed(&self, mods: &mut Vec<NativeMod>) {
        mods.push(NativeMod {
            id: core::ID.into(),
            title: format!("{} {}", core::NAME, core::VERSION),
            description: core::DESCRIPTION.into(),
            keys: Vec::new(),
        });
    }

    pub(crate) fn elytratrails_tick(&mut self) {
        self.elytratrails_step();
        self.elytratrails_net_tick();
    }
}
