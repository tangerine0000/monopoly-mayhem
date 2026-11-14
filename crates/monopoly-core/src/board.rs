use monopoly_proto::Money;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum District {
    District4, // Brown / Bình Dân 1 (2 tiles: 1, 3)
    District8, // Light Blue / Bình Dân 2 (3 tiles: 6, 8, 9)
    PhuNhuan,  // Pink / Trung Cấp 1 (3 tiles: 11, 13, 14)
    TanBinh,   // Orange / Trung Cấp 2 (3 tiles: 16, 18, 19)
    District5, // Red / Thương Gia 1 (3 tiles: 21, 23, 24)
    District3, // Yellow / Thương Gia 2 (3 tiles: 26, 27, 29)
    District2, // Green / Cao Cấp 1 - Thảo Điền (3 tiles: 31, 32, 34)
    District1, // Dark Blue / Cao Cấp 2 - Bến Nghé (2 tiles: 37, 39)
}

impl District {
    pub fn property_count(&self) -> usize {
        match self {
            District::District4 | District::District1 => 2,
            _ => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyDef {
    pub name: &'static str,
    pub district: District,
    pub cost: Money,
    pub house_cost: Money,
    pub mortgage_value: Money,
    pub rent_table: [Money; 6], // Base, 1H, 2H, 3H, 4H, Hotel
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationDef {
    pub name: &'static str,
    pub cost: Money,
    pub mortgage_value: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtilityDef {
    pub name: &'static str,
    pub cost: Money,
    pub mortgage_value: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxDef {
    pub name: &'static str,
    pub amount: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    Go,
    Property(PropertyDef),
    CommunityChest, // Khí Vận
    Tax(TaxDef),
    Station(StationDef),
    Chance, // Cơ Hội
    Jail,
    Utility(UtilityDef),
    FreeParking,
    GoToJail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub index: u8,
    pub name: &'static str,
    pub kind: TileKind,
}

pub struct BoardCatalog;

impl BoardCatalog {
    #[inline]
    pub fn all() -> &'static [Tile; 40] {
        &BOARD
    }

    #[inline]
    pub fn tile(index: u8) -> &'static Tile {
        &BOARD[index as usize % 40]
    }

    #[inline]
    pub fn district_tiles(district: District) -> &'static [u8] {
        get_district_tiles(district)
    }
}

pub static BOARD: [Tile; 40] = [
        // 0: Khởi Hành (GO)
        Tile {
            index: 0,
            name: "Khởi Hành (GO)",
            kind: TileKind::Go,
        },
        // 1: Bến Vân Đồn (District 4)
        Tile {
            index: 1,
            name: "Bến Vân Đồn",
            kind: TileKind::Property(PropertyDef {
                name: "Bến Vân Đồn",
                district: District::District4,
                cost: Money::from_vnd(600_000),
                house_cost: Money::from_vnd(500_000),
                mortgage_value: Money::from_vnd(300_000),
                rent_table: [
                    Money::from_vnd(20_000),
                    Money::from_vnd(100_000),
                    Money::from_vnd(300_000),
                    Money::from_vnd(900_000),
                    Money::from_vnd(1_600_000),
                    Money::from_vnd(2_500_000),
                ],
            }),
        },
        // 2: Khí Vận 1
        Tile {
            index: 2,
            name: "Khí Vận 1",
            kind: TileKind::CommunityChest,
        },
        // 3: Đoàn Văn Bơ (District 4)
        Tile {
            index: 3,
            name: "Đoàn Văn Bơ",
            kind: TileKind::Property(PropertyDef {
                name: "Đoàn Văn Bơ",
                district: District::District4,
                cost: Money::from_vnd(600_000),
                house_cost: Money::from_vnd(500_000),
                mortgage_value: Money::from_vnd(300_000),
                rent_table: [
                    Money::from_vnd(40_000),
                    Money::from_vnd(200_000),
                    Money::from_vnd(600_000),
                    Money::from_vnd(1_800_000),
                    Money::from_vnd(3_200_000),
                    Money::from_vnd(4_500_000),
                ],
            }),
        },
        // 4: Thuế Thu Nhập
        Tile {
            index: 4,
            name: "Thuế Thu Nhập",
            kind: TileKind::Tax(TaxDef {
                name: "Thuế Thu Nhập",
                amount: Money::from_vnd(2_000_000),
            }),
        },
        // 5: Bến Xe Miền Đông
        Tile {
            index: 5,
            name: "Bến Xe Miền Đông",
            kind: TileKind::Station(StationDef {
                name: "Bến Xe Miền Đông",
                cost: Money::from_vnd(2_000_000),
                mortgage_value: Money::from_vnd(1_000_000),
            }),
        },
        // 6: Phạm Thế Hiển (District 8)
        Tile {
            index: 6,
            name: "Phạm Thế Hiển",
            kind: TileKind::Property(PropertyDef {
                name: "Phạm Thế Hiển",
                district: District::District8,
                cost: Money::from_vnd(1_000_000),
                house_cost: Money::from_vnd(500_000),
                mortgage_value: Money::from_vnd(500_000),
                rent_table: [
                    Money::from_vnd(60_000),
                    Money::from_vnd(300_000),
                    Money::from_vnd(900_000),
                    Money::from_vnd(2_700_000),
                    Money::from_vnd(4_000_000),
                    Money::from_vnd(5_500_000),
                ],
            }),
        },
        // 7: Cơ Hội 1
        Tile {
            index: 7,
            name: "Cơ Hội 1",
            kind: TileKind::Chance,
        },
        // 8: Tạ Quang Bửu (District 8)
        Tile {
            index: 8,
            name: "Tạ Quang Bửu",
            kind: TileKind::Property(PropertyDef {
                name: "Tạ Quang Bửu",
                district: District::District8,
                cost: Money::from_vnd(1_000_000),
                house_cost: Money::from_vnd(500_000),
                mortgage_value: Money::from_vnd(500_000),
                rent_table: [
                    Money::from_vnd(60_000),
                    Money::from_vnd(300_000),
                    Money::from_vnd(900_000),
                    Money::from_vnd(2_700_000),
                    Money::from_vnd(4_000_000),
                    Money::from_vnd(5_500_000),
                ],
            }),
        },
        // 9: Dương Bá Trạc (District 8)
        Tile {
            index: 9,
            name: "Dương Bá Trạc",
            kind: TileKind::Property(PropertyDef {
                name: "Dương Bá Trạc",
                district: District::District8,
                cost: Money::from_vnd(1_200_000),
                house_cost: Money::from_vnd(500_000),
                mortgage_value: Money::from_vnd(600_000),
                rent_table: [
                    Money::from_vnd(80_000),
                    Money::from_vnd(400_000),
                    Money::from_vnd(1_000_000),
                    Money::from_vnd(3_000_000),
                    Money::from_vnd(4_500_000),
                    Money::from_vnd(6_000_000),
                ],
            }),
        },
        // 10: Vào Tù / Thăm Tù
        Tile {
            index: 10,
            name: "Vào Tù / Thăm Tù",
            kind: TileKind::Jail,
        },
        // 11: Phan Xích Long (Phu Nhuan)
        Tile {
            index: 11,
            name: "Phan Xích Long",
            kind: TileKind::Property(PropertyDef {
                name: "Phan Xích Long",
                district: District::PhuNhuan,
                cost: Money::from_vnd(1_400_000),
                house_cost: Money::from_vnd(1_000_000),
                mortgage_value: Money::from_vnd(700_000),
                rent_table: [
                    Money::from_vnd(100_000),
                    Money::from_vnd(500_000),
                    Money::from_vnd(1_500_000),
                    Money::from_vnd(4_500_000),
                    Money::from_vnd(6_250_000),
                    Money::from_vnd(7_500_000),
                ],
            }),
        },
        // 12: Nhà Máy Cấp Nước
        Tile {
            index: 12,
            name: "Nhà Máy Cấp Nước",
            kind: TileKind::Utility(UtilityDef {
                name: "Nhà Máy Cấp Nước",
                cost: Money::from_vnd(1_500_000),
                mortgage_value: Money::from_vnd(750_000),
            }),
        },
        // 13: Phan Đăng Lưu (Phu Nhuan)
        Tile {
            index: 13,
            name: "Phan Đăng Lưu",
            kind: TileKind::Property(PropertyDef {
                name: "Phan Đăng Lưu",
                district: District::PhuNhuan,
                cost: Money::from_vnd(1_400_000),
                house_cost: Money::from_vnd(1_000_000),
                mortgage_value: Money::from_vnd(700_000),
                rent_table: [
                    Money::from_vnd(100_000),
                    Money::from_vnd(500_000),
                    Money::from_vnd(1_500_000),
                    Money::from_vnd(4_500_000),
                    Money::from_vnd(6_250_000),
                    Money::from_vnd(7_500_000),
                ],
            }),
        },
        // 14: Hoàng Văn Thụ (Phu Nhuan)
        Tile {
            index: 14,
            name: "Hoàng Văn Thụ",
            kind: TileKind::Property(PropertyDef {
                name: "Hoàng Văn Thụ",
                district: District::PhuNhuan,
                cost: Money::from_vnd(1_600_000),
                house_cost: Money::from_vnd(1_000_000),
                mortgage_value: Money::from_vnd(800_000),
                rent_table: [
                    Money::from_vnd(120_000),
                    Money::from_vnd(600_000),
                    Money::from_vnd(1_800_000),
                    Money::from_vnd(5_000_000),
                    Money::from_vnd(7_000_000),
                    Money::from_vnd(9_000_000),
                ],
            }),
        },
        // 15: Ga Sài Gòn
        Tile {
            index: 15,
            name: "Ga Sài Gòn",
            kind: TileKind::Station(StationDef {
                name: "Ga Sài Gòn",
                cost: Money::from_vnd(2_000_000),
                mortgage_value: Money::from_vnd(1_000_000),
            }),
        },
        // 16: Cộng Hòa (Tan Binh)
        Tile {
            index: 16,
            name: "Cộng Hòa",
            kind: TileKind::Property(PropertyDef {
                name: "Cộng Hòa",
                district: District::TanBinh,
                cost: Money::from_vnd(1_800_000),
                house_cost: Money::from_vnd(1_000_000),
                mortgage_value: Money::from_vnd(900_000),
                rent_table: [
                    Money::from_vnd(140_000),
                    Money::from_vnd(700_000),
                    Money::from_vnd(2_000_000),
                    Money::from_vnd(5_500_000),
                    Money::from_vnd(7_500_000),
                    Money::from_vnd(9_500_000),
                ],
            }),
        },
        // 17: Khí Vận 2
        Tile {
            index: 17,
            name: "Khí Vận 2",
            kind: TileKind::CommunityChest,
        },
        // 18: Trường Chinh (Tan Binh)
        Tile {
            index: 18,
            name: "Trường Chinh",
            kind: TileKind::Property(PropertyDef {
                name: "Trường Chinh",
                district: District::TanBinh,
                cost: Money::from_vnd(1_800_000),
                house_cost: Money::from_vnd(1_000_000),
                mortgage_value: Money::from_vnd(900_000),
                rent_table: [
                    Money::from_vnd(140_000),
                    Money::from_vnd(700_000),
                    Money::from_vnd(2_000_000),
                    Money::from_vnd(5_500_000),
                    Money::from_vnd(7_500_000),
                    Money::from_vnd(9_500_000),
                ],
            }),
        },
        // 19: Lê Văn Sỹ (Tan Binh)
        Tile {
            index: 19,
            name: "Lê Văn Sỹ",
            kind: TileKind::Property(PropertyDef {
                name: "Lê Văn Sỹ",
                district: District::TanBinh,
                cost: Money::from_vnd(2_000_000),
                house_cost: Money::from_vnd(1_000_000),
                mortgage_value: Money::from_vnd(1_000_000),
                rent_table: [
                    Money::from_vnd(160_000),
                    Money::from_vnd(800_000),
                    Money::from_vnd(2_200_000),
                    Money::from_vnd(6_000_000),
                    Money::from_vnd(8_000_000),
                    Money::from_vnd(10_000_000),
                ],
            }),
        },
        // 20: Bãi Đỗ Xe Miễn Phí
        Tile {
            index: 20,
            name: "Bãi Đỗ Xe Miễn Phí",
            kind: TileKind::FreeParking,
        },
        // 21: Hải Thượng Lãn Ông (District 5)
        Tile {
            index: 21,
            name: "Hải Thượng Lãn Ông",
            kind: TileKind::Property(PropertyDef {
                name: "Hải Thượng Lãn Ông",
                district: District::District5,
                cost: Money::from_vnd(2_200_000),
                house_cost: Money::from_vnd(1_500_000),
                mortgage_value: Money::from_vnd(1_100_000),
                rent_table: [
                    Money::from_vnd(180_000),
                    Money::from_vnd(900_000),
                    Money::from_vnd(2_500_000),
                    Money::from_vnd(7_000_000),
                    Money::from_vnd(8_750_000),
                    Money::from_vnd(10_500_000),
                ],
            }),
        },
        // 22: Cơ Hội 2
        Tile {
            index: 22,
            name: "Cơ Hội 2",
            kind: TileKind::Chance,
        },
        // 23: Châu Văn Liêm (District 5)
        Tile {
            index: 23,
            name: "Châu Văn Liêm",
            kind: TileKind::Property(PropertyDef {
                name: "Châu Văn Liêm",
                district: District::District5,
                cost: Money::from_vnd(2_200_000),
                house_cost: Money::from_vnd(1_500_000),
                mortgage_value: Money::from_vnd(1_100_000),
                rent_table: [
                    Money::from_vnd(180_000),
                    Money::from_vnd(900_000),
                    Money::from_vnd(2_500_000),
                    Money::from_vnd(7_000_000),
                    Money::from_vnd(8_750_000),
                    Money::from_vnd(10_500_000),
                ],
            }),
        },
        // 24: An Dương Vương (District 5)
        Tile {
            index: 24,
            name: "An Dương Vương",
            kind: TileKind::Property(PropertyDef {
                name: "An Dương Vương",
                district: District::District5,
                cost: Money::from_vnd(2_400_000),
                house_cost: Money::from_vnd(1_500_000),
                mortgage_value: Money::from_vnd(1_200_000),
                rent_table: [
                    Money::from_vnd(200_000),
                    Money::from_vnd(1_000_000),
                    Money::from_vnd(3_000_000),
                    Money::from_vnd(7_500_000),
                    Money::from_vnd(9_250_000),
                    Money::from_vnd(11_000_000),
                ],
            }),
        },
        // 25: Bến Xe Miền Tây
        Tile {
            index: 25,
            name: "Bến Xe Miền Tây",
            kind: TileKind::Station(StationDef {
                name: "Bến Xe Miền Tây",
                cost: Money::from_vnd(2_000_000),
                mortgage_value: Money::from_vnd(1_000_000),
            }),
        },
        // 26: Nguyễn Thị Minh Khai (District 3)
        Tile {
            index: 26,
            name: "Nguyễn Thị Minh Khai",
            kind: TileKind::Property(PropertyDef {
                name: "Nguyễn Thị Minh Khai",
                district: District::District3,
                cost: Money::from_vnd(2_600_000),
                house_cost: Money::from_vnd(1_500_000),
                mortgage_value: Money::from_vnd(1_300_000),
                rent_table: [
                    Money::from_vnd(220_000),
                    Money::from_vnd(1_100_000),
                    Money::from_vnd(3_300_000),
                    Money::from_vnd(8_000_000),
                    Money::from_vnd(9_750_000),
                    Money::from_vnd(11_500_000),
                ],
            }),
        },
        // 27: Nam Kỳ Khởi Nghĩa (District 3)
        Tile {
            index: 27,
            name: "Nam Kỳ Khởi Nghĩa",
            kind: TileKind::Property(PropertyDef {
                name: "Nam Kỳ Khởi Nghĩa",
                district: District::District3,
                cost: Money::from_vnd(2_600_000),
                house_cost: Money::from_vnd(1_500_000),
                mortgage_value: Money::from_vnd(1_300_000),
                rent_table: [
                    Money::from_vnd(220_000),
                    Money::from_vnd(1_100_000),
                    Money::from_vnd(3_300_000),
                    Money::from_vnd(8_000_000),
                    Money::from_vnd(9_750_000),
                    Money::from_vnd(11_500_000),
                ],
            }),
        },
        // 28: Nhà Máy Điện Lực
        Tile {
            index: 28,
            name: "Nhà Máy Điện Lực",
            kind: TileKind::Utility(UtilityDef {
                name: "Nhà Máy Điện Lực",
                cost: Money::from_vnd(1_500_000),
                mortgage_value: Money::from_vnd(750_000),
            }),
        },
        // 29: Võ Văn Tần (District 3)
        Tile {
            index: 29,
            name: "Võ Văn Tần",
            kind: TileKind::Property(PropertyDef {
                name: "Võ Văn Tần",
                district: District::District3,
                cost: Money::from_vnd(2_800_000),
                house_cost: Money::from_vnd(1_500_000),
                mortgage_value: Money::from_vnd(1_400_000),
                rent_table: [
                    Money::from_vnd(240_000),
                    Money::from_vnd(1_200_000),
                    Money::from_vnd(3_600_000),
                    Money::from_vnd(8_500_000),
                    Money::from_vnd(10_250_000),
                    Money::from_vnd(12_000_000),
                ],
            }),
        },
        // 30: Vào Tù Ngay Lập Tức
        Tile {
            index: 30,
            name: "Vào Tù Ngay Lập Tức",
            kind: TileKind::GoToJail,
        },
        // 31: Trần Não (District 2)
        Tile {
            index: 31,
            name: "Trần Não",
            kind: TileKind::Property(PropertyDef {
                name: "Trần Não",
                district: District::District2,
                cost: Money::from_vnd(3_000_000),
                house_cost: Money::from_vnd(2_000_000),
                mortgage_value: Money::from_vnd(1_500_000),
                rent_table: [
                    Money::from_vnd(260_000),
                    Money::from_vnd(1_300_000),
                    Money::from_vnd(3_900_000),
                    Money::from_vnd(9_000_000),
                    Money::from_vnd(11_000_000),
                    Money::from_vnd(12_750_000),
                ],
            }),
        },
        // 32: Song Hành (District 2)
        Tile {
            index: 32,
            name: "Song Hành",
            kind: TileKind::Property(PropertyDef {
                name: "Song Hành",
                district: District::District2,
                cost: Money::from_vnd(3_000_000),
                house_cost: Money::from_vnd(2_000_000),
                mortgage_value: Money::from_vnd(1_500_000),
                rent_table: [
                    Money::from_vnd(260_000),
                    Money::from_vnd(1_300_000),
                    Money::from_vnd(3_900_000),
                    Money::from_vnd(9_000_000),
                    Money::from_vnd(11_000_000),
                    Money::from_vnd(12_750_000),
                ],
            }),
        },
        // 33: Khí Vận 3
        Tile {
            index: 33,
            name: "Khí Vận 3",
            kind: TileKind::CommunityChest,
        },
        // 34: Xuân Thủy (Thảo Điền) (District 2)
        Tile {
            index: 34,
            name: "Xuân Thủy (Thảo Điền)",
            kind: TileKind::Property(PropertyDef {
                name: "Xuân Thủy (Thảo Điền)",
                district: District::District2,
                cost: Money::from_vnd(3_200_000),
                house_cost: Money::from_vnd(2_000_000),
                mortgage_value: Money::from_vnd(1_600_000),
                rent_table: [
                    Money::from_vnd(280_000),
                    Money::from_vnd(1_500_000),
                    Money::from_vnd(4_500_000),
                    Money::from_vnd(10_000_000),
                    Money::from_vnd(12_000_000),
                    Money::from_vnd(14_000_000),
                ],
            }),
        },
        // 35: Tuyến Metro Số 1
        Tile {
            index: 35,
            name: "Tuyến Metro Số 1",
            kind: TileKind::Station(StationDef {
                name: "Tuyến Metro Số 1",
                cost: Money::from_vnd(2_000_000),
                mortgage_value: Money::from_vnd(1_000_000),
            }),
        },
        // 36: Cơ Hội 3
        Tile {
            index: 36,
            name: "Cơ Hội 3",
            kind: TileKind::Chance,
        },
        // 37: Đồng Khởi (District 1)
        Tile {
            index: 37,
            name: "Đồng Khởi",
            kind: TileKind::Property(PropertyDef {
                name: "Đồng Khởi",
                district: District::District1,
                cost: Money::from_vnd(3_500_000),
                house_cost: Money::from_vnd(2_000_000),
                mortgage_value: Money::from_vnd(1_750_000),
                rent_table: [
                    Money::from_vnd(350_000),
                    Money::from_vnd(1_750_000),
                    Money::from_vnd(5_000_000),
                    Money::from_vnd(11_000_000),
                    Money::from_vnd(13_000_000),
                    Money::from_vnd(15_000_000),
                ],
            }),
        },
        // 38: Thuế Xa Xỉ Phẩm
        Tile {
            index: 38,
            name: "Thuế Xa Xỉ Phẩm",
            kind: TileKind::Tax(TaxDef {
                name: "Thuế Xa Xỉ Phẩm",
                amount: Money::from_vnd(1_000_000),
            }),
        },
        // 39: Nguyễn Huệ (District 1)
        Tile {
            index: 39,
            name: "Nguyễn Huệ",
            kind: TileKind::Property(PropertyDef {
                name: "Nguyễn Huệ",
                district: District::District1,
                cost: Money::from_vnd(4_000_000),
                house_cost: Money::from_vnd(2_000_000),
                mortgage_value: Money::from_vnd(2_000_000),
                rent_table: [
                    Money::from_vnd(500_000),
                    Money::from_vnd(2_000_000),
                    Money::from_vnd(6_000_000),
                    Money::from_vnd(14_000_000),
                    Money::from_vnd(17_000_000),
                    Money::from_vnd(20_000_000),
                ],
            }),
        },
];

pub fn get_board() -> &'static [Tile; 40] {
    &BOARD
}

pub const fn get_district_tiles(district: District) -> &'static [u8] {
    match district {
        District::District4 => &[1, 3],
        District::District8 => &[6, 8, 9],
        District::PhuNhuan => &[11, 13, 14],
        District::TanBinh => &[16, 18, 19],
        District::District5 => &[21, 23, 24],
        District::District3 => &[26, 27, 29],
        District::District2 => &[31, 32, 34],
        District::District1 => &[37, 39],
    }
}
