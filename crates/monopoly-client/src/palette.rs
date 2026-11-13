use monopoly_core::board::District;

// ============================================================================
// Core UI Surfaces & Backgrounds (Warm Neutral Ramp / Template-aware CSS vars)
// ============================================================================
pub const BG_PRIMARY: &str = "var(--bg-color, #161513)";
pub const BG_SECONDARY: &str = "var(--bg-deepest, #12110f)";
pub const SURFACE_PANEL: &str = "var(--surface-color, #1c1a18)";
pub const SURFACE_HOVER: &str = "var(--surface-hover, #262421)";
pub const BORDER_DEFAULT: &str = "var(--border, rgba(239, 233, 222, 0.08))";
pub const BORDER_SUBTLE: &str = "var(--border-strong, rgba(239, 233, 222, 0.14))";

// ============================================================================
// Typography Colors (Three Strict Roles)
// ============================================================================
pub const TEXT_PRIMARY: &str = "var(--text-main, #efe9de)";
pub const TEXT_SECONDARY: &str = "var(--text-sub, #aaa398)";
pub const TEXT_MUTED: &str = "var(--text-muted, #8a847a)";
pub const TEXT_SUBTLE: &str = "var(--text-muted, #8a847a)";
pub const TEXT_ON_ACCENT: &str = "var(--text-on-accent, #efe9de)";

// ============================================================================
// Brand & Semantic Status Colors
// ============================================================================
pub const ACCENT_PRIMARY: &str = "var(--accent, #d9383a)";
pub const ACCENT_HOVER: &str = "var(--accent-hover, #b82d2f)";
pub const SECONDARY_PRIMARY: &str = "var(--surface-hover, #262421)";
pub const SECONDARY_HOVER: &str = "var(--secondary-hover, #302d29)";

pub const STATUS_SUCCESS: &str = "var(--success, #4cc38a)";
pub const STATUS_WARNING: &str = "var(--warning, #d49b35)";
pub const STATUS_DANGER: &str = "var(--danger, #e5484d)";

// ============================================================================
// Board & Tile Surface Colors (Prussian Blue + Mint Celadon + Warm Ivory)
// ============================================================================
pub const BOARD_OUTER_BG: &str = "var(--board-outer-bg, #1b334f)";
pub const BOARD_CENTER_BG: &str = "var(--board-center-bg, #dcefe8)";
pub const BOARD_CENTER_BORDER: &str = "var(--board-center-border, #21354d)";
pub const TILE_BG: &str = "var(--tile-bg, #fbf7ee)";
pub const TILE_CORNER_BG: &str = "var(--tile-corner-bg, #f2ebdc)";
pub const TILE_SELECTED_STROKE: &str = "var(--tile-selected-stroke, #d9383a)";
pub const TILE_STROKE: &str = "var(--tile-stroke, #21354d)";
pub const TILE_TEXT_PRIMARY: &str = "var(--tile-text-primary, #18283a)";
pub const TILE_TEXT_MUTED: &str = "var(--tile-text-muted, #4a5d73)";

// ============================================================================
// Special Tile Accent Colors
// ============================================================================
pub const TILE_STATION: &str = "var(--tile-station, #6e6a63)";
pub const TILE_UTILITY: &str = "var(--tile-utility, #5b7a8c)";
pub const TILE_TAX: &str = "var(--tile-tax, #b8474b)";
pub const TILE_CHANCE: &str = "var(--tile-chance, #c46b45)";
pub const TILE_CHANCE_BG: &str = "var(--tile-chance-bg, #fbd7b5)";
pub const TILE_CHANCE_INNER_BG: &str = "var(--tile-chance-inner-bg, #f6c18e)";
pub const TILE_CHANCE_BORDER: &str = "var(--tile-chance-border, #c85a28)";
pub const TILE_CHEST: &str = "var(--tile-chest, #498774)";
pub const TILE_CHEST_BG: &str = "var(--tile-chest-bg, #faecb8)";
pub const TILE_CHEST_INNER_BG: &str = "var(--tile-chest-inner-bg, #f4dd88)";
pub const TILE_CHEST_BORDER: &str = "var(--tile-chest-border, #b5851d)";
pub const TILE_CORNER_ACCENT: &str = "var(--tile-corner-accent, rgba(28, 26, 24, 0.16))";

// ============================================================================
// Player Token & Character Palette (Monogram Codes, Zero Emoji)
// ============================================================================
pub const PLAYER_COLORS: [&str; 4] = [
    "#e5484d", // Seat 1: Vermilion
    "#4a90b8", // Seat 2: Cerulean
    "#4cc38a", // Seat 3: Jade
    "#d49b35", // Seat 4: Brass
];

pub const PLAYER_AVATARS: [&str; 4] = ["T1", "T2", "T3", "T4"];

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
