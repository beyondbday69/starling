//! Punchy Moves' facial expressions: the settings page, the cue table, and the
//! per-player rig that blinks, follows the eyes and draws the face as quads on
//! the player's head.
//!
//! Pure Rust and std-only, so the tests run without a Horizon build. The game
//! side is one helper the bundle injects: `h_face.rs` appends the rig's quads
//! after a player's body is submitted (`mobs::render::render_with_hidden`),
//! and `h_frame.rs` keeps the settings and the animation clock.
//!
//! # The face textures
//!
//! The rig draws nothing but flat quads on the front of the head, each one a
//! cell of one of two of the mod's own textures:
//!
//! - `textures/face/fx.png`, 256x256, a 2x2 sheet of soft white masks: a round
//!   glow, a wide ellipse, a broad vertical band and a thin vertical streak.
//!   The rig reads them as the eye's shine, the eyelid's shade, the brow's and
//!   the mouth's strokes, and any expression's extra effect.
//! - `textures/face/marks.png`, 4x4, a 2x2 sheet of the little marks a
//!   reaction puts on a cheek: blush, sweat, a tear and a strain mark.
//!
//! Both are addressed in normalised texture space ([`FaceUv`]), so a resize of
//! either sheet only changes the constants here.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::signals::MotionSignals;

/// The effect sheet: a 2x2 atlas of white masks.
pub const FX_TEXTURE: &str = "punchy_moves:textures/face/fx.png";
/// The effect sheet's size in texels.
pub const FX_SIZE: f32 = 256.0;
/// The mark sheet: a 2x2 atlas of cheek marks.
pub const MARKS_TEXTURE: &str = "punchy_moves:textures/face/marks.png";
/// The mark sheet's size in texels.
pub const MARKS_SIZE: f32 = 4.0;

// ---------------------------------------------------------------------------
// FaceUv
// ---------------------------------------------------------------------------

/// One rectangle of a face texture in normalised coordinates, `0.0..=1.0`,
/// with `v` running down the image like every texture's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceUv {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

impl FaceUv {
    /// The whole texture.
    pub const FULL: FaceUv = FaceUv {
        u0: 0.0,
        v0: 0.0,
        u1: 1.0,
        v1: 1.0,
    };

    /// One cell of a `columns` by `rows` sheet, counted from the top left.
    pub fn cell(columns: u32, rows: u32, column: u32, row: u32) -> FaceUv {
        let columns = columns.max(1) as f32;
        let rows = rows.max(1) as f32;
        let column = (column as f32).min(columns - 1.0);
        let row = (row as f32).min(rows - 1.0);
        FaceUv {
            u0: column / columns,
            v0: row / rows,
            u1: (column + 1.0) / columns,
            v1: (row + 1.0) / rows,
        }
    }

    /// The rectangle's width and height in texture space.
    pub fn size(&self) -> [f32; 2] {
        [self.u1 - self.u0, self.v1 - self.v0]
    }

    /// The rectangle in texels of a `width` by `height` texture:
    /// `[x0, y0, x1, y1]`.
    pub fn px(&self, width: f32, height: f32) -> [f32; 4] {
        [
            self.u0 * width,
            self.v0 * height,
            self.u1 * width,
            self.v1 * height,
        ]
    }

    /// Whether the rectangle lies inside the texture and has a size.
    pub fn is_valid(&self) -> bool {
        let inside = |v: f32| (0.0..=1.0).contains(&v);
        inside(self.u0)
            && inside(self.v0)
            && inside(self.u1)
            && inside(self.v1)
            && self.u1 > self.u0
            && self.v1 > self.v0
    }

    /// A point of this rectangle from a point of the whole quad, `0.0..=1.0`
    /// across and down it: how a quad's corner becomes an atlas coordinate.
    pub fn at(&self, u: f32, v: f32) -> [f32; 2] {
        [
            self.u0 + (self.u1 - self.u0) * u.clamp(0.0, 1.0),
            self.v0 + (self.v1 - self.v0) * v.clamp(0.0, 1.0),
        ]
    }
}

/// The effect sheet's four cells, as the rig's four face layers.
///
/// `fx.png`'s top left is the round glow, top right the wide ellipse, bottom
/// left the vertical band and bottom right the thin streak.
pub const FX_IRIS: FaceUv = FaceUv {
    u0: 0.0,
    v0: 0.0,
    u1: 0.5,
    v1: 0.5,
};
pub const FX_LID: FaceUv = FaceUv {
    u0: 0.5,
    v0: 0.0,
    u1: 1.0,
    v1: 0.5,
};
pub const FX_BROW: FaceUv = FaceUv {
    u0: 0.0,
    v0: 0.5,
    u1: 0.5,
    v1: 1.0,
};
pub const FX_MOUTH: FaceUv = FaceUv {
    u0: 0.5,
    v0: 0.5,
    u1: 1.0,
    v1: 1.0,
};

/// The mark sheet's four cells.
pub const MARK_BLUSH: FaceUv = FaceUv {
    u0: 0.0,
    v0: 0.0,
    u1: 0.5,
    v1: 0.5,
};
pub const MARK_SWEAT: FaceUv = FaceUv {
    u0: 0.5,
    v0: 0.0,
    u1: 1.0,
    v1: 0.5,
};
pub const MARK_TEAR: FaceUv = FaceUv {
    u0: 0.0,
    v0: 0.5,
    u1: 0.5,
    v1: 1.0,
};
pub const MARK_VEIN: FaceUv = FaceUv {
    u0: 0.5,
    v0: 0.5,
    u1: 1.0,
    v1: 1.0,
};

// ---------------------------------------------------------------------------
// FaceSettings
// ---------------------------------------------------------------------------

/// The face's personality: how quick the rig is to smile, scowl or look away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Personality {
    /// Even-tempered: the plain thresholds.
    #[default]
    Neutral,
    /// Smiles sooner and blinks a little less.
    Cheerful,
    /// Scowls at the smallest thing and blinks more.
    Grumpy,
    /// Looks away sooner and blushes more.
    Shy,
}

impl Personality {
    /// The personalities in the order the settings page cycles them.
    pub const ALL: [Personality; 4] = [
        Personality::Neutral,
        Personality::Cheerful,
        Personality::Grumpy,
        Personality::Shy,
    ];

    /// The label the settings page shows.
    pub fn label(self) -> &'static str {
        match self {
            Personality::Neutral => "Neutral",
            Personality::Cheerful => "Cheerful",
            Personality::Grumpy => "Grumpy",
            Personality::Shy => "Shy",
        }
    }

    /// The next personality in [`Personality::ALL`].
    pub fn next(self) -> Personality {
        let at = Personality::ALL
            .iter()
            .position(|p| *p == self)
            .unwrap_or(0);
        Personality::ALL[(at + 1) % Personality::ALL.len()]
    }

    /// How much easier this personality smiles (`1.0` is neutral).
    pub fn cheer(self) -> f32 {
        match self {
            Personality::Neutral => 1.0,
            Personality::Cheerful => 1.4,
            Personality::Grumpy => 0.6,
            Personality::Shy => 0.9,
        }
    }

    /// How much quicker this personality scowls.
    pub fn grumble(self) -> f32 {
        match self {
            Personality::Neutral => 1.0,
            Personality::Cheerful => 0.6,
            Personality::Grumpy => 1.6,
            Personality::Shy => 0.8,
        }
    }

    /// How much more this personality blushes.
    pub fn blush(self) -> f32 {
        match self {
            Personality::Shy => 1.5,
            Personality::Cheerful => 1.1,
            Personality::Neutral => 1.0,
            Personality::Grumpy => 0.7,
        }
    }
}

/// The face's settings: on or off, how often it blinks, whether the eyes
/// follow the body, how strong a reaction shows and which personality wears it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceSettings {
    /// The face is drawn at all.
    pub enabled: bool,
    /// Mean seconds between blinks.
    pub blink_interval: f32,
    /// The eyes follow the walk, the climb and the swim.
    pub look_tracking: bool,
    /// How strong a reaction shows, `0.0..=1.0`.
    pub reaction_intensity: f32,
    /// How quick the rig is to react.
    pub personality: Personality,
}

/// The blink intervals one click of the page cycles through, in seconds.
const BLINK_STEPS: [f32; 5] = [2.0, 3.5, 5.0, 7.0, 10.0];
/// The reaction strengths one click cycles through.
const REACTION_STEPS: [f32; 5] = [0.25, 0.5, 0.75, 1.0, 1.25];
/// How many buttons the face section of the settings page has.
pub const BUTTONS: usize = 5;

impl Default for FaceSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            blink_interval: 5.0,
            look_tracking: true,
            reaction_intensity: 1.0,
            personality: Personality::Neutral,
        }
    }
}

impl FaceSettings {
    /// The defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the settings from stored values (`get` reads one key); a missing
    /// or unreadable value keeps its default.
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let defaults = Self::default();
        let flag = |key: &str, fallback: bool| get(key).map_or(fallback, |v| v.trim() != "false");
        let number = |key: &str, fallback: f32, min: f32, max: f32| {
            get(key)
                .and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .map(|v| v.clamp(min, max))
                .unwrap_or(fallback)
        };
        let personality = get("personality")
            .and_then(|name| {
                Personality::ALL
                    .into_iter()
                    .find(|p| p.label().eq_ignore_ascii_case(name.trim()))
            })
            .unwrap_or(defaults.personality);
        Self {
            enabled: flag("enabled", defaults.enabled),
            blink_interval: number("blink_interval", defaults.blink_interval, 0.5, 30.0),
            look_tracking: flag("look_tracking", defaults.look_tracking),
            reaction_intensity: number("reaction_intensity", defaults.reaction_intensity, 0.0, 2.0),
            personality,
        }
    }

    /// The settings page's buttons, in the order [`FaceSettings::click`]
    /// numbers them.
    pub fn labels(&self) -> Vec<String> {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("Face: {}", on(self.enabled)),
            format!("Blink every: {:.1} s", self.blink_interval),
            format!("Eyes follow: {}", on(self.look_tracking)),
            format!("Reactions: {:.0}%", self.reaction_intensity * 100.0),
            format!("Personality: {}", self.personality.label()),
        ]
    }

    /// A click on button `i` of [`FaceSettings::labels`]; an index past the
    /// section does nothing.
    pub fn click(&mut self, i: usize) {
        match i {
            0 => self.enabled = !self.enabled,
            1 => self.blink_interval = cycle(&BLINK_STEPS, self.blink_interval),
            2 => self.look_tracking = !self.look_tracking,
            3 => self.reaction_intensity = cycle(&REACTION_STEPS, self.reaction_intensity),
            4 => self.personality = self.personality.next(),
            _ => {}
        }
    }

    /// What to store, key by key.
    pub fn stored(&self) -> [(&'static str, String); BUTTONS] {
        [
            (
                "enabled",
                if self.enabled { "true" } else { "false" }.to_owned(),
            ),
            ("blink_interval", format!("{:.1}", self.blink_interval)),
            (
                "look_tracking",
                if self.look_tracking { "true" } else { "false" }.to_owned(),
            ),
            (
                "reaction_intensity",
                format!("{:.2}", self.reaction_intensity),
            ),
            ("personality", self.personality.label().to_owned()),
        ]
    }

    /// The strongest alpha a reaction may show at, `0.0..=1.0`.
    pub fn reaction_alpha(&self) -> f32 {
        self.reaction_intensity.clamp(0.0, 1.0)
    }

    /// The mean seconds between blinks, scaled by the personality: a grumpy
    /// face blinks sooner, a cheerful one later.
    pub fn blink_seconds(&self) -> f32 {
        let scale = match self.personality {
            Personality::Grumpy => 0.8,
            Personality::Cheerful => 1.15,
            Personality::Neutral | Personality::Shy => 1.0,
        };
        (self.blink_interval * scale).max(0.5)
    }
}

/// The next value of a step table; a value between steps counts as the first.
fn cycle(steps: &[f32], value: f32) -> f32 {
    let at = steps
        .iter()
        .position(|step| (step - value).abs() < 1e-4)
        .unwrap_or(0);
    steps[(at + 1) % steps.len()]
}

// ---------------------------------------------------------------------------
// Expressions and cues
// ---------------------------------------------------------------------------

/// One face the rig can wear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FaceExpression {
    /// The resting face.
    #[default]
    Idle,
    /// Eyes shut for a moment.
    Blink,
    /// A flinch: squeezed eyes, a raised brow, a small mark.
    Hurt,
    /// A smile.
    Happy,
    /// Wide eyes and open mouth.
    Surprised,
    /// Knitted brows and a flat mouth.
    Angry,
    /// Heavy lids and a drooping mouth.
    Sleepy,
    /// Water in the eyes while swimming.
    Swim,
    /// Hair and eyes streaming while gliding.
    Fly,
    /// A face an emote asked for.
    Emote,
}

impl FaceExpression {
    /// The built-in cue for this expression.
    pub fn cue(self) -> FaceCue {
        FaceCue::built_in(self)
    }
}

/// What one expression does to the face, and for how long.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceCue {
    /// The expression it plays.
    pub expression: FaceExpression,
    /// How long it lasts, seconds; `0.0` is a resting face that never times
    /// out.
    pub seconds: f32,
    /// The higher the priority, the sooner it wins over another cue.
    pub priority: u8,
    /// How far the lids close, `0.0..=1.0`, added to the blink.
    pub lid: f32,
    /// The brow's angle in radians: positive raises the inner ends.
    pub brow: f32,
    /// The mouth's openness, `0.0..=1.0`.
    pub mouth: f32,
    /// How far the eyes widen, `0.0..=1.0`.
    pub widen: f32,
    /// The extra effect drawn over the face, as a cell of the effect sheet.
    pub fx: Option<FaceUv>,
    /// The mark drawn on a cheek, as a cell of the mark sheet.
    pub mark: Option<FaceUv>,
}

impl FaceCue {
    /// The plain resting face.
    pub const IDLE: FaceCue = FaceCue {
        expression: FaceExpression::Idle,
        seconds: 0.0,
        priority: 0,
        lid: 0.0,
        brow: 0.0,
        mouth: 0.0,
        widen: 0.0,
        fx: None,
        mark: None,
    };

    /// The built-in cue of an expression.
    pub fn built_in(expression: FaceExpression) -> FaceCue {
        let base = FaceCue {
            expression,
            ..FaceCue::IDLE
        };
        match expression {
            FaceExpression::Idle => base,
            FaceExpression::Blink => FaceCue {
                seconds: 0.18,
                priority: 3,
                lid: 1.0,
                ..base
            },
            FaceExpression::Hurt => FaceCue {
                seconds: 0.55,
                priority: 6,
                lid: 0.55,
                brow: -0.22,
                mouth: 0.35,
                widen: 0.2,
                mark: Some(MARK_SWEAT),
                ..base
            },
            FaceExpression::Happy => FaceCue {
                seconds: 2.0,
                priority: 2,
                lid: 0.25,
                brow: 0.10,
                mouth: 0.45,
                ..base
            },
            FaceExpression::Surprised => FaceCue {
                seconds: 1.2,
                priority: 4,
                lid: 0.0,
                brow: 0.30,
                mouth: 0.8,
                widen: 1.0,
                fx: Some(FX_IRIS),
                ..base
            },
            FaceExpression::Angry => FaceCue {
                seconds: 1.6,
                priority: 4,
                lid: 0.35,
                brow: -0.32,
                mouth: 0.15,
                mark: Some(MARK_VEIN),
                ..base
            },
            FaceExpression::Sleepy => FaceCue {
                seconds: 3.0,
                priority: 1,
                lid: 0.65,
                brow: -0.08,
                mouth: 0.10,
                fx: Some(FX_LID),
                ..base
            },
            FaceExpression::Swim => FaceCue {
                seconds: 0.0,
                priority: 2,
                lid: 0.30,
                mouth: 0.20,
                mark: Some(MARK_TEAR),
                ..base
            },
            FaceExpression::Fly => FaceCue {
                seconds: 0.0,
                priority: 2,
                lid: 0.45,
                brow: 0.16,
                mouth: 0.25,
                fx: Some(FX_LID),
                ..base
            },
            FaceExpression::Emote => FaceCue {
                seconds: 1.5,
                priority: 5,
                mouth: 0.4,
                ..base
            },
        }
    }

    /// Whether this cue is a resting face that never runs out.
    pub fn is_resting(self) -> bool {
        self.seconds <= 0.0
    }
}

/// The cue table: the built-in faces plus any emote's own.
#[derive(Debug, Clone, Default)]
pub struct FaceLibrary {
    /// Named cues in the order they were added (emote names first).
    cues: Vec<(String, FaceCue)>,
}

impl FaceLibrary {
    /// The built-in table: one cue per expression, plus the emotes Punchy
    /// Moves knows by name.
    pub fn built_in() -> Self {
        let mut library = FaceLibrary::default();
        for (name, expression) in [
            ("idle", FaceExpression::Idle),
            ("blink", FaceExpression::Blink),
            ("hurt", FaceExpression::Hurt),
            ("happy", FaceExpression::Happy),
            ("surprised", FaceExpression::Surprised),
            ("angry", FaceExpression::Angry),
            ("sleepy", FaceExpression::Sleepy),
            ("swim", FaceExpression::Swim),
            ("fly", FaceExpression::Fly),
        ] {
            library.insert(name, FaceCue::built_in(expression));
        }
        // Emote-driven cues: an emote's name picks its face.
        for (name, expression, mouth, fx) in [
            ("wave", FaceExpression::Emote, 0.35, None),
            ("clap", FaceExpression::Happy, 0.55, None),
            ("dance", FaceExpression::Happy, 0.45, Some(FX_MOUTH)),
            ("bow", FaceExpression::Emote, 0.20, None),
            ("cry", FaceExpression::Hurt, 0.60, Some(MARK_TEAR)),
            ("laugh", FaceExpression::Happy, 0.75, Some(FX_MOUTH)),
        ] {
            library.insert(
                name,
                FaceCue {
                    mouth,
                    fx,
                    seconds: 2.4,
                    priority: 5,
                    ..FaceCue::built_in(expression)
                },
            );
        }
        library
    }

    /// The cue with this name, if the table has one.
    pub fn get(&self, name: &str) -> Option<FaceCue> {
        self.cues
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, cue)| *cue)
    }

    /// The cue for an expression: the table's own if it names one, the
    /// built-in otherwise.
    pub fn cue(&self, expression: FaceExpression) -> FaceCue {
        let name = match expression {
            FaceExpression::Idle => "idle",
            FaceExpression::Blink => "blink",
            FaceExpression::Hurt => "hurt",
            FaceExpression::Happy => "happy",
            FaceExpression::Surprised => "surprised",
            FaceExpression::Angry => "angry",
            FaceExpression::Sleepy => "sleepy",
            FaceExpression::Swim => "swim",
            FaceExpression::Fly => "fly",
            FaceExpression::Emote => "wave",
        };
        self.get(name)
            .unwrap_or_else(|| FaceCue::built_in(expression))
    }

    /// Adds or replaces a named cue.
    pub fn insert(&mut self, name: &str, cue: FaceCue) {
        match self.cues.iter_mut().find(|(key, _)| key == name) {
            Some((_, stored)) => *stored = cue,
            None => self.cues.push((name.to_string(), cue)),
        }
    }

    /// The cue names, in the order they were added.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.cues.iter().map(|(name, _)| name.as_str())
    }

    /// How many cues the table holds.
    pub fn len(&self) -> usize {
        self.cues.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.cues.is_empty()
    }
}

// ---------------------------------------------------------------------------
// The quads the head carries
// ---------------------------------------------------------------------------

/// One quad of a face on the front of a head, in head-local blocks: `x` to the
/// model's right, `y` up, `z` out through the face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceQuad {
    /// The face texture it shows.
    pub texture: &'static str,
    /// Its centre in head-local blocks.
    pub center: [f32; 3],
    /// Its width and height in head-local blocks.
    pub size: [f32; 2],
    /// Its roll about the face normal, radians.
    pub roll: f32,
    /// The rectangle of the texture it shows.
    pub uv: FaceUv,
    /// The colour and opacity it multiplies in, each `0.0..=1.0`.
    pub color: [f32; 4],
}

impl FaceQuad {
    /// A quad that draws nothing: the empty slot of a [`FaceQuads`].
    pub const EMPTY: FaceQuad = FaceQuad {
        texture: "",
        center: [0.0; 3],
        size: [0.0; 2],
        roll: 0.0,
        uv: FaceUv::FULL,
        color: [0.0; 4],
    };

    /// The quad's four corners in head-local space, anticlockwise from the
    /// bottom left.
    pub fn corners(&self) -> [[f32; 3]; 4] {
        let half_width = self.size[0] * 0.5;
        let half_height = self.size[1] * 0.5;
        let (sin, cos) = self.roll.sin_cos();
        let corner = |x: f32, y: f32| {
            [
                self.center[0] + x * cos - y * sin,
                self.center[1] + x * sin + y * cos,
                self.center[2],
            ]
        };
        [
            corner(-half_width, -half_height),
            corner(half_width, -half_height),
            corner(half_width, half_height),
            corner(-half_width, half_height),
        ]
    }

    /// The texture coordinates of [`FaceQuad::corners`], in the same order.
    pub fn uv_corners(&self) -> [[f32; 2]; 4] {
        [
            [self.uv.u0, self.uv.v1],
            [self.uv.u1, self.uv.v1],
            [self.uv.u1, self.uv.v0],
            [self.uv.u0, self.uv.v0],
        ]
    }

    /// Whether the quad has a size and is not see-through.
    pub fn is_visible(&self) -> bool {
        self.size[0] > 0.0 && self.size[1] > 0.0 && self.color[3] > 0.0
    }
}

/// The most quads one face draws: two eyes, two lids, two brows, a mouth, an
/// effect and a mark.
pub const MAX_FACE_QUADS: usize = 10;

/// The quads a rig draws this frame, at most [`MAX_FACE_QUADS`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceQuads {
    quads: [FaceQuad; MAX_FACE_QUADS],
    len: usize,
}

impl Default for FaceQuads {
    fn default() -> Self {
        Self::new()
    }
}

impl FaceQuads {
    /// No quads.
    pub fn new() -> Self {
        Self {
            quads: [FaceQuad::EMPTY; MAX_FACE_QUADS],
            len: 0,
        }
    }

    /// Adds a quad; `false` when the list is already full.
    pub fn push(&mut self, quad: FaceQuad) -> bool {
        if self.len >= MAX_FACE_QUADS {
            return false;
        }
        self.quads[self.len] = quad;
        self.len += 1;
        true
    }

    /// The quads, in draw order.
    pub fn as_slice(&self) -> &[FaceQuad] {
        &self.quads[..self.len]
    }

    /// The quads, in draw order.
    pub fn iter(&self) -> impl Iterator<Item = &FaceQuad> {
        self.as_slice().iter()
    }

    /// How many quads the face drew.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the face drew nothing.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// ---------------------------------------------------------------------------
// FaceRig
// ---------------------------------------------------------------------------

/// How long one blink takes, seconds.
pub const BLINK_SECONDS: f32 = 0.18;
/// How fast the eyes swing towards where they are looking, per second.
const LOOK_RATE: f32 = 9.0;
/// How fast they come back to centre when tracking is off, per second.
const LOOK_RELEASE: f32 = 4.0;
/// How fast a reaction fades in and out, per second.
const REACTION_RATE: f32 = 7.0;
/// The eyes' centres, `x` from the middle of the face, in head-local blocks.
const EYE_X: f32 = 0.125;
/// The eyes' centre height.
const EYE_Y: f32 = 0.30;
/// The eyes' resting size.
const EYE_SIZE: [f32; 2] = [0.11, 0.05];
/// How far in front of the head's centre the face sits.
const FACE_Z: f32 = 0.2510;
/// The mouth's centre height.
const MOUTH_Y: f32 = 0.215;
/// How far the eyes may slide with the look, in blocks.
const LOOK_SHIFT: [f32; 2] = [0.020, 0.014];

/// One player's face: the blink timer, the eyes' look, the reaction and the
/// cue being worn.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceRig {
    /// The blink timer's state.
    seed: u32,
    blink_in: f32,
    blinking: bool,
    blink_progress: f32,
    blink: f32,
    /// The eyes' look, `-1.0..=1.0` across and up.
    look: [f32; 2],
    /// How strong the reaction shows, `0.0..=1.0`.
    reaction: f32,
    /// The cue being worn.
    cue: FaceCue,
    /// Seconds left of an explicit cue; `None` when none was asked for.
    held: Option<f32>,
}

impl Default for FaceRig {
    fn default() -> Self {
        Self::new(0)
    }
}

impl FaceRig {
    /// A rig with its own blink timer seed, so two faces never blink together.
    pub fn new(seed: u32) -> Self {
        let mut rig = Self {
            seed: seed | 1,
            blink_in: 0.0,
            blinking: false,
            blink_progress: 0.0,
            blink: 0.0,
            look: [0.0, 0.0],
            reaction: 0.0,
            cue: FaceCue::IDLE,
            held: None,
        };
        rig.blink_in = rig.random01() * 5.0;
        rig
    }

    /// How far the lids are closed right now, `0.0..=1.0`.
    pub fn blink(&self) -> f32 {
        self.blink
    }

    /// Seconds until the next blink.
    pub fn blink_timer(&self) -> f32 {
        self.blink_in
    }

    /// Sets the next blink to `seconds` from now (a scene, a test).
    pub fn set_blink_in(&mut self, seconds: f32) {
        self.blink_in = if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
    }

    /// The eyes' look, `-1.0..=1.0` across and up.
    pub fn look(&self) -> [f32; 2] {
        self.look
    }

    /// How strong the reaction shows, `0.0..=1.0`.
    pub fn reaction(&self) -> f32 {
        self.reaction
    }

    /// The cue being worn.
    pub fn cue(&self) -> FaceCue {
        self.cue
    }

    /// The expression being worn.
    pub fn expression(&self) -> FaceExpression {
        self.cue.expression
    }

    /// Whether an explicit cue is still playing.
    pub fn is_playing(&self) -> bool {
        self.held.is_some()
    }

    /// Wears `expression` for as long as its cue lasts, whatever the signals
    /// say in the meantime.
    pub fn play(&mut self, expression: FaceExpression) {
        self.play_cue(expression.cue());
    }

    /// Wears `cue` for as long as it lasts.
    pub fn play_cue(&mut self, cue: FaceCue) {
        self.cue = cue;
        self.held = (!cue.is_resting()).then_some(cue.seconds);
    }

    /// Back to a resting face with the eyes centred.
    pub fn reset(&mut self) {
        let seed = self.seed;
        *self = Self::new(seed);
    }

    /// One frame of the face.
    pub fn update(&mut self, signals: &MotionSignals, settings: &FaceSettings, dt: f32) {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.25)
        } else {
            0.0
        };
        if !settings.enabled {
            self.blink = 0.0;
            self.blinking = false;
            self.look = [0.0, 0.0];
            self.reaction = 0.0;
            self.cue = FaceCue::IDLE;
            self.held = None;
            return;
        }

        // The blink: a full close and open inside `BLINK_SECONDS`, then a
        // fresh interval, jittered so a crowd never blinks in step.
        self.blink_in -= dt;
        if !self.blinking && self.blink_in <= 0.0 {
            self.blinking = true;
            self.blink_progress = 0.0;
        }
        if self.blinking {
            self.blink_progress = (self.blink_progress + dt / BLINK_SECONDS).min(1.0);
            if self.blink_progress >= 1.0 {
                self.blinking = false;
                self.blink_progress = 0.0;
                self.blink_in = self.next_blink(settings);
            }
        }
        self.blink = if self.blinking {
            (std::f32::consts::PI * self.blink_progress).sin()
        } else {
            0.0
        };

        // The eyes: on the walk, up a ladder, up while swimming, down while
        // mining; centred when tracking is off.
        let target = if !settings.look_tracking {
            [0.0, 0.0]
        } else {
            look_target(signals)
        };
        let rate = if settings.look_tracking {
            LOOK_RATE
        } else {
            LOOK_RELEASE
        };
        self.look[0] = approach(self.look[0], target[0], rate * dt);
        self.look[1] = approach(self.look[1], target[1], rate * dt);

        // The reaction: a fall or a charge shows, scaled by the setting.
        let target = reaction_target(signals) * settings.reaction_alpha();
        self.reaction = approach(self.reaction, target, REACTION_RATE * dt);

        // The cue: an explicit one runs out, then the signals pick again.
        match self.held {
            Some(left) if left > dt => self.held = Some(left - dt),
            Some(_) => self.held = None,
            None => {}
        }
        if self.held.is_none() {
            self.cue = pick_cue(signals, settings);
        }
    }

    /// The seconds until the next blink, jittered around the setting.
    fn next_blink(&mut self, settings: &FaceSettings) -> f32 {
        (settings.blink_seconds() * (0.7 + 0.6 * self.random01())).max(0.5)
    }

    /// One deterministic draw in `0.0..1.0` (xorshift32).
    fn random01(&mut self) -> f32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x >> 8) as f32 / 16_777_216.0
    }

    /// The quads this face draws this frame, in head-local space.
    pub fn quads(&self, settings: &FaceSettings) -> FaceQuads {
        let mut quads = FaceQuads::new();
        if !settings.enabled {
            return quads;
        }
        let alpha = if settings.reaction_alpha() > 0.0 {
            (0.5 + 0.5 * self.reaction) * settings.reaction_alpha()
        } else {
            0.5
        };
        let lid = self.cue.lid.max(self.blink).clamp(0.0, 1.0);
        let widen = self.cue.widen.clamp(0.0, 1.0);
        let size = [
            EYE_SIZE[0] * (1.0 + 0.35 * widen),
            EYE_SIZE[1] * (1.0 + 0.50 * widen),
        ];
        let shift = [self.look[0] * LOOK_SHIFT[0], self.look[1] * LOOK_SHIFT[1]];

        // The two eyes: the iris cell of the effect sheet, slid with the look.
        for side in [-1.0, 1.0] {
            quads.push(FaceQuad {
                texture: FX_TEXTURE,
                center: [side * EYE_X + shift[0], EYE_Y + shift[1], FACE_Z],
                size,
                roll: 0.0,
                uv: FX_IRIS,
                color: [1.0, 1.0, 1.0, alpha],
            });
        }

        // The lids: the closed share of each eye, hanging from its top.
        if lid > 0.001 {
            let height = size[1] * lid;
            let center_y = EYE_Y + size[1] * 0.5 - height * 0.5 + shift[1];
            for side in [-1.0, 1.0] {
                quads.push(FaceQuad {
                    texture: FX_TEXTURE,
                    center: [side * EYE_X + shift[0], center_y, FACE_Z + 0.0004],
                    size: [size[0], height],
                    roll: 0.0,
                    uv: FX_LID,
                    color: [0.06, 0.06, 0.09, alpha],
                });
            }
        }

        // The brows: the vertical band cell, tilted by the cue's mood.
        let brow = self.cue.brow.clamp(-0.6, 0.6);
        for side in [-1.0, 1.0] {
            quads.push(FaceQuad {
                texture: FX_TEXTURE,
                center: [side * EYE_X, EYE_Y + size[1] * 0.5 + 0.035, FACE_Z + 0.0002],
                size: [0.10, 0.018],
                roll: side * brow,
                uv: FX_BROW,
                color: [0.16, 0.11, 0.09, alpha],
            });
        }

        // The mouth: the streak cell, opened and widened by the cue.
        let mouth = self.cue.mouth.clamp(0.0, 1.0);
        quads.push(FaceQuad {
            texture: FX_TEXTURE,
            center: [0.0, MOUTH_Y, FACE_Z + 0.0002],
            size: [0.11 * (1.0 + 0.30 * mouth), 0.012 + 0.05 * mouth],
            roll: 0.0,
            uv: FX_MOUTH,
            color: [0.38, 0.13, 0.13, alpha],
        });

        // The expression's effect and its cheek mark, when it has them.
        if let Some(fx) = self.cue.fx {
            quads.push(FaceQuad {
                texture: FX_TEXTURE,
                center: [0.0, EYE_Y, FACE_Z - 0.0006],
                size: [0.34, 0.34],
                roll: 0.0,
                uv: fx,
                color: [1.0, 1.0, 1.0, 0.45 * alpha],
            });
        }
        if let Some(mark) = self.cue.mark {
            let blush = settings.personality.blush();
            quads.push(FaceQuad {
                texture: MARKS_TEXTURE,
                center: [0.145, EYE_Y - 0.02, FACE_Z + 0.0006],
                size: [0.085, 0.085],
                roll: 0.0,
                uv: mark,
                color: [1.0, 1.0, 1.0, (0.75 * alpha * blush).clamp(0.0, 1.0)],
            });
        }
        quads
    }
}

/// Where the eyes look: across the walk, up a ladder and in water, down while
/// mining.
fn look_target(signals: &MotionSignals) -> [f32; 2] {
    if signals.elytra || signals.surface_swimming || signals.submerged_swimming {
        [0.0, signals.swim_vertical as f32]
    } else if signals.mining {
        [0.0, -0.8]
    } else if signals.climbing {
        [0.0, signals.climb_direction as f32]
    } else if signals.crouching {
        [0.0, -0.35]
    } else {
        [
            signals.walk_left.clamp(-1.0, 1.0) * 0.8,
            -signals.walk_forward.clamp(-1.0, 1.0) * 0.5,
        ]
    }
}

/// How strong a reaction the signals ask for, before the setting scales it.
fn reaction_target(signals: &MotionSignals) -> f32 {
    if signals.fall_weight >= crate::signals::LIGHT {
        1.0
    } else if signals.charging_crossbow || signals.charging_spear || signals.charging_trident {
        0.6
    } else if signals.elytra && signals.elytra_speed > 1.0 {
        0.4
    } else if signals.mining {
        0.25
    } else {
        0.0
    }
}

/// The cue the movement state wears when nothing explicit is playing: the
/// highest-priority match, the resting face when none.
fn pick_cue(signals: &MotionSignals, settings: &FaceSettings) -> FaceCue {
    let cheer = settings.personality.cheer();
    let grumble = settings.personality.grumble();
    let mut best = FaceCue::IDLE;
    let mut candidate = |cue: FaceCue| {
        if cue.priority > best.priority {
            best = cue;
        }
    };
    if signals.fall_weight >= crate::signals::LIGHT {
        candidate(FaceExpression::Hurt.cue());
    }
    if signals.mining || (signals.crouching && signals.moving && grumble > 1.0) {
        candidate(FaceExpression::Angry.cue());
    }
    if signals.fast_flight || (signals.charging_spear && signals.running) {
        candidate(FaceExpression::Surprised.cue());
    }
    if signals.running || (signals.moving && !signals.grounded && cheer > 1.0) {
        candidate(FaceExpression::Happy.cue());
    }
    if signals.elytra {
        candidate(FaceExpression::Fly.cue());
    }
    if signals.surface_swimming || signals.submerged_swimming {
        candidate(FaceExpression::Swim.cue());
    }
    if signals.crouching && !signals.moving {
        candidate(FaceExpression::Sleepy.cue());
    }
    best
}

// ---------------------------------------------------------------------------
// The settings the game's hooks read
// ---------------------------------------------------------------------------

thread_local! {
    /// The face settings, read once from the mod's stored values.
    static SETTINGS: RefCell<Option<FaceSettings>> = const { RefCell::new(None) };
}

/// The settings, read once through `get`.
pub fn settings(get: impl Fn(&str) -> Option<String>) -> FaceSettings {
    SETTINGS.with_borrow_mut(|s| *s.get_or_insert_with(|| FaceSettings::read(get)))
}

/// Replaces the settings (the settings page or a key changed them).
pub fn set_settings(settings: FaceSettings) {
    SETTINGS.with_borrow_mut(|s| *s = Some(settings));
}

/// The settings as last read, or the defaults when nothing has read them yet.
pub fn current_settings() -> FaceSettings {
    SETTINGS.with_borrow(|s| s.unwrap_or_default())
}

/// Forgets the settings (a mod reload reads them again).
pub fn reset_settings() {
    SETTINGS.with_borrow_mut(|s| *s = None);
}

/// The rigs of every player, keyed by entity id: the game's head overlay
/// reads the rig of the head it is drawing.
#[derive(Debug, Clone, Default)]
pub struct FaceRigs {
    rigs: HashMap<u64, FaceRig>,
}

impl FaceRigs {
    pub fn new() -> Self {
        Self::default()
    }

    /// The rig of `entity`, made (and seeded by its id) on first use.
    pub fn rig_mut(&mut self, entity: u64) -> &mut FaceRig {
        self.rigs
            .entry(entity)
            .or_insert_with(|| FaceRig::new(entity as u32))
    }

    /// The rig of `entity`, if it has one.
    pub fn rig(&self, entity: u64) -> Option<&FaceRig> {
        self.rigs.get(&entity)
    }

    /// Steps `entity`'s face by one frame.
    pub fn update(
        &mut self,
        entity: u64,
        signals: &MotionSignals,
        settings: &FaceSettings,
        dt: f32,
    ) {
        self.rig_mut(entity).update(signals, settings, dt);
    }

    /// `entity`'s face quads, or `None` when it has no rig.
    pub fn quads(&self, entity: u64, settings: &FaceSettings) -> Option<FaceQuads> {
        self.rig(entity).map(|rig| rig.quads(settings))
    }

    /// Forgets `entity`; whether it was there.
    pub fn remove(&mut self, entity: u64) -> bool {
        self.rigs.remove(&entity).is_some()
    }

    /// Forgets every face.
    pub fn clear(&mut self) {
        self.rigs.clear();
    }

    /// How many faces are tracked.
    pub fn len(&self) -> usize {
        self.rigs.len()
    }

    /// Whether no face is tracked.
    pub fn is_empty(&self) -> bool {
        self.rigs.is_empty()
    }
}

/// `a` moved towards `b` by at most `step`.
fn approach(a: f32, b: f32, step: f32) -> f32 {
    if !a.is_finite() {
        return b;
    }
    let step = if step.is_finite() { step.max(0.0) } else { 0.0 };
    let gap = b - a;
    if gap.abs() <= step {
        b
    } else if gap > 0.0 {
        a + step
    } else {
        a - step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walking() -> MotionSignals {
        MotionSignals {
            moving: true,
            walk_forward: 1.0,
            grounded: true,
            ..MotionSignals::new()
        }
    }

    fn stored(values: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let values: Vec<(String, String)> = values
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        move |key| {
            values
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
        }
    }

    #[test]
    fn uv_cells_tile_the_sheets() {
        let whole = FaceUv::cell(1, 1, 0, 0);
        assert_eq!(whole, FaceUv::FULL);
        assert!(whole.is_valid());

        // A 2x2 sheet's four cells cover it without overlapping.
        let mut cells = Vec::new();
        for row in 0..2 {
            for column in 0..2 {
                let cell = FaceUv::cell(2, 2, column, row);
                assert!(cell.is_valid());
                assert_eq!(cell.size(), [0.5, 0.5]);
                assert!(!cells.contains(&cell));
                cells.push(cell);
            }
        }
        let area: f32 = cells.iter().map(|c| c.size()[0] * c.size()[1]).sum();
        assert!((area - 1.0).abs() < 1e-6);
        // A cell past the last is clamped into the sheet, never outside it.
        assert_eq!(FaceUv::cell(2, 2, 9, 9), FaceUv::cell(2, 2, 1, 1));
        assert_eq!(FaceUv::cell(0, 0, 0, 0), FaceUv::FULL);

        // The rig's own regions match the sheets it ships.
        assert_eq!(FX_IRIS.px(FX_SIZE, FX_SIZE), [0.0, 0.0, 128.0, 128.0]);
        assert_eq!(FX_LID.px(FX_SIZE, FX_SIZE), [128.0, 0.0, 256.0, 128.0]);
        assert_eq!(FX_BROW.px(FX_SIZE, FX_SIZE), [0.0, 128.0, 128.0, 256.0]);
        assert_eq!(FX_MOUTH.px(FX_SIZE, FX_SIZE), [128.0, 128.0, 256.0, 256.0]);
        assert_eq!(MARK_BLUSH.px(MARKS_SIZE, MARKS_SIZE), [0.0, 0.0, 2.0, 2.0]);
        assert_eq!(MARK_VEIN.px(MARKS_SIZE, MARKS_SIZE), [2.0, 2.0, 4.0, 4.0]);
        for uv in [
            FX_IRIS, FX_LID, FX_BROW, FX_MOUTH, MARK_BLUSH, MARK_SWEAT, MARK_TEAR, MARK_VEIN,
        ] {
            assert!(uv.is_valid(), "{:?} is not inside its sheet", uv);
        }

        // A quad's corner lands inside the sprite it shows.
        assert_eq!(FX_LID.at(0.0, 0.0), [0.5, 0.0]);
        assert_eq!(FX_LID.at(1.0, 1.0), [1.0, 0.5]);
    }

    #[test]
    fn the_library_names_every_expression_and_emote() {
        let library = FaceLibrary::built_in();
        for expression in [
            FaceExpression::Idle,
            FaceExpression::Blink,
            FaceExpression::Hurt,
            FaceExpression::Happy,
            FaceExpression::Surprised,
            FaceExpression::Angry,
            FaceExpression::Sleepy,
            FaceExpression::Swim,
            FaceExpression::Fly,
            FaceExpression::Emote,
        ] {
            let cue = library.cue(expression);
            assert!(cue.priority < 10);
        }
        // The named cues are all there, emotes included.
        for name in [
            "idle",
            "blink",
            "hurt",
            "happy",
            "surprised",
            "angry",
            "sleepy",
            "swim",
            "fly",
            "wave",
            "clap",
            "dance",
            "bow",
            "cry",
            "laugh",
        ] {
            assert!(library.get(name).is_some(), "no cue named {}", name);
        }
        assert_eq!(library.len(), 15);
        assert_eq!(library.names().count(), 15);
        assert!(library.get("no_such_face").is_none());

        // A replaced cue wins; an unknown expression still has a face.
        let mut library = library;
        library.insert(
            "happy",
            FaceCue {
                mouth: 0.9,
                ..FaceCue::built_in(FaceExpression::Happy)
            },
        );
        assert_eq!(library.get("happy").unwrap().mouth, 0.9);
        assert_eq!(library.cue(FaceExpression::Happy).mouth, 0.9);
        assert_eq!(
            FaceLibrary::default().cue(FaceExpression::Angry),
            FaceCue::built_in(FaceExpression::Angry)
        );
    }

    #[test]
    fn cues_only_rest_when_they_never_time_out() {
        assert!(FaceCue::IDLE.is_resting());
        assert!(FaceExpression::Idle.cue().is_resting());
        assert!(FaceExpression::Swim.cue().is_resting());
        assert!(FaceExpression::Fly.cue().is_resting());
        assert!(!FaceExpression::Hurt.cue().is_resting());
        assert!(!FaceExpression::Happy.cue().is_resting());
        // The hurt face squeezes, frowns and marks the cheek.
        let hurt = FaceExpression::Hurt.cue();
        assert!(hurt.lid > 0.0 && hurt.mouth > 0.0);
        assert!(hurt.mark.is_some());
        // The surprised face widens the eyes and opens the mouth widest.
        let surprised = FaceExpression::Surprised.cue();
        assert_eq!(surprised.widen, 1.0);
        assert!(surprised.mouth > FaceExpression::Happy.cue().mouth);
    }

    #[test]
    fn face_settings_read_keep_and_store() {
        let defaults = FaceSettings::default();
        assert_eq!(FaceSettings::read(|_| None), defaults);

        let read = FaceSettings::read(stored(&[
            ("enabled", "false"),
            ("blink_interval", "7"),
            ("look_tracking", "false"),
            ("reaction_intensity", "0.5"),
            ("personality", "grumpy"),
        ]));
        assert!(!read.enabled);
        assert_eq!(read.blink_interval, 7.0);
        assert!(!read.look_tracking);
        assert_eq!(read.reaction_intensity, 0.5);
        assert_eq!(read.personality, Personality::Grumpy);

        // Out-of-range and broken values clamp or fall back, and the labels
        // cover every button.
        let clamped = FaceSettings::read(stored(&[
            ("blink_interval", "0"),
            ("reaction_intensity", "9"),
            ("personality", "sulky"),
        ]));
        assert_eq!(clamped.blink_interval, 0.5);
        assert_eq!(clamped.reaction_intensity, 2.0);
        assert_eq!(clamped.personality, Personality::Neutral);
        assert_eq!(clamped.reaction_alpha(), 1.0);
        assert_eq!(clamped.labels().len(), BUTTONS);
        assert_eq!(clamped.stored().len(), BUTTONS);

        // The settings round-trip through what they store.
        let mut settings = defaults;
        settings.click(1);
        settings.click(3);
        settings.click(4);
        let pairs: Vec<(&str, String)> = settings.stored().to_vec();
        let values: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        assert_eq!(FaceSettings::read(stored(&values)), settings);
    }

    #[test]
    fn clicks_cycle_every_button() {
        let mut settings = FaceSettings::default();
        let start = settings;
        for button in 0..BUTTONS {
            let before = settings;
            settings.click(button);
            assert_ne!(settings, before, "button {} changed nothing", button);
        }
        let before = settings;
        settings.click(BUTTONS);
        assert_eq!(settings, before);

        let mut cycled = start;
        for _ in 0..BLINK_STEPS.len() {
            cycled.click(1);
        }
        assert_eq!(cycled.blink_interval, start.blink_interval);
        assert!(BLINK_STEPS.contains(&cycled.blink_interval));

        // The personalities come round again.
        let mut personalities = start;
        for _ in 0..Personality::ALL.len() {
            personalities.click(4);
        }
        assert_eq!(personalities.personality, start.personality);
    }

    #[test]
    fn personalities_scale_the_mood() {
        assert_eq!(Personality::Neutral.next(), Personality::Cheerful);
        assert_eq!(Personality::Shy.next(), Personality::Neutral);
        assert!(Personality::Cheerful.cheer() > Personality::Neutral.cheer());
        assert!(Personality::Grumpy.grumble() > Personality::Neutral.grumble());
        assert!(Personality::Shy.blush() > Personality::Neutral.blush());
        for personality in Personality::ALL {
            assert!(!personality.label().is_empty());
        }

        // A grumpy face blinks sooner than a cheerful one.
        let grumpy = FaceSettings {
            personality: Personality::Grumpy,
            ..FaceSettings::default()
        };
        let cheerful = FaceSettings {
            personality: Personality::Cheerful,
            ..FaceSettings::default()
        };
        assert!(cheerful.blink_seconds() > grumpy.blink_seconds());
    }

    #[test]
    fn the_face_blinks_and_opens_again() {
        let settings = FaceSettings::default();
        let mut rig = FaceRig::new(7);
        let signals = MotionSignals::new();

        // Start a blink now: the lids close and open inside one blink.
        rig.set_blink_in(0.0);
        rig.update(&signals, &settings, 0.05);
        assert!(rig.blink() > 0.5, "blink was {}", rig.blink());
        for _ in 0..4 {
            rig.update(&signals, &settings, 0.05);
        }
        assert_eq!(rig.blink(), 0.0);
        // And a fresh interval, never shorter than the setting allows.
        assert!(rig.blink_timer() >= 0.5);

        // Turned off, the face stays open and nothing moves.
        let off = FaceSettings {
            enabled: false,
            ..FaceSettings::default()
        };
        rig.set_blink_in(0.0);
        rig.update(&walking(), &off, 0.1);
        assert_eq!(rig.blink(), 0.0);
        assert_eq!(rig.look(), [0.0, 0.0]);
        assert_eq!(rig.reaction(), 0.0);
        assert_eq!(rig.expression(), FaceExpression::Idle);
    }

    #[test]
    fn the_eyes_follow_the_body_and_let_go() {
        let settings = FaceSettings::default();
        let mut rig = FaceRig::new(3);
        rig.set_blink_in(100.0);

        let strafing = MotionSignals {
            moving: true,
            walk_left: 1.0,
            ..MotionSignals::new()
        };
        for _ in 0..5 {
            rig.update(&strafing, &settings, 0.1);
        }
        let look = rig.look();
        assert!(look[0] > 0.5, "the eyes did not follow: {:?}", look);
        assert!(look[1].abs() < 0.1);

        // Looking down while mining.
        let mining = MotionSignals {
            mining: true,
            ..MotionSignals::new()
        };
        for _ in 0..10 {
            rig.update(&mining, &settings, 0.1);
        }
        assert!(rig.look()[1] < -0.5);

        // Tracking off, the eyes centre again.
        let off = FaceSettings {
            look_tracking: false,
            ..FaceSettings::default()
        };
        for _ in 0..20 {
            rig.update(&mining, &off, 0.1);
        }
        assert_eq!(rig.look(), [0.0, 0.0]);
    }

    #[test]
    fn a_reaction_shows_and_fades() {
        let settings = FaceSettings::default();
        let mut rig = FaceRig::new(11);
        rig.set_blink_in(100.0);

        let falling = MotionSignals {
            fall_weight: crate::signals::HEAVY,
            ..MotionSignals::new()
        };
        for _ in 0..10 {
            rig.update(&falling, &settings, 0.05);
        }
        assert!((rig.reaction() - 1.0).abs() < 1e-6);
        // The face shows the fall.
        assert_eq!(rig.expression(), FaceExpression::Hurt);

        // A half-strength setting halves it.
        let half = FaceSettings {
            reaction_intensity: 0.5,
            ..FaceSettings::default()
        };
        for _ in 0..10 {
            rig.update(&falling, &half, 0.05);
        }
        assert!((rig.reaction() - 0.5).abs() < 1e-6);

        // Calm again, it fades away.
        for _ in 0..40 {
            rig.update(&MotionSignals::new(), &half, 0.05);
        }
        assert!(rig.reaction() < 1e-6);
    }

    #[test]
    fn the_signals_pick_the_strongest_face() {
        let settings = FaceSettings::default();
        let mut rig = FaceRig::new(5);
        rig.set_blink_in(100.0);

        let cases = [
            (
                MotionSignals {
                    fall_weight: crate::signals::HEAVY,
                    running: true,
                    moving: true,
                    ..MotionSignals::new()
                },
                FaceExpression::Hurt,
            ),
            (
                MotionSignals {
                    mining: true,
                    ..MotionSignals::new()
                },
                FaceExpression::Angry,
            ),
            (
                MotionSignals {
                    fast_flight: true,
                    ..MotionSignals::new()
                },
                FaceExpression::Surprised,
            ),
            (
                MotionSignals {
                    running: true,
                    moving: true,
                    ..MotionSignals::new()
                },
                FaceExpression::Happy,
            ),
            (
                MotionSignals {
                    elytra: true,
                    ..MotionSignals::new()
                },
                FaceExpression::Fly,
            ),
            (
                MotionSignals {
                    submerged_swimming: true,
                    ..MotionSignals::new()
                },
                FaceExpression::Swim,
            ),
            (
                MotionSignals {
                    crouching: true,
                    ..MotionSignals::new()
                },
                FaceExpression::Sleepy,
            ),
            (MotionSignals::new(), FaceExpression::Idle),
        ];
        for (signals, expected) in cases {
            rig.update(&signals, &settings, 0.05);
            assert_eq!(rig.expression(), expected, "signals {:?}", signals);
        }
    }

    #[test]
    fn an_explicit_face_plays_and_lets_go() {
        let settings = FaceSettings::default();
        let mut rig = FaceRig::new(2);
        rig.set_blink_in(100.0);

        rig.play(FaceExpression::Happy);
        assert!(rig.is_playing());
        assert_eq!(rig.expression(), FaceExpression::Happy);
        // Even a fall leaves it alone while it lasts.
        let falling = MotionSignals {
            fall_weight: crate::signals::HEAVY,
            ..MotionSignals::new()
        };
        rig.update(&falling, &settings, 0.1);
        assert_eq!(rig.expression(), FaceExpression::Happy);

        // Happy lasts two seconds; then the signals pick again.
        for _ in 0..25 {
            rig.update(&falling, &settings, 0.1);
        }
        assert!(!rig.is_playing());
        assert_eq!(rig.expression(), FaceExpression::Hurt);

        // A resting cue never holds.
        rig.play_cue(FaceCue::IDLE);
        assert!(!rig.is_playing());
        assert_eq!(rig.expression(), FaceExpression::Idle);

        rig.play(FaceExpression::Angry);
        rig.reset();
        assert!(!rig.is_playing());
        assert_eq!(rig.expression(), FaceExpression::Idle);
    }

    #[test]
    fn a_resting_face_draws_its_eyes_brows_and_mouth() {
        let settings = FaceSettings::default();
        let mut rig = FaceRig::new(9);
        rig.set_blink_in(100.0);
        rig.update(&MotionSignals::new(), &settings, 0.05);

        let quads = rig.quads(&settings);
        assert_eq!(quads.len(), 5, "two eyes, two brows, one mouth");
        for quad in quads.iter() {
            assert!(quad.is_visible());
            assert!(quad.uv.is_valid());
            assert!((0.0..=1.0).contains(&quad.color[3]));
            assert!((quad.center[2] - FACE_Z).abs() < 0.01);
        }
        // The eyes sit either side of the middle, at the same height.
        let eyes: Vec<&FaceQuad> = quads.iter().filter(|q| q.uv == FX_IRIS).collect();
        assert_eq!(eyes.len(), 2);
        assert!((eyes[0].center[0] + eyes[1].center[0]).abs() < 1e-6);
        assert_eq!(eyes[0].center[1], eyes[1].center[1]);
        // The mouth is below them.
        let mouth = quads.iter().find(|q| q.uv == FX_MOUTH).unwrap();
        assert!(mouth.center[1] < eyes[0].center[1]);

        // A blink adds one lid per eye, hung on their top halves.
        rig.set_blink_in(0.0);
        rig.update(&MotionSignals::new(), &settings, 0.04);
        let blinking = rig.quads(&settings);
        assert!(blinking.len() > quads.len());
        let lids: Vec<&FaceQuad> = blinking.iter().filter(|q| q.uv == FX_LID).collect();
        assert_eq!(lids.len(), 2);
        for lid in lids {
            assert!(lid.center[1] > eyes[0].center[1]);
        }

        // A reaction adds its mark, and no face draws with the rig off.
        rig.set_blink_in(100.0);
        rig.play(FaceExpression::Hurt);
        rig.update(&MotionSignals::new(), &settings, 0.05);
        let hurt = rig.quads(&settings);
        assert!(hurt.iter().any(|q| q.texture == MARKS_TEXTURE));
        let off = FaceSettings {
            enabled: false,
            ..FaceSettings::default()
        };
        assert!(rig.quads(&off).is_empty());
    }

    #[test]
    fn a_mouth_opens_and_a_quad_has_corners() {
        let settings = FaceSettings::default();
        let mut rig = FaceRig::new(13);
        rig.set_blink_in(100.0);
        rig.update(&MotionSignals::new(), &settings, 0.05);
        let closed = rig
            .quads(&settings)
            .iter()
            .find(|q| q.uv == FX_MOUTH)
            .copied()
            .unwrap();

        rig.play(FaceExpression::Surprised);
        rig.update(&MotionSignals::new(), &settings, 0.05);
        let quads = rig.quads(&settings);
        let open = quads.iter().find(|q| q.uv == FX_MOUTH).copied().unwrap();
        assert!(open.size[1] > closed.size[1]);
        // Surprised also widens the eyes and draws its glow.
        assert!(
            quads
                .iter()
                .any(|q| q.uv == FX_IRIS && q.size[0] > EYE_SIZE[0])
        );

        // A quad's corners are its rectangle, turned by its roll.
        let quad = FaceQuad {
            texture: FX_TEXTURE,
            center: [0.0, 0.0, 1.0],
            size: [0.2, 0.1],
            roll: std::f32::consts::FRAC_PI_2,
            uv: FX_IRIS,
            color: [1.0; 4],
        };
        let corners = quad.corners();
        // A quarter turn swaps the rectangle's sides.
        let width = corners[1][0] - corners[0][0];
        let height = corners[1][1] - corners[0][1];
        assert!((width * width + height * height - 0.2 * 0.2).abs() < 1e-6);
        // Every corner keeps the quad's depth.
        for corner in corners {
            assert_eq!(corner[2], 1.0);
        }
        // The UV corners start at the sprite's bottom left.
        assert_eq!(quad.uv_corners()[0], [FX_IRIS.u0, FX_IRIS.v1]);
        assert_eq!(quad.uv_corners()[2], [FX_IRIS.u1, FX_IRIS.v0]);
        // A see-through or sizeless quad draws nothing.
        assert!(
            !FaceQuad {
                color: [1.0, 1.0, 1.0, 0.0],
                ..quad
            }
            .is_visible()
        );
        assert!(
            !FaceQuad {
                size: [0.0, 0.1],
                ..quad
            }
            .is_visible()
        );
    }

    #[test]
    fn the_quad_list_holds_ten_and_no_more() {
        let mut quads = FaceQuads::new();
        assert!(quads.is_empty());
        for i in 0..MAX_FACE_QUADS {
            assert!(quads.push(FaceQuad {
                center: [i as f32, 0.0, 0.0],
                ..FaceQuad::EMPTY
            }));
        }
        assert_eq!(quads.len(), MAX_FACE_QUADS);
        assert!(!quads.push(FaceQuad::EMPTY));
        assert_eq!(quads.as_slice().len(), MAX_FACE_QUADS);
        assert_eq!(quads.iter().count(), MAX_FACE_QUADS);
    }

    #[test]
    fn the_rig_map_tracks_faces_per_player() {
        let settings = FaceSettings::default();
        let mut rigs = FaceRigs::new();
        assert!(rigs.is_empty());
        assert!(rigs.rig(4).is_none());

        // A face is made on first use, seeded by the entity so two players
        // never blink together.
        rigs.update(4, &MotionSignals::new(), &settings, 0.05);
        rigs.update(9, &MotionSignals::new(), &settings, 0.05);
        assert_eq!(rigs.len(), 2);
        assert!(rigs.rig(4).is_some());
        assert!(rigs.quads(4, &settings).is_some());
        assert!(rigs.quads(7, &settings).is_none());

        // The rigs keep their own timers.
        let first = rigs.rig(4).unwrap().blink_timer();
        let second = rigs.rig(9).unwrap().blink_timer();
        assert_ne!(first, second);

        assert!(rigs.remove(4));
        assert!(!rigs.remove(4));
        assert_eq!(rigs.len(), 1);
        rigs.clear();
        assert!(rigs.is_empty());
    }

    #[test]
    fn the_settings_are_read_once_and_can_be_reset() {
        reset_settings();
        assert_eq!(current_settings(), FaceSettings::default());

        let read = settings(stored(&[("blink_interval", "7")]));
        assert_eq!(read.blink_interval, 7.0);
        // A second read through another source keeps the first: read once.
        assert_eq!(settings(|_| None), read);

        set_settings(FaceSettings {
            enabled: false,
            ..FaceSettings::default()
        });
        assert!(!current_settings().enabled);
        reset_settings();
        assert_eq!(current_settings(), FaceSettings::default());
    }
}
