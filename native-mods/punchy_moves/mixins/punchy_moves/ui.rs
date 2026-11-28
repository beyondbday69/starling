//! Punchy Moves' screens: the emote wheel and its keybinds, the shoulder
//! camera's tabbed settings, the colour picker, the face canvas, the tutorial,
//! and the near-camera dither the mod hides its own geometry with.
//!
//! Pure Rust and std-only: every screen builds a [`HudFrame`] of plain
//! rectangles and lines, and the bundle's `h_gui.rs` hands that frame to the
//! game's HUD batch after the frame's own GUI. Input is the same shape: the
//! hooks (`key.rs`, `settings.rs`, `click.rs`) call the methods here and act on
//! the [`UiRequest`] they return.
//!
//! Colours are `0xAABBGGRR`, the HUD's own packing ([`rgba`]).

use std::cell::RefCell;

use crate::camera::ShoulderCameraSettings;
use crate::engine::ItemAction;
use crate::faces::{FaceCue, FaceExpression, FaceQuads, FaceRig, FaceSettings};
use crate::signals::MotionSignals;

// ---------------------------------------------------------------------------
// Drawing primitives
// ---------------------------------------------------------------------------

/// A colour packed the way the HUD packs one: alpha high, then blue, green
/// and red, so `rgba(1, 2, 3, 255)` is `0xFF030201`.
pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (a as u32) << 24 | (b as u32) << 16 | (g as u32) << 8 | r as u32
}

/// A rectangle in GUI pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UiRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> UiRect {
        UiRect { x, y, w, h }
    }

    /// The rectangle's middle.
    pub fn center(&self) -> [f32; 2] {
        [self.x + self.w * 0.5, self.y + self.h * 0.5]
    }

    /// Whether a point lies inside it.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }

    /// The rectangle pulled in by `amount` on every side (never inverted).
    pub fn inset(&self, amount: f32) -> UiRect {
        let amount = amount.max(0.0);
        let w = (self.w - 2.0 * amount).max(0.0);
        let h = (self.h - 2.0 * amount).max(0.0);
        UiRect::new(self.x + amount, self.y + amount, w, h)
    }

    /// Its right and bottom edges.
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

/// The screen the screens are laid out on, in GUI pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UiLayout {
    pub width: f32,
    pub height: f32,
}

impl UiLayout {
    pub const fn new(width: f32, height: f32) -> UiLayout {
        UiLayout { width, height }
    }

    /// The middle of the screen.
    pub fn center(&self) -> [f32; 2] {
        [self.width * 0.5, self.height * 0.5]
    }

    /// A `w` by `h` panel in the middle, never larger than the screen.
    pub fn panel(&self, w: f32, h: f32) -> UiRect {
        let w = w.min(self.width.max(0.0));
        let h = h.min(self.height.max(0.0));
        UiRect::new((self.width - w) * 0.5, (self.height - h) * 0.5, w, h)
    }

    /// The `index`th row inside `panel`, with a line of space above the rows.
    pub fn row(&self, panel: UiRect, index: usize) -> UiRect {
        let top = panel.y + ROWS_TOP + index as f32 * ROW_HEIGHT;
        UiRect::new(
            panel.x + 8.0,
            top,
            (panel.w - 16.0).max(0.0),
            ROW_HEIGHT - 2.0,
        )
    }
}

/// The space a panel keeps above its first row, in GUI pixels.
pub const ROWS_TOP: f32 = 22.0;
/// The height of one row, in GUI pixels.
pub const ROW_HEIGHT: f32 = 13.0;

/// One thing the HUD draws.
#[derive(Debug, Clone, PartialEq)]
pub enum HudDraw {
    /// A filled rectangle.
    Rect(UiRect, u32),
    /// A filled four-cornered shape, corners in order round it.
    Quad([[f32; 2]; 4], u32),
    /// A line of text at `x, y` (top left).
    Text {
        text: String,
        x: f32,
        y: f32,
        color: u32,
        shadow: bool,
    },
}

/// Everything the open screens draw this frame, in draw order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudFrame {
    draws: Vec<HudDraw>,
}

impl HudFrame {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a filled rectangle.
    pub fn rect(&mut self, rect: UiRect, color: u32) {
        self.draws.push(HudDraw::Rect(rect, color));
    }

    /// Adds a filled four-cornered shape.
    pub fn quad(&mut self, corners: [[f32; 2]; 4], color: u32) {
        self.draws.push(HudDraw::Quad(corners, color));
    }

    /// Adds a line of text.
    pub fn text(&mut self, text: impl Into<String>, x: f32, y: f32, color: u32) {
        self.draws.push(HudDraw::Text {
            text: text.into(),
            x,
            y,
            color,
            shadow: true,
        });
    }

    /// Adds a line of text without its drop shadow.
    pub fn plain(&mut self, text: impl Into<String>, x: f32, y: f32, color: u32) {
        self.draws.push(HudDraw::Text {
            text: text.into(),
            x,
            y,
            color,
            shadow: false,
        });
    }

    /// The draws, in order.
    pub fn draws(&self) -> &[HudDraw] {
        &self.draws
    }

    /// How many draws the frame holds.
    pub fn len(&self) -> usize {
        self.draws.len()
    }

    /// Whether nothing is drawn.
    pub fn is_empty(&self) -> bool {
        self.draws.is_empty()
    }

    /// A dim wash over the whole screen, then a panel with a border: the
    /// backdrop every screen sits on.
    pub fn panel(&mut self, layout: &UiLayout, w: f32, h: f32, theme: &UiTheme) -> UiRect {
        self.rect(
            UiRect::new(0.0, 0.0, layout.width, layout.height),
            theme.dim,
        );
        let panel = layout.panel(w, h);
        self.rect(panel, theme.panel);
        let edge = 1.0;
        self.rect(UiRect::new(panel.x, panel.y, panel.w, edge), theme.border);
        self.rect(
            UiRect::new(panel.x, panel.bottom() - edge, panel.w, edge),
            theme.border,
        );
        self.rect(UiRect::new(panel.x, panel.y, edge, panel.h), theme.border);
        self.rect(
            UiRect::new(panel.right() - edge, panel.y, edge, panel.h),
            theme.border,
        );
        panel
    }

    /// A panel's title, centred at its top.
    pub fn title(&mut self, text: &str, panel: UiRect, theme: &UiTheme) {
        let width = text.chars().count() as f32 * GLYPH_WIDTH;
        self.text(
            text,
            (panel.x + (panel.w - width) * 0.5).max(panel.x + 4.0),
            panel.y + 6.0,
            theme.title,
        );
    }

    /// A row of a panel: its label on the left, its value on the right.
    pub fn row(
        &mut self,
        panel: UiRect,
        index: usize,
        label: &str,
        value: &str,
        selected: bool,
        theme: &UiTheme,
    ) {
        let row = UiLayout::new(panel.w, panel.h).row(panel, index);
        if selected {
            self.rect(row, theme.selected);
        }
        self.text(label, row.x + 3.0, row.y + 2.0, theme.text);
        let width = value.chars().count() as f32 * GLYPH_WIDTH;
        self.text(
            value,
            (row.right() - width - 3.0).max(row.x + 3.0),
            row.y + 2.0,
            theme.value,
        );
    }

    /// A footer line under a panel's rows.
    pub fn footer(&mut self, text: &str, panel: UiRect, theme: &UiTheme) {
        self.text(text, panel.x + 8.0, panel.bottom() - 12.0, theme.muted);
    }
}

/// The width one text glyph takes, for centring: the HUD font is 6 pixels
/// wide, one more between glyphs.
pub const GLYPH_WIDTH: f32 = 6.0;

// ---------------------------------------------------------------------------
// GoldUi
// ---------------------------------------------------------------------------

/// Converts a Java ARGB int (what GoldUi's constants are written as) to the
/// ABGR word the HUD batch takes.
pub const fn argb(java: i32) -> u32 {
    let v = java as u32;
    rgba(
        ((v >> 16) & 0xFF) as u8,
        ((v >> 8) & 0xFF) as u8,
        (v & 0xFF) as u8,
        ((v >> 24) & 0xFF) as u8,
    )
}

/// GoldUi's palette and drawing primitives, the Java `GoldUi` record for
/// record: the same colours, the same bevels and the same pixel geometry.
pub mod gold {
    use super::{HudFrame, UiRect, argb};

    pub const BACKDROP: u32 = argb(-1946157056);
    pub const WINDOW_EDGE: u32 = argb(-14803425);
    pub const SHADOW: u32 = argb(1493172224);
    pub const TITLEBAR: u32 = argb(-1775898);
    pub const TITLEBAR_EDGE: u32 = argb(-5131595);
    pub const TITLE_INK: u32 = argb(-14803425);
    pub const LETTERPRESS: u32 = argb(-1929379841);
    pub const RAIL: u32 = argb(-13553101);
    pub const PANE: u32 = argb(-12039862);
    pub const ROW_HOVER: u32 = argb(-11513518);
    pub const SEP: u32 = argb(-13487308);
    pub const GROOVE_LIGHT: u32 = argb(-10855588);
    pub const WELL: u32 = argb(-14737375);
    pub const OFF_FILL: u32 = argb(-13947859);
    pub const KNOB: u32 = argb(-3092012);
    pub const KNOB_HI: u32 = argb(-1841946);
    pub const TRACK: u32 = argb(-13947859);
    pub const TRACK_LIGHT: u32 = argb(-12960964);
    pub const LABEL_PLATE: u32 = argb(-1290397928);
    pub const SCROLL_THUMB: u32 = argb(-1715881283);
    pub const SCROLL_THUMB_HOVER: u32 = argb(-3092012);
    pub const TEXT_1: u32 = argb(-1);
    pub const TEXT_2: u32 = argb(-2829100);
    pub const TEXT_3: u32 = argb(-4342339);
    pub const TEXT_DIS: u32 = argb(-8487040);
    pub const BUTTON_INK: u32 = argb(-14803425);
    pub const GOLD_50: u32 = argb(-2346);
    pub const GOLD_100: u32 = argb(-5981);
    pub const GOLD_200: u32 = argb(-10138);
    pub const GOLD_300: u32 = argb(-14019);
    pub const GOLD_400: u32 = argb(-872418);
    pub const GOLD_500: u32 = argb(-2516468);
    pub const GOLD_600: u32 = argb(-5737466);
    pub const GOLD_700: u32 = argb(-8760825);
    pub const GOLD_INK: u32 = argb(-15002619);
    pub const SECONDARY_HOVER: u32 = argb(-2105118);
    pub const SECONDARY_PRESSED: u32 = argb(-4342079);
    pub const PRIMARY_DISABLED_FACE: u32 = argb(-9541028);
    pub const PRIMARY_DISABLED_INK: u32 = argb(-12961744);
    pub const WHEEL_WASH: u32 = argb(889192448);
    pub const SHORTCUT_STRIPE: u32 = argb(436207616);

    /// One bevel's five colours, the Java `Bevel` record.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Bevel {
        pub outline: u32,
        pub face: u32,
        pub highlight: u32,
        pub shine: u32,
        pub depth: u32,
    }

    impl Bevel {
        pub const fn new(outline: u32, face: u32, highlight: u32, shine: u32, depth: u32) -> Self {
            Self {
                outline,
                face,
                highlight,
                shine,
                depth,
            }
        }

        /// `withFace`.
        pub const fn with_face(self, face: u32) -> Self {
            Self { face, ..self }
        }
    }

    pub const LIGHT: Bevel = Bevel::new(WINDOW_EDGE, KNOB, GOLD_200, GOLD_100, GOLD_700);
    pub const GOLD: Bevel = Bevel::new(WINDOW_EDGE, GOLD_400, GOLD_200, GOLD_100, GOLD_700);
    pub const KNOB_BEVEL: Bevel =
        Bevel::new(WINDOW_EDGE, KNOB, GOLD_200, GOLD_100, argb(-10789793));

    pub const LIP: i32 = 2;
    pub const TOGGLE_W: i32 = 30;
    pub const TOGGLE_H: i32 = 14;
    pub const KNOB_W: i32 = 16;
    pub const KNOB_H: i32 = 16;
    pub const TRACK_H: i32 = 6;
    pub const THUMB_W: i32 = 10;
    pub const THUMB_H: i32 = 12;
    pub const TAB_BAR: i32 = 3;

    /// `drawInk`: the letterpress shadow one pixel down, then the ink.
    pub fn draw_ink(frame: &mut HudFrame, text: &str, x: i32, y: i32, ink: u32) {
        frame.text(text, x as f32, (y + 1) as f32, LETTERPRESS);
        frame.text(text, x as f32, y as f32, ink);
    }

    /// `outline`: a one-pixel ring, or a full fill when too small.
    pub fn outline(frame: &mut HudFrame, x1: i32, y1: i32, x2: i32, y2: i32, color: u32) {
        if x2 - x1 >= 2 && y2 - y1 >= 2 {
            frame.rect(
                UiRect::new(x1 as f32, y1 as f32, (x2 - x1) as f32, 1.0),
                color,
            );
            frame.rect(
                UiRect::new(x1 as f32, (y2 - 1) as f32, (x2 - x1) as f32, 1.0),
                color,
            );
            frame.rect(
                UiRect::new(x1 as f32, (y1 + 1) as f32, 1.0, (y2 - y1 - 2) as f32),
                color,
            );
            frame.rect(
                UiRect::new((x2 - 1) as f32, (y1 + 1) as f32, 1.0, (y2 - y1 - 2) as f32),
                color,
            );
        } else {
            frame.rect(
                UiRect::new(x1 as f32, y1 as f32, (x2 - x1) as f32, (y2 - y1) as f32),
                color,
            );
        }
    }

    /// `focusRing`: the focus outline one pixel out, in gold.
    pub fn focus_ring(frame: &mut HudFrame, x1: i32, y1: i32, x2: i32, y2: i32) {
        outline(frame, x1 - 1, y1 - 1, x2 + 1, y2 + 1, GOLD_200);
    }

    /// `groove`: a horizontal separator with its light edge under it.
    pub fn groove(frame: &mut HudFrame, x1: i32, x2: i32, y: i32) {
        frame.rect(UiRect::new(x1 as f32, y as f32, (x2 - x1) as f32, 1.0), SEP);
        frame.rect(
            UiRect::new(x1 as f32, (y + 1) as f32, (x2 - x1) as f32, 1.0),
            GROOVE_LIGHT,
        );
    }

    /// `vgroove`: the vertical one.
    pub fn vgroove(frame: &mut HudFrame, x: i32, y1: i32, y2: i32) {
        frame.rect(UiRect::new(x as f32, y1 as f32, 1.0, (y2 - y1) as f32), SEP);
        frame.rect(
            UiRect::new((x + 1) as f32, y1 as f32, 1.0, (y2 - y1) as f32),
            GROOVE_LIGHT,
        );
    }

    /// `bevel`: the raised plate the buttons, toggles and knobs sit on. The
    /// nine arguments mirror the Java signature.
    #[allow(clippy::too_many_arguments)]
    pub fn bevel(
        frame: &mut HudFrame,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        bevel: Bevel,
        lip: i32,
        pressed: bool,
    ) {
        let top = if pressed { y1 + lip } else { y1 };
        let depth_lip = if pressed { 0 } else { lip };
        outline(frame, x1, top, x2, y2, bevel.outline);
        let left = x1 + 1;
        let right = x2 - 1;
        let inner_top = top + 1;
        let inner_bottom = y2 - 1 - depth_lip;
        if right - left >= 2 && inner_bottom - inner_top >= 2 {
            frame.rect(
                UiRect::new(left as f32, inner_top as f32, (right - left) as f32, 1.0),
                bevel.highlight,
            );
            frame.rect(
                UiRect::new(
                    left as f32,
                    (inner_top + 1) as f32,
                    1.0,
                    (inner_bottom - inner_top - 2) as f32,
                ),
                bevel.highlight,
            );
            frame.rect(
                UiRect::new(
                    (right - 1) as f32,
                    (inner_top + 1) as f32,
                    1.0,
                    (inner_bottom - inner_top - 2) as f32,
                ),
                bevel.highlight,
            );
            frame.rect(
                UiRect::new(
                    left as f32,
                    (inner_bottom - 1) as f32,
                    (right - left) as f32,
                    1.0,
                ),
                bevel.shine,
            );
            frame.rect(
                UiRect::new(
                    (left + 1) as f32,
                    (inner_top + 1) as f32,
                    (right - left - 2) as f32,
                    (inner_bottom - inner_top - 2) as f32,
                ),
                bevel.face,
            );
            if depth_lip > 0 {
                frame.rect(
                    UiRect::new(
                        left as f32,
                        inner_bottom as f32,
                        (right - left) as f32,
                        (y2 - 1 - inner_bottom) as f32,
                    ),
                    bevel.depth,
                );
            }
        } else {
            frame.rect(
                UiRect::new(
                    left as f32,
                    inner_top as f32,
                    (right - left) as f32,
                    (y2 - 1 - inner_top) as f32,
                ),
                bevel.face,
            );
        }
    }

    /// `window`: the double shadow, the edge, the title bar and the pane.
    pub fn window(
        frame: &mut HudFrame,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        title_h: i32,
        pane: u32,
    ) {
        frame.rect(
            UiRect::new(
                (x1 + 4) as f32,
                (y1 + 4) as f32,
                (x2 - x1) as f32,
                (y2 - y1) as f32,
            ),
            SHADOW,
        );
        frame.rect(
            UiRect::new(
                (x1 + 2) as f32,
                (y1 + 2) as f32,
                (x2 - x1) as f32,
                (y2 - y1) as f32,
            ),
            SHADOW,
        );
        outline(frame, x1, y1, x2, y2, WINDOW_EDGE);
        frame.rect(
            UiRect::new(
                (x1 + 1) as f32,
                (y1 + 1) as f32,
                (x2 - x1 - 2) as f32,
                (title_h - 2) as f32,
            ),
            TITLEBAR,
        );
        frame.rect(
            UiRect::new(
                (x1 + 1) as f32,
                (y1 + title_h - 2) as f32,
                (x2 - x1 - 2) as f32,
                2.0,
            ),
            TITLEBAR_EDGE,
        );
        frame.rect(
            UiRect::new(
                (x1 + 1) as f32,
                (y1 + title_h) as f32,
                (x2 - x1 - 2) as f32,
                (y2 - 1 - y1 - title_h) as f32,
            ),
            pane,
        );
    }

    /// `button`: a bevelled button with its ink centred, the Java colours for
    /// the primary (gold) and secondary (light) faces, hover and disabled.
    /// The eight arguments mirror the Java signature.
    #[allow(clippy::too_many_arguments)]
    pub fn button(
        frame: &mut HudFrame,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        label: &str,
        primary: bool,
        hovered: bool,
        enabled: bool,
    ) {
        let style = if primary { GOLD } else { LIGHT };
        let face = if !enabled {
            if primary {
                PRIMARY_DISABLED_FACE
            } else {
                SECONDARY_PRESSED
            }
        } else if hovered {
            if primary { GOLD_300 } else { SECONDARY_HOVER }
        } else {
            style.face
        };
        bevel(frame, x, y, x + w, y + h, style.with_face(face), LIP, false);
        let ink = if !enabled {
            if primary {
                PRIMARY_DISABLED_INK
            } else {
                TEXT_DIS
            }
        } else if primary {
            GOLD_INK
        } else {
            WINDOW_EDGE
        };
        let width = label.chars().count() as f32 * super::GLYPH_WIDTH;
        let text_x = x as f32 + (w as f32 - width) * 0.5;
        let top = y + 1;
        let bottom = y + h - 1 - 2;
        let text_y = top as f32 + ((bottom - top - 8) as f32) * 0.5;
        draw_ink(frame, label, text_x as i32, text_y as i32, ink);
    }

    /// `toggle`: the 30x14 switch with its 16x16 knob.
    pub fn toggle(frame: &mut HudFrame, x: i32, y: i32, on: bool, hovered: bool) {
        let top = y + 16 - TOGGLE_H;
        let height = TOGGLE_H;
        let track_start = x;
        let track_end = x + TOGGLE_W;
        let knob_x = if on { track_end - KNOB_W } else { track_start };
        let knob_end = knob_x + KNOB_W;
        frame.rect(
            UiRect::new(
                track_start as f32,
                top as f32,
                (knob_x - track_start) as f32,
                1.0,
            ),
            WINDOW_EDGE,
        );
        frame.rect(
            UiRect::new(
                knob_end as f32,
                top as f32,
                (track_end - knob_end) as f32,
                1.0,
            ),
            WINDOW_EDGE,
        );
        frame.rect(
            UiRect::new(
                track_start as f32,
                (top + height - 1) as f32,
                (knob_x - track_start) as f32,
                1.0,
            ),
            WINDOW_EDGE,
        );
        frame.rect(
            UiRect::new(
                knob_end as f32,
                (top + height - 1) as f32,
                (track_end - knob_end) as f32,
                1.0,
            ),
            WINDOW_EDGE,
        );
        if knob_x > track_start {
            frame.rect(
                UiRect::new(
                    track_start as f32,
                    (top + 1) as f32,
                    1.0,
                    (height - 2) as f32,
                ),
                WINDOW_EDGE,
            );
        }
        if knob_end < track_end {
            frame.rect(
                UiRect::new(
                    (track_end - 1) as f32,
                    (top + 1) as f32,
                    1.0,
                    (height - 2) as f32,
                ),
                WINDOW_EDGE,
            );
        }
        if on {
            frame.rect(
                UiRect::new(
                    (track_start + 1) as f32,
                    (top + 1) as f32,
                    (knob_x - track_start - 1) as f32,
                    1.0,
                ),
                GOLD_300,
            );
            frame.rect(
                UiRect::new(
                    (track_start + 1) as f32,
                    (top + 2) as f32,
                    (knob_x - track_start - 1) as f32,
                    (height - 3) as f32,
                ),
                GOLD_500,
            );
            frame.rect(
                UiRect::new((track_start + 7) as f32, (top + 4) as f32, 1.0, 6.0),
                GOLD_50,
            );
        } else {
            frame.rect(
                UiRect::new(
                    knob_end as f32,
                    (top + 1) as f32,
                    (track_end - 1 - knob_end) as f32,
                    (height - 2) as f32,
                ),
                OFF_FILL,
            );
            let mark = track_start + 20;
            frame.rect(
                UiRect::new((mark + 1) as f32, (top + 4) as f32, 3.0, 1.0),
                TEXT_2,
            );
            frame.rect(
                UiRect::new((mark + 1) as f32, (top + 9) as f32, 3.0, 1.0),
                TEXT_2,
            );
            frame.rect(UiRect::new(mark as f32, (top + 5) as f32, 1.0, 4.0), TEXT_2);
            frame.rect(
                UiRect::new((mark + 4) as f32, (top + 5) as f32, 1.0, 4.0),
                TEXT_2,
            );
        }
        bevel(
            frame,
            knob_x,
            y,
            knob_end,
            y + 16,
            KNOB_BEVEL.with_face(if hovered { KNOB_HI } else { KNOB }),
            LIP,
            false,
        );
    }

    /// `slider`: the 6-pixel track with its filled span, the zero notch and
    /// the 10x12 knob; `zero` is the notch's place or negative for none,
    /// `default` the knob the reset would return to. The nine arguments
    /// mirror the Java signature.
    #[allow(clippy::too_many_arguments)]
    pub fn slider(
        frame: &mut HudFrame,
        x1: i32,
        x2: i32,
        y: i32,
        value: f32,
        zero: f32,
        default: f32,
        hovered: bool,
        dragging: bool,
    ) {
        outline(frame, x1, y, x2, y + TRACK_H, WINDOW_EDGE);
        let span = (x2 - x1 - 10).max(1) as f32;
        let value_x = x1 as f32 + 5.0 + value * span;
        let zero_x = if zero < 0.0 {
            (x1 + 1) as f32
        } else {
            x1 as f32 + 5.0 + zero * span
        };
        let left = x1 + 1;
        let right = x2 - 1;
        let start = left.max(right.min(value_x.min(zero_x).round() as i32));
        let end = left.max(right.min(value_x.max(zero_x).round() as i32));
        track_part(frame, left, start, y);
        track_part(frame, start, end, y);
        track_part(frame, end, right, y);
        let notch = (x1 as f32 + 5.0 + default * span).round() as i32;
        frame.rect(
            UiRect::new(notch as f32, (y - 4) as f32, 1.0, 3.0),
            GOLD_600,
        );
        let knob = (value_x - 5.0).round() as i32;
        let knob_top = y + 3 - THUMB_H;
        if dragging {
            outline(
                frame,
                knob - 1,
                knob_top + 1,
                knob + THUMB_W + 1,
                knob_top + THUMB_H + 1,
                GOLD_200,
            );
        }
        bevel(
            frame,
            knob,
            knob_top,
            knob + THUMB_W,
            knob_top + THUMB_H,
            KNOB_BEVEL.with_face(if !hovered && !dragging { KNOB } else { KNOB_HI }),
            LIP,
            dragging,
        );
    }

    fn track_part(frame: &mut HudFrame, x1: i32, x2: i32, y: i32) {
        if x2 > x1 {
            frame.rect(
                UiRect::new(x1 as f32, (y + 1) as f32, (x2 - x1) as f32, 1.0),
                TRACK_LIGHT,
            );
            frame.rect(
                UiRect::new(
                    x1 as f32,
                    (y + 2) as f32,
                    (x2 - x1) as f32,
                    (TRACK_H - 3) as f32,
                ),
                TRACK,
            );
        }
    }

    /// `scrollbar`: the well and the thumb. The eight arguments mirror the
    /// Java signature.
    #[allow(clippy::too_many_arguments)]
    pub fn scrollbar(
        frame: &mut HudFrame,
        x: i32,
        top: i32,
        bottom: i32,
        width: i32,
        thumb_top: i32,
        thumb_bottom: i32,
        dragging: bool,
    ) {
        frame.rect(
            UiRect::new(x as f32, top as f32, width as f32, (bottom - top) as f32),
            WELL,
        );
        frame.rect(
            UiRect::new(
                (x + 1) as f32,
                thumb_top as f32,
                (width - 2) as f32,
                (thumb_bottom - thumb_top) as f32,
            ),
            if dragging {
                SCROLL_THUMB_HOVER
            } else {
                SCROLL_THUMB
            },
        );
    }

    /// `tab`: a left-hand navigation tab, the shown one picked out in gold.
    /// The eight arguments mirror the Java signature.
    #[allow(clippy::too_many_arguments)]
    pub fn tab(
        frame: &mut HudFrame,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        label: &str,
        selected: bool,
        hovered: bool,
    ) {
        if selected {
            frame.rect(UiRect::new(x as f32, y as f32, w as f32, 1.0), GROOVE_LIGHT);
            frame.rect(
                UiRect::new(x as f32, (y + 1) as f32, w as f32, (h - 2) as f32),
                PANE,
            );
            frame.rect(
                UiRect::new(x as f32, (y + h - 1) as f32, w as f32, 1.0),
                SEP,
            );
            frame.rect(
                UiRect::new(x as f32, y as f32, TAB_BAR as f32, h as f32),
                GOLD_400,
            );
        } else if hovered {
            frame.rect(
                UiRect::new(x as f32, y as f32, w as f32, h as f32),
                ROW_HOVER,
            );
        }
        let text_x = x + 3 + 7;
        let text_y = y + (h - 8) / 2;
        frame.text(
            label,
            text_x as f32,
            text_y as f32,
            if !selected && !hovered {
                TEXT_2
            } else {
                TEXT_1
            },
        );
    }

    /// `keycap`: a little bevelled key with its label centred.
    pub fn keycap(frame: &mut HudFrame, x: i32, y: i32, w: i32, h: i32, label: &str) {
        bevel(
            frame,
            x,
            y,
            x + w,
            y + h,
            LIGHT,
            if h >= 14 { 1 } else { 0 },
            false,
        );
        let width = label.chars().count() as f32 * super::GLYPH_WIDTH;
        let bottom = y + h - 1 - if h >= 14 { 1 } else { 0 };
        let text_y = (y + 1) as f32 + ((bottom - y - 1 - 8) as f32) * 0.5;
        draw_ink(
            frame,
            label,
            (x as f32 + (w as f32 - width) * 0.5) as i32,
            text_y as i32,
            WINDOW_EDGE,
        );
    }

    /// `raised`: the emote wheel's tiles, two shadow layers then a bevel.
    pub fn raised(frame: &mut HudFrame, x1: i32, y1: i32, x2: i32, y2: i32, hovered: bool) {
        frame.rect(
            UiRect::new(
                (x1 + 4) as f32,
                (y1 + 4) as f32,
                (x2 - x1) as f32,
                (y2 - y1) as f32,
            ),
            SHADOW,
        );
        frame.rect(
            UiRect::new(
                (x1 + 2) as f32,
                (y1 + 2) as f32,
                (x2 - x1) as f32,
                (y2 - y1) as f32,
            ),
            SHADOW,
        );
        bevel(
            frame,
            x1,
            y1,
            x2,
            y2,
            if hovered { GOLD } else { LIGHT },
            LIP,
            false,
        );
    }
}

/// The colours every screen draws with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiTheme {
    /// The wash behind a panel.
    pub dim: u32,
    /// A panel's fill.
    pub panel: u32,
    /// A panel's and a row's border.
    pub border: u32,
    /// A panel's title.
    pub title: u32,
    /// Ordinary text.
    pub text: u32,
    /// A value on the right of a row.
    pub value: u32,
    /// Hints and footers.
    pub muted: u32,
    /// The selected row, wedge or swatch.
    pub selected: u32,
    /// Things that ask for a press.
    pub accent: u32,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            dim: gold::BACKDROP,
            panel: gold::PANE,
            border: gold::WINDOW_EDGE,
            title: gold::TEXT_1,
            text: gold::TEXT_1,
            value: gold::TEXT_2,
            muted: gold::TEXT_3,
            selected: gold::ROW_HOVER,
            accent: gold::GOLD_400,
        }
    }
}

/// The emote wheel's key, unbound until the player picks one.
pub const EMOTE_WHEEL_KEY: &str = "emote_wheel";
/// The emote wheel's default key.
pub const DEFAULT_EMOTE_WHEEL_KEY: &str = "key.keyboard.unknown";

// ---------------------------------------------------------------------------
// Emotes
// ---------------------------------------------------------------------------

/// One emote: what the wheel shows, the face it wears and the hands it moves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Emote {
    /// Its id: the settings name of its key, the name its cue is filed under.
    pub id: &'static str,
    /// The wheel's label.
    pub label: &'static str,
    /// The face it wears.
    pub expression: FaceExpression,
    /// The hand action it plays.
    pub action: ItemAction,
}

impl Emote {
    /// The cue this emote's face wears.
    pub fn cue(&self) -> FaceCue {
        FaceCue {
            seconds: EMOTE_SECONDS,
            priority: 5,
            ..FaceCue::built_in(self.expression)
        }
    }
}

/// How long an emote's face lasts, seconds.
pub const EMOTE_SECONDS: f32 = 2.4;

/// The emotes on the wheel, in wheel order from the top clockwise.
pub fn emotes() -> Vec<Emote> {
    vec![
        Emote {
            id: "wave",
            label: "Wave",
            expression: FaceExpression::Emote,
            action: ItemAction::Use,
        },
        Emote {
            id: "clap",
            label: "Clap",
            expression: FaceExpression::Happy,
            action: ItemAction::Swing,
        },
        Emote {
            id: "dance",
            label: "Dance",
            expression: FaceExpression::Happy,
            action: ItemAction::Swing,
        },
        Emote {
            id: "bow",
            label: "Bow",
            expression: FaceExpression::Emote,
            action: ItemAction::Use,
        },
        Emote {
            id: "cry",
            label: "Cry",
            expression: FaceExpression::Hurt,
            action: ItemAction::Use,
        },
        Emote {
            id: "laugh",
            label: "Laugh",
            expression: FaceExpression::Happy,
            action: ItemAction::Swing,
        },
        Emote {
            id: "point",
            label: "Point",
            expression: FaceExpression::Surprised,
            action: ItemAction::Use,
        },
        Emote {
            id: "shrug",
            label: "Shrug",
            expression: FaceExpression::Emote,
            action: ItemAction::Use,
        },
    ]
}

/// The most emotes one wheel's keybind slots carry.
pub const MAX_EMOTES: usize = 8;
/// How many tiles one wheel page shows, the Java wheel's four directions.
pub const WHEEL_PER_PAGE: usize = 4;

/// The radial emote wheel: which emote is picked, which page is showing, and
/// whether it is up. One page holds four tiles, one per direction round the
/// middle, the Java `EmoteWheelScreen`.
#[derive(Debug, Clone, PartialEq)]
pub struct EmoteWheelState {
    open: bool,
    page: usize,
    selected: usize,
    emotes: Vec<Emote>,
}

impl Default for EmoteWheelState {
    fn default() -> Self {
        Self::new()
    }
}

impl EmoteWheelState {
    /// The wheel with the built-in emotes.
    pub fn new() -> Self {
        Self::with_emotes(emotes())
    }

    /// The wheel over a given emote list (at most [`MAX_EMOTES`]).
    pub fn with_emotes(mut emotes: Vec<Emote>) -> Self {
        emotes.truncate(MAX_EMOTES);
        Self {
            open: false,
            page: 0,
            selected: 0,
            emotes,
        }
    }

    /// Whether the wheel is up.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Puts the wheel up, keeping the selection and its page.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Takes the wheel down and returns the emote it was pointing at, to play.
    pub fn close(&mut self) -> Option<Emote> {
        if !self.open {
            return None;
        }
        self.open = false;
        self.emote()
    }

    /// Closes without playing anything.
    pub fn cancel(&mut self) {
        self.open = false;
    }

    /// The wheel's emotes.
    pub fn emotes(&self) -> &[Emote] {
        &self.emotes
    }

    /// How many pages the wheel has, at least one, the Java
    /// `max(1, (entries + 3) / 4)`.
    pub fn pages(&self) -> usize {
        self.emotes.len().div_ceil(WHEEL_PER_PAGE).max(1)
    }

    /// The page being shown.
    pub fn page(&self) -> usize {
        self.page
    }

    /// Shows `page`, clamped to the wheel's pages.
    pub fn set_page(&mut self, page: usize) -> usize {
        self.page = page.min(self.pages() - 1);
        self.page
    }

    /// The selected emote's index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The selected emote.
    pub fn emote(&self) -> Option<Emote> {
        self.emotes.get(self.selected).copied()
    }

    /// Selects an emote by index (wrapping; nothing when the wheel is empty)
    /// and turns to its page.
    pub fn select(&mut self, index: usize) -> usize {
        if self.emotes.is_empty() {
            self.selected = 0;
        } else {
            self.selected = index % self.emotes.len();
            self.page = self.selected / WHEEL_PER_PAGE;
        }
        self.selected
    }

    /// Steps the selection round the wheel by `delta` emotes.
    pub fn nudge(&mut self, delta: i32) -> usize {
        if self.emotes.is_empty() {
            return 0;
        }
        let count = self.emotes.len() as i32;
        let at = (self.selected as i32 + delta).rem_euclid(count);
        self.select(at as usize)
    }

    /// Turns one page back or forward, wrapping, the Java paging buttons.
    pub fn turn_page(&mut self, delta: i32) -> usize {
        let pages = self.pages() as i32;
        self.page = (self.page as i32 + delta).rem_euclid(pages) as usize;
        self.page
    }

    /// Selects the tile a pointer at `(x, y)` on the screen lands on, the
    /// Java tiles' hit boxes; `None` keeps the selection.
    pub fn pointing_at(&mut self, layout: &UiLayout, x: f32, y: f32) -> Option<usize> {
        for tile in 0..WHEEL_PER_PAGE {
            let index = self.page * WHEEL_PER_PAGE + tile;
            if index >= self.emotes.len() {
                break;
            }
            if self.tile_rect(layout, tile).contains(x, y) {
                self.selected = index;
                return Some(index);
            }
        }
        None
    }

    /// Selects the tile a pointer offset `(dx, dy)` from the middle points
    /// at: up, right, down, left, the Java tiles' order.
    pub fn pointing(&mut self, dx: f32, dy: f32) -> usize {
        if !dx.is_finite() || !dy.is_finite() || (dx == 0.0 && dy == 0.0) {
            return self.selected;
        }
        let tile = if dy <= -dx.abs() && dy < 0.0 {
            0
        } else if dx >= dy.abs() && dx > 0.0 {
            1
        } else if dy >= dx.abs() && dy > 0.0 {
            2
        } else if dx <= -dy.abs() && dx < 0.0 {
            3
        } else {
            return self.selected;
        };
        let index = self.page * WHEEL_PER_PAGE + tile;
        if index < self.emotes.len() {
            self.selected = index;
        }
        self.selected
    }

    /// The tile rect of the `tile`th tile of the page `index`'s emote sits
    /// on, the Java tile geometry: the four directions round the middle,
    /// sized from the screen.
    pub fn slot_rect(&self, layout: &UiLayout, index: usize) -> UiRect {
        self.tile_rect(layout, index % WHEEL_PER_PAGE)
    }

    /// The rect of tile `tile` (0 up, 1 right, 2 down, 3 left) of the page
    /// being shown, the Java `init`'s bounds.
    pub fn tile_rect(&self, layout: &UiLayout, tile: usize) -> UiRect {
        let [cx, cy] = layout.center();
        let j = cx;
        let k = cy + 3.0;
        let l = WHEEL_TILE_W.min((layout.width - 36.0) / 3.0);
        let i1 = ((layout.height - 100.0) / 3.0).clamp(34.0, WHEEL_TILE_H);
        let j1 = l + 8.0;
        let k1 = i1 + 12.0;
        let (x, y) = match tile {
            0 => (j, k - k1),
            1 => (j + j1, k),
            2 => (j, k + k1),
            _ => (j - j1, k),
        };
        UiRect::new(x - l * 0.5, y - i1 * 0.5, l, i1)
    }

    /// Draws the open wheel: the wash, the page's tiles with their labels
    /// and keycap chips, the close tile in the middle, the paging buttons
    /// and the page counter.
    pub fn draw(&self, layout: &UiLayout, theme: &UiTheme, frame: &mut HudFrame) {
        if !self.open {
            return;
        }
        let _ = theme;
        frame.rect(
            UiRect::new(0.0, 0.0, layout.width, layout.height),
            crate::ui::gold::WHEEL_WASH,
        );
        let [cx, cy] = layout.center();
        let j = cx;
        let k = cy + 3.0;
        for tile in 0..WHEEL_PER_PAGE {
            let index = self.page * WHEEL_PER_PAGE + tile;
            let Some(emote) = self.emotes.get(index) else {
                break;
            };
            let slot = self.tile_rect(layout, tile);
            let hovered = index == self.selected;
            crate::ui::gold::raised(
                frame,
                slot.x as i32,
                slot.y as i32,
                slot.right() as i32,
                slot.bottom() as i32,
                hovered,
            );
            let ink = if hovered {
                crate::ui::gold::GOLD_INK
            } else {
                crate::ui::gold::WINDOW_EDGE
            };
            let width = emote.label.chars().count() as f32 * GLYPH_WIDTH;
            frame.text(
                emote.label,
                slot.center()[0] - width * 0.5,
                slot.y + 7.0,
                ink,
            );
            // The keycap chip under the label, the Java keybind chip.
            let chip_w = 26.0;
            let chip_x = slot.center()[0] - chip_w * 0.5;
            let chip_y = slot.bottom() - 21.0;
            crate::ui::gold::outline(
                frame,
                chip_x as i32,
                chip_y as i32,
                (chip_x + chip_w) as i32,
                (chip_y + 13.0) as i32,
                crate::ui::gold::WINDOW_EDGE,
            );
            frame.rect(
                UiRect::new(chip_x + 1.0, chip_y + 1.0, chip_w - 2.0, 11.0),
                if hovered {
                    crate::ui::gold::GOLD_700
                } else {
                    crate::ui::gold::WELL
                },
            );
            frame.text("key", chip_x + 4.0, chip_y + 3.0, crate::ui::gold::TEXT_1);
        }
        // The close tile in the middle.
        crate::ui::gold::raised(
            frame,
            (j - 11.0) as i32,
            (k - 11.0) as i32,
            (j + 11.0) as i32,
            (k + 11.0) as i32,
            false,
        );
        frame.text(
            "X",
            j - GLYPH_WIDTH * 0.5,
            k - 5.0,
            crate::ui::gold::WINDOW_EDGE,
        );
        // The paging buttons and the counter, when there is more than one
        // page.
        if self.pages() > 1 {
            crate::ui::gold::button(
                frame,
                (j - 76.0) as i32,
                (layout.height - 32.0) as i32,
                28,
                22,
                "punchy_moves.ui.previous",
                false,
                false,
                true,
            );
            crate::ui::gold::button(
                frame,
                (j + 48.0) as i32,
                (layout.height - 32.0) as i32,
                28,
                22,
                "punchy_moves.ui.next_page",
                false,
                false,
                true,
            );
            let counter = format!("{} / {}", self.page + 1, self.pages());
            let width = counter.chars().count() as f32 * GLYPH_WIDTH;
            frame.text(
                counter,
                layout.width * 0.5 - width * 0.5,
                layout.height - 25.0,
                crate::ui::gold::TEXT_1,
            );
        }
        if self.emotes.is_empty() {
            let text = "punchy_moves.ui.no_emotes";
            let width = text.chars().count() as f32 * GLYPH_WIDTH;
            frame.text(
                text,
                layout.width * 0.5 - width * 0.5,
                layout.height * 0.5 + 28.0,
                crate::ui::gold::TEXT_1,
            );
        }
    }
}

/// One wheel tile's widest, the Java `min(138, (width - 36) / 3)`.
pub const WHEEL_TILE_W: f32 = 138.0;
/// One wheel tile's tallest, the Java `min(52, max(34, (height - 100) / 3))`.
pub const WHEEL_TILE_H: f32 = 52.0;

/// The keys each emote is bound to: one slot per emote on the wheel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EmoteKeybindManager {
    slots: [Option<String>; MAX_EMOTES],
}

/// The setting name of slot `index`'s key.
pub fn slot_setting(index: usize) -> String {
    format!("emote_{}", index + 1)
}

impl EmoteKeybindManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the bindings from stored values (`get` reads one key).
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let mut manager = Self::default();
        for index in 0..MAX_EMOTES {
            if let Some(key) = get(&slot_setting(index))
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty() && v != "key.keyboard.unknown")
            {
                manager.bind(index, &key);
            }
        }
        manager
    }

    /// The key of slot `index`, if it has one.
    pub fn slot(&self, index: usize) -> Option<&str> {
        self.slots.get(index).and_then(|slot| slot.as_deref())
    }

    /// Binds slot `index` to `key`; an unknown index or a blank key unbinds
    /// and returns `false` only for the index.
    pub fn bind(&mut self, index: usize, key: &str) -> bool {
        let Some(slot) = self.slots.get_mut(index) else {
            return false;
        };
        let key = key.trim();
        *slot = if key.is_empty() || key == "key.keyboard.unknown" {
            None
        } else {
            Some(key.to_string())
        };
        true
    }

    /// Unbinds slot `index`.
    pub fn unbind(&mut self, index: usize) {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = None;
        }
    }

    /// The slot bound to `key`, if any.
    pub fn index_of_key(&self, key: &str) -> Option<usize> {
        self.slots
            .iter()
            .position(|slot| slot.as_deref() == Some(key))
    }

    /// One line per slot, for a settings page.
    pub fn labels(&self) -> Vec<String> {
        (0..MAX_EMOTES)
            .map(|index| {
                let emote = emotes().get(index).copied();
                let name = emote.map_or("Emote", |e| e.label);
                let key = self.slot(index).unwrap_or("unbound");
                format!("{}: {}", name, key)
            })
            .collect()
    }

    /// What to store, one line per slot; an unbound slot stores the
    /// unbound key so a cleared binding stays cleared.
    pub fn stored(&self) -> Vec<(String, String)> {
        (0..MAX_EMOTES)
            .map(|index| {
                (
                    slot_setting(index),
                    self.slot(index)
                        .unwrap_or("key.keyboard.unknown")
                        .to_string(),
                )
            })
            .collect()
    }
}

/// Plays `emote` on `entity`: the face wears its cue and the hands play its
/// action. `false` when the engine has no state for that entity.
pub fn play_emote(entity: Option<u64>, emote: Emote) -> bool {
    let Some(entity) = entity else {
        return false;
    };
    crate::engine::with_engine(|engine| {
        let Some(state) = engine.state_mut(entity) else {
            return false;
        };
        state.face.play_cue(emote.cue());
        state.items.play_emote(emote.action);
        true
    })
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

/// What a click on a screen asks the game to do beyond the screen's own state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiRequest {
    /// Nothing beyond the screen's own state.
    None,
    /// Restart the tutorial from its first step.
    ReplayTutorial,
    /// Open the mod's keys so `name` can be rebound.
    RebindKey(&'static str),
    /// Play the wheel's emote at `index` on the local player.
    PlayEmote(usize),
    /// Close every screen and go back to the game.
    CloseAll,
}

// ---------------------------------------------------------------------------
// The shoulder camera's screen
// ---------------------------------------------------------------------------

/// One page of the shoulder camera's screen, the Java `Page` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraPage {
    /// The position sliders and the on/off row.
    #[default]
    Camera,
    /// The feature toggles.
    Misc,
    /// The shortcuts and their keys.
    Shortcuts,
}

impl CameraPage {
    /// The pages, in navigation order.
    pub const ALL: [CameraPage; 3] = [CameraPage::Camera, CameraPage::Misc, CameraPage::Shortcuts];

    /// The page's label key, the Java `Page.text()`'s translation key.
    pub fn label(self) -> &'static str {
        match self {
            CameraPage::Camera => "punchy_moves.ui.camera",
            CameraPage::Misc => "punchy_moves.ui.misc",
            CameraPage::Shortcuts => "punchy_moves.ui.shortcuts",
        }
    }

    /// This page's place in [`CameraPage::ALL`].
    pub fn index(self) -> usize {
        CameraPage::ALL
            .iter()
            .position(|page| *page == self)
            .unwrap_or(0)
    }

    /// The page below (wrapping).
    pub fn next(self) -> CameraPage {
        CameraPage::ALL[(self.index() + 1) % CameraPage::ALL.len()]
    }

    /// The page above (wrapping).
    pub fn previous(self) -> CameraPage {
        CameraPage::ALL[(self.index() + CameraPage::ALL.len() - 1) % CameraPage::ALL.len()]
    }
}

/// One slider of the camera page: its label key, its range, the value the
/// reset returns to, and the setting it writes, the Java `addSlider` rows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraSliderRow {
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub setting: usize,
}

/// The camera page's sliders, in screen order.
pub const CAMERA_SLIDERS: [CameraSliderRow; 6] = [
    CameraSliderRow {
        label: "punchy_moves.ui.horizontal",
        min: -2.0,
        max: 2.0,
        default: 1.17,
        setting: crate::camera::HORIZONTAL_INDEX,
    },
    CameraSliderRow {
        label: "punchy_moves.ui.vertical",
        min: -1.5,
        max: 2.0,
        default: -0.11,
        setting: crate::camera::VERTICAL_INDEX,
    },
    CameraSliderRow {
        label: "punchy_moves.ui.depth",
        min: 1.0,
        max: 8.0,
        default: 2.5,
        setting: crate::camera::DEPTH_INDEX,
    },
    CameraSliderRow {
        label: "punchy_moves.ui.fov",
        min: 30.0,
        max: 110.0,
        default: 70.0,
        setting: crate::camera::FOV_INDEX,
    },
    CameraSliderRow {
        label: "punchy_moves.ui.rotation_yaw",
        min: -30.0,
        max: 30.0,
        default: 0.0,
        setting: crate::camera::YAW_INDEX,
    },
    CameraSliderRow {
        label: "punchy_moves.ui.rotation_pitch",
        min: -30.0,
        max: 30.0,
        default: 0.0,
        setting: crate::camera::PITCH_INDEX,
    },
];

/// One feature toggle of the misc page: its `punchy_moves.<key>.title` key
/// and the setting it flips, the Java `addFeature` rows. The two compat rows
/// (`better_combat`, `spatial_gui`) have no Horizon counterpart, so they
/// carry no setting and a click on them does nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeatureRow {
    pub key: &'static str,
    pub setting: Option<usize>,
}

/// The misc page's features, in screen order.
pub const FEATURES: [FeatureRow; 10] = [
    FeatureRow {
        key: "misc.fence",
        setting: Some(crate::camera::FENCE_INDEX),
    },
    FeatureRow {
        key: "misc.ladder_camera",
        setting: Some(crate::camera::LADDER_INDEX),
    },
    FeatureRow {
        key: "misc.proximity",
        setting: Some(crate::camera::PROXIMITY_INDEX),
    },
    FeatureRow {
        key: "misc.elytra",
        setting: Some(crate::camera::ELYTRA_INDEX),
    },
    FeatureRow {
        key: "misc.map_camera",
        setting: Some(crate::camera::MAP_INDEX),
    },
    FeatureRow {
        key: "misc.bow_and_crossbow_camera_fix",
        setting: Some(crate::camera::BOW_AND_CROSSBOW_INDEX),
    },
    FeatureRow {
        key: "misc.spear_and_trident_camera_fix",
        setting: Some(crate::camera::SPEAR_AND_TRIDENT_INDEX),
    },
    FeatureRow {
        key: "misc.ladder_lock",
        setting: Some(crate::camera::LADDER_BODY_LOCK_INDEX),
    },
    FeatureRow {
        key: "misc.better_combat",
        setting: None,
    },
    FeatureRow {
        key: "misc.spatial_gui",
        setting: None,
    },
];

/// How many features show at once, the Java `MAX_VISIBLE_FEATURES`.
pub const MAX_VISIBLE_FEATURES: usize = 4;
/// How many camera sliders show at once, the Java
/// `MAX_VISIBLE_CAMERA_SLIDERS`.
pub const MAX_VISIBLE_CAMERA_SLIDERS: usize = 4;
/// How many shortcuts show at once, the Java `MAX_VISIBLE_SHORTCUTS`.
pub const MAX_VISIBLE_SHORTCUTS: usize = 10;
/// One feature row's height, the Java `FEATURE_HEIGHT`.
pub const FEATURE_HEIGHT: f32 = 42.0;
/// The gap between feature rows, the Java `FEATURE_GAP`.
pub const FEATURE_GAP: f32 = 4.0;
/// One camera slider's height, the Java `CAMERA_SLIDER_HEIGHT`.
pub const CAMERA_SLIDER_HEIGHT: f32 = 34.0;
/// The gap between camera sliders, the Java `CAMERA_SLIDER_GAP`.
pub const CAMERA_SLIDER_GAP: f32 = 2.0;
/// The scrollbar's gutter, the Java `SCROLLBAR_GUTTER`.
pub const SCROLLBAR_GUTTER: f32 = 14.0;
/// The window's title bar, the Java `TITLE_H`.
pub const CAMERA_TITLE_H: i32 = 33;

/// One settable row the screen shows: its label key, its value as shown, and
/// what a click does.
#[derive(Debug, Clone, PartialEq)]
pub struct Stepper {
    pub label: String,
    pub value: String,
    pub setting: Option<usize>,
}

/// The shoulder camera's screen: its page, its focus and its scroll positions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShoulderCameraUiState {
    open: bool,
    page: CameraPage,
    focus: usize,
    slider_scroll: usize,
    feature_scroll: usize,
    shortcut_scroll: usize,
}

impl Default for ShoulderCameraUiState {
    fn default() -> Self {
        Self {
            open: false,
            page: CameraPage::Camera,
            focus: 0,
            slider_scroll: 0,
            feature_scroll: 0,
            shortcut_scroll: 0,
        }
    }
}

impl ShoulderCameraUiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn toggle(&mut self) -> bool {
        self.open = !self.open;
        self.open
    }

    /// The page being shown.
    pub fn page(&self) -> CameraPage {
        self.page
    }

    /// Shows `page`, putting its focus back on the first row.
    pub fn select(&mut self, page: CameraPage) -> CameraPage {
        self.page = page;
        self.focus = 0;
        self.page
    }

    /// Shows the next page.
    pub fn next_tab(&mut self) -> CameraPage {
        let page = self.page.next();
        self.select(page)
    }

    /// Shows the previous page.
    pub fn previous_tab(&mut self) -> CameraPage {
        let page = self.page.previous();
        self.select(page)
    }

    /// The focused row.
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// Focuses a row (clamped to the page's rows when they are known).
    pub fn focus_at(&mut self, index: usize, rows: usize) -> usize {
        self.focus = if rows == 0 { 0 } else { index.min(rows - 1) };
        self.focus
    }

    /// Steps the focus down or up the page's rows, wrapping.
    pub fn nudge_focus(&mut self, delta: i32, rows: usize) -> usize {
        if rows == 0 {
            self.focus = 0;
        } else {
            let rows = rows as i32;
            self.focus = (self.focus as i32 + delta).rem_euclid(rows) as usize;
        }
        self.focus
    }

    /// The camera page's scroll, and its furthest value.
    pub fn slider_scroll(&self) -> usize {
        self.slider_scroll
    }

    pub fn slider_scroll_max(&self) -> usize {
        CAMERA_SLIDERS
            .len()
            .saturating_sub(MAX_VISIBLE_CAMERA_SLIDERS)
    }

    /// The misc page's scroll, and its furthest value.
    pub fn feature_scroll(&self) -> usize {
        self.feature_scroll
    }

    pub fn feature_scroll_max(&self) -> usize {
        FEATURES.len().saturating_sub(MAX_VISIBLE_FEATURES)
    }

    /// The shortcuts page's scroll, and its furthest value.
    pub fn shortcut_scroll(&self) -> usize {
        self.shortcut_scroll
    }

    pub fn shortcut_scroll_max(&self) -> usize {
        Self::shortcut_count().saturating_sub(MAX_VISIBLE_SHORTCUTS)
    }

    /// The shortcuts the screen lists: the four fixed keys, then one per
    /// emote slot, the Java `EmoteKeybindManager.shortcuts`.
    pub fn shortcut_count() -> usize {
        4 + MAX_EMOTES
    }

    /// Scrolls a page by `delta` rows, clamped, the Java mouse wheel.
    pub fn scroll(&mut self, delta: i32) {
        match self.page {
            CameraPage::Camera => {
                let max = self.slider_scroll_max();
                self.slider_scroll = clamp_scroll(self.slider_scroll, delta, max);
            }
            CameraPage::Misc => {
                let max = self.feature_scroll_max();
                self.feature_scroll = clamp_scroll(self.feature_scroll, delta, max);
            }
            CameraPage::Shortcuts => {
                let max = self.shortcut_scroll_max();
                self.shortcut_scroll = clamp_scroll(self.shortcut_scroll, delta, max);
            }
        }
    }

    /// The rows of the page being shown.
    pub fn steppers(&self, settings: &ShoulderCameraSettings) -> Vec<Stepper> {
        let on = |b: bool| {
            if b {
                "punchy_moves.ui.on"
            } else {
                "punchy_moves.ui.off"
            }
        };
        let row = |label: &str, value: String, setting: Option<usize>| Stepper {
            label: label.to_string(),
            value,
            setting,
        };
        match self.page {
            CameraPage::Camera => {
                let mut rows = vec![row(
                    "punchy_moves.ui.custom_camera",
                    on(settings.enabled).into(),
                    Some(crate::camera::ENABLED_INDEX),
                )];
                for slider in CAMERA_SLIDERS {
                    let value = match slider.setting {
                        crate::camera::HORIZONTAL_INDEX => format!("{:.2}", settings.horizontal),
                        crate::camera::VERTICAL_INDEX => format!("{:.2}", settings.vertical),
                        crate::camera::DEPTH_INDEX => format!("{:.2}", settings.depth),
                        crate::camera::FOV_INDEX => format!("{:.0}", settings.fov),
                        crate::camera::YAW_INDEX => format!("{:.0}", settings.rotation_yaw),
                        _ => format!("{:.0}", settings.rotation_pitch),
                    };
                    rows.push(row(slider.label, value, Some(slider.setting)));
                }
                rows
            }
            CameraPage::Misc => FEATURES
                .iter()
                .map(|feature| {
                    let enabled = feature
                        .setting
                        .is_some_and(|index| settings.click_value(index));
                    row(
                        &format!("punchy_moves.{}.title", feature.key),
                        on(enabled).into(),
                        feature.setting,
                    )
                })
                .collect(),
            CameraPage::Shortcuts => vec![
                row("punchy_moves.key.camera_toggle_v", "rebind".into(), None),
                row("punchy_moves.key.camera_editor", "rebind".into(), None),
                row("punchy_moves.key.free_camera", "rebind".into(), None),
                row("punchy_moves.key.emote_menu", "rebind".into(), None),
            ],
        }
    }

    /// The rows of the shortcuts page with the emote slots' keys read from
    /// `keys`: the four fixed shortcuts, then one row per emote slot, the
    /// Java `EmoteKeybindManager.shortcuts`.
    pub fn shortcuts(&self, keys: &EmoteKeybindManager) -> Vec<Stepper> {
        let mut rows = self.steppers(&ShoulderCameraSettings::default());
        rows.truncate(4);
        for index in 0..MAX_EMOTES {
            let emote = emotes().get(index).copied();
            let name = emote.map_or("punchy_moves.ui.emote", |e| e.label);
            rows.push(Stepper {
                label: format!("punchy_moves.ui.emote {}", name),
                value: keys.slot(index).unwrap_or("unbound").to_string(),
                setting: None,
            });
        }
        rows
    }

    /// A click on row `index` of the page being shown: changes the setting or
    /// asks the game to do something only it can.
    pub fn click(&mut self, settings: &mut ShoulderCameraSettings, index: usize) -> UiRequest {
        let rows = self.steppers(settings);
        let Some(row) = rows.get(index) else {
            return UiRequest::None;
        };
        self.focus = index;
        if let Some(setting) = row.setting {
            settings.click(setting);
        }
        match self.page {
            CameraPage::Shortcuts => match index {
                0 => UiRequest::RebindKey(crate::camera::TOGGLE_CAMERA),
                1 => UiRequest::RebindKey(crate::camera::TOGGLE_EDITOR),
                2 => UiRequest::RebindKey(crate::camera::TOGGLE_ORBIT),
                _ => UiRequest::RebindKey(EMOTE_WHEEL_KEY),
            },
            _ => UiRequest::None,
        }
    }

    /// Draws the page: the window with its rail and divider, the page header,
    /// the rows, and the scrollbars.
    pub fn draw(
        &self,
        layout: &UiLayout,
        settings: &ShoulderCameraSettings,
        theme: &UiTheme,
        frame: &mut HudFrame,
    ) {
        if !self.open {
            return;
        }
        let rows = self.steppers(settings);
        let panel = frame.panel(layout, PANEL_WIDTH, PANEL_HEIGHT, theme);
        let title_h = 33.0;
        crate::ui::gold::window(
            frame,
            panel.x as i32,
            panel.y as i32,
            panel.right() as i32,
            panel.bottom() as i32,
            CAMERA_TITLE_H,
            crate::ui::gold::PANE,
        );
        // The left rail and the divider groove.
        let divider = panel.x + 46.0;
        frame.rect(
            UiRect::new(
                panel.x + 1.0,
                panel.y + title_h,
                divider - panel.x - 1.0,
                panel.h - title_h - 1.0,
            ),
            crate::ui::gold::RAIL,
        );
        crate::ui::gold::vgroove(
            frame,
            divider as i32,
            panel.y as i32 + CAMERA_TITLE_H,
            panel.bottom() as i32 - 1,
        );
        // The navigation tabs, down the rail.
        let tab_h = 30.0f32;
        for (index, page) in CameraPage::ALL.iter().enumerate() {
            let y = panel.y + 61.0 + index as f32 * 34.0;
            crate::ui::gold::tab(
                frame,
                (panel.x + 6.0) as i32,
                y as i32,
                (divider - panel.x - 12.0) as i32,
                tab_h as i32,
                page.label(),
                *page == self.page,
                index == self.focus && *page != self.page,
            );
        }
        // The page header and the groove under it.
        frame.text(
            self.page.label(),
            divider + 12.0,
            panel.y + 46.0,
            crate::ui::gold::TEXT_1,
        );
        crate::ui::gold::groove(
            frame,
            (divider + 12.0) as i32,
            panel.right() as i32 - 12,
            panel.y as i32 + 61,
        );
        // The rows of the page, from its scroll.
        let content_x = divider + 12.0;
        let content_w = panel.right() - content_x - 12.0;
        let visible = match self.page {
            CameraPage::Camera => MAX_VISIBLE_CAMERA_SLIDERS,
            CameraPage::Misc => MAX_VISIBLE_FEATURES,
            CameraPage::Shortcuts => MAX_VISIBLE_SHORTCUTS,
        };
        let scroll = match self.page {
            CameraPage::Camera => self.slider_scroll,
            CameraPage::Misc => self.feature_scroll,
            CameraPage::Shortcuts => self.shortcut_scroll,
        };
        let row_h = match self.page {
            CameraPage::Camera => CAMERA_SLIDER_HEIGHT + CAMERA_SLIDER_GAP,
            CameraPage::Misc => FEATURE_HEIGHT + FEATURE_GAP,
            CameraPage::Shortcuts => 16.0,
        };
        let mut y = panel.y + 70.0 + 44.0;
        for (index, row) in rows.iter().enumerate().skip(scroll).take(visible) {
            let focused = index == self.focus;
            if self.page == CameraPage::Misc {
                if focused {
                    frame.rect(
                        UiRect::new(content_x, y, content_w, FEATURE_HEIGHT),
                        crate::ui::gold::ROW_HOVER,
                    );
                }
                frame.text(
                    row.label.as_str(),
                    content_x + 8.0,
                    y + 8.0,
                    crate::ui::gold::TEXT_1,
                );
                frame.text(
                    row.value.as_str(),
                    content_x + 8.0,
                    y + 20.0,
                    crate::ui::gold::TEXT_2,
                );
                let toggle_x = (content_x + content_w - 30.0 - 8.0) as i32;
                let enabled = row.setting.is_some_and(|s| settings.click_value(s));
                crate::ui::gold::toggle(
                    frame,
                    toggle_x,
                    y as i32 + (FEATURE_HEIGHT as i32 - 16) / 2,
                    enabled,
                    focused,
                );
                crate::ui::gold::groove(
                    frame,
                    content_x as i32,
                    (content_x + content_w) as i32,
                    (y + FEATURE_HEIGHT + 1.0) as i32,
                );
            } else if self.page == CameraPage::Camera {
                frame.text(row.label.as_str(), content_x, y, crate::ui::gold::TEXT_1);
                frame.text(
                    row.value.as_str(),
                    content_x + content_w - 40.0,
                    y,
                    crate::ui::gold::TEXT_2,
                );
                if let Some(setting) = row.setting {
                    let slider = CAMERA_SLIDERS
                        .iter()
                        .find(|slider| slider.setting == setting);
                    if let Some(slider) = slider {
                        let value = (settings_value(settings, setting) - slider.min)
                            / (slider.max - slider.min);
                        let zero = if slider.min < 0.0 && slider.max > 0.0 {
                            -slider.min / (slider.max - slider.min)
                        } else {
                            -1.0
                        };
                        let default = (slider.default.clamp(slider.min, slider.max) - slider.min)
                            / (slider.max - slider.min);
                        crate::ui::gold::slider(
                            frame,
                            content_x as i32,
                            (content_x + content_w - SCROLLBAR_GUTTER) as i32,
                            (y + 20.0) as i32,
                            value.clamp(0.0, 1.0),
                            zero,
                            default,
                            focused,
                            false,
                        );
                    }
                }
            } else {
                if focused {
                    frame.rect(
                        UiRect::new(content_x, y, content_w, row_h),
                        crate::ui::gold::ROW_HOVER,
                    );
                }
                frame.text(
                    row.label.as_str(),
                    content_x + 4.0,
                    y + 3.0,
                    crate::ui::gold::TEXT_1,
                );
                crate::ui::gold::keycap(
                    frame,
                    (content_x + content_w * 0.62) as i32,
                    y as i32,
                    (content_w * 0.3) as i32,
                    14,
                    row.value.as_str(),
                );
            }
            y += row_h;
        }
        // The scrollbars, when a page scrolls.
        match self.page {
            CameraPage::Camera if self.slider_scroll_max() > 0 => {
                let (top, bottom, thumb) = scrollbar_geometry(
                    panel.y + 70.0 + 44.0,
                    panel.bottom() - 38.0,
                    CAMERA_SLIDERS.len(),
                    MAX_VISIBLE_CAMERA_SLIDERS,
                    self.slider_scroll,
                );
                crate::ui::gold::scrollbar(
                    frame,
                    (content_x + content_w - 9.0) as i32,
                    top,
                    bottom,
                    8,
                    thumb,
                    thumb
                        + thumb_height(
                            CAMERA_SLIDERS.len(),
                            MAX_VISIBLE_CAMERA_SLIDERS,
                            bottom - top,
                        ),
                    false,
                );
            }
            CameraPage::Misc if self.feature_scroll_max() > 0 => {
                let (top, bottom, thumb) = scrollbar_geometry(
                    panel.y + 70.0,
                    panel.bottom() - 38.0,
                    FEATURES.len(),
                    MAX_VISIBLE_FEATURES,
                    self.feature_scroll,
                );
                crate::ui::gold::scrollbar(
                    frame,
                    (content_x + content_w - 9.0) as i32,
                    top,
                    bottom,
                    8,
                    thumb,
                    thumb + thumb_height(FEATURES.len(), MAX_VISIBLE_FEATURES, bottom - top),
                    false,
                );
            }
            _ => {}
        }
        frame.footer("punchy_moves.ui.settings_hint", panel, theme);
    }
}

/// The value a settings row reads, for the screen's rows.
fn settings_value(settings: &ShoulderCameraSettings, index: usize) -> f32 {
    match index {
        crate::camera::HORIZONTAL_INDEX => settings.horizontal,
        crate::camera::VERTICAL_INDEX => settings.vertical,
        crate::camera::DEPTH_INDEX => settings.depth,
        crate::camera::FOV_INDEX => settings.fov,
        crate::camera::YAW_INDEX => settings.rotation_yaw,
        crate::camera::PITCH_INDEX => settings.rotation_pitch,
        _ => 0.0,
    }
}

/// Whether the settings row `index` reads on, for the toggle rows.
trait ClickValue {
    fn click_value(&self, index: usize) -> bool;
}

impl ClickValue for ShoulderCameraSettings {
    fn click_value(&self, index: usize) -> bool {
        match index {
            crate::camera::ENABLED_INDEX => self.enabled,
            crate::camera::FENCE_INDEX => self.fence_camera,
            crate::camera::LADDER_INDEX => self.ladder_camera,
            crate::camera::PROXIMITY_INDEX => self.proximity_camera,
            crate::camera::ELYTRA_INDEX => self.elytra_camera,
            crate::camera::MAP_INDEX => self.map_camera,
            crate::camera::BOW_AND_CROSSBOW_INDEX => self.bow_and_crossbow_camera_fix,
            crate::camera::SPEAR_AND_TRIDENT_INDEX => self.spear_and_trident_camera_fix,
            crate::camera::LADDER_BODY_LOCK_INDEX => self.ladder_body_lock,
            _ => false,
        }
    }
}

/// Steps a scroll by `delta` rows (positive moves down the list), clamped to
/// `0..=max`; the Java wheel clamps the same way.
fn clamp_scroll(current: usize, delta: i32, max: usize) -> usize {
    (current as i32 + delta).clamp(0, max as i32) as usize
}

/// A scrollbar's track and thumb top, the Java `Scrollbar` geometry.
fn scrollbar_geometry(
    top: f32,
    bottom: f32,
    count: usize,
    visible: usize,
    scroll: usize,
) -> (i32, i32, i32) {
    let top = top as i32;
    let bottom = bottom as i32;
    let max = count.saturating_sub(visible).max(1);
    let track = (bottom - top).max(1);
    let thumb = thumb_height(count, visible, track);
    let movable = (track - thumb).max(1);
    let at = top + (movable as f32 * scroll as f32 / max as f32).round() as i32;
    (top, bottom, at)
}

/// A scrollbar's thumb height, the Java `max(18, track * visible / count)`.
fn thumb_height(count: usize, visible: usize, track: i32) -> i32 {
    ((track as usize * visible / count.max(1)) as i32).max(18)
}

/// How wide a settings panel is, in GUI pixels.
pub const PANEL_WIDTH: f32 = 300.0;
/// How tall a settings panel is, in GUI pixels.
pub const PANEL_HEIGHT: f32 = 170.0;

// ---------------------------------------------------------------------------
// The colour picker
// ---------------------------------------------------------------------------

/// The swatches the picker offers, as straight RGB bytes.
pub const SWATCHES: [[u8; 3]; 8] = [
    [255, 255, 255],
    [30, 30, 34],
    [214, 64, 64],
    [236, 152, 60],
    [238, 214, 88],
    [92, 190, 108],
    [86, 148, 232],
    [178, 116, 214],
];

/// The picker's channels, in focus order: hue, saturation, brightness.
pub const COLOR_CHANNELS: usize = 3;

/// The colour square's side, the Java `SQUARE`.
pub const PICKER_SQUARE: i32 = 128;
/// The hue bar's width, the Java `BAR`.
pub const PICKER_BAR: i32 = 12;
/// The gap between the square and the bar, the Java `GAP`.
pub const PICKER_GAP: i32 = 6;
/// The picker window's widest, the Java `min(width - 12, 334)`.
pub const PICKER_MAX_W: f32 = 334.0;
/// Its tallest, the Java `min(height - 12, 217)`.
pub const PICKER_MAX_H: f32 = 217.0;

/// A colour picker over one RGB colour, the Java `ColorPickerScreen`'s
/// model: a hue bar, a saturation-brightness square, a hex field and
/// presets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPickerState {
    open: bool,
    /// The hue, `0.0..=1.0` round the wheel.
    hue: f32,
    /// The saturation, `0.0..=1.0`.
    saturation: f32,
    /// The brightness, `0.0..=1.0`.
    brightness: f32,
    /// The colour the picker opened with, RGB bytes.
    original: [u8; 3],
    /// Which picker part the pointer drags: 1 the square, 2 the bar.
    dragging: i32,
    focus: usize,
}

impl Default for ColorPickerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorPickerState {
    pub fn new() -> Self {
        Self {
            open: false,
            hue: 0.0,
            saturation: 0.0,
            brightness: 0.0,
            original: [0, 0, 0],
            dragging: 0,
            focus: 0,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Puts the picker up over `rgb`, remembering it as the colour the
    /// cancel returns to, the Java constructor.
    pub fn open_over(&mut self, rgb: [u8; 3]) {
        self.open = true;
        self.original = rgb;
        self.dragging = 0;
        let [h, s, v] = rgb_to_hsv([
            rgb[0] as f32 / 255.0,
            rgb[1] as f32 / 255.0,
            rgb[2] as f32 / 255.0,
        ]);
        self.hue = h / 360.0;
        self.saturation = s;
        self.brightness = v;
    }

    pub fn open(&mut self) {
        self.open_over(self.rgb());
    }

    /// Closes, handing back the colour it opened with (the Java cancel).
    pub fn close(&mut self) -> [u8; 3] {
        self.open = false;
        self.dragging = 0;
        self.original
    }

    pub fn toggle(&mut self) -> bool {
        self.open = !self.open;
        self.open
    }

    /// The colour as RGB bytes, the Java `rgb()`.
    pub fn rgb(&self) -> [u8; 3] {
        let [r, g, b] = hsv_to_rgb([self.hue * 360.0, self.saturation, self.brightness]);
        [
            (r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (b.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }

    /// Sets the colour from RGB bytes, the Java `set`: the hue only moves
    /// when the new colour has some saturation, so greys keep their hue.
    pub fn set_rgb(&mut self, rgb: [u8; 3]) {
        let [h, s, v] = rgb_to_hsv([
            rgb[0] as f32 / 255.0,
            rgb[1] as f32 / 255.0,
            rgb[2] as f32 / 255.0,
        ]);
        if s > 0.0 {
            self.hue = h / 360.0;
        }
        self.saturation = s;
        self.brightness = v;
    }

    /// The hue, `0.0..=1.0` round the wheel.
    pub fn hue(&self) -> f32 {
        self.hue
    }

    /// The saturation, `0.0..=1.0`.
    pub fn saturation(&self) -> f32 {
        self.saturation
    }

    /// The brightness, `0.0..=1.0`.
    pub fn brightness(&self) -> f32 {
        self.brightness
    }

    /// The colour as hue (degrees), saturation and value.
    pub fn hsv(&self) -> [f32; 3] {
        [self.hue * 360.0, self.saturation, self.brightness]
    }

    /// Sets the colour from hue (degrees), saturation and value.
    pub fn set_hsv(&mut self, hsv: [f32; 3]) {
        self.hue = (hsv[0] / 360.0).rem_euclid(1.0);
        self.saturation = hsv[1].clamp(0.0, 1.0);
        self.brightness = hsv[2].clamp(0.0, 1.0);
    }

    /// The colour as `#RRGGBB`, the Java `hexOf`.
    pub fn hex(&self) -> String {
        let [r, g, b] = self.rgb();
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }

    /// Sets the colour from `#RRGGBB` or `RRGGBB`; `false` when the text is
    /// not a colour (the colour is left alone), the Java hex responder.
    pub fn set_hex(&mut self, text: &str) -> bool {
        let text = text.trim().trim_start_matches('#');
        if text.len() != 6 || !text.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
        let channel = |at: usize| u8::from_str_radix(&text[at..at + 2], 16).ok();
        match (channel(0), channel(2), channel(4)) {
            (Some(r), Some(g), Some(b)) => {
                self.set_rgb([r, g, b]);
                true
            }
            _ => false,
        }
    }

    /// The colour the picker opened with.
    pub fn original(&self) -> [u8; 3] {
        self.original
    }

    /// The picker part the pointer drags: 0 none, 1 the square, 2 the bar.
    pub fn dragging(&self) -> i32 {
        self.dragging
    }

    /// The focused channel (0 hue, 1 saturation, 2 brightness).
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// Focuses a channel, wrapping.
    pub fn focus_at(&mut self, channel: usize) -> usize {
        self.focus = channel % COLOR_CHANNELS;
        self.focus
    }

    /// Steps the focus to the next or previous channel.
    pub fn nudge_focus(&mut self, delta: i32) -> usize {
        let channels = COLOR_CHANNELS as i32;
        self.focus = (self.focus as i32 + delta).rem_euclid(channels) as usize;
        self.focus
    }

    /// Moves the focused channel by `delta`: hue takes degrees, saturation
    /// and brightness take fractions.
    pub fn step(&mut self, delta: f32) {
        if !delta.is_finite() {
            return;
        }
        match self.focus {
            0 => self.hue = (self.hue + delta / 360.0).rem_euclid(1.0),
            1 => self.saturation = (self.saturation + delta).clamp(0.0, 1.0),
            _ => self.brightness = (self.brightness + delta).clamp(0.0, 1.0),
        }
    }

    /// Takes a swatch's colour, the Java preset clicks.
    pub fn select_swatch(&mut self, index: usize) {
        if let Some(swatch) = SWATCHES.get(index) {
            self.set_rgb(*swatch);
        }
    }

    /// The swatches.
    pub fn swatches(&self) -> [[u8; 3]; 8] {
        SWATCHES
    }

    /// The colour square's rect on the layout, the Java `squareX`/`squareY`.
    pub fn square(&self, layout: &UiLayout) -> UiRect {
        let panel = UiRect::new(
            (layout.width - PICKER_MAX_W) * 0.5,
            (layout.height - PICKER_MAX_H) * 0.5,
            PICKER_MAX_W,
            PICKER_MAX_H,
        );
        UiRect::new(
            panel.x + 10.0,
            panel.y + 29.0 + 8.0,
            PICKER_SQUARE as f32,
            PICKER_SQUARE as f32,
        )
    }

    /// The hue bar's rect, the Java `barX`.
    pub fn bar(&self, layout: &UiLayout) -> UiRect {
        let square = self.square(layout);
        UiRect::new(
            square.right() + PICKER_GAP as f32,
            square.y,
            PICKER_BAR as f32,
            PICKER_SQUARE as f32,
        )
    }

    /// A click or drag at `(x, y)`: picks from the square or the bar, the
    /// Java `pickAt`. `start` begins a drag; `true` when it landed somewhere.
    pub fn pick_at(&mut self, layout: &UiLayout, x: f32, y: f32, start: bool) -> bool {
        let square = self.square(layout);
        let bar = self.bar(layout);
        let on_square = x >= square.x && x < square.right() && y >= square.y && y < square.bottom();
        let on_bar =
            x >= bar.x - 2.0 && x < bar.right() + 2.0 && y >= square.y && y < square.bottom();
        if start {
            self.dragging = if on_square {
                1
            } else if on_bar {
                2
            } else {
                0
            };
        }
        if self.dragging == 1 {
            self.saturation = clamp01((x - square.x) / (PICKER_SQUARE - 1) as f32);
            self.brightness = 1.0 - clamp01((y - square.y) / (PICKER_SQUARE - 1) as f32);
        } else if self.dragging == 2 {
            self.hue = clamp01((y - square.y) / (PICKER_SQUARE - 1) as f32);
            // A pure white picked off the corner gets some colour back.
            if self.saturation < 0.05 && self.brightness > 0.95 {
                self.saturation = 0.6;
            }
        } else {
            return false;
        }
        true
    }

    /// Lets go of the drag.
    pub fn release(&mut self) {
        self.dragging = 0;
    }

    /// Draws the picker: the window, the hue bar's six segments, the square,
    /// the sample swatches and the hex line.
    pub fn draw(&self, layout: &UiLayout, theme: &UiTheme, frame: &mut HudFrame) {
        if !self.open {
            return;
        }
        let panel = frame.panel(layout, PICKER_MAX_W, PICKER_MAX_H, theme);
        crate::ui::gold::window(
            frame,
            panel.x as i32,
            panel.y as i32,
            panel.right() as i32,
            panel.bottom() as i32,
            29,
            crate::ui::gold::PANE,
        );
        frame.text(
            "punchy_moves.ui.face_colour",
            panel.x + 12.0,
            panel.y + 10.0,
            crate::ui::gold::TITLE_INK,
        );
        let square = self.square(layout);
        let bar = self.bar(layout);
        // The square, one column per x in the hue, darkening downward (the
        // HUD has no gradient, so the shade steps in eighths).
        let [r, g, b] = hsv_to_rgb([self.hue * 360.0, 1.0, 1.0]);
        let hue_color = rgba((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255);
        frame.rect(square, hue_color);
        for step in 0..8 {
            let shade = 1.0 - step as f32 / 8.0;
            frame.rect(
                UiRect::new(
                    square.x,
                    square.y + step as f32 * (square.h / 8.0),
                    square.w,
                    square.h / 8.0,
                ),
                rgba(
                    (r * shade * 255.0) as u8,
                    (g * shade * 255.0) as u8,
                    (b * shade * 255.0) as u8,
                    255,
                ),
            );
        }
        crate::ui::gold::outline(
            frame,
            square.x as i32 - 1,
            square.y as i32 - 1,
            square.right() as i32 + 1,
            square.bottom() as i32 + 1,
            crate::ui::gold::WINDOW_EDGE,
        );
        // The cursor on the square.
        let cursor_x = square.x + self.saturation * (PICKER_SQUARE - 1) as f32;
        let cursor_y = square.y + (1.0 - self.brightness) * (PICKER_SQUARE - 1) as f32;
        let cursor_ink = if self.brightness > 0.5 {
            crate::ui::gold::TEXT_DIS
        } else {
            crate::ui::gold::TEXT_1
        };
        crate::ui::gold::outline(
            frame,
            cursor_x as i32 - 3,
            cursor_y as i32 - 3,
            cursor_x as i32 + 4,
            cursor_y as i32 + 4,
            cursor_ink,
        );
        // The hue bar, six segments, the Java loop.
        for segment in 0..6 {
            let [r, g, b] = hsv_to_rgb([(segment as f32 + 0.5) / 6.0 * 360.0, 1.0, 1.0]);
            let y1 = square.y + segment as f32 * (square.h / 6.0);
            let y2 = square.y + (segment + 1) as f32 * (square.h / 6.0);
            frame.rect(
                UiRect::new(bar.x, y1, bar.w, y2 - y1),
                rgba((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255),
            );
        }
        crate::ui::gold::outline(
            frame,
            bar.x as i32 - 1,
            square.y as i32 - 1,
            bar.right() as i32 + 1,
            square.bottom() as i32 + 1,
            crate::ui::gold::WINDOW_EDGE,
        );
        // The hue cursor.
        let hue_y = square.y + self.hue * (PICKER_SQUARE - 1) as f32;
        frame.rect(
            UiRect::new(bar.x - 2.0, hue_y - 1.0, bar.w + 4.0, 2.0),
            crate::ui::gold::TEXT_1,
        );
        // The sample, the original and the hex well.
        let sample_x = bar.right() + PICKER_GAP as f32 + 4.0;
        let sample_w = panel.right() - 10.0 - sample_x;
        let [r, g, b] = self.rgb();
        frame.rect(
            UiRect::new(sample_x, square.y, sample_w, 18.0),
            rgba(r, g, b, 255),
        );
        let [r, g, b] = self.original;
        frame.rect(
            UiRect::new(sample_x, square.y + 18.0, sample_w, 16.0),
            rgba(r, g, b, 255),
        );
        crate::ui::gold::outline(
            frame,
            sample_x as i32 - 1,
            square.y as i32 - 1,
            (sample_x + sample_w) as i32 + 1,
            square.y as i32 + 35,
            crate::ui::gold::WINDOW_EDGE,
        );
        frame.rect(
            UiRect::new(sample_x, square.y + 40.0, sample_w, 18.0),
            crate::ui::gold::WELL,
        );
        frame.text(
            self.hex(),
            sample_x + 4.0,
            square.y + 46.0,
            crate::ui::gold::TEXT_1,
        );
        // The presets.
        for (index, swatch) in SWATCHES.iter().enumerate() {
            let x = sample_x + (index % 4) as f32 * 15.0;
            let y = square.y + 64.0 + (index / 4) as f32 * 15.0;
            frame.rect(
                UiRect::new(x, y, 12.0, 12.0),
                rgba(swatch[0], swatch[1], swatch[2], 255),
            );
            crate::ui::gold::outline(
                frame,
                x as i32 - 1,
                y as i32 - 1,
                x as i32 + 13,
                y as i32 + 13,
                if *swatch == self.rgb() {
                    crate::ui::gold::GOLD_200
                } else {
                    crate::ui::gold::WINDOW_EDGE
                },
            );
        }
        let _ = theme;
    }
}

/// Clamps to `0.0..=1.0`, the Java `clamp`.
fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

/// RGB in `0.0..=1.0` to hue in degrees, saturation and value.
pub fn rgb_to_hsv(rgb: [f32; 3]) -> [f32; 3] {
    let (r, g, b) = (
        rgb[0].clamp(0.0, 1.0),
        rgb[1].clamp(0.0, 1.0),
        rgb[2].clamp(0.0, 1.0),
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta <= f32::EPSILON {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    [
        hue.rem_euclid(360.0),
        if max <= 0.0 { 0.0 } else { delta / max },
        max,
    ]
}

/// Hue in degrees, saturation and value to RGB in `0.0..=1.0`.
pub fn hsv_to_rgb(hsv: [f32; 3]) -> [f32; 3] {
    let hue = hsv[0].rem_euclid(360.0);
    let s = hsv[1].clamp(0.0, 1.0);
    let v = hsv[2].clamp(0.0, 1.0);
    let c = v * s;
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (hue / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m]
}

// ---------------------------------------------------------------------------
// The face canvas
// ---------------------------------------------------------------------------

/// The cue fields the canvas edits, in focus order.
pub const CANVAS_FIELDS: usize = 5;

/// The face canvas: a cue's fields and a live preview of the face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceCanvasState {
    open: bool,
    expression: FaceExpression,
    cue: FaceCue,
    focus: usize,
}

impl Default for FaceCanvasState {
    fn default() -> Self {
        Self {
            open: false,
            expression: FaceExpression::Happy,
            cue: FaceExpression::Happy.cue(),
            focus: 0,
        }
    }
}

impl FaceCanvasState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn toggle(&mut self) -> bool {
        self.open = !self.open;
        self.open
    }

    /// The expression being previewed.
    pub fn expression(&self) -> FaceExpression {
        self.expression
    }

    /// Shows `expression`'s built-in cue for editing.
    pub fn select_expression(&mut self, expression: FaceExpression) -> FaceExpression {
        self.expression = expression;
        self.cue = expression.cue();
        self.focus = 0;
        self.expression
    }

    /// The cue being edited.
    pub fn cue(&self) -> FaceCue {
        self.cue
    }

    /// Replaces the cue.
    pub fn set_cue(&mut self, cue: FaceCue) {
        self.expression = cue.expression;
        self.cue = cue;
    }

    /// The focused field.
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// Focuses a field, wrapping.
    pub fn focus_at(&mut self, field: usize) -> usize {
        self.focus = field % CANVAS_FIELDS;
        self.focus
    }

    /// Steps the focus to the next or previous field.
    pub fn nudge_focus(&mut self, delta: i32) -> usize {
        let fields = CANVAS_FIELDS as i32;
        self.focus = (self.focus as i32 + delta).rem_euclid(fields) as usize;
        self.focus
    }

    /// Moves the focused field by `delta` and returns the cue.
    pub fn step(&mut self, delta: f32) -> FaceCue {
        if !delta.is_finite() {
            return self.cue;
        }
        match self.focus {
            0 => self.cue.lid = (self.cue.lid + delta).clamp(0.0, 1.0),
            1 => self.cue.brow = (self.cue.brow + delta).clamp(-0.6, 0.6),
            2 => self.cue.mouth = (self.cue.mouth + delta).clamp(0.0, 1.0),
            3 => self.cue.widen = (self.cue.widen + delta).clamp(0.0, 1.0),
            _ => self.cue.seconds = (self.cue.seconds + delta).clamp(0.0, 10.0),
        }
        self.cue
    }

    /// The face the cue draws right now.
    pub fn preview(&self, settings: &FaceSettings) -> FaceQuads {
        let mut rig = FaceRig::new(1);
        rig.play_cue(self.cue);
        rig.quads(settings)
    }

    /// Draws the canvas: the expression, its five fields, and the preview's
    /// quads as a flat sketch of the face.
    pub fn draw(&self, layout: &UiLayout, theme: &UiTheme, frame: &mut HudFrame) {
        if !self.open {
            return;
        }
        let panel = frame.panel(layout, PANEL_WIDTH, PANEL_HEIGHT, theme);
        frame.title("Face canvas", panel, theme);
        let rows = [
            ("Lids", format!("{:.0}%", self.cue.lid * 100.0)),
            ("Brow", format!("{:.2} rad", self.cue.brow)),
            ("Mouth", format!("{:.0}%", self.cue.mouth * 100.0)),
            ("Eyes wide", format!("{:.0}%", self.cue.widen * 100.0)),
            ("Lasts", format!("{:.1} s", self.cue.seconds)),
        ];
        frame.row(
            panel,
            0,
            "Expression",
            &format!("{:?}", self.expression),
            false,
            theme,
        );
        for (index, (label, value)) in rows.iter().enumerate() {
            frame.row(panel, index + 1, label, value, index == self.focus, theme);
        }
        // A sketch of the preview in the panel's right half: each quad a box.
        let settings = FaceSettings::default();
        let quads = self.preview(&settings);
        let face = UiRect::new(panel.right() - 110.0, panel.bottom() - 96.0, 96.0, 84.0);
        frame.rect(face, theme.panel);
        for quad in quads.iter() {
            let center = quad.center;
            let rect = UiRect::new(
                face.x + face.w * 0.5 + center[0] * face.w,
                face.y + face.h * 0.5 - center[1] * face.h,
                quad.size[0] * face.w,
                quad.size[1] * face.h,
            );
            let color = rgba(
                (quad.color[0] * 255.0) as u8,
                (quad.color[1] * 255.0) as u8,
                (quad.color[2] * 255.0) as u8,
                (quad.color[3].clamp(0.4, 1.0) * 255.0) as u8,
            );
            frame.rect(rect, color);
        }
        frame.footer(
            "Left and right change the row; Up and down change it",
            panel,
            theme,
        );
    }
}

// ---------------------------------------------------------------------------
// The tutorial
// ---------------------------------------------------------------------------

/// One key hint of a step: which key mapping's suffix to look up, and the
/// label shown beside it, the Java `KeyHint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyHint {
    /// The key mapping's name suffix, the part after `key.punchy_moves.`.
    pub mapping: &'static str,
    /// The label key beside the keycap.
    pub label: &'static str,
}

/// One step of the tutorial: its area, its key hints, and the tab its open
/// button shows, the Java `Step` record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TutorialStep {
    /// The area key, shown small and uppercase above the title.
    pub area: &'static str,
    /// The key hints, wrapped left to right.
    pub keys: &'static [KeyHint],
    /// The camera page's tab the open button shows, if any.
    pub tab: Option<&'static str>,
    /// The open button's label key, if the step has one.
    pub open_label: Option<&'static str>,
}

/// The tutorial's step count, the Java `STEPS`.
pub const TUTORIAL_STEPS: usize = 7;

/// The tutorial's steps, in order, the Java `GUIDE`.
pub const GUIDE: [TutorialStep; TUTORIAL_STEPS] = [
    TutorialStep {
        area: "guide",
        keys: &[],
        tab: None,
        open_label: None,
    },
    TutorialStep {
        area: "camera",
        keys: &[
            KeyHint {
                mapping: "camera_toggle_v",
                label: "key.camera",
            },
            KeyHint {
                mapping: "free_camera",
                label: "key.orbit",
            },
        ],
        tab: None,
        open_label: None,
    },
    TutorialStep {
        area: "emotes",
        keys: &[KeyHint {
            mapping: "emote_menu",
            label: "key.emotes",
        }],
        tab: None,
        open_label: None,
    },
    TutorialStep {
        area: "settings",
        keys: &[KeyHint {
            mapping: "camera_editor",
            label: "key.settings",
        }],
        tab: Some("camera"),
        open_label: Some("open.settings"),
    },
    TutorialStep {
        area: "camera_settings",
        keys: &[],
        tab: Some("camera"),
        open_label: Some("open.camera"),
    },
    TutorialStep {
        area: "camera_settings",
        keys: &[],
        tab: Some("camera"),
        open_label: Some("open.camera"),
    },
    TutorialStep {
        area: "shortcuts",
        keys: &[],
        tab: Some("shortcuts"),
        open_label: Some("open.shortcuts"),
    },
];

/// The tutorial window's widest, in GUI pixels, the Java `MAX_W`.
pub const TUTORIAL_MAX_W: f32 = 440.0;
/// Its shortest, the Java `MIN_H`.
pub const TUTORIAL_MIN_H: f32 = 180.0;
/// The title bar's height, the Java `TITLE_BAR`.
pub const TUTORIAL_TITLE_BAR: i32 = 29;
/// The padding round the body, the Java `PAD`.
pub const TUTORIAL_PAD: i32 = 12;
/// The footer's height, the Java `FOOT`.
pub const TUTORIAL_FOOT: i32 = 38;
/// A button's height, the Java `BUTTON_H`.
pub const TUTORIAL_BUTTON_H: i32 = 22;
/// A text line's height, the Java `LINE`.
pub const TUTORIAL_LINE: i32 = 11;
/// A keycap's height, the Java `KEY_H`.
pub const TUTORIAL_KEY_H: i32 = 16;
/// A progress segment's height, the Java `SEGMENT_H`.
pub const TUTORIAL_SEGMENT_H: i32 = 6;

/// The tutorial: which step it is on, and whether it is up.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TutorialState {
    open: bool,
    step: usize,
}

impl TutorialState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Puts the tutorial up at its first step.
    pub fn start(&mut self) {
        self.open = true;
        self.step = 0;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The step being shown.
    pub fn step(&self) -> usize {
        self.step
    }

    /// The step being shown, if the tutorial is up.
    pub fn current(&self) -> Option<TutorialStep> {
        if self.open {
            GUIDE.get(self.step).copied()
        } else {
            None
        }
    }

    /// Jumps to a step (the progress track's clicks); past the end closes.
    pub fn go(&mut self, step: usize) -> bool {
        if step >= TUTORIAL_STEPS {
            self.close();
            self.step = 0;
            return true;
        }
        self.step = step;
        false
    }

    /// Moves on; `true` when the last step is done and the tutorial closed.
    // A step forward, not an iterator's item: the name reads as the page does.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> bool {
        self.go(self.step + 1)
    }

    /// Goes back; `false` at the first step.
    pub fn back(&mut self) -> bool {
        if self.step == 0 {
            false
        } else {
            self.step -= 1;
            true
        }
    }

    /// Skips the tutorial out (the Java `skip` button, and `onClose`).
    pub fn close(&mut self) {
        self.open = false;
    }

    /// The x of the left edge of progress segment `index`, the Java
    /// `segmentX`: the track runs the panel's inner width, each segment
    /// overlapping the next by two pixels.
    pub fn segment_x(&self, panel: UiRect, index: usize) -> f32 {
        panel.x
            + TUTORIAL_PAD as f32
            + index as f32 * ((panel.w - TUTORIAL_PAD as f32 * 2.0 + 2.0) / TUTORIAL_STEPS as f32)
    }

    /// The progress segment a click at `(x, y)` lands on, or `None`, the
    /// Java `segmentAt`.
    pub fn segment_at(&self, panel: UiRect, x: f32, y: f32) -> Option<usize> {
        let track_y = panel.y + TUTORIAL_TITLE_BAR as f32 + 8.0;
        if y < track_y - 3.0 || y >= track_y + TUTORIAL_SEGMENT_H as f32 + 3.0 {
            return None;
        }
        for index in 0..TUTORIAL_STEPS {
            let start = self.segment_x(panel, index);
            let end = self.segment_x(panel, index + 1) - 2.0;
            if x >= start && x < end {
                return Some(index);
            }
        }
        None
    }

    /// Draws the step: the window, the step counter, the progress track, the
    /// area, the title, the keycaps and the body, and the footer groove.
    pub fn draw(&self, layout: &UiLayout, theme: &UiTheme, frame: &mut HudFrame) {
        let Some(step) = self.current() else {
            return;
        };
        let _ = layout;
        let width = TUTORIAL_MAX_W.min(theme_width(panel_width_cap()));
        let height = TUTORIAL_MIN_H;
        let panel = frame.panel(layout, width, height, theme);
        crate::ui::gold::window(
            frame,
            panel.x as i32,
            panel.y as i32,
            panel.right() as i32,
            panel.bottom() as i32,
            TUTORIAL_TITLE_BAR,
            crate::ui::gold::PANE,
        );
        // The step counter, top right; the window title, top left.
        let counter = format!(
            "punchy_moves.tutorial.step {}/{}",
            self.step + 1,
            TUTORIAL_STEPS
        );
        let counter_width = counter.chars().count() as f32 * GLYPH_WIDTH;
        crate::ui::gold::draw_ink(
            frame,
            &counter,
            (panel.right() - TUTORIAL_PAD as f32 - counter_width) as i32,
            panel.y as i32 + 10,
            crate::ui::gold::TITLE_INK,
        );
        crate::ui::gold::draw_ink(
            frame,
            "punchy_moves.tutorial.window",
            panel.x as i32 + TUTORIAL_PAD,
            panel.y as i32 + 10,
            crate::ui::gold::TITLE_INK,
        );
        // The progress track: one segment per step, the passed ones gold.
        let track_y = panel.y + TUTORIAL_TITLE_BAR as f32 + 8.0;
        for index in 0..TUTORIAL_STEPS {
            let start = self.segment_x(panel, index);
            let end = self.segment_x(panel, index + 1) - 2.0;
            let passed = index <= self.step;
            crate::ui::gold::outline(
                frame,
                start as i32,
                track_y as i32,
                end as i32,
                (track_y + TUTORIAL_SEGMENT_H as f32) as i32,
                crate::ui::gold::WINDOW_EDGE,
            );
            frame.rect(
                UiRect::new(start + 1.0, track_y + 1.0, end - start - 2.0, 1.0),
                if passed {
                    crate::ui::gold::GOLD_300
                } else {
                    crate::ui::gold::TRACK_LIGHT
                },
            );
            frame.rect(
                UiRect::new(
                    start + 1.0,
                    track_y + 2.0,
                    end - start - 2.0,
                    TUTORIAL_SEGMENT_H as f32 - 2.0,
                ),
                if passed {
                    crate::ui::gold::GOLD_500
                } else {
                    crate::ui::gold::TRACK
                },
            );
        }
        // The area, then the title.
        let mut body_y = track_y + TUTORIAL_SEGMENT_H as f32 + 12.0;
        frame.text(
            step.area.to_uppercase(),
            panel.x + TUTORIAL_PAD as f32,
            body_y,
            crate::ui::gold::GOLD_300,
        );
        body_y += 14.0;
        frame.text(
            step_title(self.step),
            panel.x + TUTORIAL_PAD as f32,
            body_y,
            crate::ui::gold::TEXT_1,
        );
        body_y += TUTORIAL_LINE as f32;
        // The keycaps, then the body.
        if !step.keys.is_empty() {
            body_y += 5.0;
            let mut x = panel.x + TUTORIAL_PAD as f32;
            for hint in step.keys {
                let cap_w = 22.0f32
                    .max((hint.mapping.chars().count() as f32 * GLYPH_WIDTH + 12.0).min(120.0));
                crate::ui::gold::keycap(
                    frame,
                    x as i32,
                    body_y as i32,
                    cap_w as i32,
                    TUTORIAL_KEY_H,
                    hint.mapping,
                );
                frame.text(
                    hint.label,
                    x + cap_w + 6.0,
                    body_y + 4.0,
                    crate::ui::gold::TEXT_2,
                );
                x += cap_w + 6.0 + hint.label.chars().count() as f32 * GLYPH_WIDTH + 16.0;
            }
            body_y += TUTORIAL_KEY_H as f32 + 8.0;
        }
        frame.text(
            step_body(self.step),
            panel.x + TUTORIAL_PAD as f32,
            body_y,
            crate::ui::gold::TEXT_2,
        );
        // The footer groove and the step's buttons, as labels.
        crate::ui::gold::groove(
            frame,
            panel.x as i32 + 1,
            panel.right() as i32 - 1,
            (panel.bottom() - TUTORIAL_FOOT as f32) as i32,
        );
        let finish = self.step + 1 >= TUTORIAL_STEPS;
        let next_label = if finish {
            "punchy_moves.tutorial.finish"
        } else {
            "punchy_moves.tutorial.next"
        };
        let next_w = 64.0f32.max(next_label.chars().count() as f32 * GLYPH_WIDTH + 24.0);
        crate::ui::gold::button(
            frame,
            (panel.right() - TUTORIAL_PAD as f32 - next_w) as i32,
            (panel.bottom() - TUTORIAL_FOOT as f32 + 9.0) as i32,
            next_w as i32,
            TUTORIAL_BUTTON_H,
            next_label,
            true,
            false,
            true,
        );
        if self.step > 0 {
            let back_w = 64.0f32
                .max("punchy_moves.tutorial.back".chars().count() as f32 * GLYPH_WIDTH + 24.0);
            crate::ui::gold::button(
                frame,
                (panel.right() - TUTORIAL_PAD as f32 - next_w - 6.0 - back_w) as i32,
                (panel.bottom() - TUTORIAL_FOOT as f32 + 9.0) as i32,
                back_w as i32,
                TUTORIAL_BUTTON_H,
                "punchy_moves.tutorial.back",
                false,
                false,
                true,
            );
        }
        if !finish {
            frame.text(
                "punchy_moves.tutorial.skip",
                panel.x + TUTORIAL_PAD as f32 - 4.0,
                panel.bottom() - TUTORIAL_FOOT as f32 + 16.0,
                crate::ui::gold::TEXT_3,
            );
        }
    }
}

/// The step's title line, the Java `title.<step>` key.
fn step_title(step: usize) -> String {
    let area = GUIDE.get(step).map_or("guide", |s| s.area);
    format!("punchy_moves.tutorial.title.{}", area)
}

/// The step's body line, the Java `body.<step>` key.
fn step_body(step: usize) -> String {
    let area = GUIDE.get(step).map_or("guide", |s| s.area);
    format!("punchy_moves.tutorial.body.{}", area)
}

/// The widest a panel may be, in GUI pixels (the HUD caps its own width).
fn panel_width_cap() -> f32 {
    480.0
}

/// The width a panel of `want` takes under the cap.
fn theme_width(want: f32) -> f32 {
    want.min(panel_width_cap())
}

// ---------------------------------------------------------------------------
// The near-camera dither
// ---------------------------------------------------------------------------

/// The 4x4 ordered-dither matrix the entity shader carries, in reading order:
/// the Java `PUNCHY_ORDER`, each of `0..=15` once.
pub const PUNCHY_ORDER: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

/// The threshold the entity shader discards at, the fixed `>= 6` of
/// `entity_dither.fsh`.
pub const PUNCHY_THRESHOLD: u8 = 6;

/// The threshold of a screen pixel, `0..=15`, the shader's
/// `PUNCHY_ORDER[punchyPixel.y * 4 + punchyPixel.x]`.
pub fn punchy_order(x: u32, y: u32) -> u8 {
    PUNCHY_ORDER[((y & 3) * 4 + (x & 3)) as usize]
}

/// Whether the entity shader discards the pixel: the fixed
/// `PUNCHY_ORDER[...] >= 6` of `entity_dither.fsh`, every entity, every frame.
pub fn punchy_discard(x: u32, y: u32) -> bool {
    punchy_order(x, y) >= PUNCHY_THRESHOLD
}

/// Whether a screen pixel draws at `alpha`: a pixel whose threshold is below
/// the alpha shows, so a dithered shape covers the share of the screen its
/// alpha asks for. Reads the same 4x4 order the shader does.
pub fn dither(x: u32, y: u32, alpha: f32) -> bool {
    if !alpha.is_finite() || alpha <= 0.0 {
        return false;
    }
    if alpha >= 1.0 {
        return true;
    }
    f32::from(punchy_order(x, y)) < alpha * 16.0
}

/// How the mod fades its own geometry out as it comes close to the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DitherVisibility {
    /// Inside this many blocks the geometry is gone.
    pub near: f32,
    /// Over this many blocks past `near` it comes back.
    pub fade: f32,
}

impl Default for DitherVisibility {
    fn default() -> Self {
        Self {
            near: 0.35,
            fade: 0.45,
        }
    }
}

impl DitherVisibility {
    pub fn new() -> Self {
        Self::default()
    }

    /// The alpha at `distance` blocks from the eye: `0.0` inside the near
    /// distance, rising to `1.0` at the far end of the fade.
    pub fn alpha(&self, distance: f32) -> f32 {
        if !distance.is_finite() {
            return 1.0;
        }
        if self.fade <= 0.0 {
            return if distance >= self.near { 1.0 } else { 0.0 };
        }
        ((distance - self.near) / self.fade).clamp(0.0, 1.0)
    }

    /// Whether a pixel at `distance` draws at its screen position.
    pub fn visible(&self, x: u32, y: u32, distance: f32) -> bool {
        dither(x, y, self.alpha(distance))
    }
}

// ---------------------------------------------------------------------------
// Every screen together
// ---------------------------------------------------------------------------

/// A screen Options > Mods can open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    EmoteWheel,
    Camera,
    ColorPicker,
    FaceCanvas,
    Tutorial,
}

/// The buttons Options > Mods shows for the mod's own screens, in order.
pub fn settings_buttons() -> [(&'static str, Screen); 5] {
    [
        ("Emote wheel", Screen::EmoteWheel),
        ("Camera settings", Screen::Camera),
        ("Face colour", Screen::ColorPicker),
        ("Face canvas", Screen::FaceCanvas),
        ("Tutorial", Screen::Tutorial),
    ]
}

/// Every screen's state, on one value so a hook opens one at a time.
#[derive(Debug, Clone, PartialEq)]
pub struct PunchyUi {
    pub emote_wheel: EmoteWheelState,
    pub emote_keys: EmoteKeybindManager,
    pub camera: ShoulderCameraUiState,
    pub color: ColorPickerState,
    pub canvas: FaceCanvasState,
    pub tutorial: TutorialState,
    pub dither: DitherVisibility,
}

impl Default for PunchyUi {
    fn default() -> Self {
        Self::new()
    }
}

impl PunchyUi {
    pub fn new() -> Self {
        Self {
            emote_wheel: EmoteWheelState::new(),
            emote_keys: EmoteKeybindManager::new(),
            camera: ShoulderCameraUiState::new(),
            color: ColorPickerState::new(),
            canvas: FaceCanvasState::new(),
            tutorial: TutorialState::new(),
            dither: DitherVisibility::new(),
        }
    }

    /// Whether any screen is up.
    pub fn any_open(&self) -> bool {
        self.emote_wheel.is_open()
            || self.camera.is_open()
            || self.color.is_open()
            || self.canvas.is_open()
            || self.tutorial.is_open()
    }

    /// Takes every screen down.
    pub fn close_all(&mut self) {
        self.emote_wheel.cancel();
        self.camera.close();
        self.color.close();
        self.canvas.close();
        self.tutorial.close();
    }

    /// Puts one screen up (the others stay as they are, the new one on top).
    pub fn open_screen(&mut self, screen: Screen) {
        match screen {
            Screen::EmoteWheel => self.emote_wheel.open(),
            Screen::Camera => self.camera.open(),
            Screen::ColorPicker => self.color.open(),
            Screen::FaceCanvas => self.canvas.open(),
            Screen::Tutorial => self.tutorial.start(),
        }
    }

    /// A click on the mod's own buttons on Options > Mods; `index` counts from
    /// the first of [`settings_buttons`].
    pub fn click_settings(&mut self, index: usize) -> UiRequest {
        let Some((_, screen)) = settings_buttons().get(index).copied() else {
            return UiRequest::None;
        };
        self.open_screen(screen);
        UiRequest::None
    }

    /// A key press while a screen is up. `None` when no screen wanted the key,
    /// so the game may still see it.
    pub fn key(&mut self, key: &str, pressed: bool) -> Option<UiRequest> {
        if !pressed {
            // Only the emote wheel's own key cares about a release.
            if key == crate::camera::TOGGLE_ORBIT {
                return None;
            }
            return None;
        }
        if self.emote_wheel.is_open() {
            return Some(self.wheel_key(key));
        }
        if self.tutorial.is_open() {
            return Some(match key {
                "key.keyboard.enter" | "key.keyboard.space" | "key.keyboard.right" => {
                    if self.tutorial.next() {
                        UiRequest::CloseAll
                    } else {
                        UiRequest::None
                    }
                }
                "key.keyboard.backspace" | "key.keyboard.left" => {
                    self.tutorial.back();
                    UiRequest::None
                }
                "key.keyboard.escape" => {
                    self.tutorial.close();
                    UiRequest::None
                }
                _ => return None,
            });
        }
        if self.camera.is_open() {
            let rows = self
                .camera
                .steppers(&ShoulderCameraSettings::default())
                .len();
            return Some(match key {
                "key.keyboard.tab" => {
                    self.camera.next_tab();
                    UiRequest::None
                }
                "key.keyboard.escape" => {
                    self.camera.close();
                    UiRequest::None
                }
                "key.keyboard.up" => {
                    self.camera.nudge_focus(-1, rows);
                    UiRequest::None
                }
                "key.keyboard.down" => {
                    self.camera.nudge_focus(1, rows);
                    UiRequest::None
                }
                "key.keyboard.enter" => {
                    let mut settings = ShoulderCameraSettings::default();
                    self.camera.click(&mut settings, self.camera.focus())
                }
                _ => return None,
            });
        }
        if self.color.is_open() {
            return Some(match key {
                "key.keyboard.escape" => {
                    self.color.close();
                    UiRequest::None
                }
                "key.keyboard.left" => {
                    self.color.nudge_focus(-1);
                    UiRequest::None
                }
                "key.keyboard.right" => {
                    self.color.nudge_focus(1);
                    UiRequest::None
                }
                "key.keyboard.up" => {
                    self.color.step(0.05);
                    UiRequest::None
                }
                "key.keyboard.down" => {
                    self.color.step(-0.05);
                    UiRequest::None
                }
                _ => return None,
            });
        }
        if self.canvas.is_open() {
            return Some(match key {
                "key.keyboard.escape" => {
                    self.canvas.close();
                    UiRequest::None
                }
                "key.keyboard.left" => {
                    self.canvas.nudge_focus(-1);
                    UiRequest::None
                }
                "key.keyboard.right" => {
                    self.canvas.nudge_focus(1);
                    UiRequest::None
                }
                "key.keyboard.up" => {
                    self.canvas.step(0.05);
                    UiRequest::None
                }
                "key.keyboard.down" => {
                    self.canvas.step(-0.05);
                    UiRequest::None
                }
                _ => return None,
            });
        }
        None
    }

    /// A key while the wheel is up: a number picks a wedge, the arrows turn
    /// the wheel, Enter plays and Escape puts it away.
    fn wheel_key(&mut self, key: &str) -> UiRequest {
        if key == "key.keyboard.escape" {
            self.emote_wheel.cancel();
            return UiRequest::None;
        }
        if key == "key.keyboard.enter" || key == "key.keyboard.space" {
            let selected = self.emote_wheel.selected();
            if self.emote_wheel.close().is_some() {
                return UiRequest::PlayEmote(selected);
            }
            return UiRequest::None;
        }
        if let Some(digit) = key.strip_prefix("key.keyboard.").filter(|k| k.len() == 1)
            && let Ok(number) = digit.parse::<usize>()
        {
            // 1 is the first wedge, 0 the tenth (there are at most eight).
            let index = if number == 0 { 9 } else { number - 1 };
            if index < MAX_EMOTES {
                self.emote_wheel.select(index);
            }
            return UiRequest::None;
        }
        match key {
            "key.keyboard.right" | "key.keyboard.d" => {
                self.emote_wheel.nudge(1);
            }
            "key.keyboard.left" | "key.keyboard.a" => {
                self.emote_wheel.nudge(-1);
            }
            _ => {}
        }
        UiRequest::None
    }

    /// Draws every open screen, the tutorial last so it stays on top.
    pub fn draw(
        &self,
        layout: &UiLayout,
        settings: &ShoulderCameraSettings,
        theme: &UiTheme,
    ) -> HudFrame {
        let mut frame = HudFrame::new();
        self.emote_wheel.draw(layout, theme, &mut frame);
        self.camera.draw(layout, settings, theme, &mut frame);
        self.color.draw(layout, theme, &mut frame);
        self.canvas.draw(layout, theme, &mut frame);
        self.tutorial.draw(layout, theme, &mut frame);
        frame
    }
}

thread_local! {
    /// Every screen's state, per thread (the client draws on one).
    static UI: RefCell<PunchyUi> = RefCell::new(PunchyUi::new());
}

/// Runs `f` with this thread's screens.
pub fn ui<R>(f: impl FnOnce(&mut PunchyUi) -> R) -> R {
    UI.with_borrow_mut(f)
}

/// Puts every screen and the emote bindings back to how they start.
pub fn reset_ui() {
    UI.with_borrow_mut(|ui| *ui = PunchyUi::new());
}

// ---------------------------------------------------------------------------
// Signals for the screens
// ---------------------------------------------------------------------------

/// The signals the screens need while they are up: a screen holds still, so
/// the body and the face keep their resting pose.
pub fn resting_signals() -> MotionSignals {
    MotionSignals::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faces::FaceExpression;

    fn theme() -> UiTheme {
        UiTheme::default()
    }

    fn layout() -> UiLayout {
        UiLayout::new(320.0, 240.0)
    }

    #[test]
    fn colours_pack_the_way_the_hud_packs_them() {
        assert_eq!(rgba(1, 2, 3, 255), 0xFF03_0201);
        assert_eq!(rgba(255, 0, 0, 255), 0xFF00_00FF);
        assert_eq!(rgba(0, 0, 0, 0), 0);
    }

    #[test]
    fn rectangles_and_layout_keep_to_the_screen() {
        let rect = UiRect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(rect.center(), [60.0, 45.0]);
        assert_eq!((rect.right(), rect.bottom()), (110.0, 70.0));
        assert!(rect.contains(10.0, 20.0));
        assert!(!rect.contains(9.0, 20.0));
        assert_eq!(rect.inset(5.0), UiRect::new(15.0, 25.0, 90.0, 40.0));
        // An inset larger than the rectangle never inverts it.
        assert_eq!(rect.inset(100.0).w, 0.0);

        let layout = layout();
        assert_eq!(layout.center(), [160.0, 120.0]);
        let panel = layout.panel(200.0, 100.0);
        assert_eq!(panel, UiRect::new(60.0, 70.0, 200.0, 100.0));
        // A panel larger than the screen fills it.
        assert_eq!(
            layout.panel(1000.0, 1000.0),
            UiRect::new(0.0, 0.0, 320.0, 240.0)
        );

        // Rows stack inside the panel without leaving it.
        let panel = layout.panel(PANEL_WIDTH, PANEL_HEIGHT);
        for index in 0..4 {
            let row = layout.row(panel, index);
            assert!(row.y >= panel.y);
            assert!(row.bottom() <= panel.bottom());
            assert!(row.x >= panel.x && row.right() <= panel.right());
        }
    }

    #[test]
    fn a_frame_records_what_it_draws_in_order() {
        let mut frame = HudFrame::new();
        assert!(frame.is_empty());
        frame.rect(UiRect::new(0.0, 0.0, 1.0, 1.0), rgba(1, 2, 3, 255));
        frame.quad([[0.0; 2]; 4], 0);
        frame.text("hi", 0.0, 0.0, 0xFFFF_FFFF);
        frame.plain("plain", 0.0, 8.0, 0xFFFF_FFFF);
        assert_eq!(frame.len(), 4);
        assert!(matches!(frame.draws()[0], HudDraw::Rect(..)));
        assert!(matches!(frame.draws()[1], HudDraw::Quad(..)));
        assert!(matches!(
            &frame.draws()[2],
            HudDraw::Text { shadow: true, .. }
        ));
        assert!(matches!(
            &frame.draws()[3],
            HudDraw::Text { shadow: false, .. }
        ));
    }

    #[test]
    fn a_panel_draws_a_wash_a_fill_and_a_border() {
        let mut frame = HudFrame::new();
        let theme = theme();
        let panel = frame.panel(&layout(), 200.0, 100.0, &theme);
        // The wash, the fill and four border edges.
        assert_eq!(frame.len(), 6);
        assert!(panel.w > 0.0 && panel.h > 0.0);
        frame.title("Title", panel, &theme);
        frame.row(panel, 0, "Label", "Value", true, &theme);
        frame.footer("Hint", panel, &theme);
        // The wash, the panel, four edges, the title, the row's highlight and
        // its two texts, and the footer.
        assert_eq!(frame.len(), 11);
    }

    #[test]
    fn the_wheel_carries_eight_emotes_and_wraps() {
        let mut wheel = EmoteWheelState::new();
        assert_eq!(wheel.emotes().len(), MAX_EMOTES);
        assert!(!wheel.is_open());
        assert_eq!(wheel.selected(), 0);
        assert_eq!(wheel.emote().unwrap().id, "wave");

        wheel.open();
        assert!(wheel.is_open());
        assert_eq!(wheel.nudge(1), 1);
        assert_eq!(wheel.nudge(-2), MAX_EMOTES - 1);
        assert_eq!(wheel.select(3), 3);
        assert_eq!(wheel.select(MAX_EMOTES + 2), 2);

        // Closing hands back what a release would play.
        let played = wheel.close().expect("an emote");
        assert_eq!(played.id, wheel.emotes()[2].id);
        assert!(!wheel.is_open());
        assert_eq!(wheel.close(), None);

        // Cancelling plays nothing.
        wheel.open();
        wheel.cancel();
        assert!(!wheel.is_open());
        assert_eq!(wheel.close(), None);

        // An empty wheel has nothing to pick.
        let mut empty = EmoteWheelState::with_emotes(Vec::new());
        assert_eq!(empty.selected(), 0);
        assert_eq!(empty.emote(), None);
        assert_eq!(empty.nudge(3), 0);
    }

    #[test]
    fn pointing_picks_the_tile_under_the_offset() {
        let mut wheel = EmoteWheelState::new();
        // Four tiles a page: up, right, down, left.
        assert_eq!(wheel.pointing(0.0, -100.0), 0);
        assert_eq!(wheel.pointing(100.0, 0.0), 1);
        assert_eq!(wheel.pointing(0.0, 100.0), 2);
        assert_eq!(wheel.pointing(-100.0, 0.0), 3);
        // No offset keeps the selection, and a broken one does not move it.
        wheel.select(3);
        assert_eq!(wheel.pointing(0.0, 0.0), 3);
        assert_eq!(wheel.pointing(f32::NAN, 1.0), 3);
        // A diagonal keeps the nearest axis.
        assert_eq!(wheel.pointing(100.0, -100.0), 0);
    }

    #[test]
    fn the_wheel_pages_four_tiles_at_a_time() {
        let mut wheel = EmoteWheelState::new();
        assert_eq!(wheel.pages(), 2, "eight emotes, four a page");
        // The page's tiles sit up, right, down, left round the middle.
        let l = layout();
        let [cx, cy] = l.center();
        let top = wheel.tile_rect(&l, 0);
        assert!(top.center()[1] < cy);
        assert!((top.center()[0] - cx).abs() < 1e-3);
        let right = wheel.tile_rect(&l, 1);
        assert!(right.center()[0] > cx);
        let bottom = wheel.tile_rect(&l, 2);
        assert!(bottom.center()[1] > cy);
        let left = wheel.tile_rect(&l, 3);
        assert!(left.center()[0] < cx);
        // Selecting an emote turns to its page.
        wheel.select(5);
        assert_eq!(wheel.page(), 1);
        assert_eq!(wheel.selected(), 5);
        // Paging wraps and stays in range.
        assert_eq!(wheel.turn_page(1), 0);
        assert_eq!(wheel.turn_page(-1), 1);
        wheel.set_page(99);
        assert_eq!(wheel.page(), 1);
        // A pointer over a tile of the shown page picks it.
        let rect = wheel.tile_rect(&l, 2);
        let [x, y] = rect.center();
        assert_eq!(wheel.pointing_at(&l, x, y), Some(6));
        // A pointer off the tiles keeps the selection.
        assert_eq!(wheel.pointing_at(&l, 2.0, 2.0), None);
        // An empty wheel still has one page.
        let empty = EmoteWheelState::with_emotes(Vec::new());
        assert_eq!(empty.pages(), 1);
    }

    #[test]
    fn keybinds_find_their_emotes_and_round_trip() {
        let mut keys = EmoteKeybindManager::new();
        assert_eq!(keys.slot(0), None);
        assert!(keys.bind(0, "key.keyboard.1"));
        assert!(keys.bind(3, "key.keyboard.4"));
        assert_eq!(keys.slot(0), Some("key.keyboard.1"));
        assert_eq!(keys.index_of_key("key.keyboard.4"), Some(3));
        assert_eq!(keys.index_of_key("key.keyboard.9"), None);

        // A blank key unbinds; an out-of-range slot is refused.
        assert!(keys.bind(0, "  "));
        assert_eq!(keys.slot(0), None);
        assert!(!keys.bind(MAX_EMOTES, "key.keyboard.1"));
        keys.unbind(3);
        assert_eq!(keys.slot(3), None);
        assert!(keys.bind(MAX_EMOTES - 1, "key.keyboard.unknown"));
        assert_eq!(keys.slot(MAX_EMOTES - 1), None);

        // The labels name every slot, bound or not.
        assert_eq!(keys.labels().len(), MAX_EMOTES);
        assert!(keys.labels()[0].contains("unbound"));

        // What it stores reads back the same bindings.
        keys.bind(1, "key.keyboard.g");
        let stored = keys.stored();
        assert_eq!(stored.len(), MAX_EMOTES);
        let read = EmoteKeybindManager::read(|key| {
            stored
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
        });
        assert_eq!(read.slot(1), Some("key.keyboard.g"));
        assert_eq!(read.slot(0), None);
    }

    #[test]
    fn playing_an_emote_feeds_the_engine_and_the_face() {
        crate::engine::with_engine(|engine| {
            engine.update(77, crate::signals::MotionSignals::new(), 0.05);
        });
        let emote = emotes()[0];
        assert!(play_emote(Some(77), emote));
        crate::engine::with_engine(|engine| {
            let state = engine.state(77).unwrap();
            assert_eq!(state.face.expression(), emote.expression);
            assert_eq!(state.items.emote(), Some(emote.action));
        });
        // An emote needs a player the engine knows.
        assert!(!play_emote(Some(4242), emote));
        assert!(!play_emote(None, emote));
    }

    #[test]
    fn the_camera_screen_has_three_pages() {
        let settings = ShoulderCameraSettings::default();
        let mut page = ShoulderCameraUiState::new();
        assert!(!page.is_open());
        assert_eq!(page.page(), CameraPage::Camera);

        // Every page names rows, and its label is unique.
        for tab in CameraPage::ALL {
            page.select(tab);
            let rows = page.steppers(&settings);
            assert!(!rows.is_empty(), "{:?} has no rows", tab);
            assert!(!tab.label().is_empty());
        }
        assert_eq!(CameraPage::Camera.next(), CameraPage::Misc);
        assert_eq!(CameraPage::Camera.previous(), CameraPage::Shortcuts);
        assert_eq!(CameraPage::Shortcuts.next(), CameraPage::Camera);
        assert_eq!(CameraPage::ALL.len(), 3);

        // The camera page carries the on/off row and the six sliders.
        page.select(CameraPage::Camera);
        assert_eq!(page.steppers(&settings).len(), 7);
        assert_eq!(CAMERA_SLIDERS.len(), 6);
        assert_eq!(CAMERA_SLIDERS[0].default, 1.17);
        assert_eq!(CAMERA_SLIDERS[1].default, -0.11);
        assert_eq!(CAMERA_SLIDERS[2].default, 2.5);
        assert_eq!(CAMERA_SLIDERS[3].default, 70.0);
        assert_eq!(
            (CAMERA_SLIDERS[3].min, CAMERA_SLIDERS[3].max),
            (30.0, 110.0)
        );

        // The misc page carries the ten features, four at a time.
        page.select(CameraPage::Misc);
        assert_eq!(page.steppers(&settings).len(), 10);
        assert_eq!(MAX_VISIBLE_FEATURES, 4);
        assert_eq!(page.feature_scroll_max(), 6);
        assert_eq!(
            FEATURES[8].setting, None,
            "better_combat has no Horizon setting"
        );
        assert_eq!(
            FEATURES[9].setting, None,
            "spatial_gui has no Horizon setting"
        );

        // The shortcuts page asks for the four keys.
        page.select(CameraPage::Shortcuts);
        assert_eq!(page.steppers(&settings).len(), 4);

        // The focus stays inside the rows it walks.
        page.focus_at(99, 3);
        assert_eq!(page.focus(), 2);
        assert_eq!(page.nudge_focus(1, 3), 0);
        assert_eq!(page.nudge_focus(-1, 3), 2);
        page.focus_at(0, 0);
        assert_eq!(page.focus(), 0);
    }

    #[test]
    fn clicking_the_camera_screen_changes_its_settings() {
        let mut settings = ShoulderCameraSettings::default();
        let mut page = ShoulderCameraUiState::new();
        page.select(CameraPage::Camera);

        // The first row is the camera's on/off switch.
        assert_eq!(page.click(&mut settings, 0), UiRequest::None);
        assert!(!settings.enabled);
        page.click(&mut settings, 0);
        assert!(settings.enabled);

        // A feature row toggles its own flag; the compat rows do nothing.
        page.select(CameraPage::Misc);
        page.click(&mut settings, 0);
        assert!(!settings.fence_camera);
        assert_eq!(page.click(&mut settings, 8), UiRequest::None);

        // The shortcuts page asks for the four keys.
        page.select(CameraPage::Shortcuts);
        assert_eq!(
            page.click(&mut settings, 0),
            UiRequest::RebindKey(crate::camera::TOGGLE_CAMERA)
        );
        assert_eq!(
            page.click(&mut settings, 1),
            UiRequest::RebindKey(crate::camera::TOGGLE_EDITOR)
        );
        assert_eq!(
            page.click(&mut settings, 2),
            UiRequest::RebindKey(crate::camera::TOGGLE_ORBIT)
        );
        assert_eq!(
            page.click(&mut settings, 3),
            UiRequest::RebindKey(EMOTE_WHEEL_KEY)
        );
        // A row past the page does nothing.
        assert_eq!(page.click(&mut settings, 99), UiRequest::None);
    }

    #[test]
    fn the_camera_screen_scrolls_its_pages() {
        let mut page = ShoulderCameraUiState::new();
        page.select(CameraPage::Misc);
        assert_eq!(page.feature_scroll_max(), 6);
        page.scroll(1);
        assert_eq!(page.feature_scroll(), 1);
        for _ in 0..10 {
            page.scroll(1);
        }
        assert_eq!(page.feature_scroll(), 6);
        for _ in 0..10 {
            page.scroll(-1);
        }
        assert_eq!(page.feature_scroll(), 0);
        page.select(CameraPage::Camera);
        assert_eq!(page.slider_scroll_max(), 2);
        page.scroll(5);
        assert_eq!(page.slider_scroll(), 2);
        page.select(CameraPage::Shortcuts);
        // The four fixed keys plus one row per emote slot, ten at a time.
        assert_eq!(ShoulderCameraUiState::shortcut_count(), 4 + MAX_EMOTES);
        assert_eq!(page.shortcut_scroll_max(), 2);
        page.scroll(3);
        assert_eq!(page.shortcut_scroll(), 2);
    }

    #[test]
    fn the_camera_page_draws_its_tabs_and_rows() {
        let mut page = ShoulderCameraUiState::new();
        let mut frame = HudFrame::new();
        page.draw(
            &layout(),
            &ShoulderCameraSettings::default(),
            &theme(),
            &mut frame,
        );
        assert!(frame.is_empty());

        page.open();
        page.draw(
            &layout(),
            &ShoulderCameraSettings::default(),
            &theme(),
            &mut frame,
        );
        assert!(frame.len() > 6);
        // The tabs' labels are drawn on the header strip.
        let labels = frame
            .draws()
            .iter()
            .filter_map(|draw| match draw {
                HudDraw::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for tab in CameraPage::ALL {
            assert!(labels.contains(&tab.label()), "{} missing", tab.label());
        }
    }

    #[test]
    fn the_colour_picker_moves_through_rgb_hsv_and_hex() {
        let mut picker = ColorPickerState::new();
        picker.set_hex("#336699");
        assert_eq!(picker.rgb(), [0x33, 0x66, 0x99]);
        assert_eq!(picker.hex(), "#336699");
        // Without the hash too.
        assert!(picker.set_hex("FF0000"));
        assert_eq!(picker.rgb(), [255, 0, 0]);
        // Bad text leaves the colour alone.
        assert!(!picker.set_hex("#12345"));
        assert!(!picker.set_hex("zzzzzz"));
        assert_eq!(picker.rgb(), [255, 0, 0]);

        // Saturation steps down from pure red toward white and stops there.
        picker.focus_at(1);
        picker.step(-0.75);
        assert_eq!(picker.rgb(), [255, 191, 191]);
        picker.step(-4.0);
        assert_eq!(picker.rgb(), [255, 255, 255]);

        // Hue focus turns the colour round.
        picker.set_hex("#FF0000");
        picker.focus_at(0);
        picker.step(120.0);
        assert_eq!(picker.rgb(), [0, 255, 0]);
        picker.step(-240.0);
        assert_eq!(picker.rgb(), [0, 0, 255]);

        // Brightness down to black.
        picker.set_hex("#FF0000");
        picker.focus_at(2);
        picker.step(-1.0);
        assert_eq!(picker.rgb(), [0, 0, 0]);

        // Focus wraps through three channels.
        picker.focus_at(0);
        assert_eq!(picker.nudge_focus(-1), COLOR_CHANNELS - 1);
        assert_eq!(picker.nudge_focus(1), 0);

        // A swatch is a colour.
        picker.select_swatch(4);
        assert_eq!(picker.swatches()[4], picker.rgb());
        picker.select_swatch(99);
        assert_eq!(picker.swatches()[4], picker.rgb());
    }

    #[test]
    fn hsv_and_rgb_round_trip() {
        for hex in [
            "#000000", "#FFFFFF", "#FF0000", "#00FF00", "#0000FF", "#336699", "#C08040",
        ] {
            let mut picker = ColorPickerState::new();
            assert!(picker.set_hex(hex));
            let hsv = picker.hsv();
            let rgb = hsv_to_rgb(hsv);
            let back = picker.rgb();
            for channel in 0..3 {
                assert!(
                    (rgb[channel] * 255.0 - back[channel] as f32).abs() <= 1.0,
                    "{} channel {} drifted",
                    hex,
                    channel
                );
            }
        }
        // Grey has no hue and no saturation.
        let grey = rgb_to_hsv([0.5, 0.5, 0.5]);
        assert_eq!(grey[0], 0.0);
        assert_eq!(grey[1], 0.0);
        assert!((grey[2] - 0.5).abs() < 1e-6);
        // Black is no value at all.
        assert_eq!(rgb_to_hsv([0.0, 0.0, 0.0])[2], 0.0);
    }

    #[test]
    fn the_picker_keeps_the_hue_of_greys_and_opens_over_a_colour() {
        let mut picker = ColorPickerState::new();
        // Opening remembers the colour the cancel returns to.
        picker.open_over([0x33, 0x66, 0x99]);
        assert_eq!(picker.original(), [0x33, 0x66, 0x99]);
        assert_eq!(picker.rgb(), [0x33, 0x66, 0x99]);
        // A grey keeps its hue: red to white to green stays green-hued.
        picker.set_hex("#FF0000");
        picker.set_hex("#808080");
        let hue = picker.hsv()[0];
        picker.set_hex("#00FF00");
        picker.set_hex("#FFFFFF");
        picker.set_hex("#FF0000");
        let _ = hue;
        // The Java rule: a set with saturation keeps the new hue, a grey
        // keeps the old one.
        picker.set_hex("#40FF40");
        let green_hue = picker.hsv()[0];
        picker.set_hex("#202020");
        assert!(
            (picker.hsv()[0] - green_hue).abs() < 1e-3,
            "grey kept the hue"
        );
        picker.set_hex("#FF0000");
        assert!(
            (picker.hsv()[0] - 0.0).abs() < 1e-3,
            "saturated set took the hue"
        );
    }

    #[test]
    fn the_picker_picks_from_the_square_and_the_bar() {
        let mut picker = ColorPickerState::new();
        picker.open_over([0, 0, 0]);
        let l = layout();
        let square = picker.square(&l);
        // Top right of the square: full saturation, full brightness.
        assert!(picker.pick_at(&l, square.right() - 1.0, square.y + 1.0, true));
        assert_eq!(picker.dragging(), 1);
        assert!(picker.saturation() > 0.95, "{}", picker.saturation());
        assert!(picker.brightness() > 0.95, "{}", picker.brightness());
        // Bottom left: no saturation, no brightness.
        picker.pick_at(&l, square.x, square.bottom() - 1.0, false);
        assert!(picker.saturation() < 0.05);
        assert!(picker.brightness() < 0.05);
        // The bar sets the hue; a near-white pick gets colour back.
        picker.set_hex("#FFFFFF");
        let bar = picker.bar(&l);
        picker.pick_at(
            &l,
            bar.center()[0],
            square.y + (square.h - 1.0) * 0.25,
            true,
        );
        assert_eq!(picker.dragging(), 2);
        assert!((picker.hue() - 0.25).abs() < 0.02, "{}", picker.hue());
        assert!(
            (picker.saturation() - 0.6).abs() < 1e-4,
            "the white got colour back"
        );
        // A click off both parts starts no drag.
        picker.release();
        assert!(!picker.pick_at(&l, 1.0, 1.0, true));
        assert_eq!(picker.dragging(), 0);
        // Closing hands back the original.
        let original = picker.close();
        assert_eq!(original, [0, 0, 0]);
        assert!(!picker.is_open());
    }

    #[test]
    fn the_colour_picker_draws_its_rows_and_swatches() {
        let mut picker = ColorPickerState::new();
        let mut frame = HudFrame::new();
        picker.draw(&layout(), &theme(), &mut frame);
        assert!(frame.is_empty());
        picker.open();
        picker.draw(&layout(), &theme(), &mut frame);
        // Rows, the hex line, the preview and eight swatches.
        assert!(frame.len() > 8);
    }

    #[test]
    fn the_canvas_edits_a_cue_and_previews_it() {
        let mut canvas = FaceCanvasState::new();
        assert_eq!(canvas.expression(), FaceExpression::Happy);
        canvas.open();

        // A new expression starts from its built-in cue.
        canvas.select_expression(FaceExpression::Angry);
        assert_eq!(canvas.cue(), FaceExpression::Angry.cue());

        // The focused field moves, and stops at its bounds.
        canvas.focus_at(0);
        canvas.step(0.5);
        assert!(canvas.cue().lid > FaceExpression::Angry.cue().lid);
        canvas.step(10.0);
        assert_eq!(canvas.cue().lid, 1.0);
        canvas.focus_at(2);
        canvas.step(-0.2);
        assert!(canvas.cue().mouth < 0.15 + 1e-6);
        canvas.focus_at(1);
        canvas.step(-10.0);
        assert_eq!(canvas.cue().brow, -0.6);

        // The last field is the cue's length, and the focus wraps.
        canvas.focus_at(4);
        assert_eq!(canvas.nudge_focus(1), 0);
        assert_eq!(canvas.nudge_focus(-1), CANVAS_FIELDS - 1);
        canvas.step(1.0);
        assert!(canvas.cue().seconds > 0.0);

        // The preview draws the eyes, brows and mouth at least.
        let quads = canvas.preview(&FaceSettings::default());
        assert!(quads.len() >= 5);
        assert!(quads.iter().any(|q| q.uv == crate::faces::FX_IRIS));
    }

    #[test]
    fn the_canvas_draws_its_fields_and_the_face() {
        let mut canvas = FaceCanvasState::new();
        let mut frame = HudFrame::new();
        canvas.draw(&layout(), &theme(), &mut frame);
        assert!(frame.is_empty());
        canvas.open();
        canvas.draw(&layout(), &theme(), &mut frame);
        assert!(frame.len() > 8);
    }

    #[test]
    fn the_tutorial_walks_its_steps_and_ends() {
        let mut tutorial = TutorialState::new();
        assert!(!tutorial.is_open());
        assert!(tutorial.current().is_none());

        tutorial.start();
        assert_eq!(tutorial.step(), 0);
        assert_eq!(tutorial.current().unwrap().area, GUIDE[0].area);
        assert!(!tutorial.back());

        // Forward through every step; the last one closes it.
        for step in 1..TUTORIAL_STEPS {
            assert!(!tutorial.next());
            assert_eq!(tutorial.step(), step);
        }
        assert!(tutorial.next());
        assert!(!tutorial.is_open());
        assert_eq!(tutorial.step(), 0);

        // Backing up works from the second step on.
        tutorial.start();
        tutorial.next();
        assert!(tutorial.back());
        assert_eq!(tutorial.step(), 0);
        tutorial.close();
        assert!(!tutorial.is_open());
    }

    #[test]
    fn the_tutorial_draws_only_while_it_is_up() {
        let mut tutorial = TutorialState::new();
        let mut frame = HudFrame::new();
        tutorial.draw(&layout(), &theme(), &mut frame);
        assert!(frame.is_empty());
        tutorial.start();
        tutorial.draw(&layout(), &theme(), &mut frame);
        // The wash, the panel and its border, the title, the body and the hint.
        assert!(frame.len() >= 7);
    }

    #[test]
    fn gold_colours_read_the_java_ints() {
        // Opaque white is -1 in Java.
        assert_eq!(gold::TEXT_1, rgba(255, 255, 255, 255));
        // The wheel's wash carries its alpha in the top byte, 0x35.
        assert_eq!(gold::WHEEL_WASH, rgba(0, 0, 0, 0x35));
        // The backdrop is 0x8C alpha over black.
        assert_eq!(gold::BACKDROP, rgba(0, 0, 0, 0x8C));
        // The gold ramp's deepest step, 0xFF7A5207 read as ARGB.
        assert_eq!(gold::GOLD_700, rgba(0x7A, 0x52, 0x07, 0xFF));
        // The bevels keep their five colours and swap a face.
        assert_eq!(gold::GOLD.face, gold::GOLD_400);
        assert_eq!(gold::GOLD.with_face(gold::KNOB).face, gold::KNOB);
        assert_eq!(gold::GOLD.with_face(gold::KNOB).outline, gold::GOLD.outline);
    }

    #[test]
    fn the_punchy_order_is_a_threshold_tile() {
        // Every threshold appears exactly once, in the shader's order.
        let mut seen = [false; 16];
        for value in PUNCHY_ORDER {
            assert!(value < 16);
            assert!(!seen[value as usize], "{} repeats", value);
            seen[value as usize] = true;
        }
        assert!(seen.iter().all(|v| *v));
        assert_eq!(
            PUNCHY_ORDER,
            [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5]
        );
        // It tiles every 4 pixels, read the shader's way.
        assert_eq!(punchy_order(0, 0), punchy_order(4, 8));
        assert_eq!(punchy_order(1, 2), PUNCHY_ORDER[2 * 4 + 1]);
        // The shader's fixed cut: six of the sixteen cells survive.
        assert_eq!(PUNCHY_THRESHOLD, 6);
        let mut kept = 0;
        for y in 0..4 {
            for x in 0..4 {
                kept += u32::from(!punchy_discard(x, y));
            }
        }
        assert_eq!(kept, 6);
        assert!(!punchy_discard(0, 0), "cell 0, order 0, survives");
    }

    #[test]
    fn dithering_covers_the_share_its_alpha_asks_for() {
        assert!(!dither(0, 0, 0.0));
        assert!(!dither(0, 0, f32::NAN));
        assert!(dither(0, 0, 1.0));
        assert!(dither(0, 0, 2.0));

        // Half opacity shows eight of the sixteen pixels.
        let mut shown = 0;
        for y in 0..4 {
            for x in 0..4 {
                if dither(x, y, 0.5) {
                    shown += 1;
                }
            }
        }
        assert_eq!(shown, 8);
        // A quarter shows four, and a full tile always shows.
        let mut quarter = 0;
        for y in 0..4 {
            for x in 0..4 {
                quarter += u32::from(dither(x, y, 0.25));
            }
        }
        assert_eq!(quarter, 4);
    }

    #[test]
    fn the_near_fade_goes_from_none_to_full() {
        let dither = DitherVisibility::new();
        assert_eq!(dither.alpha(0.0), 0.0);
        assert_eq!(dither.alpha(dither.near), 0.0);
        let middle = dither.near + dither.fade * 0.5;
        assert!((dither.alpha(middle) - 0.5).abs() < 1e-6);
        assert!((dither.alpha(dither.near + dither.fade) - 1.0).abs() < 1e-6);
        assert_eq!(dither.alpha(100.0), 1.0);
        // A broken distance is far away, so fully drawn.
        assert_eq!(dither.alpha(f32::NAN), 1.0);

        // A body in front of the eye is dithered away; one far off is drawn.
        assert!(!dither.visible(0, 0, 0.0));
        assert!(dither.visible(0, 0, 4.0));
        // A zero fade never divides by zero.
        let sharp = DitherVisibility {
            near: 0.5,
            fade: 0.0,
        };
        assert_eq!(sharp.alpha(0.5), 1.0);
        assert_eq!(sharp.alpha(0.4), 0.0);
    }

    #[test]
    fn every_screen_opens_draws_and_closes() {
        let mut screens = PunchyUi::new();
        assert!(!screens.any_open());
        for (index, (label, screen)) in settings_buttons().iter().copied().enumerate() {
            assert!(!label.is_empty());
            assert_eq!(screens.click_settings(index), UiRequest::None);
            assert!(screens.any_open(), "{:?} did not open", screen);
            let frame = screens.draw(&layout(), &ShoulderCameraSettings::default(), &theme());
            assert!(!frame.is_empty(), "{:?} drew nothing", screen);
            screens.close_all();
            assert!(!screens.any_open());
        }
        // A button past the list does nothing.
        assert_eq!(screens.click_settings(99), UiRequest::None);
        assert!(!screens.any_open());
    }

    #[test]
    fn keys_drive_the_screens_that_are_up() {
        let mut screens = PunchyUi::new();
        // Nothing up: no screen wants the key.
        assert_eq!(screens.key("key.keyboard.escape", true), None);

        // The wheel: numbers pick a wedge, Enter plays, Escape puts it away.
        screens.emote_wheel.open();
        assert_eq!(screens.key("key.keyboard.3", true), Some(UiRequest::None));
        assert_eq!(screens.emote_wheel.selected(), 2);
        assert_eq!(
            screens.key("key.keyboard.right", true),
            Some(UiRequest::None)
        );
        assert_eq!(screens.emote_wheel.selected(), 3);
        screens.key("key.keyboard.3", true);
        assert_eq!(
            screens.key("key.keyboard.enter", true),
            Some(UiRequest::PlayEmote(2))
        );
        assert!(!screens.emote_wheel.is_open());

        screens.emote_wheel.open();
        assert_eq!(
            screens.key("key.keyboard.escape", true),
            Some(UiRequest::None)
        );
        assert!(!screens.emote_wheel.is_open());

        // The tutorial: Enter walks on, Escape closes it.
        screens.tutorial.start();
        let start = screens.tutorial.step();
        assert_eq!(
            screens.key("key.keyboard.enter", true),
            Some(UiRequest::None)
        );
        assert_eq!(screens.tutorial.step(), start + 1);
        assert_eq!(
            screens.key("key.keyboard.escape", true),
            Some(UiRequest::None)
        );
        assert!(!screens.tutorial.is_open());

        // The camera page: Tab changes page, Escape closes.
        screens.camera.open();
        let tab = screens.camera.page();
        screens.key("key.keyboard.tab", true);
        assert_ne!(screens.camera.page(), tab);
        screens.key("key.keyboard.escape", true);
        assert!(!screens.camera.is_open());

        // The colour picker and the canvas: arrows move their focus.
        screens.color.open();
        screens.color.focus_at(0);
        screens.key("key.keyboard.right", true);
        assert_eq!(screens.color.focus(), 1);
        screens.key("key.keyboard.escape", true);
        assert!(!screens.color.is_open());

        screens.canvas.open();
        screens.canvas.focus_at(0);
        screens.key("key.keyboard.right", true);
        assert_eq!(screens.canvas.focus(), 1);
        screens.key("key.keyboard.escape", true);
        assert!(!screens.canvas.is_open());
    }

    #[test]
    fn the_screens_thread_state_starts_fresh() {
        reset_ui();
        ui(|screens| screens.emote_wheel.open());
        assert!(ui(|screens| screens.emote_wheel.is_open()));
        reset_ui();
        assert!(!ui(|screens| screens.any_open()));
    }

    #[test]
    fn resting_signals_hold_still() {
        let signals = resting_signals();
        assert!(!signals.moving);
        assert!(!signals.mining);
        assert!(!signals.charging_crossbow);
        assert_eq!(signals.tick, crate::signals::NEVER);
    }
}
