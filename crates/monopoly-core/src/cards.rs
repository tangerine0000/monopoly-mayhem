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
            description: "Bạn trúng giải khuyến khích vé số kiến thiết TP.HCM, nhận 500.000 đ.".to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(500_000)),
        },
        Card {
            id: 103,
            deck: DeckType::Chance,
            title: "Sửa xe ngập nước".to_string(),
            description: "Xe bị ngập nước thủy kích trên đường Nguyễn Hữu Cảnh. Chi phí đại tu 1.000.000 đ.".to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(-1_000_000)),
        },
        Card {
            id: 104,
            deck: DeckType::Chance,
            title: "Bị giữ xe vì nồng độ cồn".to_string(),
            description: "Vi phạm nồng độ cồn sau khi nhậu ở Bờ Kè, chuyển vào Khu tạm giam.".to_string(),
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
        Card {
            id: 109,
            deck: DeckType::Chance,
            title: "Đi tàu tại Ga Sài Gòn".to_string(),
            description: "Bắt chuyến tàu đêm tại Ga Sài Gòn. Nếu đi qua Khởi Hành, nhận 2.000.000 đ.".to_string(),
            effect: CardEffect::MoveTo(15),
        },
        Card {
            id: 110,
            deck: DeckType::Chance,
            title: "Khám phá ẩm thực Phan Xích Long".to_string(),
            description: "Đi ăn tối tại khu phố ẩm thực Phan Xích Long (Phú Nhuận).".to_string(),
            effect: CardEffect::MoveTo(11),
        },
        Card {
            id: 111,
            deck: DeckType::Chance,
            title: "Mua sắm tại An Dương Vương".to_string(),
            description: "Tiến đến đường An Dương Vương (Quận 5). Nếu đi qua Khởi Hành, nhận 2.000.000 đ.".to_string(),
            effect: CardEffect::MoveTo(24),
        },
        Card {
            id: 112,
            deck: DeckType::Chance,
            title: "Trải nghiệm Tuyến Metro Số 1".to_string(),
            description: "Đi thử chuyến tàu điện ngầm tới Suối Tiên tại Tuyến Metro Số 1.".to_string(),
            effect: CardEffect::MoveTo(35),
        },
        Card {
            id: 113,
            deck: DeckType::Chance,
            title: "Ghé thăm phố Đồng Khởi".to_string(),
            description: "Dạo phố và mua sắm trên đường Đồng Khởi (Quận 1).".to_string(),
            effect: CardEffect::MoveTo(37),
        },
        Card {
            id: 114,
            deck: DeckType::Chance,
            title: "Bầu làm Trưởng ban Quản lý Khu phố".to_string(),
            description: "Bạn đắc cử Trưởng ban Quản lý Khu phố. Thưởng mỗi người chơi 500.000 đ.".to_string(),
            effect: CardEffect::PayToAll(Money::from_vnd(500_000)),
        },
        Card {
            id: 115,
            deck: DeckType::Chance,
            title: "Phạt quá tốc độ qua hầm Thủ Thiêm".to_string(),
            description: "Chạy quá tốc độ quy định khi qua hầm Thủ Thiêm. Nộp phạt 300.000 đ.".to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(-300_000)),
        },
        Card {
            id: 116,
            deck: DeckType::Chance,
            title: "Nhận tiền lãi tiết kiệm".to_string(),
            description: "Ngân hàng thanh toán tiền lãi gửi tiết kiệm định kỳ. Nhận 300.000 đ.".to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(300_000)),
        },
        Card {
            id: 117,
            deck: DeckType::Chance,
            title: "Giải ngân khoản vay xây dựng".to_string(),
            description: "Hồ sơ vay vốn xây dựng tại Sài Gòn được giải ngân. Nhận 1.500.000 đ.".to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(1_500_000)),
        },
        Card {
            id: 118,
            deck: DeckType::Chance,
            title: "Dạo bước Thảo Điền".to_string(),
            description: "Ghé thăm khu phố Xuân Thủy (Thảo Điền, Quận 2).".to_string(),
            effect: CardEffect::MoveTo(34),
        },
    ]
}

pub fn get_community_chest_deck() -> Vec<Card> {
    vec![
        Card {
            id: 201,
            deck: DeckType::CommunityChest,
            title: "Mừng tuổi đầu năm".to_string(),
            description: "Họ hàng đến chúc Tết mừng tuổi, nhận 500.000 đ từ mỗi người chơi khác."
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
            title: "Cổ tức ngân hàng cuối năm".to_string(),
            description: "Nhận tiền cổ tức đầu tư cuối năm. Nhận 1.000.000 đ.".to_string(),
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
                "Bị phát hiện kê khai hai giá khi công chứng nhà đất, chuyển vào Khu tạm giam."
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
            description: "Hóa đơn tiền điện và sửa chữa điều hòa mùa nắng nóng tăng vọt. Chi trả 1.500.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(-1_500_000)),
        },
        Card {
            id: 209,
            deck: DeckType::CommunityChest,
            title: "Ngân hàng chuyển nhầm có lợi cho bạn".to_string(),
            description: "Hệ thống ngân hàng đối soát nhầm có lợi cho tài khoản của bạn. Nhận 2.000.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(2_000_000)),
        },
        Card {
            id: 210,
            deck: DeckType::CommunityChest,
            title: "Truy thu thuế thu nhập cá nhân".to_string(),
            description: "Quyết toán thuế thu nhập cá nhân cuối năm phát sinh khoản truy thu. Nộp 1.500.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(-1_500_000)),
        },
        Card {
            id: 211,
            deck: DeckType::CommunityChest,
            title: "Chi phí khám sức khỏe định kỳ".to_string(),
            description: "Khám sức khỏe tổng quát chuyên sâu và chăm sóc nha khoa. Trả 1.000.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(-1_000_000)),
        },
        Card {
            id: 212,
            deck: DeckType::CommunityChest,
            title: "Đóng học phí đầu năm".to_string(),
            description: "Đóng học phí và mua sắm sách vở đầu năm học mới. Chi trả 1.000.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(-1_000_000)),
        },
        Card {
            id: 213,
            deck: DeckType::CommunityChest,
            title: "Nhận thừa kế đất đai gia tộc".to_string(),
            description: "Họ hàng để lại một phần tài sản thừa kế tại Sài Gòn. Nhận 1.000.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(1_000_000)),
        },
        Card {
            id: 214,
            deck: DeckType::CommunityChest,
            title: "Bán cổ phiếu chốt lời".to_string(),
            description: "Chốt lời danh mục đầu tư chứng khoán. Nhận 600.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(600_000)),
        },
        Card {
            id: 215,
            deck: DeckType::CommunityChest,
            title: "Đáo hạn sổ tiết kiệm cuối năm".to_string(),
            description: "Sổ tiết kiệm kỳ hạn 12 tháng đến ngày tất toán. Nhận 1.500.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(1_500_000)),
        },
        Card {
            id: 216,
            deck: DeckType::CommunityChest,
            title: "Đóng góp nâng cấp hẻm khu phố".to_string(),
            description: "Đóng phí bê tông hóa hẻm và chỉnh trang đô thị: 250.000 đ mỗi căn nhà và 1.000.000 đ mỗi khách sạn."
                .to_string(),
            effect: CardEffect::BuildingRepairs {
                house: Money::from_vnd(250_000),
                hotel: Money::from_vnd(1_000_000),
            },
        },
        Card {
            id: 217,
            deck: DeckType::CommunityChest,
            title: "Mua vé đêm nhạc tại Nhà Hát Thành Phố".to_string(),
            description: "Mua vé thưởng thức đêm nhạc giao hưởng tại Nhà Hát Thành Phố. Chi 300.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(-300_000)),
        },
        Card {
            id: 218,
            deck: DeckType::CommunityChest,
            title: "Đoạt giải cuộc thi ảnh đẹp Sài Gòn".to_string(),
            description: "Bạn đạt giải nhì cuộc thi nhiếp ảnh đường phố Sài Gòn. Nhận 300.000 đ."
                .to_string(),
            effect: CardEffect::CashDelta(Money::from_vnd(300_000)),
        },
    ]
}
