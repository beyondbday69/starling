// Touch input handling for Cinematic Interactions:
// Replaces absolute touch positioning with a virtual trackpad cursor and 1-finger gestures:
// - Swipe / Slide: Moves the '+' crosshair cursor relative to finger delta.
// - Single Tap: Left Click (LMB) — pick up full stack, drop stack, craft result, select recipe.
// - Double Tap: Right Click (RMB) — place 1 item, or split half stack.
// - Double Tap & Drag: Real-time RMB Quick-Craft Drag across cells (distribute 1 per slot in line).
// - Long Press (> 350ms): Shift + Click (Quick-Move stack between inventory and table).
// - Direct or cursor tap on [ X ]: Cleanly exit workstation.
// - Direct or cursor tap on [ ^ ]: Toggle Shift.

use horizon_client_lib::native_cinematic_interactions_scene as ci;
use horizon_client::hud::HudBatch;
use std::sync::Mutex;
use std::time::Instant;
use winit::event::{TouchPhase, WindowEvent};
use winit::keyboard::KeyCode;

#[derive(Clone, Copy, Debug)]
struct TouchState {
    id: u64,
    start_time: Instant,
    last_move_time: Instant,
    start_pos: (f32, f32),
    last_pos: (f32, f32),
    total_motion: f32,
    is_double_tap: bool,
    is_rmb_drag: bool,
    long_press_done: bool,
}

static CURRENT_TOUCH: Mutex<Option<TouchState>> = Mutex::new(None);
static LAST_TAP: Mutex<Option<(Instant, (f32, f32))>> = Mutex::new(None);

fn cinematic_touch(
    game: &mut crate::game::Game<'_>,
    event: &winit::event::WindowEvent,
) -> ::core::ops::ControlFlow<()> {
    if !ci::controls() {
        return ::core::ops::ControlFlow::Continue(());
    }

    let WindowEvent::Touch(t) = event else {
        return ::core::ops::ControlFlow::Continue(());
    };

    let [w, h] = game.touch_size();
    let batch = HudBatch::new([w, h]);
    let scale = batch.scale.max(1.0);
    let gui_pos = (t.location.x as f32 / scale, t.location.y as f32 / scale);
    let (bw, bh) = (batch.width, batch.height);
    let now = Instant::now();

    match t.phase {
        TouchPhase::Started => {
            // Check if this tap started within double-tap window of previous tap
            let mut last_tap = LAST_TAP.lock().unwrap();
            let is_double_tap = if let Some((prev_time, prev_pos)) = *last_tap {
                let dt = now.duration_since(prev_time).as_millis();
                let dx = gui_pos.0 - prev_pos.0;
                let dy = gui_pos.1 - prev_pos.1;
                let dist = (dx * dx + dy * dy).sqrt();
                dt < 320 && dist < 50.0
            } else {
                false
            };
            *last_tap = None;

            // If a single tap was pending from previous tap, cancel it since second tap arrived
            ci::cancel_pending_tap();

            let mut cur = CURRENT_TOUCH.lock().unwrap();
            *cur = Some(TouchState {
                id: t.id,
                start_time: now,
                last_move_time: now,
                start_pos: gui_pos,
                last_pos: gui_pos,
                total_motion: 0.0,
                is_double_tap,
                is_rmb_drag: false,
                long_press_done: false,
            });

            // Ensure vcursor is initialized
            let cur_vpos = ci::vcursor(bw, bh);
            game.cursor = cur_vpos;
            ::core::ops::ControlFlow::Break(())
        }

        TouchPhase::Moved => {
            let mut cur = CURRENT_TOUCH.lock().unwrap();
            let Some(mut state) = *cur else {
                return ::core::ops::ControlFlow::Break(());
            };
            if state.id != t.id {
                return ::core::ops::ControlFlow::Break(());
            }

            let dx = gui_pos.0 - state.last_pos.0;
            let dy = gui_pos.1 - state.last_pos.1;
            let step_dist = (dx * dx + dy * dy).sqrt();
            state.total_motion += step_dist;
            state.last_pos = gui_pos;
            state.last_move_time = now;

            // Move virtual cursor with sensitivity factor
            const SENSITIVITY: f32 = 1.15;
            let new_cursor = ci::move_vcursor(dx * SENSITIVITY, dy * SENSITIVITY, bw, bh);
            game.cursor = new_cursor;

            // Check Double Tap & Drag transition
            if state.is_double_tap && !state.is_rmb_drag && state.total_motion > 6.0 {
                state.is_rmb_drag = true;
                ci::set_rmb_dragging(true);
                // Start RMB drag in Starling
                game.inventory_press(true);
            }

            if state.is_rmb_drag {
                // Distribute items across slots in real time
                game.inventory_drag_move();
            } else if !state.long_press_done && state.total_motion < 12.0 && now.duration_since(state.start_time).as_millis() >= 350 {
                // Long press fired! (Quick-Move / Shift+Click)
                state.long_press_done = true;
                let old_shift = game.keys.contains(&KeyCode::ShiftLeft);
                game.keys.insert(KeyCode::ShiftLeft);
                game.mobile.controls.inventory_shift = true;
                game.inventory_press(false);
                game.inventory_mouse_release(false);
                if !old_shift && !ci::shift_active() {
                    game.keys.remove(&KeyCode::ShiftLeft);
                    game.mobile.controls.inventory_shift = false;
                }
                game.audio.play_button_click();
                if let Some(key) = game.container {
                    game.container_changed(key);
                }
            }

            *cur = Some(state);
            ::core::ops::ControlFlow::Break(())
        }

        TouchPhase::Ended => {
            let mut cur = CURRENT_TOUCH.lock().unwrap();
            let Some(state) = cur.take() else {
                return ::core::ops::ControlFlow::Break(());
            };
            if state.id != t.id {
                return ::core::ops::ControlFlow::Break(());
            }

            // 1. Finish active RMB drag
            if state.is_rmb_drag {
                ci::set_rmb_dragging(false);
                game.inventory_mouse_release(true);
                if let Some(key) = game.container {
                    game.container_changed(key);
                }
                return ::core::ops::ControlFlow::Break(());
            }

            // 2. If long press already executed, finish
            if state.long_press_done {
                return ::core::ops::ControlFlow::Break(());
            }

            let duration_ms = now.duration_since(state.start_time).as_millis();
            let is_tap = duration_ms < 280 && state.total_motion < 18.0;

            if is_tap {
                let cursor_p = ci::vcursor(bw, bh);
                let touch_p = state.start_pos;
                let exit_rect = ci::exit_button_rect(bw, bh);
                let shift_rect = ci::shift_button_rect(bw, bh);

                // Exit button tapped directly or with cursor
                if ci::inside(exit_rect, touch_p.0, touch_p.1) || ci::inside(exit_rect, cursor_p.0, cursor_p.1) {
                    game.audio.play_button_click();
                    ci::stand_up();
                    ci::reset_vcursor();
                    ci::set_rmb(false);
                    ci::set_rmb_dragging(false);
                    ci::cancel_pending_tap();
                    if game.mobile.ui.modifier_latched(KeyCode::ShiftLeft) {
                        game.mobile.ui.toggle_modifier(KeyCode::ShiftLeft);
                    }
                    game.mobile.controls.inventory_shift = false;
                    game.keys.remove(&KeyCode::ShiftLeft);
                    game.close_container();
                    game.screen = crate::game::Screen::Playing;
                    game.capture(true);
                    game.inv_clicks.skip_release();
                    return ::core::ops::ControlFlow::Break(());
                }

                // Shift button tapped directly or with cursor
                if ci::inside(shift_rect, touch_p.0, touch_p.1) || ci::inside(shift_rect, cursor_p.0, cursor_p.1) {
                    game.audio.play_button_click();
                    let on = ci::toggle_shift();
                    game.mobile.controls.inventory_shift = on;
                    if on {
                        if !game.mobile.ui.modifier_latched(KeyCode::ShiftLeft) {
                            game.mobile.ui.toggle_modifier(KeyCode::ShiftLeft);
                        }
                        game.keys.insert(KeyCode::ShiftLeft);
                    } else {
                        if game.mobile.ui.modifier_latched(KeyCode::ShiftLeft) {
                            game.mobile.ui.toggle_modifier(KeyCode::ShiftLeft);
                        }
                        game.keys.remove(&KeyCode::ShiftLeft);
                    }
                    return ::core::ops::ControlFlow::Break(());
                }

                if state.is_double_tap {
                    // DOUBLE TAP CLICK (RMB Click: place 1 item, or split half stack)
                    game.cursor = cursor_p;
                    game.inventory_press(true);
                    game.inventory_mouse_release(true);
                    game.audio.play_button_click();
                    if let Some(key) = game.container {
                        game.container_changed(key);
                    }
                } else {
                    // First tap of a potential double-tap, or single tap
                    // Record last tap time and schedule pending single tap (180ms window)
                    let mut last_tap = LAST_TAP.lock().unwrap();
                    *last_tap = Some((now, touch_p));
                    ci::schedule_pending_tap(now, cursor_p);
                }
            }

            ::core::ops::ControlFlow::Break(())
        }

        TouchPhase::Cancelled => {
            let mut cur = CURRENT_TOUCH.lock().unwrap();
            if let Some(state) = cur.take() {
                if state.is_rmb_drag {
                    ci::set_rmb_dragging(false);
                    game.inventory_mouse_release(true);
                }
            }
            ::core::ops::ControlFlow::Break(())
        }
    }
}
