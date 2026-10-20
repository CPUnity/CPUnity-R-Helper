//! The Gold Edition's Father Time: slumped, depressed, smoking a cigarette. Click him and he says
//! "What do YOU want?" with a lot of attitude; click again and it is "Begone with thee, Devil...".
//! Grab his cigarette and throw it, and he holds out his staff to you; take it and the next game
//! starts.
//!
//! This file is the rules and the shapes, in picture pixels (the same 1200 x 1600 picture as
//! `father.rs`); `ui/backdrop.rs` turns them into drawing and feeds in the pointer. Nothing here
//! touches the screen, and time is always passed in, so every part can be tested.
pub type Pt = (f32, f32);

/// The corner of his mouth where the cigarette sits, and where its tip is at rest.
pub const MOUTH: Pt = (668.0, 746.0);
pub const REST_TIP: Pt = (792.0, 812.0);
/// Half the width of the cigarette.
pub const HALF_WIDTH: f32 = 5.5;
/// A click on his face (top left, bottom right). It starts below the clock's hub, which is the
/// secret that starts the first game.
pub const FACE_BOX: (Pt, Pt) = ((455.0, 600.0), (745.0, 1000.0));
/// Where the cigarette can be grabbed (with some room around it).
pub const GRAB_BOX: (Pt, Pt) = ((640.0, 716.0), (822.0, 842.0));
/// How long "What do YOU want?" and "Begone with thee, Devil..." stay up, in seconds.
pub const SASS_LEN: f64 = 3.4;
pub const AWAY_LEN: f64 = 3.6;
/// A second click this soon after the first gets the second answer.
pub const SECOND_CLICK_WITHIN: f64 = 7.0;
/// Slowest throw that counts, in picture pixels a second (anything slower and it drops back).
pub const THROW_MIN: f32 = 500.0;
/// Fastest a throw can be.
pub const THROW_MAX: f32 = 2600.0;
/// Pull of gravity on the thrown cigarette, in picture pixels a second a second.
pub const GRAVITY: f32 = 900.0;
/// A throw is over when the cigarette is out of the picture, or after this long.
pub const FLIGHT_MAX: f32 = 1.8;
/// How long a dropped cigarette takes to fly back to his mouth.
pub const RETURN_LEN: f64 = 0.45;
/// How long he stays without it (shocked, then relieved) before he lights another.
pub const GONE_LEN: f64 = 3.0;
const SHOCK_AFTER: f64 = 1.4;
/// How long the staff takes to come out to you, how long it is held out waiting, and how long it
/// takes to go back if you leave it.
pub const OFFER_LEN: f64 = 1.6;
pub const OFFER_WAIT: f64 = 15.0;
pub const WITHDRAW_LEN: f64 = 1.3;
/// After the cigarette lands: he is in shock for `SHOCK_AFTER`, his face goes back to normal, and
/// only then does he say "...Fine. Take it." (this long after the landing). The staff waits until
/// the words are fully in and have been there a moment (`STAFF_START_AFTER`), and it can only be
/// taken when it has finished coming out.
const OFFER_WORDS_AFTER: f64 = SHOCK_AFTER + 0.4;
const STAFF_START_AFTER: f64 = OFFER_WORDS_AFTER + 0.7;
/// Only the last part of a drag counts as the speed of the throw.
const SPEED_WINDOW: f64 = 0.12;

/// How long the cigarette is.
pub fn length() -> f32 {
    ((REST_TIP.0 - MOUTH.0).powi(2) + (REST_TIP.1 - MOUTH.1).powi(2)).sqrt()
}

/// The angle of the cigarette at rest, in degrees (clockwise from pointing right).
pub fn rest_angle() -> f32 {
    (REST_TIP.1 - MOUTH.1).atan2(REST_TIP.0 - MOUTH.0).to_degrees()
}

/// The middle of the cigarette at rest.
pub fn rest_centre() -> Pt {
    ((MOUTH.0 + REST_TIP.0) / 2.0, (MOUTH.1 + REST_TIP.1) / 2.0)
}

/// The two ends of a cigarette of this length centred on `centre` at `angle` degrees: (mouth end,
/// lit end).
pub fn stick(centre: Pt, angle: f32) -> (Pt, Pt) {
    let (s, c) = angle.to_radians().sin_cos();
    let h = length() / 2.0;
    ((centre.0 - c * h, centre.1 - s * h), (centre.0 + c * h, centre.1 + s * h))
}

fn inside(p: Pt, b: (Pt, Pt)) -> bool {
    p.0 >= b.0 .0 && p.0 <= b.1 .0 && p.1 >= b.0 .1 && p.1 <= b.1 .1
}

/// Whether a point (picture pixels) is on his face.
pub fn on_face(p: Pt) -> bool {
    inside(p, FACE_BOX)
}

/// Whether a point (picture pixels) is where the cigarette can be grabbed.
pub fn on_cigarette(p: Pt) -> bool {
    inside(p, GRAB_BOX)
}

/// What his face is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    /// Heavy lids, sad brows.
    Droop,
    /// One eyebrow up, eyes rolling.
    Sass,
    /// Eyes shut, brows low: go away.
    Away,
    /// Wide eyes: somebody has his cigarette.
    Shock,
}

/// Something that happened and the program has to act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The cigarette has been thrown and has landed: the next game starts.
    Thrown,
    /// The staff was taken from his hand: the next game starts.
    StaffTaken,
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Rest,
    Sass(f64),
    Away(f64),
}

/// Where his staff is.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Staff {
    /// Planted at his side, as in the picture.
    Held,
    /// Coming out to you (since).
    Offering(f64),
    /// In your hands.
    Taken,
    /// Going back, because you left it (since).
    Withdrawing(f64),
}

#[derive(Debug, Clone, PartialEq)]
enum Cig {
    /// In his mouth.
    Mouth,
    /// In the player's hand: where it is, and where the pointer has been lately.
    Held(Pt, Vec<(f64, Pt)>),
    /// Dropped, and flying back: (where from, since).
    Returning(Pt, f64),
    /// Thrown: (where from, velocity, since).
    Flying(Pt, Pt, f64),
    /// Gone: (since).
    Gone(f64),
}

/// All of it: what he says, and where the cigarette is.
#[derive(Debug, Clone, PartialEq)]
pub struct Smoker {
    phase: Phase,
    cig: Cig,
    staff: Staff,
    /// When the first click of a pair was made.
    first_click: Option<f64>,
}

impl Default for Smoker {
    fn default() -> Self {
        Smoker::new()
    }
}

/// Where the cigarette is `t` seconds into a throw from `from` at velocity `v`.
pub fn flight_at(from: Pt, v: Pt, t: f32) -> Pt {
    (from.0 + v.0 * t, from.1 + v.1 * t + 0.5 * GRAVITY * t * t)
}

fn out_of_picture(p: Pt) -> bool {
    p.0 < -150.0 || p.0 > 1350.0 || p.1 < -150.0 || p.1 > 1750.0
}

/// How long a throw lasts: until the cigarette leaves the picture, or `FLIGHT_MAX`.
pub fn flight_len(from: Pt, v: Pt) -> f32 {
    let mut t = 0.0;
    while t < FLIGHT_MAX {
        t += 1.0 / 60.0;
        if out_of_picture(flight_at(from, v, t)) {
            return t;
        }
    }
    FLIGHT_MAX
}

fn clamp_speed(v: Pt) -> Pt {
    let speed = (v.0 * v.0 + v.1 * v.1).sqrt();
    if speed > THROW_MAX {
        (v.0 * THROW_MAX / speed, v.1 * THROW_MAX / speed)
    } else {
        v
    }
}

impl Smoker {
    pub fn new() -> Smoker {
        Smoker { phase: Phase::Rest, cig: Cig::Mouth, staff: Staff::Held, first_click: None }
    }

    /// Moves everything on to `now` (egui time, seconds); call it once a frame, first.
    pub fn tick(&mut self, now: f64) -> Option<Event> {
        match self.phase {
            Phase::Sass(at) if now - at >= SASS_LEN => self.phase = Phase::Rest,
            Phase::Away(at) if now - at >= AWAY_LEN => self.phase = Phase::Rest,
            _ => {}
        }
        match self.cig.clone() {
            Cig::Returning(_, at) if now - at >= RETURN_LEN => self.cig = Cig::Mouth,
            // he only lights another when the staff is back in his hand
            Cig::Gone(at) if now - at >= GONE_LEN && self.staff == Staff::Held => self.cig = Cig::Mouth,
            Cig::Flying(from, v, at) => {
                if (now - at) as f32 >= flight_len(from, v) {
                    self.cig = Cig::Gone(now);
                    self.phase = Phase::Rest;
                    self.staff = Staff::Offering(now);
                    return Some(Event::Thrown);
                }
            }
            _ => {}
        }
        match self.staff {
            Staff::Offering(at) if now - at >= STAFF_START_AFTER + OFFER_LEN + OFFER_WAIT => self.staff = Staff::Withdrawing(now),
            Staff::Withdrawing(at) if now - at >= WITHDRAW_LEN => {
                self.staff = Staff::Held;
                self.cig = Cig::Mouth;
            }
            _ => {}
        }
        None
    }

    /// How far out the staff is: 0 at his side, 1 held out to you.
    fn staff_out(&self, now: f64) -> f32 {
        match self.staff {
            Staff::Held | Staff::Taken => 0.0,
            Staff::Offering(at) => crate::father::ease((now - at - STAFF_START_AFTER) / OFFER_LEN),
            Staff::Withdrawing(at) => 1.0 - crate::father::ease((now - at) / WITHDRAW_LEN),
        }
    }

    /// Whether the staff has finished coming out and can be taken now, and the pointer (picture pixels) is on it.
    pub fn staff_can_be_taken_at(&self, now: f64, p: Pt) -> bool {
        matches!(self.staff, Staff::Offering(_)) && self.staff_out(now) >= 1.0 && staff_hit(&self.staff_pose_at(now), p)
    }

    /// Whether the staff is held out and ready (for the pointer to change over it).
    pub fn staff_offered(&self, now: f64) -> bool {
        matches!(self.staff, Staff::Offering(_)) && self.staff_out(now) >= 1.0
    }

    /// The pointer went down on the staff. If it is held out and the pointer is on its head, it is
    /// taken: the next game starts.
    pub fn take_staff(&mut self, now: f64, p: Pt) -> Option<Event> {
        self.tick(now);
        if self.staff_can_be_taken_at(now, p) {
            self.staff = Staff::Taken;
            self.phase = Phase::Rest;
            return Some(Event::StaffTaken);
        }
        None
    }

    /// The game is over: he has his staff back, and lights another cigarette.
    pub fn give_back(&mut self) {
        if self.staff == Staff::Taken {
            self.staff = Staff::Held;
            self.cig = Cig::Mouth;
        }
    }

    fn staff_pose_at(&self, now: f64) -> StaffPose {
        let k = self.staff_out(now);
        let mut pose = staff_pose(k);
        // held out, it bobs a little
        let still = (k - 1.0).abs() < 1e-4;
        if still {
            pose.head.1 += 7.0 * (now as f32 * 2.2).sin();
        }
        pose
    }

    /// Whether a click on his face is heard (not while somebody is playing with his cigarette).
    pub fn can_click(&self) -> bool {
        matches!(self.cig, Cig::Mouth | Cig::Gone(_))
    }

    /// Whether the cigarette can be picked up.
    pub fn can_grab(&self) -> bool {
        self.cig == Cig::Mouth
    }

    /// Whether it is in the player's hand.
    pub fn is_held(&self) -> bool {
        matches!(self.cig, Cig::Held(..))
    }

    /// A click on his face. The first gets "What do YOU want?"; a second one soon after, "Begone
    /// with thee, Devil..."; one made while he is saying that is not heard.
    pub fn click(&mut self, now: f64) {
        self.tick(now);
        if !self.can_click() || matches!(self.phase, Phase::Away(_)) {
            return;
        }
        match self.first_click {
            Some(first) if now - first < SECOND_CLICK_WITHIN => {
                self.phase = Phase::Away(now);
                self.first_click = None;
            }
            _ => {
                self.phase = Phase::Sass(now);
                self.first_click = Some(now);
            }
        }
    }

    /// The pointer went down on the cigarette (at `pos`, picture pixels).
    pub fn grab(&mut self, now: f64, pos: Pt) {
        self.tick(now);
        if self.can_grab() {
            self.cig = Cig::Held(pos, vec![(now, pos)]);
            self.phase = Phase::Rest;
        }
    }

    /// The pointer moved with the cigarette in hand.
    pub fn drag(&mut self, now: f64, pos: Pt) {
        if let Cig::Held(at, samples) = &mut self.cig {
            *at = pos;
            samples.push((now, pos));
            if samples.len() > 12 {
                samples.remove(0);
            }
        }
    }

    /// The pointer came up. A fast enough flick throws the cigarette; anything else and it drops
    /// back into his mouth.
    pub fn release(&mut self, now: f64, pos: Pt) {
        let Cig::Held(_, mut samples) = self.cig.clone() else { return };
        samples.push((now, pos));
        let recent: Vec<(f64, Pt)> = samples.into_iter().filter(|(t, _)| now - t <= SPEED_WINDOW).collect();
        let velocity = match (recent.first(), recent.last()) {
            (Some(&(t0, p0)), Some(&(t1, p1))) if t1 - t0 > 0.01 => {
                let dt = (t1 - t0) as f32;
                ((p1.0 - p0.0) / dt, (p1.1 - p0.1) / dt)
            }
            _ => (0.0, 0.0),
        };
        let speed = (velocity.0 * velocity.0 + velocity.1 * velocity.1).sqrt();
        self.cig = if speed >= THROW_MIN { Cig::Flying(pos, clamp_speed(velocity), now) } else { Cig::Returning(pos, now) };
    }

    /// Everything to draw at `now`.
    pub fn view(&self, now: f64) -> View {
        let rest = (rest_centre(), rest_angle());
        let (cigarette, mood) = match &self.cig {
            Cig::Mouth => (Some(rest), None),
            Cig::Held(at, _) => (Some((*at, rest.1 + 6.0 * (now as f32 * 9.0).sin())), Some(Mood::Shock)),
            Cig::Returning(from, at) => {
                let k = (((now - at) / RETURN_LEN) as f32).clamp(0.0, 1.0);
                let k = k * k * (3.0 - 2.0 * k);
                (Some(((from.0 + (rest.0 .0 - from.0) * k, from.1 + (rest.0 .1 - from.1) * k), rest.1)), Some(Mood::Shock))
            }
            Cig::Flying(from, v, at) => {
                let t = (now - at) as f32;
                (Some((flight_at(*from, *v, t), rest.1 + 900.0 * t)), Some(Mood::Shock))
            }
            // still shocked for a moment, unless you are talking to him
            Cig::Gone(since) => (None, if now - since < SHOCK_AFTER && self.phase == Phase::Rest { Some(Mood::Shock) } else { None }),
        };
        let mood = mood.unwrap_or(match self.phase {
            Phase::Sass(_) => Mood::Sass,
            Phase::Away(_) => Mood::Away,
            Phase::Rest => Mood::Droop,
        });
        let mut speech_at = SPEECH_AT;
        let speech = match (&self.phase, &self.cig) {
            (Phase::Sass(at), Cig::Mouth | Cig::Gone(_)) => Some(("What do YOU want?", fade(now - at, SASS_LEN))),
            (Phase::Away(at), Cig::Mouth | Cig::Gone(_)) => Some(("Begone with thee, Devil...", fade(now - at, AWAY_LEN))),
            (Phase::Rest, Cig::Gone(_)) => match self.staff {
                Staff::Offering(at) if now - at >= OFFER_WORDS_AFTER && now - at < OFFER_WORDS_AFTER + SASS_LEN => {
                    speech_at = STAFF_SPEECH_AT;
                    Some(("...Fine. Take it.", fade(now - at - OFFER_WORDS_AFTER, SASS_LEN)))
                }
                _ => None,
            },
            _ => None,
        };
        let staff = match self.staff {
            Staff::Held => StaffShow::InHand,
            Staff::Taken => StaffShow::Gone,
            // not out yet: it is still at his side while the words come in
            Staff::Offering(_) | Staff::Withdrawing(_) if self.staff_out(now) <= 0.0 => StaffShow::InHand,
            Staff::Offering(_) | Staff::Withdrawing(_) => {
                let k = self.staff_out(now);
                StaffShow::Out { pose: self.staff_pose_at(now), glow: k * (0.75 + 0.25 * (now as f32 * 3.0).sin()) }
            }
        };
        let (stick_ends, ember, smoke_from) = match cigarette {
            Some((centre, angle)) => {
                let (base, tip) = stick(centre, angle);
                let flying = matches!(self.cig, Cig::Flying(..));
                (Some((base, tip)), 0.85 + 0.15 * (now as f32 * 7.0).sin(), if flying { None } else { Some(tip) })
            }
            None => (None, 0.0, None),
        };
        let smoke = match (&self.phase, smoke_from) {
            (_, None) => 0.0,
            (Phase::Away(at), Some(_)) => 1.0 + 0.9 * fade(now - at, AWAY_LEN),
            _ => 1.0,
        };
        let trail = match &self.cig {
            Cig::Flying(from, v, at) => trail(*from, *v, (now - at) as f32),
            _ => Vec::new(),
        };
        View { mood, speech, speech_at, cigarette: stick_ends, ember, smoke_from, smoke, trail, staff }
    }
}

/// How visible words are `since` seconds after they started and lasting `len`: they come in fast
/// and go out slowly.
fn fade(since: f64, len: f64) -> f32 {
    ((since / 0.15).min((len - since) / 0.4) as f32).clamp(0.0, 1.0)
}

/// Everything to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    pub mood: Mood,
    /// The words and how visible they are (0..1).
    pub speech: Option<(&'static str, f32)>,
    /// The middle of the speech bubble (picture pixels).
    pub speech_at: Pt,
    /// The two ends of the cigarette (the mouth end, the lit end), if he has one.
    pub cigarette: Option<(Pt, Pt)>,
    /// How bright the ember is (0..1).
    pub ember: f32,
    /// Where the smoke comes from (not while the cigarette is flying).
    pub smoke_from: Option<Pt>,
    /// How much smoke: 1 is a thin wisp, more is a plume.
    pub smoke: f32,
    /// Embers left behind a thrown cigarette.
    pub trail: Vec<Puff>,
    /// What his staff is doing.
    pub staff: StaffShow,
}

/// His staff.
#[derive(Debug, Clone, PartialEq)]
pub enum StaffShow {
    /// At his side, in the picture as it is.
    InHand,
    /// Out of the picture and drawn by hand: where, and how brightly it glows (0..1).
    Out { pose: StaffPose, glow: f32 },
    /// Taken by you: not in the picture any more.
    Gone,
}

/// A soft round thing: smoke, or an ember.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Puff {
    pub pos: Pt,
    pub radius: f32,
    pub alpha: f32,
}

fn hash01(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    ((x.wrapping_mul(0x297A_2D39) >> 8) & 0xFFFF) as f32 / 65535.0
}

fn frac(x: f32) -> f32 {
    x - x.floor()
}

/// The embers behind a cigarette `t` seconds into its throw: a fading line along its path, with
/// a few sparks thrown off it.
pub fn trail(from: Pt, v: Pt, t: f32) -> Vec<Puff> {
    let mut out = Vec::new();
    for k in 0..16u32 {
        let tk = t - k as f32 * 0.03;
        if tk < 0.0 {
            break;
        }
        let fade = 1.0 - k as f32 / 16.0;
        let pos = flight_at(from, v, tk);
        out.push(Puff { pos, radius: 3.0 + 3.0 * fade, alpha: 0.8 * fade });
        // a spark that flies off sideways and falls
        let (a, b) = (hash01(k, 11), hash01(k, 23));
        out.push(Puff {
            pos: (pos.0 + (a - 0.5) * 60.0, pos.1 + (b - 0.3) * 50.0),
            radius: 1.5 + 2.0 * hash01(k, 5),
            alpha: 0.7 * fade,
        });
    }
    out
}

/// The smoke rising from `from` at time `t` (seconds): three strands that wind upwards and fade.
/// `amount` 1 is a thin wisp, 2 a plume.
pub fn smoke(from: Pt, t: f32, amount: f32) -> Vec<Puff> {
    const PER_STRAND: usize = 24;
    let mut out = Vec::with_capacity(3 * PER_STRAND);
    for strand in 0..3u32 {
        let (phase, amp) = (hash01(strand, 3) * 6.28, 22.0 + strand as f32 * 10.0);
        let weight = if strand == 0 { 1.0 } else { 0.5 };
        for i in 0..PER_STRAND {
            let age = frac(i as f32 / PER_STRAND as f32 + t * 0.12 + strand as f32 * 0.31);
            let pos = (
                from.0 + (age * 7.0 + phase + t * 0.5).sin() * amp * (0.3 + age) + 60.0 * age * age,
                from.1 - age * 520.0 * (0.6 + 0.2 * strand as f32),
            );
            let fade_in = (age / 0.05).min(1.0);
            out.push(Puff {
                pos,
                radius: 3.0 + 26.0 * age.powf(1.2),
                alpha: (0.37 * amount * weight * (1.0 - age).powf(1.4) * fade_in).clamp(0.0, 1.0),
            });
        }
    }
    out
}

// ---- the staff ---------------------------------------------------------------------------------

/// Where the staff picture (`assets/backdrop-staff.png`) sits in the 1200 x 1600 picture: top left,
/// bottom right. At rest it is drawn exactly there.
pub const STAFF_PLACE: (Pt, Pt) = ((104.0, 198.0), (340.0, 1486.0));
/// Where the middle of the hourglass is, in picture rows.
const HEAD_CENTRE_Y: f32 = 344.0;
/// How far the hourglass reaches from the middle of the shaft, in picture pixels at rest.
const HEAD_RADIUS: f32 = 150.0;
const STAFF_ROWS: usize = 32;
/// The shaft leans a little: its middle is at x = 202 + 0.0201 y in the picture.
const SHAFT_SLOPE: f32 = 0.0201;

fn shaft_x(y: f32) -> f32 {
    202.0 + SHAFT_SLOPE * y
}

/// Where the staff is: its top (the hourglass) is at `head`, it is turned `angle` degrees
/// clockwise about that, it is `w_head` times its size at the top and `w_base` at the tip (so a
/// staff held out to you is bigger at its head), and it bows sideways by `bow` picture pixels in
/// the middle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaffPose {
    pub head: Pt,
    pub angle: f32,
    pub w_head: f32,
    pub w_base: f32,
    pub bow: f32,
}

/// Where the top of the staff is when it is held out to you (the hourglass is a little way below
/// and to the left of this, as the staff is turned: about (840, 880)).
pub const OFFERED_HEAD: Pt = (971.0, 670.0);
/// Where the words go while he holds it out.
pub const STAFF_SPEECH_AT: Pt = (470.0, 1010.0);

/// The staff `k` of the way from his side (0) to held out to you (1).
pub fn staff_pose(k: f32) -> StaffPose {
    let k = k.clamp(0.0, 1.0);
    let mix = |a: f32, b: f32| a + (b - a) * k;
    StaffPose {
        head: (mix(shaft_x(STAFF_PLACE.0 .1), OFFERED_HEAD.0), mix(STAFF_PLACE.0 .1, OFFERED_HEAD.1)),
        angle: mix(0.0, 32.0),
        w_head: mix(1.0, 1.7),
        w_base: 1.0,
        bow: 90.0 * (std::f32::consts::PI * k).sin(),
    }
}

/// The middle of the staff `t` of the way down it (0 the top, 1 the tip), after the pose.
fn spine(pose: &StaffPose, t: f32) -> Pt {
    let length = STAFF_PLACE.1 .1 - STAFF_PLACE.0 .1;
    let steps = ((t * 48.0).ceil() as usize).max(1);
    let width = |t: f32| pose.w_head + (pose.w_base - pose.w_head) * t;
    let mut q = (0.0_f32, 0.0_f32);
    for i in 0..steps {
        let (a, b) = (t * i as f32 / steps as f32, t * (i + 1) as f32 / steps as f32);
        let d = length * (b - a) * (width(a) + width(b)) / 2.0;
        q.0 += SHAFT_SLOPE * d;
        q.1 += d;
    }
    q.0 += pose.bow * (std::f32::consts::PI * t).sin();
    let (s, c) = pose.angle.to_radians().sin_cos();
    (pose.head.0 + q.0 * c - q.1 * s, pose.head.1 + q.0 * s + q.1 * c)
}

/// One row of the staff picture, drawn as a strip: where its two ends go, and how far down the
/// picture it is (0..1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaffRow {
    pub left: Pt,
    pub right: Pt,
    pub v: f32,
}

/// The staff picture as rows to draw, top to bottom.
pub fn staff_rows(pose: &StaffPose) -> Vec<StaffRow> {
    let length = STAFF_PLACE.1 .1 - STAFF_PLACE.0 .1;
    let (s, c) = pose.angle.to_radians().sin_cos();
    (0..=STAFF_ROWS)
        .map(|i| {
            let t = i as f32 / STAFF_ROWS as f32;
            let y = STAFF_PLACE.0 .1 + length * t;
            let w = pose.w_head + (pose.w_base - pose.w_head) * t;
            let mid = spine(pose, t);
            let side = |x: f32| {
                let d = (x - shaft_x(y)) * w;
                (mid.0 + d * c, mid.1 + d * s)
            };
            StaffRow { left: side(STAFF_PLACE.0 .0), right: side(STAFF_PLACE.1 .0), v: t }
        })
        .collect()
}

/// The middle of the hourglass, after the pose.
pub fn head_centre(pose: &StaffPose) -> Pt {
    spine(pose, (HEAD_CENTRE_Y - STAFF_PLACE.0 .1) / (STAFF_PLACE.1 .1 - STAFF_PLACE.0 .1))
}

/// How far from `head_centre` a point can be and still be on the head (generous, so it is easy to grab).
pub fn head_radius(pose: &StaffPose) -> f32 {
    HEAD_RADIUS * pose.w_head
}

/// Whether a point (picture pixels) is on the head of the staff.
pub fn staff_hit(pose: &StaffPose, p: Pt) -> bool {
    let c = head_centre(pose);
    (p.0 - c.0).hypot(p.1 - c.1) <= head_radius(pose)
}

// ---- the face ----------------------------------------------------------------------------------

/// A bright shape: the points, how strongly it is filled and how strongly it is outlined (0..1).
#[derive(Debug, Clone, PartialEq)]
pub struct Glow {
    pub points: Vec<Pt>,
    pub fill: f32,
    pub edge: f32,
}

/// A line: the points, how thick it is (picture pixels) and how strong (0..1).
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub points: Vec<Pt>,
    pub width: f32,
    pub alpha: f32,
}

/// His eyes, brows and the rest of the expression, over the eyes that are in the picture.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    pub eyes: Vec<Glow>,
    /// Dark dots in the eyes: where he is looking.
    pub pupils: Vec<(Pt, f32)>,
    pub strokes: Vec<Stroke>,
}

fn mirror(points: &[Pt]) -> Vec<Pt> {
    points.iter().map(|p| (1200.0 - p.0, p.1)).collect()
}

fn curve(p0: Pt, p1: Pt, p2: Pt) -> Vec<Pt> {
    (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            let u = 1.0 - t;
            (u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0, u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1)
        })
        .collect()
}

/// A shape scaled about its middle, turned `rot` degrees, then moved.
fn transformed(points: &[Pt], sx: f32, sy: f32, rot: f32, dx: f32, dy: f32) -> Vec<Pt> {
    let n = points.len() as f32;
    let (cx, cy) = (points.iter().map(|p| p.0).sum::<f32>() / n, points.iter().map(|p| p.1).sum::<f32>() / n);
    let (s, c) = rot.to_radians().sin_cos();
    points
        .iter()
        .map(|p| {
            let (x, y) = ((p.0 - cx) * sx, (p.1 - cy) * sy);
            (cx + x * c - y * s + dx, cy + x * s + y * c + dy)
        })
        .collect()
}

fn both(points: &[Pt], width: f32, alpha: f32) -> [Stroke; 2] {
    [
        Stroke { points: points.to_vec(), width, alpha },
        Stroke { points: mirror(points), width, alpha },
    ]
}

/// The two places the eyes in the picture are covered before the new ones are drawn (a little
/// bigger than the eyes, so their glow goes too).
pub fn eye_covers() -> [Vec<Pt>; 2] {
    let right = transformed(&crate::father::EYE, 1.45, 1.8, 0.0, 0.0, 0.0);
    [mirror(&right), right]
}

/// His face for a mood.
pub fn face(mood: Mood) -> Face {
    let right = crate::father::EYE.to_vec();
    let left = mirror(&right);
    let mut eyes = Vec::new();
    let mut pupils = Vec::new();
    let mut strokes = Vec::new();
    match mood {
        Mood::Droop => {
            for e in [&right, &left] {
                eyes.push(Glow { points: transformed(e, 0.92, 0.42, 0.0, 0.0, 16.0), fill: 0.59, edge: 0.78 });
            }
            strokes.extend(both(&curve((632.0, 506.0), (690.0, 512.0), (772.0, 548.0)), 5.0, 1.0));
            strokes.extend(both(&[(640.0, 586.0), (700.0, 600.0), (756.0, 584.0)], 3.0, 0.43));
        }
        Mood::Sass => {
            eyes.push(Glow { points: transformed(&right, 0.95, 0.62, -6.0, 0.0, -2.0), fill: 0.92, edge: 1.0 });
            eyes.push(Glow { points: transformed(&left, 0.95, 0.5, 0.0, 0.0, 6.0), fill: 0.84, edge: 1.0 });
            pupils = vec![((716.0, 538.0), 9.0), ((484.0, 548.0), 9.0)];
            strokes.push(Stroke { points: curve((640.0, 492.0), (708.0, 448.0), (776.0, 488.0)), width: 6.0, alpha: 1.0 });
            strokes.push(Stroke { points: vec![(424.0, 522.0), (500.0, 520.0), (570.0, 534.0)], width: 5.0, alpha: 1.0 });
            // little arcs either side of his head: attitude
            for (side, x) in [(1.0_f32, 905.0_f32), (-1.0, 295.0)] {
                for (k, (y, len)) in [(470.0_f32, 34.0_f32), (520.0, 46.0), (570.0, 34.0)].into_iter().enumerate() {
                    let k = k as f32;
                    strokes.push(Stroke {
                        points: curve((x + side * 6.0 * k, y - len), (x + side * (26.0 + 6.0 * k), y), (x + side * 6.0 * k, y + len)),
                        width: 4.0,
                        alpha: 0.78,
                    });
                }
            }
        }
        Mood::Away => {
            for e in [&right, &left] {
                let n = e.len() as f32;
                let (cx, cy) = (e.iter().map(|p| p.0).sum::<f32>() / n, e.iter().map(|p| p.1).sum::<f32>() / n);
                strokes.push(Stroke { points: curve((cx - 40.0, cy - 2.0), (cx, cy + 26.0), (cx + 40.0, cy - 2.0)), width: 6.0, alpha: 1.0 });
            }
            strokes.extend(both(&curve((640.0, 506.0), (700.0, 520.0), (772.0, 506.0)), 5.0, 1.0));
        }
        Mood::Shock => {
            for e in [&right, &left] {
                let turn = if std::ptr::eq(e, &right) { -4.0 } else { 4.0 };
                eyes.push(Glow { points: transformed(e, 1.12, 1.55, turn, 0.0, 0.0), fill: 1.0, edge: 1.0 });
            }
            pupils = vec![((696.0, 548.0), 7.0), ((504.0, 548.0), 7.0)];
            strokes.extend(both(&curve((636.0, 470.0), (704.0, 440.0), (772.0, 468.0)), 5.0, 1.0));
        }
    }
    Face { eyes, pupils, strokes }
}

/// Where the words go (the middle of the bubble, picture pixels) and where its tail points.
pub const SPEECH_AT: Pt = (600.0, 990.0);
pub const SPEECH_TAIL: Pt = (640.0, 840.0);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::info;

    fn smoker() -> Smoker {
        Smoker::new()
    }

    #[test]
    fn he_starts_slumped_with_a_cigarette_and_nothing_to_say() {
        let v = smoker().view(0.0);
        assert_eq!(v.mood, Mood::Droop);
        assert!(v.speech.is_none());
        assert!(v.cigarette.is_some() && v.smoke_from.is_some());
        assert!(v.smoke >= 1.0 && v.trail.is_empty());
    }

    #[test]
    fn the_cigarette_rests_in_the_corner_of_his_mouth() {
        let (base, tip) = smoker().view(0.0).cigarette.unwrap();
        let near = |a: Pt, b: Pt| (a.0 - b.0).abs() < 2.0 && (a.1 - b.1).abs() < 2.0;
        // the ember wobbles a little with the time, but the angle at rest is the rest angle
        assert!(near(base, MOUTH), "{base:?}");
        assert!(near(tip, REST_TIP), "{tip:?}");
        assert!((length() - 140.5).abs() < 1.0);
        assert!((rest_angle() - 28.0).abs() < 1.0);
    }

    #[test]
    fn the_first_click_is_sass_the_second_sends_you_away() {
        let mut s = smoker();
        s.click(10.0);
        let v = s.view(10.5);
        assert_eq!(v.mood, Mood::Sass);
        assert_eq!(v.speech.map(|x| x.0), Some("What do YOU want?"));
        s.click(11.0);
        let v = s.view(11.5);
        assert_eq!(v.mood, Mood::Away);
        assert_eq!(v.speech.map(|x| x.0), Some("Begone with thee, Devil..."));
        assert!(v.smoke > 1.0, "a plume in your face");
    }

    #[test]
    fn a_click_while_he_is_saying_begone_is_not_heard() {
        let mut s = smoker();
        s.click(0.0);
        s.click(1.0);
        s.click(2.0);
        s.click(3.0);
        assert_eq!(s.view(3.5).mood, Mood::Away, "still the same dismissal");
        // it ends on schedule, from the second click, not from the later ones
        s.tick(1.0 + AWAY_LEN + 0.01);
        assert_eq!(s.view(1.0 + AWAY_LEN + 0.02).mood, Mood::Droop);
    }

    #[test]
    fn after_the_words_he_sulks_again_and_the_next_click_starts_over() {
        let mut s = smoker();
        s.click(0.0);
        s.tick(SASS_LEN + 0.1);
        assert_eq!(s.view(SASS_LEN + 0.1).mood, Mood::Droop);
        assert!(s.view(SASS_LEN + 0.1).speech.is_none());
        // still inside the window for a second click: that is the dismissal
        s.click(5.0);
        assert_eq!(s.view(5.1).mood, Mood::Away);
        // a click long after the first one is a first click again
        let mut s = smoker();
        s.click(0.0);
        s.click(SECOND_CLICK_WITHIN + 1.0);
        assert_eq!(s.view(SECOND_CLICK_WITHIN + 1.1).mood, Mood::Sass);
        // and after a dismissal the next click is a first one
        let mut s = smoker();
        s.click(0.0);
        s.click(1.0);
        s.click(1.0 + AWAY_LEN + 1.0);
        assert_eq!(s.view(1.0 + AWAY_LEN + 1.1).mood, Mood::Sass);
    }

    #[test]
    fn the_words_fade_in_and_out() {
        let mut s = smoker();
        s.click(0.0);
        let a = |t: f64| s.view(t).speech.map_or(0.0, |x| x.1);
        assert!(a(0.0) < 0.05);
        assert!(a(0.1) > 0.3 && a(0.1) < 1.0);
        assert_eq!(a(1.5), 1.0);
        assert!(a(SASS_LEN - 0.1) < 0.5);
        for t in [0.0, 0.5, 2.0, 3.3] {
            assert!((0.0..=1.0).contains(&a(t)));
        }
    }

    #[test]
    fn the_face_boxes_leave_the_clocks_hub_alone() {
        let hub = info::CLOCK_CENTER;
        assert!(!on_face((hub.0, hub.1)), "the secret of the first game is not a click on his face");
        assert!(!on_face((hub.0 + 55.0, hub.1 + 55.0)));
        assert!(on_face((600.0, 800.0)));
        assert!(on_cigarette(rest_centre()));
        assert!(on_cigarette(MOUTH) && on_cigarette(REST_TIP));
        assert!(!on_cigarette((300.0, 300.0)));
    }

    #[test]
    fn picking_up_the_cigarette_cancels_the_chat_and_shocks_him() {
        let mut s = smoker();
        s.click(0.0);
        s.grab(0.5, rest_centre());
        assert!(s.is_held() && !s.can_click() && !s.can_grab());
        let v = s.view(0.6);
        assert_eq!(v.mood, Mood::Shock);
        assert!(v.speech.is_none());
        // it follows the pointer
        s.drag(0.6, (900.0, 900.0));
        let (base, tip) = s.view(0.6).cigarette.unwrap();
        let mid = ((base.0 + tip.0) / 2.0, (base.1 + tip.1) / 2.0);
        assert!((mid.0 - 900.0).abs() < 1.0 && (mid.1 - 900.0).abs() < 1.0);
        // clicks are ignored while it is in the hand
        s.click(0.7);
        assert_eq!(s.view(0.7).mood, Mood::Shock);
    }

    #[test]
    fn a_slow_let_go_drops_it_back_into_his_mouth() {
        let mut s = smoker();
        s.grab(0.0, rest_centre());
        s.drag(0.5, (900.0, 900.0));
        s.drag(1.0, (900.0, 901.0));
        s.release(1.0, (900.0, 901.0));
        assert!(!s.is_held());
        assert!(s.tick(1.1).is_none());
        // on its way back it is somewhere between
        let mid = s.view(1.0 + RETURN_LEN / 2.0).cigarette.unwrap();
        assert!(mid.0 .0 > MOUTH.0 && mid.0 .1 > MOUTH.1 + 10.0);
        s.tick(1.0 + RETURN_LEN + 0.01);
        assert!(s.can_grab(), "it is back in his mouth");
        assert_eq!(s.view(1.0 + RETURN_LEN + 0.02).mood, Mood::Droop);
    }

    fn throw(s: &mut Smoker, now: f64, from: Pt, to: Pt, secs: f64) {
        s.grab(now, from);
        for i in 1..=6 {
            let k = i as f32 / 6.0;
            s.drag(now + secs * f64::from(k), (from.0 + (to.0 - from.0) * k, from.1 + (to.1 - from.1) * k));
        }
        s.release(now + secs, to);
    }

    #[test]
    fn a_fast_flick_throws_it_and_the_game_starts_when_it_lands() {
        let mut s = smoker();
        // 700 picture pixels in a tenth of a second
        throw(&mut s, 5.0, (760.0, 790.0), (1000.0, 300.0), 0.1);
        let v = s.view(5.2);
        assert_eq!(v.mood, Mood::Shock);
        assert!(!v.trail.is_empty() && v.smoke_from.is_none(), "flying cigarettes leave embers, not smoke");
        // it is under way: nothing yet
        assert!(s.tick(5.2).is_none());
        // and over eventually: exactly one event, then nothing again
        let mut events = 0;
        let mut t = 5.2;
        while t < 9.0 {
            t += 0.016;
            if s.tick(t) == Some(Event::Thrown) {
                events += 1;
            }
        }
        assert_eq!(events, 1);
        assert!(s.view(t).cigarette.is_none() || s.can_grab(), "he is without it or has lit another");
    }

    #[test]
    fn the_throw_speed_counts_the_end_of_the_drag_not_the_whole_of_it() {
        let mut s = smoker();
        s.grab(0.0, rest_centre());
        // dragged far and fast, then held still for half a second: that is no throw
        s.drag(0.05, (1000.0, 300.0));
        s.drag(0.55, (1000.0, 300.0));
        s.release(0.6, (1000.0, 300.0));
        s.tick(0.7);
        assert!(!matches!(s.view(0.7).trail.len(), 1..), "no embers: it dropped, it was not thrown");
        assert!(s.tick(0.6 + RETURN_LEN + 0.1).is_none());
    }

    #[test]
    fn a_throw_has_a_top_speed_and_ends_off_the_picture_or_on_time() {
        let v = clamp_speed((10_000.0, 0.0));
        assert!((v.0 - THROW_MAX).abs() < 0.1);
        for (from, v) in [((800.0, 800.0), (2000.0, -500.0)), ((800.0, 800.0), (0.0, -600.0)), ((300.0, 700.0), (-1500.0, 0.0))] {
            let len = flight_len(from, clamp_speed(v));
            assert!(len > 0.0 && len <= FLIGHT_MAX + 0.001, "{len}");
            assert!(len < FLIGHT_MAX || !out_of_picture(flight_at(from, v, len)));
        }
        // straight up the picture lasts longer than one sideways
        assert!(flight_len((600.0, 700.0), (0.0, -700.0)) > flight_len((600.0, 700.0), (2400.0, 0.0)) - 0.5);
    }

    #[test]
    fn he_is_shocked_without_it_then_lights_another() {
        let mut s = smoker();
        throw(&mut s, 0.0, (760.0, 790.0), (1000.0, 300.0), 0.1);
        let mut t = 0.2;
        let landed = loop {
            t += 0.016;
            if s.tick(t) == Some(Event::Thrown) {
                break t;
            }
            assert!(t < 5.0);
        };
        assert!(s.view(landed + 0.2).cigarette.is_none());
        assert_eq!(s.view(landed + 0.2).mood, Mood::Shock);
        assert_eq!(s.view(landed + SHOCK_AFTER + 0.1).mood, Mood::Droop);
        assert!(s.view(landed + 1.0).smoke_from.is_none() && s.view(landed + 1.0).smoke == 0.0);
        // you may talk to him meanwhile
        s.click(landed + 1.0);
        assert_eq!(s.view(landed + 1.1).mood, Mood::Sass);
        // he does not light another while his staff is out
        s.tick(landed + GONE_LEN + 0.1);
        assert!(!s.can_grab());
    }

    /// A smoker whose cigarette has just landed, and when.
    fn landed() -> (Smoker, f64) {
        let mut s = smoker();
        throw(&mut s, 0.0, (760.0, 790.0), (1000.0, 300.0), 0.1);
        let mut t = 0.2;
        loop {
            t += 0.016;
            if s.tick(t) == Some(Event::Thrown) {
                return (s, t);
            }
            assert!(t < 5.0);
        }
    }

    #[test]
    fn at_rest_the_staff_is_drawn_exactly_where_it_is_in_the_picture() {
        let rows = staff_rows(&staff_pose(0.0));
        assert_eq!(rows.len(), STAFF_ROWS + 1);
        for r in &rows {
            assert!((r.left.0 - STAFF_PLACE.0 .0).abs() < 0.05 && (r.right.0 - STAFF_PLACE.1 .0).abs() < 0.05, "{r:?}");
            assert!(r.left.1 >= STAFF_PLACE.0 .1 - 0.05 && r.left.1 <= STAFF_PLACE.1 .1 + 0.05);
            assert!((r.left.1 - r.right.1).abs() < 0.05);
        }
        assert!((rows[0].left.1 - STAFF_PLACE.0 .1).abs() < 0.05 && (rows[STAFF_ROWS].left.1 - STAFF_PLACE.1 .1).abs() < 0.05);
        assert_eq!((rows[0].v, rows[STAFF_ROWS].v), (0.0, 1.0));
        assert!(rows.windows(2).all(|w| w[1].v > w[0].v && w[1].left.1 > w[0].left.1));
    }

    #[test]
    fn held_out_the_staff_comes_toward_you_and_is_bigger_at_its_head() {
        let out = staff_pose(1.0);
        let c = head_centre(&out);
        assert!((c.0 - 840.0).abs() < 20.0 && (c.1 - 880.0).abs() < 20.0, "{c:?}");
        assert!(c.1 + head_radius(&out) < 1110.0 + 200.0, "the head is on the stage: {c:?}");
        let rows = staff_rows(&out);
        let width = |r: &StaffRow| (r.right.0 - r.left.0).hypot(r.right.1 - r.left.1);
        assert!(width(&rows[0]) > width(&rows[STAFF_ROWS]) * 1.6);
        assert!(width(&rows[0]) > (STAFF_PLACE.1 .0 - STAFF_PLACE.0 .0) * 1.6);
        for r in &rows {
            assert!(r.left.0.is_finite() && r.left.1.is_finite() && r.right.0.is_finite() && r.right.1.is_finite());
        }
        // the head, with its reach, is inside the picture
        assert!(c.0 + head_radius(&out) < 1200.0 && c.0 - head_radius(&out) > 0.0);
        // between the two the staff is between them
        let half = staff_pose(0.5);
        assert!(half.head.0 > staff_pose(0.0).head.0 && half.head.0 < out.head.0);
        assert!(half.bow > 0.0 && staff_pose(0.0).bow.abs() < 1e-3 && out.bow.abs() < 1e-2, "it sways on the way and settles");
    }

    #[test]
    fn he_holds_out_the_staff_after_the_cigarette_and_the_game_starts_when_it_is_taken() {
        let (mut s, landed) = landed();
        // at first it is at his side, then it comes out
        assert_eq!(s.view(landed).staff, StaffShow::InHand);
        assert!(!s.staff_offered(landed + 0.1));
        // too soon to take, and still while it is on its way
        let head = head_centre(&staff_pose(1.0));
        assert_eq!(s.take_staff(landed + 0.1, head), None);
        let under_way = landed + STAFF_START_AFTER + OFFER_LEN * 0.8;
        assert!(matches!(s.view(under_way).staff, StaffShow::Out { .. }));
        assert!(!s.staff_offered(under_way) && s.take_staff(under_way, head).is_none(), "not until it has arrived");
        // held out, but you have to be on its head
        let ready = landed + STAFF_START_AFTER + OFFER_LEN + 0.1;
        assert!(s.staff_offered(ready));
        assert_eq!(s.take_staff(ready, (100.0, 100.0)), None);
        assert!(s.staff_can_be_taken_at(ready, head));
        assert_eq!(s.take_staff(ready, head), Some(Event::StaffTaken));
        assert_eq!(s.view(ready + 0.1).staff, StaffShow::Gone);
        // only once
        assert_eq!(s.take_staff(ready + 0.2, head), None);
        // he does not light another while you have it
        s.tick(ready + 60.0);
        assert!(!s.can_grab() && s.view(ready + 60.0).cigarette.is_none());
        // and gets it back, with another cigarette, when the game is over
        s.give_back();
        assert!(s.can_grab());
        assert_eq!(s.view(ready + 61.0).staff, StaffShow::InHand);
        s.give_back(); // harmless
    }

    #[test]
    fn a_staff_nobody_takes_goes_back_and_he_lights_another() {
        let (mut s, landed) = landed();
        s.tick(landed + STAFF_START_AFTER + OFFER_LEN + OFFER_WAIT + 0.1);
        let v = s.view(landed + STAFF_START_AFTER + OFFER_LEN + OFFER_WAIT + 0.1 + WITHDRAW_LEN / 2.0);
        match v.staff {
            StaffShow::Out { pose, glow } => {
                assert!(pose.head.0 > staff_pose(0.0).head.0 && pose.head.0 < OFFERED_HEAD.0 + 1.0, "on its way back");
                assert!((0.0..=1.0).contains(&glow));
            }
            other => panic!("{other:?}"),
        }
        // it cannot be taken on its way back
        assert!(!s.staff_offered(landed + STAFF_START_AFTER + OFFER_LEN + OFFER_WAIT + 0.1 + WITHDRAW_LEN * 0.9));
        s.tick(landed + STAFF_START_AFTER + OFFER_LEN + OFFER_WAIT + WITHDRAW_LEN + 0.5);
        assert!(s.can_grab());
        assert_eq!(s.view(landed + STAFF_START_AFTER + OFFER_LEN + OFFER_WAIT + WITHDRAW_LEN + 0.6).staff, StaffShow::InHand);
    }

    #[test]
    fn he_says_fine_take_it_while_holding_it_out() {
        let (s, landed) = landed();
        assert!(s.view(landed + 0.2).speech.is_none(), "first the staff moves");
        let v = s.view(landed + OFFER_WORDS_AFTER + 1.0);
        assert_eq!(v.speech.map(|x| x.0), Some("...Fine. Take it."));
        // his face is back to normal before he speaks, and the words are all the way in before the
        // staff moves
        assert_eq!(s.view(landed + SHOCK_AFTER + 0.05).mood, Mood::Droop);
        assert_eq!(s.view(landed + SHOCK_AFTER + 0.05).speech, None);
        assert_eq!(s.view(landed + OFFER_WORDS_AFTER + 0.01).mood, Mood::Droop);
        assert!(s.view(landed + SHOCK_AFTER - 0.1).mood == Mood::Shock && s.view(landed + SHOCK_AFTER - 0.1).speech.is_none());
        let words_in = landed + OFFER_WORDS_AFTER + 0.15;
        assert_eq!(s.view(words_in).speech.map(|x| x.1), Some(1.0));
        assert!(STAFF_START_AFTER > OFFER_WORDS_AFTER + 0.15);
        assert_eq!(s.view(landed + STAFF_START_AFTER - 0.01).staff, StaffShow::InHand);
        assert!(matches!(s.view(landed + STAFF_START_AFTER + 0.3).staff, StaffShow::Out { .. }));
        // the words are still up while it comes out
        assert!(s.view(landed + STAFF_START_AFTER + OFFER_LEN * 0.5).speech.is_some());
        assert_eq!(v.speech_at, STAFF_SPEECH_AT);
        assert!(s.view(landed + OFFER_WORDS_AFTER + SASS_LEN + 0.1).speech.is_none());
        // the other words keep their place
        assert_eq!(smoker().view(0.0).speech_at, SPEECH_AT);
    }

    #[test]
    fn the_grab_and_release_calls_are_harmless_out_of_turn() {
        let mut s = smoker();
        s.drag(0.0, (1.0, 1.0));
        s.release(0.0, (1.0, 1.0));
        assert_eq!(s, smoker());
        s.grab(0.0, rest_centre());
        s.grab(0.1, (5.0, 5.0)); // a second grab while holding does nothing
        s.drag(0.2, (700.0, 700.0));
        assert!(s.is_held());
    }

    #[test]
    fn the_embers_and_smoke_are_plain_numbers_that_depend_only_on_the_time() {
        let a = smoke(REST_TIP, 12.0, 1.0);
        assert_eq!(a, smoke(REST_TIP, 12.0, 1.0));
        assert_ne!(a, smoke(REST_TIP, 13.0, 1.0));
        assert_eq!(a.len(), 72);
        for p in &a {
            assert!((0.0..=1.0).contains(&p.alpha) && p.radius >= 3.0 && p.radius <= 29.1);
            assert!(p.pos.1 <= REST_TIP.1 + 0.1, "smoke rises");
            assert!(p.pos.0.is_finite() && p.pos.1.is_finite());
        }
        // a plume is thicker than a wisp
        let sum = |v: &[Puff]| v.iter().map(|p| p.alpha).sum::<f32>();
        assert!(sum(&smoke(REST_TIP, 4.0, 2.0)) > sum(&smoke(REST_TIP, 4.0, 1.0)) * 1.9);
        assert!(smoke(REST_TIP, 4.0, 0.0).iter().all(|p| p.alpha == 0.0));
        // the trail fades from the cigarette backwards and never starts before the throw
        let tr = trail((800.0, 800.0), (1000.0, -800.0), 0.2);
        assert!(!tr.is_empty() && tr.iter().all(|p| (0.0..=1.0).contains(&p.alpha)));
        assert!(trail((800.0, 800.0), (1000.0, -800.0), 0.0).len() <= 2);
    }

    #[test]
    fn every_mood_has_its_own_face_inside_the_picture() {
        let faces: Vec<Face> = [Mood::Droop, Mood::Sass, Mood::Away, Mood::Shock].into_iter().map(face).collect();
        for (i, f) in faces.iter().enumerate() {
            for p in f.eyes.iter().flat_map(|e| &e.points).chain(f.strokes.iter().flat_map(|s| &s.points)) {
                assert!(p.0 > 0.0 && p.0 < 1200.0 && p.1 > 0.0 && p.1 < 1600.0, "face {i}: {p:?}");
            }
            for e in &f.eyes {
                assert!(e.points.len() >= 3 && (0.0..=1.0).contains(&e.fill) && (0.0..=1.0).contains(&e.edge));
            }
            assert!(f.strokes.iter().all(|s| s.points.len() >= 2 && s.width > 0.0 && (0.0..=1.0).contains(&s.alpha)));
        }
        for i in 0..faces.len() {
            for j in 0..i {
                assert_ne!(faces[i], faces[j], "moods look different");
            }
        }
        // both eyes in every open mood, none when they are shut
        assert_eq!(faces[0].eyes.len(), 2);
        assert_eq!(faces[1].eyes.len(), 2);
        assert!(faces[2].eyes.is_empty());
        assert_eq!(faces[3].eyes.len(), 2);
        // only the sassy and the shocked eyes have a pupil (they are looking at something)
        assert_eq!((faces[0].pupils.len(), faces[1].pupils.len(), faces[2].pupils.len(), faces[3].pupils.len()), (0, 2, 0, 2));
        // the covers are bigger than the eyes they hide
        for (cover, eye) in eye_covers().iter().zip([mirror(&crate::father::EYE), crate::father::EYE.to_vec()]) {
            let width = |v: &[Pt]| v.iter().map(|p| p.0).fold(f32::MIN, f32::max) - v.iter().map(|p| p.0).fold(f32::MAX, f32::min);
            assert!(width(cover) > width(&eye) * 1.3);
        }
    }

    #[test]
    fn the_speech_and_its_tail_sit_below_his_mouth() {
        assert!(SPEECH_AT.1 > MOUTH.1 && SPEECH_TAIL.1 > MOUTH.1 && SPEECH_TAIL.1 < SPEECH_AT.1);
    }

    /// Prints the staff rows for a few poses, for tools/staff_pose_check.py. Run with
    /// `cargo test dump_staff -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn dump_staff() {
        for k in [0.0, 0.3, 0.6, 1.0] {
            println!("== {k}");
            let pose = staff_pose(k);
            for r in staff_rows(&pose) {
                println!("ROW {:.2} {:.2} {:.2} {:.2} {:.4}", r.left.0, r.left.1, r.right.0, r.right.1, r.v);
            }
            let c = head_centre(&pose);
            println!("HEAD {:.1} {:.1} {:.1}", c.0, c.1, head_radius(&pose));
        }
    }

    /// Prints what the four beats look like, for tools/gold_father_check.py to draw. Run with
    /// `cargo test dump_scene -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn dump_scene() {
        let mut idle = smoker();
        idle.tick(0.0);
        let mut sass = smoker();
        sass.click(0.0);
        let mut away = smoker();
        away.click(0.0);
        away.click(1.0);
        let mut thrown = smoker();
        throw(&mut thrown, 0.0, (760.0, 790.0), (1000.0, 300.0), 0.1);
        for (name, s, at) in [("idle", &idle, 3.0), ("sass", &sass, 1.5), ("away", &away, 2.5), ("thrown", &thrown, 0.35)] {
            let v = s.view(at);
            println!("== {name}");
            let f = face(v.mood);
            for e in &f.eyes {
                println!("EYE {} {} {}", e.fill, e.edge, e.points.iter().map(|p| format!("{:.1},{:.1}", p.0, p.1)).collect::<Vec<_>>().join(" "));
            }
            for (p, r) in &f.pupils {
                println!("PUPIL {:.1} {:.1} {}", p.0, p.1, r);
            }
            for l in &f.strokes {
                println!("STROKE {} {} {}", l.width, l.alpha, l.points.iter().map(|p| format!("{:.1},{:.1}", p.0, p.1)).collect::<Vec<_>>().join(" "));
            }
            if let Some((b, t)) = v.cigarette {
                println!("CIG {:.1} {:.1} {:.1} {:.1} {}", b.0, b.1, t.0, t.1, v.ember);
            }
            if let Some(from) = v.smoke_from {
                for p in smoke(from, 12.0, v.smoke) {
                    println!("SMOKE {:.1} {:.1} {:.1} {:.3}", p.pos.0, p.pos.1, p.radius, p.alpha);
                }
            }
            for p in &v.trail {
                println!("TRAIL {:.1} {:.1} {:.1} {:.3}", p.pos.0, p.pos.1, p.radius, p.alpha);
            }
            if let Some((text, a)) = v.speech {
                println!("SPEECH {a:.2} {text}");
            }
        }
    }
}
