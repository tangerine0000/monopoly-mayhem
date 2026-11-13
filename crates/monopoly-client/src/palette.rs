use monopoly_core::board::District;

// ============================================================================
// Core UI Surfaces & Backgrounds (Warm Neutral Ramp)
// ============================================================================
pub const BG_PRIMARY: &str = "#161513";
pub const BG_SECONDARY: &str = "#12110f";
pub const SURFACE_PANEL: &str = "#1c1a18";
pub const SURFACE_HOVER: &str = "#262421";
pub const BORDER_DEFAULT: &str = "rgba(239, 233, 222, 0.08)";
pub const BORDER_SUBTLE: &str = "rgba(239, 233, 222, 0.14)";

// ============================================================================
// Typography Colors (Three Strict Roles)
// ============================================================================
pub const TEXT_PRIMARY: &str = "#efe9de";
pub const TEXT_SECONDARY: &str = "#aaa398";
pub const TEXT_MUTED: &str = "#8a847a";
pub const TEXT_SUBTLE: &str = "#8a847a";
pub const TEXT_ON_ACCENT: &str = "#efe9de";

// ============================================================================
// Brand & Semantic Status Colors
// ============================================================================
pub const ACCENT_PRIMARY: &str = "#d9383a";
pub const ACCENT_HOVER: &str = "#b82d2f";
pub const SECONDARY_PRIMARY: &str = "#262421";
pub const SECONDARY_HOVER: &str = "#302d29";

pub const STATUS_SUCCESS: &str = "#4cc38a";
pub const STATUS_WARNING: &str = "#d49b35";
pub const STATUS_DANGER: &str = "#e5484d";

// ============================================================================
// Board & Tile Surface Colors
// ============================================================================
pub const BOARD_OUTER_BG: &str = "#1c1a18";
pub const BOARD_CENTER_BG: &str = "#141311";
pub const BOARD_CENTER_BORDER: &str = "rgba(239, 233, 222, 0.10)";
pub const TILE_BG: &str = "#1c1a18";
pub const TILE_CORNER_BG: &str = "#22201d";
pub const TILE_SELECTED_STROKE: &str = "#d49b35";
pub const TILE_STROKE: &str = "rgba(239, 233, 222, 0.10)";

// ============================================================================
// Special Tile Accent Colors
// ============================================================================
pub const TILE_STATION: &str = "#6e6a63";
pub const TILE_UTILITY: &str = "#5b7a8c";
pub const TILE_TAX: &str = "#b8474b";
pub const TILE_CHANCE: &str = "#c46b45";
pub const TILE_CHEST: &str = "#498774";
pub const TILE_CORNER_ACCENT: &str = "rgba(239, 233, 222, 0.12)";

// ============================================================================
// Player Token & Character Palette (Monogram Codes, Zero Emoji)
// ============================================================================
pub const PLAYER_COLORS: [&str; 4] = [
    "#e5484d", // Seat 1: Vermilion
    "#4a90b8", // Seat 2: Cerulean
    "#4cc38a", // Seat 3: Jade
    "#d49b35", // Seat 4: Brass
];

pub const PLAYER_AVATARS: [&str; 4] = [
    "T1",
    "T2",
    "T3",
    "T4",
];

// ============================================================================
// Saigon District Color Bands
// ============================================================================
pub const DISTRICT_4_BROWN: &str = "#8c583a";
pub const DISTRICT_8_LIGHT_BLUE: &str = "#4a90b8";
pub const PHU_NHUAN_PINK: &str = "#c46387";
pub const TAN_BINH_ORANGE: &str = "#d4763b";
pub const DISTRICT_5_RED: &str = "#c94444";
pub const DISTRICT_3_YELLOW: &str = "#c79a36";
pub const DISTRICT_2_GREEN: &str = "#469468";
pub const DISTRICT_1_DARK_BLUE: &str = "#3b66a8";

pub fn get_district_color(district: District) -> &'static str {
    match district {
        District::District4 => DISTRICT_4_BROWN,
        District::District8 => DISTRICT_8_LIGHT_BLUE,
        District::PhuNhuan => PHU_NHUAN_PINK,
        District::TanBinh => TAN_BINH_ORANGE,
        District::District5 => DISTRICT_5_RED,
        District::District3 => DISTRICT_3_YELLOW,
        District::District2 => DISTRICT_2_GREEN,
        District::District1 => DISTRICT_1_DARK_BLUE,
    }
}

pub fn get_district_label(district: District) -> &'static str {
    match district {
        District::District4 => "Quận 4",
        District::District8 => "Quận 8",
        District::PhuNhuan => "Phú Nhuận",
        District::TanBinh => "Tân Bình",
        District::District5 => "Quận 5 · Chợ Lớn",
        District::District3 => "Quận 3",
        District::District2 => "Quận 2 · Thảo Điền",
        District::District1 => "Quận 1 · Bến Nghé",
    }
}
