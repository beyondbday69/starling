pub(crate) mod glue;
pub(crate) mod trails;

use crate::first_person::Hand;
use crate::game::Game;
use crate::game::native_hooks::NativeMod;
use crate::items::ItemStack;
use crate::mesher::Vertex;

impl Game<'_> {
    pub(crate) fn livingmotion_frame(&mut self) {
        glue::frame(self);
    }

    pub(crate) fn livingmotion_key(&mut self, key: &str, pressed: bool) {
        glue::key(self, key, pressed);
    }

    pub(crate) fn livingmotion_listed(&self, mods: &mut Vec<NativeMod>) {
        glue::listed(mods);
    }

    pub(crate) fn livingmotion_settings_page(&mut self, mod_id: &str, buttons: &mut Vec<String>) {
        glue::settings_page(self, mod_id, buttons);
    }

    pub(crate) fn livingmotion_settings_click(&mut self, mod_id: &str, button: usize) {
        glue::settings_click(self, mod_id, button);
    }

    pub(crate) fn livingmotion_hand_pose(
        &self,
        hand: Hand,
        stack: Option<&ItemStack>,
        partial: f32,
        swing: f32,
        x: &mut f32,
        y: &mut f32,
        z: &mut f32,
        pitch: &mut f32,
        yaw: &mut f32,
        roll: &mut f32,
    ) {
        glue::hand_pose(
            self,
            hand,
            stack.is_some(),
            partial,
            swing,
            x,
            y,
            z,
            pitch,
            yaw,
            roll,
        );
    }

    pub(crate) fn livingmotion_world(
        &mut self,
        camera: glam::DVec3,
        partial: f64,
        normal: &mut Vec<Vertex>,
    ) {
        glue::world(self, camera, partial, normal);
    }
}
