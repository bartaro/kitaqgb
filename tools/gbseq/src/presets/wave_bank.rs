//! Original single-cycle oscillators. Each entry specifies its own spectrum
//! or oscillator recipe; there are no name suffixes or ID-derived variations.
//! CH3 has one static cycle of 32 four-bit samples (no simulated ADSR/filter).
use std::f64::consts::TAU;

#[derive(Clone, Copy)]
enum Shape {
    // Signed partial amplitudes at harmonics 1, 2, 3, 4, 5, 7, 9, 13.
    Spectrum([i8; 8]),
    // FM carrier, modulator, depth in tenths of a radian, dry fundamental %.
    Fm(u8, u8, u8, u8),
    // Ramp breakpoint /32, drive %, octave mix %, third harmonic mix %.
    Ramp(u8, u8, i8, i8),
    // Pulse width /32, softening %, second and fifth harmonic mix %.
    Pulse(u8, u8, i8, i8),
}
use Shape::{Fm, Pulse, Ramp, Spectrum as S};

const VOICES: [(&str, Shape); 128] = [
    // Bass and low register
    ("Velvet Sub", S([100, 7, 0, 0, 0, 0, 0, 0])),
    ("Rubber Heel", Fm(1, 1, 12, 25)),
    ("Pocket Fretless", S([90, 46, -20, 6, 1, 0, 0, 0])),
    ("Boot Relay", Pulse(11, 28, -28, 9)),
    ("Cellar Stairs", Ramp(6, 140, 14, -12)),
    ("Upright Lantern", S([96, 55, 26, -9, 8, 2, 0, 0])),
    ("Underpass", Fm(2, 3, 9, 88)),
    ("Square Anchor", Pulse(16, 16, 6, 0)),
    // Plucked strings
    ("Paper Koto", S([60, 85, -38, 35, -16, 3, 0, 4])),
    ("Copper Plectrum", Ramp(3, 105, -32, 25)),
    ("Thumb Harp", S([90, -30, 14, 3, 0, 12, 0, 0])),
    ("Salt Lute", Pulse(8, 50, 42, -16)),
    ("Nylon Postcard", Ramp(13, 85, 21, 2)),
    ("Zither Spark", Fm(1, 5, 13, 55)),
    ("Shortwave Clav", Pulse(5, 58, -21, 30)),
    ("Bridge Pin", S([80, 25, -60, 0, 36, -17, 0, 10])),
    // Keys and tines
    ("Suitcase Tine", Fm(1, 2, 11, 54)),
    ("Felt Hammers", S([80, 58, -28, 11, 4, 0, 1, 0])),
    ("Reed Wurli", Ramp(10, 160, 42, -18)),
    ("Toy Upright", S([68, 24, 46, 68, -6, 8, 2, 0])),
    ("Porcelain Keys", Fm(2, 5, 7, 75)),
    ("Cinder Spinet", Pulse(7, 42, 52, 33)),
    ("Celeste Pin", S([65, 0, 94, -5, 0, 13, 29, 0])),
    ("Midnight Felt", S([100, -44, 10, 0, 0, 0, 0, 0])),
    // Pipes and reeds
    ("Bamboo Lip", S([100, 34, 8, -4, 0, 7, 0, 0])),
    ("Hollow Clarinet", S([84, 2, 74, 0, -36, 18, -7, 0])),
    ("Shawm Call", Pulse(10, 29, 12, 64)),
    ("Bottle Neck", Fm(1, 3, 5, 68)),
    ("Paper Oboe", S([32, 71, 95, 68, 30, -9, 1, 0])),
    ("Tin Whistle", S([97, 9, -3, 0, 0, 0, 12, 1])),
    ("Dusk Bassoon", Ramp(9, 108, -47, 15)),
    ("Accordion Fold", Pulse(13, 41, 58, -26)),
    // Organ registrations
    ("Chapel Bourdon", S([100, 24, 10, 42, 0, 0, 0, 0])),
    ("Carousel Vox", S([56, 84, 40, 73, -28, 0, 2, 0])),
    ("Jazz Drawbars", S([90, 95, 72, 18, 12, 3, 0, 0])),
    ("Portative", Pulse(14, 68, 45, 9)),
    ("Glass Mixture", S([35, 64, 73, 78, 50, 27, 16, 0])),
    ("Rotunda", Fm(2, 2, 9, 76)),
    ("Stopped Pipe", S([100, 0, -42, 0, 18, -8, 0, 0])),
    ("Lantern Harmonium", Ramp(12, 145, -23, 48)),
    // Brass
    ("Cornet Banner", Ramp(4, 146, 51, -19)),
    ("Muted Bugle", S([30, 60, 98, 82, -34, 12, 4, 0])),
    ("French Hornvale", S([96, 82, 48, 21, -7, 2, 0, 0])),
    ("Brass Knuckle", Fm(1, 1, 27, 34)),
    ("Pocket Trombone", Ramp(7, 178, 35, -40)),
    ("Velour Flugel", S([71, 91, 33, -13, 7, 0, 0, 0])),
    ("Parade Mute", Pulse(4, 43, 31, 49)),
    ("Tuba Stack", S([99, -80, 33, 28, 5, 0, 0, 0])),
    // Strings and bowed colours
    ("Rosin Thread", Ramp(2, 100, 18, 20)),
    ("Amber Viola", S([57, 86, -68, 41, 24, -12, 7, 0])),
    ("Cello Bridge", Ramp(5, 128, -36, 43)),
    ("Wire Ensemble", S([72, 70, 54, -49, 35, 20, -12, 6])),
    ("Silk Bow", Fm(1, 2, 7, 38)),
    ("Glass Fiddle", Fm(2, 3, 17, 44)),
    ("Wheel Fiddle", Pulse(12, 15, 58, -37)),
    ("Mute Quartet", S([88, -51, -31, 13, 4, 0, 0, 0])),
    // Solo synths
    ("Vector Lance", Fm(1, 3, 23, 30)),
    ("Needlepoint", Pulse(3, 19, 25, 12)),
    ("Ribbon Glide", Ramp(20, 117, -35, -18)),
    ("Arcade Siren", S([56, -76, 88, -24, 17, 11, 0, 0])),
    ("Honeycomb Lead", Pulse(9, 36, -63, 31)),
    ("Phase Scissor", Fm(3, 2, 19, 60)),
    ("Signal Flare", Ramp(28, 155, 58, 25)),
    ("Blue Neon", S([79, 9, 62, -44, 22, 0, 13, 0])),
    // Mallets
    ("Rosewood Block", S([85, 3, 0, 76, 0, 0, 14, 0])),
    ("Bronze Slab", Fm(1, 4, 14, 40)),
    ("Rain Marimba", S([100, 0, 5, -48, 0, 0, 5, 0])),
    ("Ice Hammer", Fm(3, 5, 10, 61)),
    ("Stone Lithophone", S([64, 0, -22, 88, 0, 18, 36, 0])),
    ("Thumb Piano", Fm(1, 6, 8, 64)),
    ("Cork Tap", Ramp(17, 187, 43, 26)),
    ("Bamboo Slit", Pulse(6, 75, -17, -43)),
    // Bells and resonators
    ("Temple Bronze", S([68, 11, 80, 0, -55, 22, 0, 13])),
    ("Orbit Chime", Fm(2, 7, 13, 55)),
    ("Silver Spoon", S([46, 80, 0, 0, -44, 0, 67, 0])),
    ("Harbor Buoy", Fm(1, 4, 7, 85)),
    ("Glass Rim", S([98, 0, 0, 0, 0, 9, 0, 20])),
    ("Bicycle Bell", Fm(4, 7, 10, 36)),
    ("Gong Halo", S([38, 83, -78, 20, 46, 0, -25, 8])),
    ("Star Anvil", Fm(3, 8, 19, 56)),
    // Formants (fixed resonant bands)
    ("Ochre Ah", S([45, 76, 90, 24, 0, 29, 7, 0])),
    ("Velvet Oo", S([84, 95, 14, -18, 0, 0, 0, 0])),
    ("Lime Ee", S([54, 25, 0, 6, 74, 90, -14, 0])),
    ("Tin Talkbox", Fm(2, 1, 29, 40)),
    ("Cathedral Oh", S([58, 96, -48, 9, 22, 0, 6, 0])),
    ("Nasal Mask", Pulse(6, 31, -75, 81)),
    ("Radio Choir", S([31, 70, 31, -58, 84, -20, 2, 0])),
    ("Vowel Prism", Fm(4, 3, 11, 65)),
    // Soft sustained colours
    ("Polar Linen", Ramp(16, 85, 6, -6)),
    ("Window Frost", S([89, -8, 16, 0, 24, 0, -5, 0])),
    ("Low Aurora", Fm(1, 2, 5, 85)),
    ("Moth Wing", Pulse(14, 86, 21, -7)),
    ("Cloud Chamber", S([78, 41, 0, -40, 8, 0, 3, 0])),
    ("Night Ferry", Ramp(21, 93, 44, 12)),
    ("Blue Hour", S([93, 0, 32, -20, 0, 0, 0, 7])),
    ("Lantern Fog", Fm(2, 4, 5, 91)),
    // Hard sync / digital colours
    ("Sync Ripper", Ramp(2, 244, -63, 57)),
    ("Circuit Teeth", Pulse(5, 8, 80, -74)),
    ("Folded Mirror", Fm(1, 2, 38, 18)),
    ("Bit Staircase", S([55, -23, 40, 0, -54, 0, 49, -30])),
    ("Pixel Rasp", Pulse(3, 35, -60, -44)),
    ("Rotor Blade", Fm(3, 4, 23, 40)),
    ("Sawtooth Loom", Ramp(30, 133, -72, -42)),
    ("Alias Alley", Fm(2, 9, 28, 35)),
    // Rhythmic colours
    ("Spring Drum", Fm(1, 3, 17, 71)),
    ("Clave Chip", S([60, 0, 88, 0, -8, 0, 0, 32])),
    ("Tin Can", Fm(3, 7, 16, 31)),
    ("Frame Drum", S([92, -63, 3, 52, 12, 0, 0, 0])),
    ("Sand Grain", S([22, 31, -54, 19, 60, -65, 59, -54])),
    ("Slap Wire", Ramp(5, 225, 81, 6)),
    ("Tube Knock", Pulse(12, 63, -39, 57)),
    ("Coin Drop", Fm(5, 4, 14, 52)),
    // Experimental
    ("Vacuum Diode", Ramp(19, 235, -75, 46)),
    ("Magnetic Ink", Fm(2, 3, 37, 39)),
    ("Broken Raster", S([40, 0, -69, 72, 0, 55, -48, 38])),
    ("Signal Buoy", Pulse(2, 66, 71, -32)),
    ("Hollow Satellite", Fm(1, 7, 21, 67)),
    ("Prism Scanner", S([33, 80, -34, 0, 53, 74, 0, -51])),
    ("Relay Coil", Pulse(15, 24, -84, 67)),
    ("Geiger Glass", Fm(4, 9, 23, 45)),
    // Hybrids
    ("Ember Harpsichord", Ramp(3, 178, -14, -56)),
    ("Moonlit Reed", S([60, 36, 62, -52, 16, 0, 8, 0])),
    ("Reef Organ", S([68, 86, 5, 37, -65, 0, 0, 0])),
    ("Kite String", Fm(2, 5, 21, 49)),
    ("Diving Beacon", Pulse(10, 72, 69, 42)),
    ("Porcelain Horn", S([74, -82, 26, 0, 0, 0, 44, 0])),
    ("Twilight Alloy", Fm(3, 6, 10, 83)),
    ("Afterglow", S([96, 21, 0, -18, 0, 0, 0, 5])),
];

pub(super) fn voice(id: u8) -> (&'static str, [u8; 16]) {
    let (name, shape) = VOICES[usize::from(id & 127)];
    let mut cycle = [0.; 32];
    for (i, v) in cycle.iter_mut().enumerate() {
        let x = (i as f64 + 0.5) / 32.;
        let t = x * TAU;
        *v = match shape {
            S(weights) => [1., 2., 3., 4., 5., 7., 9., 13.]
                .into_iter()
                .zip(weights)
                .map(|(h, w)| (t * h).sin() * f64::from(w))
                .sum(),
            Fm(c, m, depth, dry) => {
                (t * f64::from(c) + (t * f64::from(m)).sin() * f64::from(depth) / 10.).sin()
                    + t.sin() * f64::from(dry) / 100.
            }
            Ramp(k, drive, oct, third) => {
                let knee = f64::from(k) / 32.;
                let ramp = if x < knee {
                    2. * x / knee - 1.
                } else {
                    1. - 2. * (x - knee) / (1. - knee)
                };
                (ramp * f64::from(drive) / 100.).tanh()
                    + (2. * t).sin() * f64::from(oct) / 100.
                    + (3. * t).sin() * f64::from(third) / 100.
            }
            Pulse(k, soft, oct, fifth) => {
                let edge = if x < f64::from(k) / 32. { 1. } else { -1. };
                let mix = f64::from(soft) / 100.;
                edge * (1. - mix)
                    + t.sin() * mix
                    + (2. * t).sin() * f64::from(oct) / 100.
                    + (5. * t).sin() * f64::from(fifth) / 100.
            }
        };
    }
    let mean = cycle.iter().sum::<f64>() / 32.;
    let peak = cycle.iter().map(|v| (v - mean).abs()).fold(0., f64::max);
    let nibble = |i: usize| {
        (7.5 + 7.5 * (cycle[i] - mean) / peak)
            .round()
            .clamp(0., 15.) as u8
    };
    (
        name,
        std::array::from_fn(|i| (nibble(2 * i) << 4) | nibble(2 * i + 1)),
    )
}
