//! Vendored Phosphor glyphs for built-in widget affordances.
//!
//! These definitions are byte-for-byte copies of the generated Phosphor bold
//! definitions in `crates/stern-icons-phosphor` (`@phosphor-icons/core 2.1.1`,
//! pinned and checksum-verified there):
//!
//! - `stern_icons_phosphor::bold::CARET_DOWN`
//!   (`src/generated/bold/shard_031.rs`, `STERN_PHOSPHOR_2_1_1:bold:caret-down`)
//! - `stern_icons_phosphor::bold::CARET_UP`
//!   (`src/generated/bold/shard_031.rs`, `STERN_PHOSPHOR_2_1_1:bold:caret-up`)
//! - `stern_icons_phosphor::bold::CHECK`
//!   (`src/generated/bold/shard_037.rs`, `STERN_PHOSPHOR_2_1_1:bold:check`)
//!
//! They are vendored instead of adding `stern-icons-phosphor` as a normal
//! dependency because that crate carries the complete generated icon set
//! (~1500 definitions × 6 weights, tens of megabytes of source) and widgets
//! need exactly these three glyphs. `IconId`s are kept identical to the
//! upstream definitions so both resolve as the same icon to renderer-side
//! caches, and a unit test in `components/tests` pins each vendored graphic
//! against the `stern-icons-phosphor` dev-dependency to catch drift.

// Path data is copied verbatim from the generated phosphor shards, which
// carry the same lint expectation.
#![allow(clippy::unreadable_literal)]

use stern_core::{
    FillRule, IconGraphic, IconId, IconLayer, IconPath, PathElement, Point, Rect, StaticIcon,
};

static CARET_DOWN_ELEMENTS_0: [PathElement; 14] = [
    PathElement::MoveTo(Point::new(216.49_f32, 104.49_f32)),
    PathElement::LineTo(Point::new(136.49_f32, 184.49_f32)),
    PathElement::CubicTo {
        ctrl1: Point::new(134.23837_f32, 186.74945_f32),
        ctrl2: Point::new(131.17982_f32, 188.01947_f32),
        to: Point::new(127.99_f32, 188.01947_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(124.80018_f32, 188.01947_f32),
        ctrl2: Point::new(121.74162_f32, 186.74945_f32),
        to: Point::new(119.49_f32, 184.49_f32),
    },
    PathElement::LineTo(Point::new(39.49_f32, 104.49_f32)),
    PathElement::CubicTo {
        ctrl1: Point::new(36.453243_f32, 101.45324_f32),
        ctrl2: Point::new(35.267254_f32, 97.02707_f32),
        to: Point::new(36.378784_f32, 92.878784_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(37.490314_f32, 88.7305_f32),
        ctrl2: Point::new(40.730495_f32, 85.49032_f32),
        to: Point::new(44.878784_f32, 84.378784_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(49.027073_f32, 83.26725_f32),
        ctrl2: Point::new(53.453243_f32, 84.45324_f32),
        to: Point::new(56.49_f32, 87.49_f32),
    },
    PathElement::LineTo(Point::new(128.0_f32, 159.0_f32)),
    PathElement::LineTo(Point::new(199.51_f32, 87.48_f32)),
    PathElement::CubicTo {
        ctrl1: Point::new(202.54675_f32, 84.443245_f32),
        ctrl2: Point::new(206.97293_f32, 83.257256_f32),
        to: Point::new(211.12122_f32, 84.36878_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(215.2695_f32, 85.480316_f32),
        ctrl2: Point::new(218.50969_f32, 88.7205_f32),
        to: Point::new(219.62122_f32, 92.86878_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(220.73274_f32, 97.017075_f32),
        ctrl2: Point::new(219.54675_f32, 101.443245_f32),
        to: Point::new(216.51_f32, 104.48_f32),
    },
    PathElement::Close,
];
static CARET_DOWN_PATH_0: [IconPath; 1] = [IconPath::new(
    &CARET_DOWN_ELEMENTS_0,
    Some(FillRule::NonZero),
    None,
    1.0_f32,
)];
static CARET_DOWN_LAYERS: [IconLayer; 1] = [IconLayer::new(&CARET_DOWN_PATH_0, 1.0_f32)];
static CARET_DOWN_GRAPHIC: IconGraphic = IconGraphic::new(
    Rect::new(0.0_f32, 0.0_f32, 256.0_f32, 256.0_f32),
    &CARET_DOWN_LAYERS,
);
/// Phosphor bold `caret-down`, vendored for the select-trigger disclosure.
pub(crate) const CARET_DOWN_ICON: StaticIcon =
    StaticIcon::new(IconId::from_raw(0x62991a44406198db), &CARET_DOWN_GRAPHIC);

static CARET_UP_ELEMENTS_0: [PathElement; 15] = [
    PathElement::MoveTo(Point::new(216.49_f32, 168.49_f32)),
    PathElement::CubicTo {
        ctrl1: Point::new(214.23837_f32, 170.74945_f32),
        ctrl2: Point::new(211.17982_f32, 172.01947_f32),
        to: Point::new(207.99_f32, 172.01947_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(204.80019_f32, 172.01947_f32),
        ctrl2: Point::new(201.74162_f32, 170.74945_f32),
        to: Point::new(199.49_f32, 168.49_f32),
    },
    PathElement::LineTo(Point::new(128.0_f32, 97.0_f32)),
    PathElement::LineTo(Point::new(56.49_f32, 168.49_f32)),
    PathElement::CubicTo {
        ctrl1: Point::new(53.453243_f32, 171.52676_f32),
        ctrl2: Point::new(49.027073_f32, 172.71275_f32),
        to: Point::new(44.878784_f32, 171.60121_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(40.730495_f32, 170.48969_f32),
        ctrl2: Point::new(37.490314_f32, 167.2495_f32),
        to: Point::new(36.378784_f32, 163.10121_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(35.267254_f32, 158.95293_f32),
        ctrl2: Point::new(36.453243_f32, 154.52676_f32),
        to: Point::new(39.49_f32, 151.49_f32),
    },
    PathElement::LineTo(Point::new(119.49_f32, 71.49_f32)),
    PathElement::CubicTo {
        ctrl1: Point::new(121.74162_f32, 69.230545_f32),
        ctrl2: Point::new(124.80018_f32, 67.96054_f32),
        to: Point::new(127.99_f32, 67.96054_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(131.17982_f32, 67.96054_f32),
        ctrl2: Point::new(134.23837_f32, 69.230545_f32),
        to: Point::new(136.49_f32, 71.49_f32),
    },
    PathElement::LineTo(Point::new(216.49_f32, 151.49_f32)),
    PathElement::CubicTo {
        ctrl1: Point::new(218.74945_f32, 153.74162_f32),
        ctrl2: Point::new(220.01947_f32, 156.80019_f32),
        to: Point::new(220.01947_f32, 159.99_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(220.01947_f32, 163.17982_f32),
        ctrl2: Point::new(218.74945_f32, 166.23837_f32),
        to: Point::new(216.49_f32, 168.49_f32),
    },
    PathElement::Close,
];
static CARET_UP_PATH_0: [IconPath; 1] = [IconPath::new(
    &CARET_UP_ELEMENTS_0,
    Some(FillRule::NonZero),
    None,
    1.0_f32,
)];
static CARET_UP_LAYERS: [IconLayer; 1] = [IconLayer::new(&CARET_UP_PATH_0, 1.0_f32)];
static CARET_UP_GRAPHIC: IconGraphic = IconGraphic::new(
    Rect::new(0.0_f32, 0.0_f32, 256.0_f32, 256.0_f32),
    &CARET_UP_LAYERS,
);
/// Phosphor bold `caret-up`, vendored for the open select-trigger disclosure.
pub(crate) const CARET_UP_ICON: StaticIcon =
    StaticIcon::new(IconId::from_raw(0x4766100cde277010), &CARET_UP_GRAPHIC);

static CHECK_ELEMENTS_0: [PathElement; 14] = [
    PathElement::MoveTo(Point::new(232.49_f32, 80.49_f32)),
    PathElement::LineTo(Point::new(104.49_f32, 208.49_f32)),
    PathElement::CubicTo {
        ctrl1: Point::new(102.23838_f32, 210.74945_f32),
        ctrl2: Point::new(99.17982_f32, 212.01947_f32),
        to: Point::new(95.99_f32, 212.01947_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(92.80018_f32, 212.01947_f32),
        ctrl2: Point::new(89.74162_f32, 210.74945_f32),
        to: Point::new(87.49_f32, 208.49_f32),
    },
    PathElement::LineTo(Point::new(31.49_f32, 152.49_f32)),
    PathElement::CubicTo {
        ctrl1: Point::new(28.453243_f32, 149.45325_f32),
        ctrl2: Point::new(27.267254_f32, 145.02707_f32),
        to: Point::new(28.378784_f32, 140.87878_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(29.490314_f32, 136.7305_f32),
        ctrl2: Point::new(32.730495_f32, 133.49031_f32),
        to: Point::new(36.878784_f32, 132.37878_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(41.027073_f32, 131.26726_f32),
        ctrl2: Point::new(45.453243_f32, 132.45325_f32),
        to: Point::new(48.49_f32, 135.49_f32),
    },
    PathElement::LineTo(Point::new(96.0_f32, 183.0_f32)),
    PathElement::LineTo(Point::new(215.51_f32, 63.51_f32)),
    PathElement::CubicTo {
        ctrl1: Point::new(218.54675_f32, 60.473244_f32),
        ctrl2: Point::new(222.97293_f32, 59.287254_f32),
        to: Point::new(227.12122_f32, 60.398785_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(231.2695_f32, 61.510315_f32),
        ctrl2: Point::new(234.50969_f32, 64.750496_f32),
        to: Point::new(235.62122_f32, 68.89878_f32),
    },
    PathElement::CubicTo {
        ctrl1: Point::new(236.73274_f32, 73.04707_f32),
        ctrl2: Point::new(235.54675_f32, 77.47324_f32),
        to: Point::new(232.51_f32, 80.51_f32),
    },
    PathElement::Close,
];
static CHECK_PATH_0: [IconPath; 1] = [IconPath::new(
    &CHECK_ELEMENTS_0,
    Some(FillRule::NonZero),
    None,
    1.0_f32,
)];
static CHECK_LAYERS: [IconLayer; 1] = [IconLayer::new(&CHECK_PATH_0, 1.0_f32)];
static CHECK_GRAPHIC: IconGraphic = IconGraphic::new(
    Rect::new(0.0_f32, 0.0_f32, 256.0_f32, 256.0_f32),
    &CHECK_LAYERS,
);
/// Phosphor bold `check`, vendored for the checked checkbox glyph.
pub(crate) const CHECK_ICON: StaticIcon =
    StaticIcon::new(IconId::from_raw(0x52d5d7e94e5e8b2d), &CHECK_GRAPHIC);
