//! The cat king of the Gold Edition's Info tab. He is built from polygons (the same cat as the big
//! cat backdrop, crowned and cloaked) so he can move: he breathes and blinks, and when he is
//! clicked, or when the Gold Edition is first unlocked, he throws his head back and laughs, says
//! "Because of you, I'm King" and the window glitches (see `gold.rs`).
//!
//! Everything here is pure: `scene` turns a pose into polygons in picture coordinates (the
//! colosseum picture is `PICTURE` big), and `ui/king.rs` draws them.

use std::f32::consts::TAU;

/// Size of the colosseum picture the king stands in, in its own pixels.
pub const PICTURE: (f32, f32) = (1400.0, 900.0);
/// How long a laugh lasts, in seconds.
pub const REACTION: f32 = 7.0;
/// The moment in the laugh when the window glitches.
pub const GLITCH_AT: f32 = 2.0;
/// What he says, and where (picture coordinates, centre of the text).
pub const SPEECH: &str = "Because of you, I'm King";
pub const SPEECH_AT: (f32, f32) = (700.0, 96.0);

// where the cat head template (the big backdrop's cat) sits in the picture, and how big
const HEAD_CENTRE: (f32, f32) = (600.0, 830.0);
const KING_AT: (f32, f32) = (700.0, 462.0);
const SCALE: f32 = 0.54;
/// Where the head turns (the neck), in head coordinates.
const NECK: (f32, f32) = (600.0, 1250.0);
/// How far the jaw drops when it is wide open, in head coordinates.
const JAW_DROP: f32 = 150.0;

/// How a polygon is shaded; the painter turns a tone into a gold colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tone {
    /// the inside of the mouth
    Mouth,
    Cloth,
    Fur,
    Mane,
    Crown,
    Gem,
    Tongue,
    /// the eyes: bright
    Eye,
    /// nothing: only the outline
    None,
}

impl Tone {
    /// 0..1 on the gold ramp.
    pub fn value(self) -> f32 {
        match self {
            Tone::Mouth => 0.04,
            Tone::Cloth => 0.30,
            Tone::Fur => 0.50,
            Tone::Mane => 0.42,
            Tone::Crown => 0.80,
            Tone::Gem => 0.97,
            Tone::Tongue => 0.34,
            Tone::Eye => 0.96,
            Tone::None => 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Poly {
    pub points: Vec<(f32, f32)>,
    pub fill: Tone,
    /// Outline brightness 0..1 on the gold ramp and its width in picture pixels; 0 width = none.
    pub line: (f32, f32),
    pub closed: bool,
    /// A soft glow round it (eyes, gems).
    pub glow: f32,
}

/// What the king is doing at one moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// picture pixels, up is negative
    pub bob: f32,
    /// degrees, clockwise
    pub tilt: f32,
    /// 0 closed .. 1 wide open
    pub jaw: f32,
    /// 0 eyes open .. 1 shut tight (laughing, blinking)
    pub squint: f32,
    /// eyebrows raised 0..1 (below 0: lowered into a frown)
    pub brow: f32,
    /// the shoulders shake (picture pixels up and down)
    pub shoulders: f32,
    /// the crown slips sideways (picture pixels) and tilts (degrees)
    pub crown_dx: f32,
    pub crown_tilt: f32,
    /// how bright the eyes glow 0..1
    pub glow: f32,
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// How hard the laugh is at `since` seconds into it: 0 before and after, up to 1.
fn laugh_envelope(since: f32) -> f32 {
    smooth((since - 0.3) / 0.4) * (1.0 - smooth((since - 4.3) / 0.6))
}

/// Laughs per second.
const HA_RATE: f32 = 4.5;

/// The pose `idle` seconds after the program started, `since` seconds into a laugh (None when he
/// is not laughing). With `moving` false he holds still.
pub fn pose(idle: f32, since: Option<f32>, moving: bool) -> Pose {
    let t = if moving { idle } else { 0.0 };
    let mut p = Pose {
        bob: if moving { 1.6 * (t * 1.3).sin() } else { 0.0 },
        tilt: if moving { 0.4 * (t * 0.7).sin() } else { 0.0 },
        jaw: 0.0,
        squint: if moving { blink(t) } else { 0.0 },
        brow: 0.0,
        shoulders: 0.0,
        crown_dx: 0.0,
        crown_tilt: 0.0,
        glow: if moving { 0.8 + 0.2 * (t * 1.7).sin() } else { 0.9 },
    };
    let Some(s) = since else { return p };
    if !(0.0..REACTION).contains(&s) {
        return p;
    }
    // the gasp of triumph: head thrown back, eyebrows up, mouth opening
    let gasp = smooth(s / 0.15) * (1.0 - smooth((s - 0.3) / 0.15));
    p.tilt += -3.0 * gasp;
    p.brow = gasp;
    p.jaw = p.jaw.max(0.25 * gasp);
    // the laugh
    let env = laugh_envelope(s);
    let ha = 0.5 + 0.5 * (TAU * HA_RATE * s).sin();
    p.jaw = p.jaw.max(env * (0.5 + 0.5 * ha));
    p.bob += -env * 9.0 * ha;
    p.tilt += env * 3.2 * (TAU * HA_RATE * 0.5 * s).sin();
    p.squint = p.squint.max(env * 0.95);
    p.shoulders = env * 3.5 * (TAU * HA_RATE * 2.0 * s).sin();
    p.crown_dx = env * 6.0 * (TAU * HA_RATE * s + 1.0).sin();
    p.crown_tilt = env * 4.0 * (TAU * HA_RATE * 0.5 * s + 0.6).sin();
    p.glow = 1.0;
    // content afterwards: a small smile and a slow blink
    let after = smooth((s - 4.6) / 0.5);
    p.jaw = p.jaw.max(0.10 * after);
    p.squint = p.squint.max(0.6 * after * (1.0 - smooth((s - 5.6) / 0.6)));
    p
}

/// A blink every few seconds: 1 for an instant, else 0.
fn blink(t: f32) -> f32 {
    let phase = t.rem_euclid(5.3);
    if phase < 0.16 {
        (phase / 0.16 * std::f32::consts::PI).sin()
    } else {
        0.0
    }
}

/// "HA!" that pop up round his head while he laughs: (text, position, strength 0..1, size).
pub fn has(since: f32) -> Vec<(&'static str, (f32, f32), f32, f32)> {
    const SPOTS: [(f32, f32); 4] = [(440.0, 330.0), (960.0, 300.0), (500.0, 230.0), (905.0, 215.0)];
    let mut out = Vec::new();
    for k in 0..7 {
        let start = 0.5 + 0.55 * k as f32;
        let age = since - start;
        if !(0.0..0.95).contains(&age) {
            continue;
        }
        let (x, y) = SPOTS[k % SPOTS.len()];
        let rise = age * 55.0;
        let strength = smooth(age / 0.12) * (1.0 - smooth((age - 0.55) / 0.4));
        let text = if k % 2 == 0 { "HA!" } else { "HA HA!" };
        let size = 34.0 + 10.0 * ((k * 7) % 3) as f32;
        out.push((text, (x, y - rise), strength, size));
    }
    out
}

/// What he has said so far and how clearly: it is typed out, holds, and fades.
pub fn speech(since: f32) -> Option<(String, f32)> {
    const START: f32 = 1.1;
    const TYPING: f32 = 1.0;
    if since < START || since >= REACTION - 0.2 {
        return None;
    }
    let total = SPEECH.chars().count();
    let shown = (((since - START) / TYPING).clamp(0.0, 1.0) * total as f32).ceil() as usize;
    let fade = 1.0 - smooth((since - (REACTION - 1.0)) / 0.8);
    Some((SPEECH.chars().take(shown).collect(), fade))
}

// ---- the cat, in head coordinates (x mirrored about 600) ----

const UPPER: [(f32, f32); 6] =
    [(600.0, 498.0), (722.0, 494.0), (886.0, 566.0), (952.0, 664.0), (1006.0, 796.0), (978.0, 852.0)];
/// The jaw's outline, from the hinge round the cheek to the chin, with how much each point moves
/// when the mouth opens.
const JAW: [((f32, f32), f32); 8] = [
    ((978.0, 852.0), 0.0),
    ((1034.0, 908.0), 0.3),
    ((954.0, 930.0), 0.5),
    ((990.0, 990.0), 0.7),
    ((900.0, 1014.0), 0.85),
    ((822.0, 1112.0), 1.0),
    ((704.0, 1196.0), 1.0),
    ((600.0, 1228.0), 1.0),
];
/// The upper lip, from the hinge to the middle; the jaw's top edge meets it when the mouth is shut.
const LIP: [(f32, f32); 4] = [(978.0, 852.0), (860.0, 985.0), (718.0, 1048.0), (600.0, 1020.0)];
const EAR: [(f32, f32); 5] = [(724.0, 512.0), (800.0, 408.0), (868.0, 372.0), (906.0, 452.0), (916.0, 556.0)];
const INNER_EAR: [(f32, f32); 5] = [(772.0, 504.0), (820.0, 440.0), (862.0, 418.0), (884.0, 470.0), (884.0, 540.0)];
const BROW: [(f32, f32); 4] = [(604.0, 694.0), (704.0, 726.0), (818.0, 700.0), (906.0, 642.0)];
const EYE: [(f32, f32); 5] = [(642.0, 792.0), (714.0, 754.0), (814.0, 752.0), (760.0, 792.0), (702.0, 808.0)];
const EYE_SHUT: [(f32, f32); 4] = [(646.0, 804.0), (712.0, 764.0), (774.0, 754.0), (818.0, 776.0)];
const NOSE: [(f32, f32); 3] = [(558.0, 892.0), (642.0, 892.0), (600.0, 944.0)];
const FANG: [(f32, f32); 3] = [(646.0, 1020.0), (664.0, 1126.0), (684.0, 1016.0)];
const STRIPES: [[(f32, f32); 2]; 3] =
    [[(602.0, 528.0), (602.0, 628.0)], [(652.0, 534.0), (636.0, 618.0)], [(704.0, 546.0), (672.0, 632.0)]];
const FACETS: [[(f32, f32); 2]; 3] = [
    [(952.0, 664.0), (814.0, 752.0)],
    [(1006.0, 796.0), (820.0, 800.0)],
    [(954.0, 930.0), (770.0, 968.0)],
];
const MANE: [(f32, f32); 7] = [
    (1006.0, 796.0),
    (1062.0, 760.0),
    (1040.0, 840.0),
    (1098.0, 850.0),
    (1040.0, 920.0),
    (1086.0, 976.0),
    (1000.0, 990.0),
];
const CROWN: [(f32, f32); 9] = [
    (462.0, 536.0),
    (452.0, 420.0),
    (524.0, 476.0),
    (560.0, 372.0),
    (600.0, 462.0),
    (640.0, 372.0),
    (676.0, 476.0),
    (748.0, 420.0),
    (738.0, 536.0),
];
const BAND: [(f32, f32); 4] = [(462.0, 536.0), (738.0, 536.0), (730.0, 580.0), (470.0, 580.0)];
/// Jewels in the crown: (x, y, radius) in head coordinates.
const GEMS: [(f32, f32, f32); 7] = [
    (600.0, 558.0, 15.0),
    (528.0, 558.0, 11.0),
    (672.0, 558.0, 11.0),
    (560.0, 372.0, 9.0),
    (640.0, 372.0, 9.0),
    (452.0, 420.0, 8.0),
    (748.0, 420.0, 8.0),
];
const MANTLE: [(f32, f32); 9] = [
    (330.0, 1150.0),
    (460.0, 1196.0),
    (600.0, 1232.0),
    (740.0, 1196.0),
    (870.0, 1150.0),
    (1010.0, 1260.0),
    (1110.0, 1500.0),
    (90.0, 1500.0),
    (190.0, 1260.0),
];
const COLLAR: [(f32, f32); 10] = [
    (330.0, 1150.0),
    (460.0, 1212.0),
    (600.0, 1252.0),
    (740.0, 1212.0),
    (870.0, 1150.0),
    (900.0, 1196.0),
    (740.0, 1270.0),
    (600.0, 1312.0),
    (460.0, 1270.0),
    (300.0, 1196.0),
];
const PENDANT: [(f32, f32); 4] = [(600.0, 1312.0), (640.0, 1370.0), (600.0, 1430.0), (560.0, 1370.0)];

fn mirror(p: (f32, f32)) -> (f32, f32) {
    (1200.0 - p.0, p.1)
}

/// A point turned about the neck by `deg` degrees, then placed in the picture.
fn place(at: (f32, f32), scale: f32, p: (f32, f32), deg: f32, bob: f32) -> (f32, f32) {
    let (s, c) = deg.to_radians().sin_cos();
    let (dx, dy) = (p.0 - NECK.0, p.1 - NECK.1);
    let (x, y) = (NECK.0 + dx * c - dy * s, NECK.1 + dx * s + dy * c);
    (at.0 + (x - HEAD_CENTRE.0) * scale, at.1 + (y - HEAD_CENTRE.1) * scale + bob)
}

/// Where a point lands that is given in head units, measured from the middle of the head (the way
/// `cat_scene` lays the head out at scale 1 with `at` = 0,0), when the head is drawn in the pose `p`
/// at `at` and `scale`: turned about the neck with the head, and bobbing with it.
pub fn head_point(p: &Pose, at: (f32, f32), scale: f32, q: (f32, f32)) -> (f32, f32) {
    place(at, scale, (q.0 + HEAD_CENTRE.0, q.1 + HEAD_CENTRE.1), p.tilt, p.bob)
}

/// Every polygon of the king in the pose `p`, back to front, in picture coordinates.
pub fn scene(p: &Pose) -> Vec<Poly> {
    build(p, KING_AT, SCALE, true, true)
}

/// The same cat, plain (no crown, no cloak), anywhere at any size: `at` is where the middle of
/// the head goes and `scale` how many picture pixels one head-coordinate pixel is (the king is
/// 0.54). Used for the cat you play in the games.
pub fn cat_scene(p: &Pose, at: (f32, f32), scale: f32) -> Vec<Poly> {
    build(p, at, scale, false, false)
}

fn build(p: &Pose, at_: (f32, f32), scale: f32, with_crown: bool, with_cloak: bool) -> Vec<Poly> {
    let mut out: Vec<Poly> = Vec::new();
    let tilt = p.tilt;
    let head = |q: (f32, f32)| place(at_, scale, q, tilt, p.bob);
    // the body does not turn with the head; it only shakes
    let body = |q: (f32, f32)| place(at_, scale, q, 0.0, p.shoulders * -1.0 + p.bob * 0.35);
    let push = |out: &mut Vec<Poly>, points: Vec<(f32, f32)>, fill: Tone, line: (f32, f32), closed: bool, glow: f32| {
        out.push(Poly { points, fill, line, closed, glow });
    };

    // mantle, collar, pendant
    if with_cloak {
    push(&mut out, MANTLE.iter().map(|&q| body(q)).collect(), Tone::Cloth, (0.75, 2.4), true, 0.0);
    for fx in [250.0f32, 400.0, 600.0, 800.0, 950.0] {
        let a = (fx, 1290.0 + 20.0 * (fx - 600.0).abs() / 350.0);
        let b = (fx - (600.0 - fx) * 0.12, 1500.0);
        push(&mut out, vec![body(a), body(b)], Tone::None, (0.5, 1.4), false, 0.0);
    }
    push(&mut out, COLLAR.iter().map(|&q| body(q)).collect(), Tone::Crown, (0.9, 2.0), true, 0.0);
    push(&mut out, PENDANT.iter().map(|&q| body(q)).collect(), Tone::Gem, (1.0, 2.0), true, 0.25);
    }

    // ears, behind the head
    for m in [false, true] {
        let f = |q: (f32, f32)| head(if m { mirror(q) } else { q });
        push(&mut out, EAR.iter().map(|&q| f(q)).collect(), Tone::Fur, (0.85, 2.2), true, 0.0);
        push(&mut out, INNER_EAR.iter().map(|&q| f(q)).collect(), Tone::Mane, (0.6, 1.5), true, 0.0);
        push(&mut out, MANE.iter().map(|&q| f(q)).collect(), Tone::Mane, (0.8, 2.0), true, 0.0);
    }

    // the mouth: the dark space between the upper lip and the dropped jaw
    let drop = p.jaw * JAW_DROP;
    let jaw_pt = |q: (f32, f32), w: f32| (q.0, q.1 + drop * w);
    // the jaw's top edge, right side, from the hinge to the middle, and how much it moves
    let top_right: [((f32, f32), f32); 2] = [((860.0, 985.0), 0.8), ((718.0, 1048.0), 1.0)];
    let top_centre = ((600.0, 1020.0), 1.0f32);
    if drop > 1.0 {
        let mut cavity: Vec<(f32, f32)> = Vec::new();
        cavity.push(head(LIP[0]));
        for &(q, _) in top_right.iter() {
            cavity.push(head(q));
        }
        cavity.push(head(top_centre.0));
        for &(q, _) in top_right.iter().rev() {
            cavity.push(head(mirror(q)));
        }
        cavity.push(head(mirror(LIP[0])));
        // back along the jaw's top edge, which has dropped
        for &(q, w) in top_right.iter() {
            cavity.push(head(jaw_pt(mirror(q), w)));
        }
        cavity.push(head(jaw_pt(top_centre.0, top_centre.1)));
        for &(q, w) in top_right.iter().rev() {
            cavity.push(head(jaw_pt(q, w)));
        }
        push(&mut out, cavity, Tone::Mouth, (0.0, 0.0), true, 0.0);
    }
    // the jaw: one piece, chin to hinge on the right, along the top edge, and down the left
    {
        let right_outer: Vec<((f32, f32), f32)> = JAW.iter().rev().cloned().collect();
        let mut jaw: Vec<(f32, f32)> = Vec::new();
        for &(q, w) in right_outer.iter() {
            jaw.push(head(jaw_pt(q, w)));
        }
        for &(q, w) in top_right.iter() {
            jaw.push(head(jaw_pt(q, w)));
        }
        jaw.push(head(jaw_pt(top_centre.0, top_centre.1)));
        for &(q, w) in top_right.iter().rev() {
            jaw.push(head(jaw_pt(mirror(q), w)));
        }
        let left_outer: Vec<((f32, f32), f32)> = right_outer.iter().rev().cloned().collect();
        for &(q, w) in left_outer.iter().take(left_outer.len() - 1) {
            jaw.push(head(jaw_pt(mirror(q), w)));
        }
        push(&mut out, jaw, Tone::Fur, (0.9, 2.4), true, 0.0);
    }
    // tongue, once it is open enough to see: it rises from the jaw's top edge
    if p.jaw > 0.35 {
        let up = (p.jaw - 0.35) / 0.65;
        let top = 1034.0 + drop;
        let tongue = vec![
            (535.0, top + 24.0),
            (568.0, top - 36.0 * up),
            (600.0, top - 52.0 * up),
            (632.0, top - 36.0 * up),
            (665.0, top + 24.0),
        ];
        push(&mut out, tongue.into_iter().map(head).collect(), Tone::Tongue, (0.55, 1.4), true, 0.0);
    }
    // lower teeth, when the mouth is open
    if drop > 6.0 {
        for m in [false, true] {
            let f = |q: (f32, f32)| head(if m { mirror(q) } else { q });
            let y = 1048.0 + drop;
            push(&mut out, vec![f((700.0, y)), f((716.0, y - 52.0)), f((734.0, y - 4.0))], Tone::Gem, (1.0, 1.2), true, 0.0);
        }
    }

    // the upper head, with the lip: one piece
    {
        let mut right: Vec<(f32, f32)> = UPPER.to_vec();
        right.extend(LIP.iter().skip(1));
        let mut upper: Vec<(f32, f32)> = right.iter().map(|&q| head(q)).collect();
        for &q in right.iter().rev().skip(1).take(right.len() - 2) {
            upper.push(head(mirror(q)));
        }
        push(&mut out, upper, Tone::Fur, (0.95, 2.4), true, 0.0);
    }
    for f_ in FACETS.iter() {
        for m in [false, true] {
            push(&mut out, f_.iter().map(|&q| head(if m { mirror(q) } else { q })).collect(), Tone::None, (0.45, 1.2), false, 0.0);
        }
    }
    for s_ in STRIPES.iter() {
        for m in [false, true] {
            push(&mut out, s_.iter().map(|&q| head(if m { mirror(q) } else { q })).collect(), Tone::None, (0.8, 2.4), false, 0.0);
        }
    }
    push(&mut out, NOSE.iter().map(|&q| head(q)).collect(), Tone::Gem, (0.95, 1.7), true, 0.0);
    for m in [false, true] {
        let f = |q: (f32, f32)| head(if m { mirror(q) } else { q });
        push(&mut out, FANG.iter().map(|&q| f(q)).collect(), Tone::Gem, (1.0, 1.5), true, 0.0);
    }
    // brows
    for m in [false, true] {
        let f = |q: (f32, f32)| head(if m { mirror(q) } else { q });
        // raised (above zero) the whole brow goes up; lowered (below zero) the end by the nose goes
        // down most, so the brows slant into a frown
        let lift = |q: (f32, f32)| {
            if p.brow >= 0.0 {
                p.brow * 22.0
            } else {
                let inner = 1.0 - ((q.0 - 600.0) / 310.0).clamp(0.0, 1.0);
                p.brow * 22.0 * (0.25 + 0.75 * inner)
            }
        };
        push(&mut out, BROW.iter().map(|&q| f((q.0, q.1 - lift(q)))).collect(), Tone::None, (0.9, 2.6), false, 0.0);
    }
    // eyes: open diamonds that flatten, then a happy arch when shut
    for m in [false, true] {
        let f = |q: (f32, f32)| head(if m { mirror(q) } else { q });
        let open = (1.0 - p.squint * 1.6).max(0.0);
        if open > 0.05 {
            let cy = 780.0;
            let pts = EYE.iter().map(|&(x, y)| f((x, cy + (y - cy) * open))).collect();
            push(&mut out, pts, Tone::Eye, (1.0, 1.8), true, p.glow);
        }
        if p.squint > 0.35 {
            push(&mut out, EYE_SHUT.iter().map(|&q| f(q)).collect(), Tone::None, (1.0, 3.6), false, p.glow * p.squint);
        }
    }

    // the crown, riding on the head, slipping when he laughs
    if !with_crown {
        return out;
    }
    let crown_pt = |q: (f32, f32)| {
        let r = place(at_, scale, q, tilt + p.crown_tilt, p.bob);
        (r.0 + p.crown_dx, r.1)
    };
    push(&mut out, CROWN.iter().map(|&q| crown_pt(q)).collect(), Tone::Crown, (1.0, 2.6), true, 0.1);
    push(&mut out, BAND.iter().map(|&q| crown_pt(q)).collect(), Tone::Crown, (1.0, 2.4), true, 0.0);
    for &(x, y, r) in GEMS.iter() {
        let c = crown_pt((x, y));
        let rr = r * scale * 1.7;
        let pts: Vec<(f32, f32)> = (0..8)
            .map(|i| {
                let a = TAU * i as f32 / 8.0;
                (c.0 + a.cos() * rr, c.1 + a.sin() * rr)
            })
            .collect();
        push(&mut out, pts, Tone::Gem, (1.0, 1.0), true, 0.6);
    }
    out
}

// ---- the dead king (The Void's backdrop) ---------------------------------------------------------
//
// The same king, his gold gone black and his eyes dead. When he is clicked he laughs, slowly, while
// his head turns all the way round on his neck, and everything glitches. He is only in The Void.

/// How long his laugh lasts, in seconds.
pub const DEAD_REACTION: f32 = 12.0;
/// When the head starts turning and when it is back on his neck.
const SPIN_FROM: f32 = 1.4;
const SPIN_TO: f32 = 8.0;
/// Laughs per second: slow.
const DEAD_HA_RATE: f32 = 1.3;

/// His colours, 0..1 like `gold::ramp` but blackened: near black, then a dull tarnish that only
/// shows at the very top of the ramp.
pub fn dead_ramp(v: f32) -> [u8; 3] {
    const STOPS: [(f32, [f32; 3]); 5] = [
        (0.0, [3.0, 2.0, 4.0]),
        (0.22, [16.0, 12.0, 14.0]),
        (0.5, [34.0, 27.0, 28.0]),
        (0.75, [66.0, 52.0, 40.0]),
        (1.0, [112.0, 92.0, 70.0]),
    ];
    let v = v.clamp(0.0, 1.0);
    for w in STOPS.windows(2) {
        let ((a, ca), (b, cb)) = (w[0], w[1]);
        if v <= b {
            let k = (v - a) / (b - a);
            return [0, 1, 2].map(|i| (ca[i] + (cb[i] - ca[i]) * k).round() as u8);
        }
    }
    [112, 92, 70]
}

/// How far his head has turned (degrees, 0 to 360), `since` seconds into the laugh: it starts
/// slowly, swings round, and settles back on his neck.
fn spin_degrees(since: f32) -> f32 {
    let x = smooth((since - SPIN_FROM) / (SPIN_TO - SPIN_FROM));
    // it shudders as it goes, and not at the ends
    let shudder = 5.0 * (TAU * 3.3 * since).sin() * (std::f32::consts::PI * x).sin();
    360.0 * x + shudder
}

/// The dead king's pose: still, dead-eyed, with a twitch now and then; and in a laugh (`since`
/// seconds in) his jaw hangs open and shuts in slow "ha"s while his head turns all the way round.
pub fn dead_pose(idle: f32, since: Option<f32>, moving: bool) -> Pose {
    let t = if moving { idle } else { 0.0 };
    // a twitch of the head, a moment long, every few seconds
    let twitch = {
        let phase = t.rem_euclid(3.7);
        if moving && phase < 0.09 { 2.6 * (phase / 0.09 * std::f32::consts::PI).sin() } else { 0.0 }
    };
    let mut p = Pose {
        bob: if moving { 0.6 * (t * 0.5).sin() } else { 0.0 },
        tilt: twitch,
        jaw: 0.0,
        squint: 0.0,
        brow: 0.0,
        shoulders: 0.0,
        crown_dx: 0.0,
        crown_tilt: 0.0,
        glow: 0.0,
    };
    let Some(s) = since else { return p };
    if !(0.0..DEAD_REACTION).contains(&s) {
        return p;
    }
    let env = smooth((s - 0.5) / 1.4) * (1.0 - smooth((s - 10.2) / 1.4));
    let ha = 0.5 + 0.5 * (TAU * DEAD_HA_RATE * s).sin();
    p.jaw = env * (0.45 + 0.55 * ha);
    p.bob += -env * 6.0 * ha;
    p.tilt += spin_degrees(s);
    p.shoulders = env * 2.5 * (TAU * DEAD_HA_RATE * 2.0 * s).sin();
    p.crown_dx = env * 7.0 * (TAU * DEAD_HA_RATE * s + 1.0).sin();
    p.crown_tilt = env * 5.0 * (TAU * DEAD_HA_RATE * 0.5 * s + 0.6).sin();
    p
}

/// How badly everything glitches (0 to 1) `since` seconds into his laugh: it builds from the first
/// moment, is at its worst while the head goes round, and fades once he is still.
pub fn dead_glitch(since: Option<f32>) -> f32 {
    let Some(s) = since else { return 0.0 };
    if !(0.0..DEAD_REACTION).contains(&s) {
        return 0.0;
    }
    (0.3 * smooth(s / 0.8) + 0.7 * smooth((s - 1.0) / 1.6)) * (1.0 - smooth((s - 9.2) / 2.6))
}

/// Slow, broken "ha"s that float up round his head while he laughs: (text, position, strength 0..1,
/// size), like `has` but fewer and further apart.
pub fn dead_has(since: f32) -> Vec<(&'static str, (f32, f32), f32, f32)> {
    const SPOTS: [(f32, f32); 4] = [(420.0, 360.0), (990.0, 330.0), (480.0, 240.0), (930.0, 230.0)];
    let mut out = Vec::new();
    for k in 0..6 {
        let age = since - (1.2 + 1.5 * k as f32);
        if !(0.0..1.8).contains(&age) {
            continue;
        }
        let (x, y) = SPOTS[k % SPOTS.len()];
        let strength = smooth(age / 0.3) * (1.0 - smooth((age - 1.1) / 0.7));
        out.push((if k % 2 == 0 { "ha..." } else { "HA..." }, (x, y - age * 28.0), strength, 36.0 + 8.0 * (k % 3) as f32));
    }
    out
}

// ---- what he is made of now: no life left in him, and yet it will not leave --------------------------
//
// His eyes are pitch black with a white X burnt into each, and round him there is a glow of life
// (a sick mint) that hugs his outline and swells every time a heart that is not there strikes, with
// the death (violet, darker than the sky) behind it. Rings go out from his chest on each beat, small
// lights lift off him and ash falls, and a pale copy of him rises and dissolves.

/// The colour of the life still in him, of the death round it, and of the dark behind that.
pub const LIFE: [u8; 3] = [120, 255, 205];
pub const DEATH: [u8; 3] = [155, 107, 255];
pub const DEATH_DEEP: [u8; 3] = [60, 20, 120];
/// Seconds between one beat and the next, and where in the picture the beat comes from.
pub const HEART_PERIOD: f32 = 2.4;
pub const HEART: (f32, f32) = (700.0, 470.0);

/// How hard the heart has just struck at time `t`, 0 to 1: two thumps close together, each one
/// arriving at once and fading away.
pub fn dead_beat(t: f32) -> f32 {
    let c = t.rem_euclid(HEART_PERIOD) / HEART_PERIOD;
    let thump = |at: f32, power: f32| {
        let a = c - at;
        if a < 0.0 {
            0.0
        } else {
            power * smooth(a / 0.012) * (-a * 6.0).exp()
        }
    };
    thump(0.0, 1.0).max(thump(0.27, 0.62)).clamp(0.0, 1.0)
}

/// The rings going out from his chest at time `t`: three of them, each (how far out it is, 0 to 1,
/// and how bright, 0 to 1). They appear, widen and fade, a third of a cycle apart.
pub fn dead_rings(t: f32) -> [(f32, f32); 3] {
    [0, 1, 2].map(|k| {
        let phase = (t / (HEART_PERIOD * 1.5) + k as f32 / 3.0).rem_euclid(1.0);
        (phase, smooth(phase / 0.08) * (1.0 - phase))
    })
}

/// A pale copy of him, rising off him and dissolving, then another: (how far up it is, in picture
/// pixels, and how bright, 0 to 1).
pub fn dead_soul(t: f32) -> (f32, f32) {
    let p = (t / (HEART_PERIOD * 2.0)).rem_euclid(1.0);
    (150.0 * p, 0.18 * smooth(p / 0.1) * (1.0 - p).powi(2))
}

/// One light in the air round him.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mote {
    pub pos: (f32, f32),
    /// true for the life lifting off him (mint, rising, with a thin trail), false for ash (violet,
    /// drifting down)
    pub rising: bool,
    pub size: f32,
    pub alpha: f32,
}

/// A number from 0 to 1 that is always the same for the same `i` and `salt`.
fn hash01(i: u32, salt: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B1) ^ salt.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFF_FFFF) as f32 / 0x100_0000 as f32
}

/// The lights round him at time `t`. Each one lives a few seconds: it fades in, drifts, and fades out
/// again, and there are always the same number of them.
pub fn dead_motes(t: f32) -> Vec<Mote> {
    (0..72u32)
        .map(|i| {
            let rising = hash01(i, 1) < 0.7;
            let rate = 0.08 + 0.1 * hash01(i, 2);
            let c = (t * rate + hash01(i, 3)).rem_euclid(1.0);
            let x = HEART.0 + (hash01(i, 4) - 0.5) * 600.0 + 10.0 * (TAU * (2.0 * c + hash01(i, 5))).sin();
            let y = if rising { 800.0 - c * 760.0 } else { 140.0 + c * 640.0 };
            let size = [2.0, 2.0, 3.0, 4.0][(hash01(i, 6) * 4.0) as usize % 4];
            let alpha = smooth(c / 0.12) * (1.0 - smooth((c - 0.7) / 0.3)) * (0.35 + 0.65 * hash01(i, 7));
            Mote { pos: (x, y), rising, size, alpha }
        })
        .collect()
}

/// The two strokes of the white X inside an eye whose outline is `eye`: each is a pair of ends, and
/// they cross in the middle of the eye and stay well inside it.
pub fn eye_cross(eye: &[(f32, f32)]) -> [((f32, f32), (f32, f32)); 2] {
    if eye.is_empty() {
        return [((0.0, 0.0), (0.0, 0.0)); 2];
    }
    let n = eye.len() as f32;
    let (cx, cy) = (eye.iter().map(|p| p.0).sum::<f32>() / n, eye.iter().map(|p| p.1).sum::<f32>() / n);
    let (x0, x1) = eye.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let (y0, y1) = eye.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
    let (hw, hh) = ((x1 - x0) * 0.26, (y1 - y0) * 0.4);
    let r = (hw * 1.25).min(hh * 1.15);
    [((cx - r, cy - r), (cx + r, cy + r)), ((cx - r, cy + r), (cx + r, cy - r))]
}

/// Where a point of the picture goes in a glitch picture (see `gold::glitch_frame_with`): the strips
/// that are torn sideways take the points that lie in them with them, and in a pixelated picture
/// every point snaps to the grid of blocks, which turns the king's outline into steps.
pub fn glitch_point(p: (f32, f32), frame: &crate::gold::GlitchFrame) -> (f32, f32) {
    let (mut x, mut y) = p;
    let fy = y / PICTURE.1;
    for band in &frame.bands {
        if fy >= band.top && fy <= band.bottom {
            x += band.shift * PICTURE.0 * 1.5;
        }
    }
    if let Some(block) = frame.block {
        let cell = block as f32 * 1.6;
        x = (x / cell).round() * cell;
        y = (y / cell).round() * cell;
    }
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inside(p: (f32, f32)) -> bool {
        p.0 >= 0.0 && p.0 <= PICTURE.0 && p.1 >= 0.0 && p.1 <= PICTURE.1
    }

    #[test]
    fn at_rest_he_is_closed_mouthed_and_open_eyed() {
        let p = pose(0.0, None, true);
        assert_eq!(p.jaw, 0.0);
        assert!(p.squint < 0.01);
        assert!(p.bob.abs() < 2.0);
    }

    #[test]
    fn holding_still_means_no_movement_at_all() {
        let a = pose(3.1, None, false);
        let b = pose(77.7, None, false);
        assert_eq!(a, b);
        assert_eq!(a.bob, 0.0);
    }

    #[test]
    fn he_blinks_now_and_then() {
        let shut = (0..600).filter(|k| pose(*k as f32 * 0.05, None, true).squint > 0.5).count();
        assert!(shut > 0 && shut < 60, "shut for {} samples", shut);
    }

    #[test]
    fn the_laugh_opens_the_jaw_wide_and_often() {
        let mut max_jaw = 0.0f32;
        let mut opens = 0;
        let mut last_open = false;
        for k in 0..400 {
            let s = 0.5 + k as f32 * 0.01;
            if s > 4.2 {
                break;
            }
            let p = pose(0.0, Some(s), true);
            max_jaw = max_jaw.max(p.jaw);
            let open = p.jaw > 0.8;
            if open && !last_open {
                opens += 1;
            }
            last_open = open;
            assert!(p.squint > 0.0);
        }
        assert!(max_jaw > 0.95, "jaw only opened to {}", max_jaw);
        assert!(opens >= 10, "only {} ha's", opens);
    }

    #[test]
    fn the_laugh_ends_calm() {
        let end = pose(0.0, Some(REACTION - 0.01), true);
        assert!(end.jaw <= 0.11);
        assert!(end.squint < 0.05);
        assert!(end.bob.abs() < 2.0);
        let after = pose(0.0, Some(REACTION + 1.0), true);
        assert_eq!(after, pose(0.0, None, true));
    }

    #[test]
    fn he_gasps_before_the_laugh() {
        let p = pose(0.0, Some(0.15), true);
        assert!(p.brow > 0.9);
        assert!(p.tilt < -2.0);
    }

    #[test]
    fn the_scene_stays_inside_the_picture_in_every_pose() {
        for s in [None, Some(0.0), Some(0.15), Some(0.5), Some(1.0), Some(2.0), Some(3.3), Some(4.5), Some(6.5)] {
            for k in 0..8 {
                let p = pose(k as f32 * 0.7, s, true);
                for poly in scene(&p) {
                    assert!(poly.points.len() >= 2);
                    for pt in &poly.points {
                        assert!(pt.0.is_finite() && pt.1.is_finite());
                        // the mantle runs off the bottom; everything else is on the picture
                        assert!(pt.0 > -2.0 && pt.0 < PICTURE.0 + 2.0, "x {} in {:?}", pt.0, s);
                        assert!(pt.1 > -2.0 && pt.1 < PICTURE.1 + 80.0, "y {} in {:?}", pt.1, s);
                    }
                }
            }
        }
        assert!(inside((700.0, 450.0)));
    }

    #[test]
    fn the_mouth_shows_only_when_the_jaw_is_down() {
        let shut = scene(&pose(0.0, None, false));
        let wide = scene(&Pose { jaw: 1.0, ..pose(0.0, None, false) });
        let dark = |s: &[Poly]| s.iter().filter(|p| p.fill == Tone::Mouth).count();
        assert_eq!(dark(&shut), 0);
        assert_eq!(dark(&wide), 1);
        let tongue = |s: &[Poly]| s.iter().filter(|p| p.fill == Tone::Tongue).count();
        assert_eq!(tongue(&shut), 0);
        assert_eq!(tongue(&wide), 1);
    }

    #[test]
    fn the_chin_goes_down_when_the_jaw_does() {
        let lowest = |jaw: f32| {
            scene(&Pose { jaw, ..pose(0.0, None, false) })
                .iter()
                .filter(|p| p.fill == Tone::Fur && p.points.len() > 13)
                .flat_map(|p| p.points.iter().map(|q| q.1))
                .fold(0.0f32, f32::max)
        };
        assert!(lowest(1.0) > lowest(0.0) + 30.0);
    }

    #[test]
    fn eyes_turn_into_arches_when_squinting() {
        let eyes = |s: &[Poly]| s.iter().filter(|p| p.fill == Tone::Eye).count();
        let open = scene(&Pose { squint: 0.0, ..pose(0.0, None, false) });
        let shut = scene(&Pose { squint: 1.0, ..pose(0.0, None, false) });
        assert_eq!(eyes(&open), 2);
        assert_eq!(eyes(&shut), 0);
        let arches = |s: &[Poly]| s.iter().filter(|p| !p.closed && p.line.1 > 3.0).count();
        assert_eq!(arches(&open), 0);
        assert_eq!(arches(&shut), 2);
    }

    #[test]
    fn the_speech_is_typed_out_then_fades() {
        assert!(speech(0.5).is_none());
        let (first, a) = speech(1.3).unwrap();
        assert!(!first.is_empty() && first.len() < SPEECH.len());
        assert!((a - 1.0).abs() < 1e-5);
        let (full, _) = speech(3.0).unwrap();
        assert_eq!(full, SPEECH);
        let (_, fading) = speech(REACTION - 0.5).unwrap();
        assert!(fading < 1.0);
        assert!(speech(REACTION).is_none());
        // the words are the ones he was given
        assert_eq!(SPEECH, "Because of you, I'm King");
    }

    #[test]
    fn has_pop_up_during_the_laugh_and_rise() {
        assert!(has(0.2).is_empty());
        let busy = has(1.4);
        assert!(!busy.is_empty());
        assert!(has(REACTION).is_empty());
        let early = has(0.62)[0].1;
        let later = has(1.0)[0].1;
        assert!(later.1 < early.1, "should rise");
        for k in 0..70 {
            for (_, pos, strength, _) in has(k as f32 * 0.1) {
                assert!((0.0..=1.0).contains(&strength));
                assert!(inside(pos));
            }
        }
    }

    #[test]
    fn the_glitch_comes_while_he_is_laughing() {
        assert!(GLITCH_AT > 0.3 && GLITCH_AT < 4.3);
        assert!(laugh_envelope(GLITCH_AT) > 0.99);
    }

    /// Writes the scene at KING_SINCE seconds (or at rest) as JSON to KING_DUMP for the preview
    /// tool: `KING_DUMP=/tmp/k.json KING_SINCE=2.1 cargo test dump_scene -- --ignored`.
    #[test]
    #[ignore]
    fn dump_scene() {
        let Ok(path) = std::env::var("KING_DUMP") else { return };
        let since = std::env::var("KING_SINCE").ok().and_then(|v| v.parse::<f32>().ok());
        let idle: f32 = std::env::var("KING_IDLE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
        let dead = std::env::var("KING_DEAD").is_ok();
        let p = if dead { dead_pose(idle, since, true) } else { pose(idle, since, true) };
        let mut out = String::from("{\"polys\":[");
        for (i, poly) in scene(&p).iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let pts: Vec<String> = poly.points.iter().map(|q| format!("[{:.2},{:.2}]", q.0, q.1)).collect();
            out.push_str(&format!(
                "{{\"eye\":{},\"pts\":[{}],\"fill\":{:.3},\"filled\":{},\"line\":{:.3},\"w\":{:.2},\"closed\":{},\"glow\":{:.2}}}",
                poly.fill == Tone::Eye,
                pts.join(","),
                poly.fill.value(),
                poly.fill != Tone::None,
                poly.line.0,
                poly.line.1,
                poly.closed,
                poly.glow
            ));
        }
        out.push_str("],\"has\":[");
        if let Some(s) = since {
            let items: Vec<String> = has(s)
                .iter()
                .map(|(t, pos, a, size)| format!("{{\"t\":\"{}\",\"x\":{:.1},\"y\":{:.1},\"a\":{:.2},\"size\":{:.0}}}", t, pos.0, pos.1, a, size))
                .collect();
            out.push_str(&items.join(","));
        }
        out.push_str("],\"speech\":");
        match since.and_then(speech) {
            Some((t, a)) => out.push_str(&format!("{{\"t\":\"{}\",\"a\":{:.2}}}", t.replace('"', ""), a)),
            None => out.push_str("null"),
        }
        out.push('}');
        std::fs::write(path, out).unwrap();
    }

    #[test]
    fn the_dead_kings_gold_is_black_with_only_a_trace_of_tarnish_at_the_top() {
        let lum = |c: [u8; 3]| c[0] as u32 + c[1] as u32 + c[2] as u32;
        let mut last = 0;
        for k in 0..=20 {
            let c = dead_ramp(k as f32 / 20.0);
            assert!(lum(c) >= last, "darker going up at {k}");
            last = lum(c);
        }
        // far darker than the gold everywhere, and nothing bright at all
        for k in 0..=20 {
            let v = k as f32 / 20.0;
            let (dead, gold) = (dead_ramp(v), crate::gold::ramp(v));
            assert!(lum(dead) < lum(gold) / 2 || lum(gold) < 30, "{v}: {dead:?} vs {gold:?}");
            assert!(dead.iter().all(|c| *c <= 120));
        }
        assert!(dead_ramp(0.0).iter().all(|c| *c < 6));
        // the tarnish is warm, never blue
        let top = dead_ramp(1.0);
        assert!(top[0] > top[2]);
        assert_eq!(dead_ramp(-1.0), dead_ramp(0.0));
        assert_eq!(dead_ramp(2.0), dead_ramp(1.0));
    }

    #[test]
    fn the_dead_king_sits_still_with_dead_eyes_and_twitches_now_and_then() {
        let p = dead_pose(5.0, None, true);
        assert_eq!((p.jaw, p.squint, p.brow, p.glow), (0.0, 0.0, 0.0, 0.0), "no blinking, no glow in the eyes");
        // still means still
        let still = dead_pose(5.0, None, false);
        assert_eq!((still.bob, still.tilt), (0.0, 0.0));
        // a twitch of a couple of degrees, brief
        let twitching = (0..4000).filter(|k| dead_pose(*k as f32 * 0.005, None, true).tilt.abs() > 0.5).count();
        assert!(twitching > 20 && twitching < 400, "{twitching}");
        assert!((0..4000).all(|k| dead_pose(k as f32 * 0.005, None, true).tilt.abs() <= 2.7));
    }

    #[test]
    fn the_head_goes_all_the_way_round_and_is_back_on_the_neck_when_it_stops() {
        let tilt = |s: f32| dead_pose(0.0, Some(s), true).tilt;
        // before it starts it is upright
        assert!(tilt(0.5).abs() < 3.0 && tilt(1.0).abs() < 3.0);
        // it goes round: through every quarter turn, once, to 360
        let mut max = f32::MIN;
        let mut quarters = [false; 4];
        for k in 0..=1200 {
            let t = tilt(k as f32 * 0.01);
            max = max.max(t);
            if t > 0.0 && t < 360.0 {
                quarters[(t / 90.0) as usize % 4] = true;
            }
        }
        assert!(quarters.iter().all(|q| *q), "{quarters:?}");
        assert!((max - 360.0).abs() < 6.0, "{max}");
        // done by the time it has been turning for a bit more than SPIN_TO, exactly on its neck
        for s in [SPIN_TO, SPIN_TO + 0.5, 10.0, 11.9] {
            assert!((tilt(s) - 360.0).abs() < 3.1, "at {s}: {}", tilt(s));
        }
        // upside down in the middle
        assert!((tilt((SPIN_FROM + SPIN_TO) / 2.0) - 180.0).abs() < 8.0);
        // it never turns back more than the shudder
        let mut last = 0.0;
        for k in 0..1200 {
            let t = tilt(k as f32 * 0.01);
            assert!(t >= last - 5.5, "turned back at {k}");
            last = t.max(last);
        }
        // after the laugh he is just sitting there again (0 degrees, not 360)
        assert!(dead_pose(0.0, Some(DEAD_REACTION + 1.0), true).tilt.abs() < 3.0);
    }

    #[test]
    fn the_laugh_is_slow_with_the_mouth_hanging_open_and_the_eyes_staying_dead() {
        let poses: Vec<Pose> = (0..1200).map(|k| dead_pose(0.0, Some(k as f32 * 0.01), true)).collect();
        assert!(poses.iter().all(|p| p.squint == 0.0 && p.glow == 0.0 && p.brow == 0.0), "the eyes never change");
        assert!(poses.iter().map(|p| p.jaw).fold(0.0, f32::max) > 0.9);
        assert!(poses.iter().all(|p| (0.0..=1.0).contains(&p.jaw)));
        // it hangs open and sags and gapes about 1.3 times a second: slow, not the king's 4.5
        let sags = poses.windows(2).filter(|w| w[0].jaw > 0.7 && w[1].jaw <= 0.7).count();
        assert!(sags >= 6 && sags <= 14, "{sags} sags in twelve seconds");
        assert!(poses[400..900].iter().all(|p| p.jaw > 0.3), "the mouth stays open through the laugh");
        // and it is over: mouth shut at the end
        assert_eq!(dead_pose(0.0, Some(DEAD_REACTION - 0.01), true).jaw, 0.0);
    }

    #[test]
    fn everything_glitches_from_the_first_moment_worst_while_the_head_turns_and_stops_when_he_is_still() {
        assert_eq!(dead_glitch(None), 0.0);
        assert_eq!(dead_glitch(Some(-1.0)), 0.0);
        assert_eq!(dead_glitch(Some(DEAD_REACTION)), 0.0);
        assert!(dead_glitch(Some(0.5)) > 0.05, "it starts at once");
        for s in [3.0, 4.0, 5.0, 6.0, 7.0] {
            assert!(dead_glitch(Some(s)) > 0.9, "horrible at {s}: {}", dead_glitch(Some(s)));
        }
        assert!(dead_glitch(Some(11.5)) < 0.1);
        assert!((0..=1200).all(|k| (0.0..=1.0).contains(&dead_glitch(Some(k as f32 * 0.01)))));
    }

    #[test]
    fn his_heart_strikes_twice_and_dies_away_every_beat() {
        let samples: Vec<f32> = (0..2400).map(|k| dead_beat(k as f32 * HEART_PERIOD / 2400.0)).collect();
        assert!(samples.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!(samples.iter().cloned().fold(0.0, f32::max) > 0.9, "the first thump is hard");
        assert!(samples.iter().cloned().fold(1.0, f32::min) < 0.12, "and it dies away");
        // two separate thumps: count the rises past 0.35
        let mut peaks = 0;
        for w in samples.windows(2) {
            if w[0] < 0.35 && w[1] >= 0.35 {
                peaks += 1;
            }
        }
        assert_eq!(peaks, 2, "a double beat");
        assert!((dead_beat(1.3) - dead_beat(1.3 + HEART_PERIOD)).abs() < 1e-4, "the same every time");
    }

    #[test]
    fn rings_go_out_from_his_chest_one_after_another_and_fade() {
        for k in 0..600 {
            let rings = dead_rings(k as f32 * 0.05);
            for (p, a) in rings {
                assert!((0.0..=1.0).contains(&p) && (0.0..=1.0).contains(&a));
            }
            // they are never all at the same distance
            assert!((rings[0].0 - rings[1].0).abs() > 0.2);
        }
        let near_end = dead_rings(0.0).iter().map(|r| if r.0 > 0.95 { r.1 } else { 0.0 }).fold(0.0, f32::max);
        assert!(near_end < 0.06, "a ring is gone by the time it reaches the edge");
        let (up, bright) = dead_soul(HEART_PERIOD * 0.8);
        assert!(up > 50.0 && bright > 0.0 && bright < 0.2);
        assert_eq!(dead_soul(0.0), (0.0, 0.0));
    }

    #[test]
    fn the_lights_round_him_stay_in_the_picture_and_are_always_the_same() {
        for t in [0.0_f32, 1.7, 33.3, 1234.5] {
            let motes = dead_motes(t);
            assert_eq!(motes.len(), 72);
            assert_eq!(motes, dead_motes(t));
            for m in &motes {
                assert!(m.pos.0 > 380.0 && m.pos.0 < 1020.0, "{m:?}");
                assert!(m.pos.1 > 20.0 && m.pos.1 < 820.0, "{m:?}");
                assert!((0.0..=1.0).contains(&m.alpha) && m.size >= 2.0);
            }
            let rising = motes.iter().filter(|m| m.rising).count();
            assert!((35..=65).contains(&rising), "most lift off, some fall: {rising}");
        }
        // they move
        assert_ne!(dead_motes(0.0), dead_motes(2.0));
        // life goes up and ash goes down
        let (a, b) = (dead_motes(0.0), dead_motes(0.2));
        for (x, y) in a.iter().zip(&b) {
            if x.alpha > 0.1 && (y.pos.1 - x.pos.1).abs() < 100.0 {
                assert_eq!(y.pos.1 < x.pos.1, x.rising);
            }
        }
    }

    #[test]
    fn each_dead_eye_gets_a_white_cross_that_sits_inside_it() {
        let scene = scene(&dead_pose(1.0, None, true));
        let eyes: Vec<&Poly> = scene.iter().filter(|p| p.fill == Tone::Eye).collect();
        assert_eq!(eyes.len(), 2);
        for eye in eyes {
            let [a, b] = eye_cross(&eye.points);
            let (x0, x1) = eye.points.iter().fold((f32::MAX, f32::MIN), |(l, h), p| (l.min(p.0), h.max(p.0)));
            let (y0, y1) = eye.points.iter().fold((f32::MAX, f32::MIN), |(l, h), p| (l.min(p.1), h.max(p.1)));
            for (from, to) in [a, b] {
                for p in [from, to] {
                    assert!(p.0 > x0 && p.0 < x1 && p.1 > y0 && p.1 < y1, "{p:?} outside {x0}..{x1} {y0}..{y1}");
                }
                assert!((to.0 - from.0).abs() > 4.0 && (to.1 - from.1).abs() > 4.0, "big enough to see");
            }
            // the two strokes cross at one point and are the same length
            let (m1, m2) = (((a.0 .0 + a.1 .0) / 2.0, (a.0 .1 + a.1 .1) / 2.0), ((b.0 .0 + b.1 .0) / 2.0, (b.0 .1 + b.1 .1) / 2.0));
            assert!((m1.0 - m2.0).abs() < 1e-3 && (m1.1 - m2.1).abs() < 1e-3);
            let len = |s: ((f32, f32), (f32, f32))| ((s.1 .0 - s.0 .0).powi(2) + (s.1 .1 - s.0 .1).powi(2)).sqrt();
            assert!((len(a) - len(b)).abs() < 1e-3);
        }
        assert_eq!(eye_cross(&[]), [((0.0, 0.0), (0.0, 0.0)); 2]);
    }

    #[test]
    fn the_dead_laughs_float_up_slowly_one_after_another() {
        assert!(dead_has(0.5).is_empty());
        let active = |s: f32| dead_has(s).len();
        assert!((0..120).any(|k| active(k as f32 * 0.1) >= 1));
        assert!((0..120).all(|k| active(k as f32 * 0.1) <= 2));
        for (text, pos, strength, size) in dead_has(2.0) {
            assert!(text.to_lowercase().starts_with("ha") && text.ends_with("..."));
            assert!((0.0..=1.0).contains(&strength) && size > 20.0 && pos.0 > 0.0 && pos.1 > 0.0);
        }
        assert!(dead_has(DEAD_REACTION + 3.0).is_empty());
    }

    #[test]
    fn a_glitch_tears_the_points_in_a_strip_sideways_and_snaps_them_to_blocks_when_pixelated() {
        use crate::gold::{Band, GlitchFrame};
        let calm = GlitchFrame { block: None, bands: vec![], flash: 0.0 };
        assert_eq!(glitch_point((700.0, 400.0), &calm), (700.0, 400.0));
        let torn = GlitchFrame { block: None, bands: vec![Band { top: 0.4, bottom: 0.5, shift: 0.05 }], flash: 0.0 };
        let moved = glitch_point((700.0, 0.45 * PICTURE.1), &torn);
        assert!((moved.0 - (700.0 + 0.05 * PICTURE.0 * 1.5)).abs() < 1e-3 && moved.1 == 0.45 * PICTURE.1);
        assert_eq!(glitch_point((700.0, 0.2 * PICTURE.1), &torn), (700.0, 0.2 * PICTURE.1), "outside the strip it stays");
        let blocky = GlitchFrame { block: Some(28), bands: vec![], flash: 0.0 };
        let cell = 28.0 * 1.6;
        for p in [(713.0, 417.0), (90.0, 880.0), (1399.0, 1.0)] {
            let q = glitch_point(p, &blocky);
            assert!(((q.0 / cell) - (q.0 / cell).round()).abs() < 1e-3 && ((q.1 / cell) - (q.1 / cell).round()).abs() < 1e-3);
            assert!((q.0 - p.0).abs() <= cell / 2.0 + 1e-3 && (q.1 - p.1).abs() <= cell / 2.0 + 1e-3);
        }
    }

    #[test]
    fn a_point_in_head_units_lands_where_the_head_is_drawn() {
        let still = Pose { bob: 0.0, tilt: 0.0, jaw: 0.0, squint: 0.0, brow: 0.0, shoulders: 0.0, crown_dx: 0.0, crown_tilt: 0.0, glow: 0.5 };
        assert_eq!(head_point(&still, (100.0, 200.0), 0.5, (0.0, 0.0)), (100.0, 200.0));
        // the tip of the nose, (600, 944) in the template, is 114 units below the middle of the head
        let nose = cat_scene(&still, (100.0, 200.0), 0.5).into_iter().find(|p| p.fill == Tone::Gem).expect("the nose");
        let tip = head_point(&still, (100.0, 200.0), 0.5, (0.0, 114.0));
        assert!(nose.points.iter().any(|q| (q.0 - tip.0).abs() < 0.01 && (q.1 - tip.1).abs() < 0.01), "{tip:?} in {:?}", nose.points);
        // turned and bobbing, the same point follows the nose
        let turned = Pose { tilt: 9.0, bob: 3.0, ..still };
        let nose = cat_scene(&turned, (100.0, 200.0), 0.5).into_iter().find(|p| p.fill == Tone::Gem).expect("the nose");
        let tip = head_point(&turned, (100.0, 200.0), 0.5, (0.0, 114.0));
        assert!(nose.points.iter().any(|q| (q.0 - tip.0).abs() < 0.01 && (q.1 - tip.1).abs() < 0.01));
    }
}
