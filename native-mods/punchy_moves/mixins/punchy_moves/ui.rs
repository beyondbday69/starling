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
            dim: rgba(0, 0, 0, 150),
            panel: rgba(14, 14, 22, 240),
            border: rgba(96, 98, 128, 255),
            title: rgba(255, 236, 170, 255),
            text: rgba(236, 236, 244, 255),
            value: rgba(150, 205, 255, 255),
            muted: rgba(150, 152, 172, 255),
            selected: rgba(70, 110, 170, 255),
            accent: rgba(255, 210, 90, 255),
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

/// The most emotes one wheel holds.
pub const MAX_EMOTES: usize = 8;
/// How far from the middle the wheel's emote boxes sit, in GUI pixels.
pub const WHEEL_RADIUS: f32 = 74.0;
/// One emote box's size, in GUI pixels.
pub const WHEEL_SLOT: f32 = 46.0;

/// The radial emote wheel: which emote is picked, and whether it is open.
#[derive(Debug, Clone, PartialEq)]
pub struct EmoteWheelState {
    open: bool,
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
            selected: 0,
            emotes,
        }
    }

    /// Whether the wheel is up.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Puts the wheel up, keeping the selection.
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

    /// The selected emote's index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The selected emote.
    pub fn emote(&self) -> Option<Emote> {
        self.emotes.get(self.selected).copied()
    }

    /// Selects an emote by index (wrapping; nothing when the wheel is empty).
    pub fn select(&mut self, index: usize) -> usize {
        if self.emotes.is_empty() {
            self.selected = 0;
        } else {
            self.selected = index % self.emotes.len();
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

    /// Selects the wedge a pointer offset `(dx, dy)` from the middle points
    /// at: the wheel's first emote is at the top, going clockwise.
    pub fn pointing(&mut self, dx: f32, dy: f32) -> usize {
        if !dx.is_finite() || !dy.is_finite() || (dx == 0.0 && dy == 0.0) {
            // No direction at all: the selection stays where it was.
            return self.selected;
        }
        let wedges = self.emotes.len().max(1) as f32;
        // Degrees clockwise from straight up, which is where wedge 0 sits in
        // the middle of its span.
        let degrees = dx.atan2(-dy).to_degrees();
        let angle = (degrees.rem_euclid(360.0) + 180.0 / wedges) % 360.0;
        let index = (angle / (360.0 / wedges)).floor() as usize;
        self.select(index)
    }

    /// The box the `index`th emote is drawn in, centred `WHEEL_RADIUS` from
    /// the middle of the screen at its wedge's middle angle.
    pub fn slot_rect(&self, layout: &UiLayout, index: usize) -> UiRect {
        let count = self.emotes.len().max(1) as f32;
        let middle = -90.0 + index as f32 * (360.0 / count);
        let (dx, dy) = (
            middle.to_radians().cos() * WHEEL_RADIUS,
            middle.to_radians().sin() * WHEEL_RADIUS,
        );
        let [cx, cy] = layout.center();
        UiRect::new(
            cx + dx - WHEEL_SLOT * 0.5,
            cy + dy - WHEEL_SLOT * 0.5,
            WHEEL_SLOT,
            WHEEL_SLOT,
        )
    }

    /// Draws the open wheel: a dim wash, a ring of emote boxes and the
    /// selected emote's name in the middle.
    pub fn draw(&self, layout: &UiLayout, theme: &UiTheme, frame: &mut HudFrame) {
        if !self.open {
            return;
        }
        frame.rect(
            UiRect::new(0.0, 0.0, layout.width, layout.height),
            theme.dim,
        );
        let [cx, cy] = layout.center();
        for (index, emote) in self.emotes.iter().enumerate() {
            let slot = self.slot_rect(layout, index);
            let color = if index == self.selected {
                theme.selected
            } else {
                theme.panel
            };
            frame.rect(slot, color);
            let width = emote.label.chars().count() as f32 * GLYPH_WIDTH;
            frame.text(
                emote.label,
                slot.center()[0] - width * 0.5,
                slot.center()[1] - 4.0,
                if index == self.selected {
                    theme.accent
                } else {
                    theme.text
                },
            );
        }
        // The middle: what a release would play.
        let name = self.emote().map_or("", |emote| emote.label);
        let width = name.chars().count() as f32 * GLYPH_WIDTH;
        frame.text(name.to_uppercase(), cx - width * 0.5, cy - 4.0, theme.title);
    }
}

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
        state.items.play(emote.action);
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
// The shoulder camera's page
// ---------------------------------------------------------------------------

/// One tab of the shoulder camera's page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraTab {
    /// On or off, the field of view and how far back.
    #[default]
    Camera,
    /// Where the camera sits beside and above the eye, and how it turns.
    Position,
    /// Which situations let the camera engage.
    Features,
    /// The keys, with a way to change them.
    Shortcuts,
    /// The tutorial again.
    ReplayTutorial,
}

impl CameraTab {
    /// The tabs, left to right.
    pub const ALL: [CameraTab; 5] = [
        CameraTab::Camera,
        CameraTab::Position,
        CameraTab::Features,
        CameraTab::Shortcuts,
        CameraTab::ReplayTutorial,
    ];

    /// The tab's name on its header.
    pub fn label(self) -> &'static str {
        match self {
            CameraTab::Camera => "Camera",
            CameraTab::Position => "Position",
            CameraTab::Features => "Features",
            CameraTab::Shortcuts => "Shortcuts",
            CameraTab::ReplayTutorial => "Replay Tutorial",
        }
    }

    /// This tab's place in [`CameraTab::ALL`].
    pub fn index(self) -> usize {
        CameraTab::ALL
            .iter()
            .position(|tab| *tab == self)
            .unwrap_or(0)
    }

    /// The tab to the right (wrapping).
    pub fn next(self) -> CameraTab {
        CameraTab::ALL[(self.index() + 1) % CameraTab::ALL.len()]
    }

    /// The tab to the left (wrapping).
    pub fn previous(self) -> CameraTab {
        CameraTab::ALL[(self.index() + CameraTab::ALL.len() - 1) % CameraTab::ALL.len()]
    }
}

/// One settable row of the camera page: its label, its value as shown, and the
/// button of [`ShoulderCameraSettings::click`] it stands for (`None` for a
/// row that asks to rebind a key or replay the tutorial).
#[derive(Debug, Clone, PartialEq)]
pub struct Stepper {
    pub label: String,
    pub value: String,
    pub setting: Option<usize>,
}

/// The shoulder camera's page: its tab, its focus and its open state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShoulderCameraUiState {
    open: bool,
    tab: CameraTab,
    focus: usize,
}

impl Default for ShoulderCameraUiState {
    fn default() -> Self {
        Self {
            open: false,
            tab: CameraTab::Camera,
            focus: 0,
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

    /// The tab being shown.
    pub fn tab(&self) -> CameraTab {
        self.tab
    }

    /// Shows `tab`, putting its focus back on the first row.
    pub fn select(&mut self, tab: CameraTab) -> CameraTab {
        self.tab = tab;
        self.focus = 0;
        self.tab
    }

    /// Shows the next tab.
    pub fn next_tab(&mut self) -> CameraTab {
        let tab = self.tab.next();
        self.select(tab)
    }

    /// Shows the previous tab.
    pub fn previous_tab(&mut self) -> CameraTab {
        let tab = self.tab.previous();
        self.select(tab)
    }

    /// The focused row.
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// Focuses a row (clamped to the tab's rows when they are known).
    pub fn focus_at(&mut self, index: usize, rows: usize) -> usize {
        self.focus = if rows == 0 { 0 } else { index.min(rows - 1) };
        self.focus
    }

    /// Steps the focus down or up the tab's rows, wrapping.
    pub fn nudge_focus(&mut self, delta: i32, rows: usize) -> usize {
        if rows == 0 {
            self.focus = 0;
        } else {
            let rows = rows as i32;
            self.focus = (self.focus as i32 + delta).rem_euclid(rows) as usize;
        }
        self.focus
    }

    /// The rows of the tab being shown.
    pub fn steppers(&self, settings: &ShoulderCameraSettings) -> Vec<Stepper> {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        let row = |label: &str, value: String, setting: Option<usize>| Stepper {
            label: label.to_string(),
            value,
            setting,
        };
        match self.tab {
            CameraTab::Camera => vec![
                row(
                    "Shoulder camera",
                    on(settings.enabled).into(),
                    Some(crate::camera::ENABLED_INDEX),
                ),
                row(
                    "Field of view",
                    format!("{:.0} degrees", settings.fov),
                    Some(crate::camera::FOV_INDEX),
                ),
                row(
                    "Distance back",
                    format!("{:.1} blocks", settings.depth),
                    Some(crate::camera::DEPTH_INDEX),
                ),
            ],
            CameraTab::Position => vec![
                row(
                    "Sideways",
                    format!("{:.2} blocks", settings.horizontal),
                    Some(crate::camera::HORIZONTAL_INDEX),
                ),
                row(
                    "Height",
                    format!("{:.2} blocks", settings.vertical),
                    Some(crate::camera::VERTICAL_INDEX),
                ),
                row(
                    "Turn right",
                    format!("{:.0} degrees", settings.rotation_yaw),
                    Some(crate::camera::YAW_INDEX),
                ),
                row(
                    "Turn down",
                    format!("{:.0} degrees", settings.rotation_pitch),
                    Some(crate::camera::PITCH_INDEX),
                ),
            ],
            CameraTab::Features => vec![
                row(
                    "On fences",
                    on(settings.fence_camera).into(),
                    Some(crate::camera::FENCE_INDEX),
                ),
                row(
                    "On ladders",
                    on(settings.ladder_camera).into(),
                    Some(crate::camera::LADDER_INDEX),
                ),
                row(
                    "Beside walls",
                    on(settings.proximity_camera).into(),
                    Some(crate::camera::PROXIMITY_INDEX),
                ),
                row(
                    "While gliding",
                    on(settings.elytra_camera).into(),
                    Some(crate::camera::ELYTRA_INDEX),
                ),
                row(
                    "With a map",
                    on(settings.map_camera).into(),
                    Some(crate::camera::MAP_INDEX),
                ),
                row(
                    "Ranged aim fix",
                    on(settings.ranged_camera_fix).into(),
                    Some(crate::camera::RANGED_INDEX),
                ),
                row(
                    "Orbit camera",
                    on(settings.free_camera).into(),
                    Some(crate::camera::FREE_INDEX),
                ),
            ],
            CameraTab::Shortcuts => vec![
                row("Shoulder camera key", "rebind".into(), None),
                row("Orbit camera key", "rebind".into(), None),
                row("Emote wheel key", "rebind".into(), None),
            ],
            CameraTab::ReplayTutorial => vec![row("Replay tutorial", "start".into(), None)],
        }
    }

    /// A click on row `index` of the tab being shown: changes the setting or
    /// asks the game to do something only it can.
    pub fn click(&mut self, settings: &mut ShoulderCameraSettings, index: usize) -> UiRequest {
        let rows = self.steppers(settings);
        let Some(row) = rows.get(index) else {
            return UiRequest::None;
        };
        if let Some(setting) = row.setting {
            settings.click(setting);
            self.focus = index;
            return UiRequest::None;
        }
        self.focus = index;
        match self.tab {
            CameraTab::Shortcuts => match index {
                0 => UiRequest::RebindKey(crate::camera::TOGGLE_CAMERA),
                1 => UiRequest::RebindKey(crate::camera::TOGGLE_ORBIT),
                _ => UiRequest::RebindKey("emotes"),
            },
            CameraTab::ReplayTutorial => UiRequest::ReplayTutorial,
            CameraTab::Camera | CameraTab::Position | CameraTab::Features => UiRequest::None,
        }
    }

    /// Draws the page: a tab header strip, the rows, and a hint line.
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
        // The tabs, left to right, the shown one highlighted.
        let tab_width = (panel.w - 16.0) / CameraTab::ALL.len() as f32;
        for (index, tab) in CameraTab::ALL.iter().enumerate() {
            let rect = UiRect::new(
                panel.x + 8.0 + index as f32 * tab_width,
                panel.y + 5.0,
                tab_width - 2.0,
                12.0,
            );
            frame.rect(
                rect,
                if *tab == self.tab {
                    theme.selected
                } else {
                    theme.panel
                },
            );
            let width = tab.label().chars().count() as f32 * GLYPH_WIDTH;
            frame.text(
                tab.label(),
                rect.x + (rect.w - width) * 0.5,
                rect.y + 2.0,
                if *tab == self.tab {
                    theme.accent
                } else {
                    theme.muted
                },
            );
        }
        for (index, row) in rows.iter().enumerate() {
            frame.row(
                panel,
                index + 1,
                &row.label,
                &row.value,
                index == self.focus,
                theme,
            );
        }
        frame.footer(
            "Click a row to change it; Tab changes the page",
            panel,
            theme,
        );
    }
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

/// The picker's channels, in focus order.
pub const COLOR_CHANNELS: usize = 6;

/// A colour picker over one RGB colour, with HSV, hex and swatches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPickerState {
    open: bool,
    rgb: [f32; 3],
    focus: usize,
}

impl Default for ColorPickerState {
    fn default() -> Self {
        Self {
            open: false,
            rgb: [0.80, 0.58, 0.42],
            focus: 0,
        }
    }
}

impl ColorPickerState {
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

    /// The colour as RGB bytes.
    pub fn rgb(&self) -> [u8; 3] {
        [
            (self.rgb[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.rgb[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.rgb[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }

    /// Sets the colour from RGB bytes.
    pub fn set_rgb(&mut self, rgb: [u8; 3]) {
        self.rgb = [
            rgb[0] as f32 / 255.0,
            rgb[1] as f32 / 255.0,
            rgb[2] as f32 / 255.0,
        ];
    }

    /// The colour as hue (degrees), saturation and value.
    pub fn hsv(&self) -> [f32; 3] {
        rgb_to_hsv(self.rgb)
    }

    /// Sets the colour from hue (degrees), saturation and value.
    pub fn set_hsv(&mut self, hsv: [f32; 3]) {
        self.rgb = hsv_to_rgb(hsv);
    }

    /// The colour as `#RRGGBB`.
    pub fn hex(&self) -> String {
        let [r, g, b] = self.rgb();
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }

    /// Sets the colour from `#RRGGBB` or `RRGGBB`; `false` when the text is
    /// not a colour (the colour is left alone).
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

    /// The focused channel (0 red, 1 green, 2 blue, 3 hue, 4 saturation,
    /// 5 value).
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

    /// Moves the focused channel by `delta`: RGB channels take `-1.0..=1.0`
    /// steps, hue takes degrees and saturation and value take fractions.
    pub fn step(&mut self, delta: f32) {
        if !delta.is_finite() {
            return;
        }
        match self.focus {
            0 => self.rgb[0] = (self.rgb[0] + delta).clamp(0.0, 1.0),
            1 => self.rgb[1] = (self.rgb[1] + delta).clamp(0.0, 1.0),
            2 => self.rgb[2] = (self.rgb[2] + delta).clamp(0.0, 1.0),
            3 => {
                let mut hsv = self.hsv();
                hsv[0] = (hsv[0] + delta).rem_euclid(360.0);
                self.set_hsv(hsv);
            }
            4 => {
                let mut hsv = self.hsv();
                hsv[1] = (hsv[1] + delta).clamp(0.0, 1.0);
                self.set_hsv(hsv);
            }
            _ => {
                let mut hsv = self.hsv();
                hsv[2] = (hsv[2] + delta).clamp(0.0, 1.0);
                self.set_hsv(hsv);
            }
        }
    }

    /// Takes a swatch's colour.
    pub fn select_swatch(&mut self, index: usize) {
        if let Some(swatch) = SWATCHES.get(index) {
            self.set_rgb(*swatch);
        }
    }

    /// The swatches.
    pub fn swatches(&self) -> [[u8; 3]; 8] {
        SWATCHES
    }

    /// Draws the picker: the six channels, the hex line and the swatch strip.
    pub fn draw(&self, layout: &UiLayout, theme: &UiTheme, frame: &mut HudFrame) {
        if !self.open {
            return;
        }
        let panel = frame.panel(layout, PANEL_WIDTH, PANEL_HEIGHT, theme);
        frame.title("Face colour", panel, theme);
        let hsv = self.hsv();
        let [r, g, b] = self.rgb();
        let rows = [
            ("Red", format!("{}", r)),
            ("Green", format!("{}", g)),
            ("Blue", format!("{}", b)),
            ("Hue", format!("{:.0} degrees", hsv[0])),
            ("Saturation", format!("{:.0}%", hsv[1] * 100.0)),
            ("Value", format!("{:.0}%", hsv[2] * 100.0)),
        ];
        for (index, (label, value)) in rows.iter().enumerate() {
            frame.row(panel, index, label, value, index == self.focus, theme);
        }
        frame.row(panel, rows.len(), "Hex", &self.hex(), false, theme);
        // The colour itself and the swatches, along the bottom.
        let [rr, gg, bb] = self.rgb();
        let preview = UiRect::new(panel.x + 8.0, panel.bottom() - 26.0, 24.0, 18.0);
        frame.rect(preview, rgba(rr, gg, bb, 255));
        for (index, swatch) in SWATCHES.iter().enumerate() {
            let rect = UiRect::new(
                preview.right() + 8.0 + index as f32 * 24.0,
                preview.y,
                20.0,
                18.0,
            );
            frame.rect(rect, rgba(swatch[0], swatch[1], swatch[2], 255));
            frame.rect(UiRect::new(rect.x, rect.y, rect.w, 1.0), theme.border);
        }
    }
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

/// One step of the tutorial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TutorialStep {
    pub title: &'static str,
    pub body: &'static str,
    pub hint: &'static str,
}

/// The tutorial's steps, in order.
pub const TUTORIAL_STEPS: [TutorialStep; 5] = [
    TutorialStep {
        title: "The shoulder camera",
        body: "Punchy Moves puts the camera over your shoulder instead of behind your head.",
        hint: "Set a key in Options > Mods, then press it while playing.",
    },
    TutorialStep {
        title: "The emote wheel",
        body: "Hold the wheel key and point at an emote, then let go to play it.",
        hint: "The wheel's wedges follow your mouse; the number keys pick one.",
    },
    TutorialStep {
        title: "Your face",
        body: "The face blinks, follows what you do and reacts to a fall or a charge.",
        hint: "Open Face colour and Face canvas to change its look.",
    },
    TutorialStep {
        title: "The near fade",
        body: "Geometry closer than the fade distance fades out through a 4x4 dither.",
        hint: "The fade keeps your own body from filling the screen.",
    },
    TutorialStep {
        title: "All set",
        body: "Everything here is on the shoulder camera's page and in Options > Mods.",
        hint: "Choose Replay Tutorial there to see this again.",
    },
];

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
            TUTORIAL_STEPS.get(self.step).copied()
        } else {
            None
        }
    }

    /// Moves on; `true` when the last step is done and the tutorial closed.
    // A step forward, not an iterator's item: the name reads as the page does.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> bool {
        if self.step + 1 >= TUTORIAL_STEPS.len() {
            self.open = false;
            self.step = 0;
            true
        } else {
            self.step += 1;
            false
        }
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

    pub fn close(&mut self) {
        self.open = false;
    }

    /// Draws the step, its hint and its place in the run.
    pub fn draw(&self, layout: &UiLayout, theme: &UiTheme, frame: &mut HudFrame) {
        let Some(step) = self.current() else {
            return;
        };
        let panel = frame.panel(layout, PANEL_WIDTH, 120.0, theme);
        frame.title(step.title, panel, theme);
        frame.text(step.body, panel.x + 10.0, panel.y + 34.0, theme.text);
        frame.text(step.hint, panel.x + 10.0, panel.y + 52.0, theme.muted);
        let place = format!("{}/{}", self.step + 1, TUTORIAL_STEPS.len());
        let width = place.chars().count() as f32 * GLYPH_WIDTH;
        frame.text(
            place,
            panel.right() - width - 10.0,
            panel.y + 34.0,
            theme.value,
        );
        frame.footer(
            "Enter for the next step, Backspace for the last",
            panel,
            theme,
        );
    }
}

// ---------------------------------------------------------------------------
// The near-camera dither
// ---------------------------------------------------------------------------

/// The 4x4 Bayer matrix: each cell's threshold in sixteenths, in reading order.
pub const BAYER_4X4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// The Bayer threshold of a screen pixel, `0..=15`.
pub fn bayer4(x: u32, y: u32) -> u8 {
    BAYER_4X4[(y & 3) as usize][(x & 3) as usize]
}

/// Whether a screen pixel draws at `alpha`: a pixel whose threshold is below
/// the alpha shows, so a dithered shape covers the share of the screen its
/// alpha asks for.
pub fn dither(x: u32, y: u32, alpha: f32) -> bool {
    if !alpha.is_finite() || alpha <= 0.0 {
        return false;
    }
    if alpha >= 1.0 {
        return true;
    }
    (bayer4(x, y) as f32) < alpha * 16.0
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
    fn pointing_picks_the_wedge_under_the_mouse() {
        let mut wheel = EmoteWheelState::new();
        // Eight wedges: up is the first, then clockwise.
        assert_eq!(wheel.pointing(0.0, -100.0), 0);
        assert_eq!(wheel.pointing(100.0, -100.0), 1);
        assert_eq!(wheel.pointing(100.0, 0.0), 2);
        assert_eq!(wheel.pointing(100.0, 100.0), 3);
        assert_eq!(wheel.pointing(0.0, 100.0), 4);
        assert_eq!(wheel.pointing(-100.0, 100.0), 5);
        assert_eq!(wheel.pointing(-100.0, 0.0), 6);
        assert_eq!(wheel.pointing(-100.0, -100.0), 7);
        // No offset keeps the selection, and a broken one does not move it.
        wheel.select(3);
        assert_eq!(wheel.pointing(0.0, 0.0), 3);
        assert_eq!(wheel.pointing(f32::NAN, 1.0), 3);
    }

    #[test]
    fn the_wheels_slots_sit_round_the_middle() {
        let wheel = EmoteWheelState::new();
        let [cx, cy] = layout().center();
        let top = wheel.slot_rect(&layout(), 0);
        assert!(top.center()[1] < cy);
        assert!((top.center()[0] - cx).abs() < 1e-3);
        let bottom = wheel.slot_rect(&layout(), MAX_EMOTES / 2);
        assert!(bottom.center()[1] > cy);
        // Every slot is the same size and clear of the next.
        for index in 0..MAX_EMOTES {
            let slot = wheel.slot_rect(&layout(), index);
            assert_eq!(slot.w, WHEEL_SLOT);
            assert_eq!(slot.h, WHEEL_SLOT);
            let distance =
                ((slot.center()[0] - cx).powi(2) + (slot.center()[1] - cy).powi(2)).sqrt();
            assert!((distance - WHEEL_RADIUS).abs() < 1e-3);
        }
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
            assert_eq!(state.items.action(), emote.action);
        });
        // An emote needs a player the engine knows.
        assert!(!play_emote(Some(4242), emote));
        assert!(!play_emote(None, emote));
    }

    #[test]
    fn the_camera_page_has_five_tabs_and_five_screens() {
        let settings = ShoulderCameraSettings::default();
        let mut page = ShoulderCameraUiState::new();
        assert!(!page.is_open());
        assert_eq!(page.tab(), CameraTab::Camera);

        // Every tab names rows, and its label is unique.
        for tab in CameraTab::ALL {
            page.select(tab);
            let rows = page.steppers(&settings);
            assert!(!rows.is_empty(), "{:?} has no rows", tab);
            assert!(!tab.label().is_empty());
        }
        assert_eq!(CameraTab::Camera.next(), CameraTab::Position);
        assert_eq!(CameraTab::Camera.previous(), CameraTab::ReplayTutorial);
        assert_eq!(CameraTab::ReplayTutorial.next(), CameraTab::Camera);
        assert_eq!(CameraTab::ALL.len(), 5);

        // The focus stays inside the rows it walks.
        page.focus_at(99, 3);
        assert_eq!(page.focus(), 2);
        assert_eq!(page.nudge_focus(1, 3), 0);
        assert_eq!(page.nudge_focus(-1, 3), 2);
        page.focus_at(0, 0);
        assert_eq!(page.focus(), 0);
    }

    #[test]
    fn clicking_the_camera_page_changes_its_settings() {
        let mut settings = ShoulderCameraSettings::default();
        let mut page = ShoulderCameraUiState::new();
        page.select(CameraTab::Camera);

        // The first row is the camera's on/off switch.
        assert_eq!(page.click(&mut settings, 0), UiRequest::None);
        assert!(!settings.enabled);
        page.click(&mut settings, 0);
        assert!(settings.enabled);

        // A features row toggles its own flag.
        page.select(CameraTab::Features);
        let rows = page.steppers(&settings);
        assert_eq!(rows.len(), 7);
        page.click(&mut settings, 0);
        assert!(settings.fence_camera);

        // Shortcuts and Replay Tutorial ask the game to do something.
        page.select(CameraTab::Shortcuts);
        assert_eq!(
            page.click(&mut settings, 0),
            UiRequest::RebindKey(crate::camera::TOGGLE_CAMERA)
        );
        assert_eq!(
            page.click(&mut settings, 1),
            UiRequest::RebindKey(crate::camera::TOGGLE_ORBIT)
        );
        page.select(CameraTab::ReplayTutorial);
        assert_eq!(page.click(&mut settings, 0), UiRequest::ReplayTutorial);
        // A row past the page does nothing.
        assert_eq!(page.click(&mut settings, 99), UiRequest::None);
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
        for tab in CameraTab::ALL {
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

        // Steps move the focused channel and stop at its end.
        picker.focus_at(1);
        picker.step(0.25);
        assert_eq!(picker.rgb(), [255, 64, 0]);
        picker.step(-4.0);
        assert_eq!(picker.rgb(), [255, 0, 0]);

        // Hue focus turns the colour round.
        picker.set_hex("#FF0000");
        picker.focus_at(3);
        picker.step(120.0);
        assert_eq!(picker.rgb(), [0, 255, 0]);
        picker.step(-240.0);
        assert_eq!(picker.rgb(), [0, 0, 255]);

        // Saturation and value.
        picker.set_hex("#FF0000");
        picker.focus_at(4);
        picker.step(-1.0);
        assert_eq!(picker.rgb(), [255, 255, 255]);
        picker.focus_at(5);
        picker.step(-1.0);
        assert_eq!(picker.rgb(), [0, 0, 0]);

        // Focus wraps through six channels.
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
        assert_eq!(tutorial.current().unwrap().title, TUTORIAL_STEPS[0].title);
        assert!(!tutorial.back());

        // Forward through every step; the last one closes it.
        for step in 1..TUTORIAL_STEPS.len() {
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
    fn the_bayer_matrix_is_a_threshold_tile() {
        // Every threshold appears exactly once.
        let mut seen = [false; 16];
        for row in BAYER_4X4 {
            for value in row {
                assert!(value < 16);
                assert!(!seen[value as usize], "{} repeats", value);
                seen[value as usize] = true;
            }
        }
        assert!(seen.iter().all(|v| *v));
        // It tiles every 4 pixels.
        assert_eq!(bayer4(0, 0), bayer4(4, 8));
        assert_eq!(bayer4(1, 2), BAYER_4X4[2][1]);
        // The tile's bright cells are spread, not stacked.
        assert_eq!(BAYER_4X4[0][0], 0);
        assert_eq!(BAYER_4X4[3][0], 15);
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
        let tab = screens.camera.tab();
        screens.key("key.keyboard.tab", true);
        assert_ne!(screens.camera.tab(), tab);
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
        assert!(!signals.is_acting());
        assert_eq!(signals.locomotion(), 0.0);
    }
}
