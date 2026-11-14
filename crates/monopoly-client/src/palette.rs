//! Bảng màu, icon và định dạng hiển thị cho hướng visual "Gạch bông & Bảng hiệu".
//! Màu dùng trong CSS được khai báo token trong `index.html`; file này chỉ giữ các
//! giá trị cần tính trong Rust (màu quận, màu người chơi, icon SVG).

use monopoly_core::board::{District, Tile, TileKind};
use monopoly_core::money::Money;

// ============================================================================
// Màu quận (biển hiệu) — bão hòa vừa để đọc tốt trên nền ngà
// ============================================================================
pub const DISTRICT_4_BROWN: &str = "#8a5a3c";
pub const DISTRICT_8_LIGHT_BLUE: &str = "#3f97c4";
pub const PHU_NHUAN_PINK: &str = "#c9548a";
pub const TAN_BINH_ORANGE: &str = "#e07a2e";
pub const DISTRICT_5_RED: &str = "#d23f3f";
pub const DISTRICT_3_YELLOW: &str = "#e2b028";
pub const DISTRICT_2_GREEN: &str = "#3b9a5e";
pub const DISTRICT_1_DARK_BLUE: &str = "#24477e";

pub const TILE_STATION: &str = "#6e6a63";
pub const TILE_UTILITY: &str = "#2b7c8c";
pub const TILE_TAX: &str = "#c8372d";
pub const TILE_CHANCE: &str = "#e3a72f";
pub const TILE_CHEST: &str = "#c8372d";
pub const TILE_CORNER: &str = "#1f6d5f";

pub const ALL_DISTRICTS: [District; 8] = [
    District::District4,
    District::District8,
    District::PhuNhuan,
    District::TanBinh,
    District::District5,
    District::District3,
    District::District2,
    District::District1,
];

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
        District::District5 => "Quận 5",
        District::District3 => "Quận 3",
        District::District2 => "Quận 2",
        District::District1 => "Quận 1",
    }
}

/// Màu nhận diện của một ô (dải biển hiệu / đầu thẻ khế ước).
pub fn tile_color(tile: &Tile) -> &'static str {
    match &tile.kind {
        TileKind::Property(p) => get_district_color(p.district),
        TileKind::Station(_) => TILE_STATION,
        TileKind::Utility(_) => TILE_UTILITY,
        TileKind::Tax(_) => TILE_TAX,
        TileKind::Chance => TILE_CHANCE,
        TileKind::CommunityChest => TILE_CHEST,
        _ => TILE_CORNER,
    }
}

/// Nhãn nhóm của ô, viết thường, ngắn.
pub fn tile_group_label(tile: &Tile) -> &'static str {
    match &tile.kind {
        TileKind::Property(p) => get_district_label(p.district),
        TileKind::Station(_) => "Bến xe & nhà ga",
        TileKind::Utility(_) => "Công trình công cộng",
        TileKind::Tax(_) => "Thuế",
        TileKind::Chance => "Cơ hội",
        TileKind::CommunityChest => "Khí vận",
        TileKind::Go => "Khởi hành",
        TileKind::Jail => "Tạm giam",
        TileKind::FreeParking => "Nghỉ chân",
        TileKind::GoToJail => "Vào tạm giam",
    }
}

/// Tên ngắn hiển thị trên ô cờ (tên đầy đủ vẫn dùng trong thẻ khế ước).
pub fn tile_short_name(tile: &Tile) -> String {
    match &tile.kind {
        // "Xuân Thủy (Thảo Điền)" -> "Xuân Thủy"
        TileKind::Property(_) => tile
            .name
            .split(" (")
            .next()
            .unwrap_or(&tile.name)
            .to_string(),
        TileKind::Station(_) => tile
            .name
            .replace("Bến Xe ", "BX ")
            .replace("Tuyến Metro Số 1", "Metro số 1"),
        TileKind::Utility(_) => {
            if tile.name.contains("Nước") {
                "Cấp nước".to_string()
            } else {
                "Điện lực".to_string()
            }
        }
        TileKind::Tax(_) => {
            if tile.name.contains("Xa Xỉ") {
                "Thuế xa xỉ".to_string()
            } else {
                "Thuế thu nhập".to_string()
            }
        }
        TileKind::Chance => "Cơ hội".to_string(),
        TileKind::CommunityChest => "Khí vận".to_string(),
        TileKind::Go => "Khởi hành".to_string(),
        TileKind::Jail => "Tạm giam".to_string(),
        TileKind::FreeParking => "Bãi đỗ xe".to_string(),
        TileKind::GoToJail => "Vào tạm giam".to_string(),
    }
}

// ============================================================================
// Người chơi: màu + phương tiện (thay cho T1..T4)
// ============================================================================
pub const PLAYER_COLORS: [&str; 4] = ["#d6453b", "#2f7fd0", "#2f9b62", "#8a55c9"];
pub const PLAYER_VEHICLE_NAMES: [&str; 4] = ["Xe Cub", "Xích lô", "Vespa", "Xe buýt"];

pub fn player_color(seat: usize) -> &'static str {
    PLAYER_COLORS[seat % 4]
}

pub fn player_vehicle_svg(seat: usize) -> &'static str {
    match seat % 4 {
        0 => VEHICLE_CUB,
        1 => VEHICLE_XICHLO,
        2 => VEHICLE_VESPA,
        _ => VEHICLE_BUS,
    }
}

const VEHICLE_CUB: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="6" cy="16" r="3"/><circle cx="18" cy="16" r="3"/><path d="M6 16h4l3-6h3l2 6M13 10l-1-3h-2"/></svg>"#;
const VEHICLE_XICHLO: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="5" cy="17" r="2.5"/><circle cx="12" cy="17" r="2.5"/><circle cx="19" cy="17" r="2.5"/><path d="M3 8h8v5H3zM11 13l8 4M16 9l3 8M15 9h3"/></svg>"#;
const VEHICLE_VESPA: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="6" cy="17" r="2.5"/><circle cx="18" cy="17" r="2.5"/><path d="M4 14c0-3 3-5 7-5h2l3 8H8.5M15 9l-1-4h3"/></svg>"#;
const VEHICLE_BUS: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><rect x="3" y="5" width="18" height="11" rx="2.5"/><path d="M3 10h18"/><circle cx="7.5" cy="18" r="1.8"/><circle cx="16.5" cy="18" r="1.8"/></svg>"#;

// ============================================================================
// Icon ô đặc biệt
// ============================================================================
pub fn tile_icon_svg(tile: &Tile) -> Option<&'static str> {
    Some(match &tile.kind {
        TileKind::Property(_) => return None,
        TileKind::Go => ICON_GO,
        TileKind::CommunityChest => ICON_CHEST,
        TileKind::Chance => ICON_CHANCE,
        TileKind::Tax(_) => ICON_TAX,
        TileKind::Station(_) => {
            if tile.name.contains("Bến Xe") {
                ICON_BUS
            } else {
                ICON_TRAIN
            }
        }
        TileKind::Utility(_) => {
            if tile.name.contains("Nước") {
                ICON_WATER
            } else {
                ICON_BOLT
            }
        }
        TileKind::Jail => ICON_JAIL,
        TileKind::FreeParking => ICON_PARK,
        TileKind::GoToJail => ICON_GOJAIL,
    })
}

pub const ICON_GO: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 12H5M11 6l-6 6 6 6"/></svg>"#;
pub const ICON_CHEST: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke-width="1.8" stroke-linejoin="round"><rect x="5" y="3" width="14" height="18" rx="2" fill="#c8372d" stroke="#c8372d"/><circle cx="12" cy="10" r="3.2" stroke="#f2c14e" stroke-width="1.6"/><path d="M8 16h8" stroke="#f2c14e"/></svg>"##;
pub const ICON_CHANCE: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke-width="1.8" stroke-linejoin="round"><path d="M3 7a2 2 0 0 0 2-2h14a2 2 0 0 0 2 2v10a2 2 0 0 0-2 2H5a2 2 0 0 0-2-2z" fill="#f2c14e" stroke="#b5801c"/><path d="M10 10a2 2 0 1 1 2.8 1.8c-.6.3-.8.7-.8 1.2M12 15.5v.2" stroke="#2a1712" stroke-width="2" stroke-linecap="round"/></svg>"##;
pub const ICON_TAX: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><path d="M6 3h12v18l-3-2-3 2-3-2-3 2z"/><path d="M9 8h6M9 12h6M9 16h3"/></svg>"#;
pub const ICON_BUS: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><rect x="4" y="4" width="16" height="13" rx="3"/><path d="M4 11h16M7 20v-3M17 20v-3"/><circle cx="8" cy="14" r=".8" fill="currentColor"/><circle cx="16" cy="14" r=".8" fill="currentColor"/></svg>"#;
pub const ICON_TRAIN: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><rect x="6" y="3" width="12" height="14" rx="4"/><path d="M6 10h12M9 21l2-4M15 21l-2-4"/><circle cx="9.5" cy="13.5" r=".9" fill="currentColor"/><circle cx="14.5" cy="13.5" r=".9" fill="currentColor"/></svg>"#;
pub const ICON_WATER: &str = r##"<svg viewBox="0 0 24 24" fill="#2b7c8c22" stroke="#2b7c8c" stroke-width="1.8"><path d="M12 3s6 7 6 11a6 6 0 0 1-12 0c0-4 6-11 6-11z"/></svg>"##;
pub const ICON_BOLT: &str = r##"<svg viewBox="0 0 24 24" fill="#e3a72f" stroke="#b5801c" stroke-width="1.4" stroke-linejoin="round"><path d="M13 2 4 14h7l-1 8 9-12h-7z"/></svg>"##;
pub const ICON_JAIL: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="4" y="4" width="16" height="16" rx="2"/><path d="M8.5 4v16M12 4v16M15.5 4v16"/></svg>"#;
pub const ICON_PARK: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M5 9h11v5a5 5 0 0 1-5 5h-1a5 5 0 0 1-5-5z"/><path d="M16 11h1.5a2.5 2.5 0 0 1 0 5H16M8 3v3M11 3v3M14 3v3"/></svg>"#;
pub const ICON_GOJAIL: &str = r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><path d="M6 18a6 6 0 0 1 12 0z" fill="#c8372d33" stroke="#c8372d"/><path d="M12 4v4M5 7l2 2M19 7l-2 2M4 21h16"/></svg>"##;
pub const ICON_STAR: &str = r##"<svg viewBox="0 0 24 24" fill="#f2c14e" stroke="#b5801c" stroke-width="1.5" stroke-linejoin="round"><path d="M12 2l3 6.5 7 .8-5.2 4.8 1.5 7L12 17.6 5.7 21l1.5-7L2 9.3l7-.8z"/></svg>"##;
pub const ICON_INFO: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M12 8v5M12 16v.2"/></svg>"#;
pub const ICON_CLOSE: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18"/></svg>"#;
pub const ICON_OFFLINE: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 3l18 18M12 20h.01M8.5 16.5a5 5 0 0 1 7 0"/></svg>"#;
pub const ICON_BARS: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="4" y="4" width="16" height="16" rx="2"/><path d="M9 4v16M15 4v16"/></svg>"#;
pub const ICON_DICE: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><rect x="4" y="4" width="16" height="16" rx="3"/><circle cx="9" cy="9" r="1" fill="currentColor"/><circle cx="15" cy="15" r="1" fill="currentColor"/></svg>"#;

// ============================================================================
// Định dạng tiền
// ============================================================================

/// Dạng ngắn cho chỗ hẹp: 2.200.000 đ -> "2,2tr", 600.000 đ -> "600k".
pub fn fmt_short(m: Money) -> String {
    let vnd = m.as_vnd();
    let sign = if vnd < 0 { "−" } else { "" };
    let abs = vnd.unsigned_abs();
    if abs >= 1_000_000 {
        let tenths = (abs + 50_000) / 100_000; // làm tròn 1 chữ số thập phân
        let whole = tenths / 10;
        let frac = tenths % 10;
        if frac == 0 {
            format!("{sign}{whole}tr")
        } else {
            format!("{sign}{whole},{frac}tr")
        }
    } else {
        format!("{sign}{}k", abs / 1_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monopoly_core::board::get_board;

    #[test]
    fn fmt_short_rounds_and_signs() {
        assert_eq!(fmt_short(Money::from_vnd(2_200_000)), "2,2tr");
        assert_eq!(fmt_short(Money::from_vnd(4_000_000)), "4tr");
        assert_eq!(fmt_short(Money::from_vnd(600_000)), "600k");
        assert_eq!(fmt_short(Money::from_vnd(-1_500_000)), "−1,5tr");
        assert_eq!(fmt_short(Money::from_vnd(6_450_000)), "6,5tr");
    }

    #[test]
    fn short_names_are_compact() {
        for tile in get_board() {
            let n = tile_short_name(&tile);
            assert!(n.chars().count() <= 22, "{n} quá dài");
            assert!(!n.is_empty());
        }
    }
}
