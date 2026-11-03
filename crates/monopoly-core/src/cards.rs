use crate::money::Money;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeckType {
    Chance,         // Cơ Hội
    CommunityChest, // Khí Vận
}

impl std::fmt::Display for DeckType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeckType::Chance => write!(f, "Cơ Hội"),
            DeckType::CommunityChest => write!(f, "Khí Vận"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardEffect {
    CashDelta(Money),
    MoveTo(u8),
    GoToJail,
    GetOutOfJailFree,
    BuildingRepairs { house: Money, hotel: Money },
    CollectFromAll(Money),
    PayToAll(Money),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub id: u16,
    pub deck: DeckType,
    pub title: String,
    pub description: String,
    pub effect: CardEffect,
}

pub fn get_chance_deck() -> Vec<Card> {
    vec![
        Card {
            id: 101,
            deck: DeckType::Chance,
            title: "Tiến thẳng tới Nguyễn Huệ".to_string(),
            description: "Tiến vào phố đi bộ Nguyễn Huệ trải nghiệm ẩm thực Sài Gòn.".to_string(),
            effect: CardEffect::MoveTo(39),
        },
        Card {
            id: 102,
            deck: DeckType::Chance,
            title: "Trúng vé số kiến thiết".to_string(),
            description: "Bạn trúng giải an ủi vé số kiến thiết TP.HCM! Nhận 1.000.000 đ.".to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(1_000_000)),
        },
        Card {
            id: 103,
            deck: DeckType::Chance,
            title: "Sửa xe ngập nước".to_string(),
            description: "Xe bị ngập nước chết máy trên đường Nguyễn Hữu Cảnh. Chi phí sửa chữa 500.000 đ.".to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(-500_000)),
        },
        Card {
            id: 104,
            deck: DeckType::Chance,
            title: "Bị giữ xe vì nồng độ cồn".to_string(),
            description: "Vi phạm nồng độ cồn sau khi nhậu ở Bờ Kè. Vào tù ngay lập tức!".to_string(),
            effect: CardEffect::GoToJail,
        },
        Card {
            id: 105,
            deck: DeckType::Chance,
            title: "Thẻ bảo lãnh giao thông".to_string(),
            description: "Người quen bảo lãnh xe vi phạm. Thẻ này được giữ lại cho đến khi dùng hoặc bán.".to_string(),
            effect: CardEffect::GetOutOfJailFree,
        },
        Card {
            id: 106,
            deck: DeckType::Chance,
            title: "Trùng tu nhà cửa sau mùa mưa".to_string(),
            description: "Mùa mưa triều cường gây hư hại nhà cửa. Trả 250.000 đ mỗi căn nhà và 1.000.000 đ mỗi khách sạn.".to_string(),
            effect: CardEffect::BuildingRepairs {
                house: Money::from_vnd(250_000),
                hotel: Money::from_vnd(1_000_000),
            },
        },
        Card {
            id: 107,
            deck: DeckType::Chance,
            title: "Quay về Khởi Hành".to_string(),
            description: "Quay về vạch Khởi Hành (GO) nhận 2.000.000 đ lương tháng.".to_string(),
            effect: CardEffect::MoveTo(0),
        },
        Card {
            id: 108,
            deck: DeckType::Chance,
            title: "Trúng thầu dự án Metro".to_string(),
            description: "Nhận tiền tư vấn dự án Metro Bến Thành - Suối Tiên. Nhận 1.500.000 đ.".to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(1_500_000)),
        },
    ]
}

pub fn get_community_chest_deck() -> Vec<Card> {
    vec![
        Card {
            id: 201,
            deck: DeckType::CommunityChest,
            title: "Mừng tuổi đầu năm".to_string(),
            description: "Họ hàng đến chúc Tết mừng tuổi! Nhận 500.000 đ từ mỗi người chơi khác."
                .to_string(),
            effect: CardEffect::CollectFromAll(Money::from_vnd(500_000)),
        },
        Card {
            id: 202,
            deck: DeckType::CommunityChest,
            title: "Ủng hộ quỹ từ thiện Bếp Ăn 0 Đồng".to_string(),
            description: "Đóng góp cho bếp ăn từ thiện người nghèo Sài Gòn. Chi 500.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(-500_000)),
        },
        Card {
            id: 203,
            deck: DeckType::CommunityChest,
            title: "Cổ tức ngân hàng Sacombank".to_string(),
            description: "Nhận cổ tức cuối năm. Nhận 1.000.000 đ.".to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(1_000_000)),
        },
        Card {
            id: 204,
            deck: DeckType::CommunityChest,
            title: "Bồi thường bảo hiểm nhà đất".to_string(),
            description: "Bảo hiểm chi trả bồi thường sửa mái tôn. Nhận 500.000 đ.".to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(500_000)),
        },
        Card {
            id: 205,
            deck: DeckType::CommunityChest,
            title: "Vào tù vì trốn thuế bất động sản".to_string(),
            description:
                "Bị phát hiện kê khai hai giá khi công chứng nhà đất. Vào tù ngay lập tức!"
                    .to_string(),
            effect: CardEffect::GoToJail,
        },
        Card {
            id: 206,
            deck: DeckType::CommunityChest,
            title: "Bảo lãnh tại ngoại".to_string(),
            description: "Thẻ cứu trợ được bảo lãnh miễn phí khỏi trại tạm giam.".to_string(),
            effect: CardEffect::GetOutOfJailFree,
        },
        Card {
            id: 207,
            deck: DeckType::CommunityChest,
            title: "Đãi tiệc tân gia".to_string(),
            description:
                "Mở tiệc tân gia mời bạn bè. Trả mỗi người chơi 200.000 đ tiền quà lưu niệm."
                    .to_string(),
            effect: CardEffect::PayToAll(Money::from_vnd(200_000)),
        },
        Card {
            id: 208,
            deck: DeckType::CommunityChest,
            title: "Chi phí điện nước sinh hoạt".to_string(),
            description: "Hóa đơn tiền điện mùa nắng nóng tăng vọt. Nộp phạt 400.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(-400_000)),
        },
    ]
}
