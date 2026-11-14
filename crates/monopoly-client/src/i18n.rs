use monopoly_core::board::{BoardCatalog, District};
use monopoly_core::cards::{get_chance_deck, get_community_chest_deck};
use monopoly_proto::{CardId, DeckId, Locale, MessageKey, Money};

pub struct LocaleCatalog;

impl LocaleCatalog {
    pub fn district_name(locale: Locale, district: District) -> &'static str {
        match (locale, district) {
            (Locale::Vi, District::District4) => "Quận 4",
            (Locale::Vi, District::District8) => "Quận 8",
            (Locale::Vi, District::PhuNhuan) => "Phú Nhuận",
            (Locale::Vi, District::TanBinh) => "Tân Bình",
            (Locale::Vi, District::District5) => "Quận 5 · Chợ Lớn",
            (Locale::Vi, District::District3) => "Quận 3",
            (Locale::Vi, District::District2) => "Quận 2 · Thảo Điền",
            (Locale::Vi, District::District1) => "Quận 1 · Bến Nghé",
            (Locale::En, District::District4) => "District 4",
            (Locale::En, District::District8) => "District 8",
            (Locale::En, District::PhuNhuan) => "Phu Nhuan",
            (Locale::En, District::TanBinh) => "Tan Binh",
            (Locale::En, District::District5) => "District 5 · Cho Lon",
            (Locale::En, District::District3) => "District 3",
            (Locale::En, District::District2) => "District 2 · Thao Dien",
            (Locale::En, District::District1) => "District 1 · Ben Nghe",
        }
    }

    pub fn tile_name(locale: Locale, tile_index: u8) -> &'static str {
        match locale {
            Locale::Vi => BoardCatalog::tile(tile_index).name,
            Locale::En => match tile_index % 40 {
                0 => "GO (Start)",
                2 => "Community Chest 1",
                4 => "Income Tax",
                5 => "Saigon Railway Station",
                7 => "Chance 1",
                10 => "Jail / Just Visiting",
                12 => "Saigon Waterworks",
                15 => "Ben Thanh Metro",
                17 => "Community Chest 2",
                20 => "Free Parking",
                22 => "Chance 2",
                25 => "Mien Dong Bus Terminal",
                28 => "Saigon Power Co.",
                30 => "Go To Jail",
                33 => "Community Chest 3",
                35 => "Tan Son Nhat Airport",
                36 => "Chance 3",
                38 => "Luxury Tax",
                idx => BoardCatalog::tile(idx).name,
            },
        }
    }

    pub fn deck_name(locale: Locale, deck: DeckId) -> &'static str {
        match (locale, deck) {
            (Locale::Vi, DeckId::Chance) => "Cơ Hội",
            (Locale::Vi, DeckId::CommunityChest) => "Khí Vận",
            (Locale::En, DeckId::Chance) => "Chance",
            (Locale::En, DeckId::CommunityChest) => "Community Chest",
        }
    }

    pub fn card_text(locale: Locale, card_id: CardId) -> (&'static str, &'static str) {
        if locale == Locale::En {
            if let Some(pair) = en_card_text(card_id.0) {
                return pair;
            }
        }
        // Fallback / Vi adapter reads canonical static deck definitions
        for c in get_chance_deck()
            .into_iter()
            .chain(get_community_chest_deck())
        {
            if c.id == card_id.0 {
                return (c.title, c.description);
            }
        }
        ("Thẻ sự kiện", "")
    }

    pub fn message(locale: Locale, key: &MessageKey) -> String {
        match (locale, key) {
            (Locale::Vi, MessageKey::RoomAlreadyStarted) => "Ván đấu đã bắt đầu".to_string(),
            (Locale::En, MessageKey::RoomAlreadyStarted) => "Game already started".to_string(),
            (Locale::Vi, MessageKey::RoomFull) => "Phòng đã đầy".to_string(),
            (Locale::En, MessageKey::RoomFull) => "Room is full".to_string(),
            (Locale::Vi, MessageKey::HostOnlyStart) => "Chỉ chủ phòng mới có thể bắt đầu ván đấu".to_string(),
            (Locale::En, MessageKey::HostOnlyStart) => "Only the host can start the game".to_string(),
            (Locale::Vi, MessageKey::HostOnlyAddBot) => "Chỉ chủ phòng mới có thể thêm máy AI".to_string(),
            (Locale::En, MessageKey::HostOnlyAddBot) => "Only the host can add bots".to_string(),
            (Locale::Vi, MessageKey::NeedAtLeastTwoPlayers) => "Cần ít nhất 2 người chơi để bắt đầu".to_string(),
            (Locale::En, MessageKey::NeedAtLeastTwoPlayers) => "At least 2 players are required".to_string(),
            (Locale::Vi, MessageKey::BotName { index }) => format!("Máy AI #{}", index + 1),
            (Locale::En, MessageKey::BotName { index }) => format!("Bot #{}", index + 1),
            (Locale::Vi, MessageKey::Timeout) => "Hết thời gian".to_string(),
            (Locale::En, MessageKey::Timeout) => "Timeout".to_string(),
            (Locale::Vi, MessageKey::Disconnected) => "Mất kết nối".to_string(),
            (Locale::En, MessageKey::Disconnected) => "Disconnected".to_string(),
            (Locale::Vi, MessageKey::GameNotStarted) => "Ván đấu chưa bắt đầu".to_string(),
            (Locale::En, MessageKey::GameNotStarted) => "Game has not started".to_string(),
            (Locale::Vi, MessageKey::GameAlreadyStarted) => "Ván đấu đã diễn ra".to_string(),
            (Locale::En, MessageKey::GameAlreadyStarted) => "Game already in progress".to_string(),
            (Locale::Vi, MessageKey::GameAlreadyOver) => "Ván đấu đã kết thúc".to_string(),
            (Locale::En, MessageKey::GameAlreadyOver) => "Game is already over".to_string(),
            (Locale::Vi, MessageKey::NotYourTurn) => "Chưa tới lượt của bạn".to_string(),
            (Locale::En, MessageKey::NotYourTurn) => "Not your turn".to_string(),
            (Locale::Vi, MessageKey::InvalidPhase) => "Không thể thực hiện ở giai đoạn này".to_string(),
            (Locale::En, MessageKey::InvalidPhase) => "Invalid action for current phase".to_string(),
            (Locale::Vi, MessageKey::InsufficientFunds) => "Không đủ tiền mặt".to_string(),
            (Locale::En, MessageKey::InsufficientFunds) => "Insufficient funds".to_string(),
            (Locale::Vi, MessageKey::PropertyNotPurchasable) => "Ô này không thể mua".to_string(),
            (Locale::En, MessageKey::PropertyNotPurchasable) => "Property is not purchasable".to_string(),
            (Locale::Vi, MessageKey::PropertyAlreadyOwned) => "Tài sản đã có chủ sở hữu".to_string(),
            (Locale::En, MessageKey::PropertyAlreadyOwned) => "Property is already owned".to_string(),
            (Locale::Vi, MessageKey::MustOwnFullDistrict) => "Phải sở hữu trọn quận mới được xây nhà".to_string(),
            (Locale::En, MessageKey::MustOwnFullDistrict) => "Must own full district to build".to_string(),
            (Locale::Vi, MessageKey::PropertyMortgaged) => "Tài sản hoặc quận đang bị thế chấp".to_string(),
            (Locale::En, MessageKey::PropertyMortgaged) => "Property or district is mortgaged".to_string(),
            (Locale::Vi, MessageKey::MustBuildEvenly) => "Phải xây nhà đồng đều trong quận".to_string(),
            (Locale::En, MessageKey::MustBuildEvenly) => "Must build evenly across district".to_string(),
            (Locale::Vi, MessageKey::MustSellEvenly) => "Phải bán nhà đồng đều trong quận".to_string(),
            (Locale::En, MessageKey::MustSellEvenly) => "Must sell evenly across district".to_string(),
            (Locale::Vi, MessageKey::MaxHousesReached) => "Đã đạt số nhà tối đa (khách sạn)".to_string(),
            (Locale::En, MessageKey::MaxHousesReached) => "Maximum houses (hotel) reached".to_string(),
            (Locale::Vi, MessageKey::NoHousesToSell) => "Không có nhà để bán".to_string(),
            (Locale::En, MessageKey::NoHousesToSell) => "No houses to sell".to_string(),
            (Locale::Vi, MessageKey::CannotMortgageWithBuildings) => "Phải bán hết nhà trước khi thế chấp".to_string(),
            (Locale::En, MessageKey::CannotMortgageWithBuildings) => "Cannot mortgage with buildings".to_string(),
            (Locale::Vi, MessageKey::PropertyNotMortgaged) => "Tài sản chưa bị thế chấp".to_string(),
            (Locale::En, MessageKey::PropertyNotMortgaged) => "Property is not mortgaged".to_string(),
            (Locale::Vi, MessageKey::NotInJail) => "Bạn không ở trong trạm giam".to_string(),
            (Locale::En, MessageKey::NotInJail) => "You are not in jail".to_string(),
            (Locale::Vi, MessageKey::NoJailCard) => "Không có thẻ bảo lãnh".to_string(),
            (Locale::En, MessageKey::NoJailCard) => "No get-out-of-jail card".to_string(),
            (Locale::Vi, MessageKey::AuctionNotActive) => "Phiên đấu giá không hoạt động".to_string(),
            (Locale::En, MessageKey::AuctionNotActive) => "Auction is not active".to_string(),
            (Locale::Vi, MessageKey::BidTooLow) => "Giá đặt phải cao hơn giá hiện tại".to_string(),
            (Locale::En, MessageKey::BidTooLow) => "Bid must exceed current bid".to_string(),
            (Locale::Vi, MessageKey::NotEligibleToBid) => "Bạn không thể tham gia đấu giá".to_string(),
            (Locale::En, MessageKey::NotEligibleToBid) => "Not eligible to bid".to_string(),
            (Locale::Vi, MessageKey::InvalidTileIndex) => "Vị trí ô không hợp lệ".to_string(),
            (Locale::En, MessageKey::InvalidTileIndex) => "Invalid tile index".to_string(),
            (Locale::Vi, MessageKey::InvalidPlayerCount) => "Số người chơi không hợp lệ".to_string(),
            (Locale::En, MessageKey::InvalidPlayerCount) => "Invalid player count".to_string(),
            (Locale::Vi, MessageKey::DebtNotExceeded) => "Chưa đủ điều kiện phá sản".to_string(),
            (Locale::En, MessageKey::DebtNotExceeded) => "Cannot declare bankruptcy yet".to_string(),
        }
    }

    pub fn money(locale: Locale, amount: Money) -> String {
        amount.format(locale)
    }
}

fn en_card_text(id: u16) -> Option<(&'static str, &'static str)> {
    Some(match id {
        101 => ("Advance to Nguyen Hue", "Stroll down Nguyen Hue Walking Street."),
        102 => ("Lottery Winner", "You won a Saigon lottery prize. Collect 500,000 VND."),
        103 => ("Flooded Engine Repair", "Pay 1,000,000 VND for engine repairs."),
        104 => ("Traffic Detention", "Go directly to Jail. Do not pass GO."),
        105 => ("Traffic Bail Pass", "Get Out of Jail Free. Keep until needed."),
        106 => ("Rainy Season Repairs", "Pay 250,000 VND per house and 1,000,000 VND per hotel."),
        107 => ("Advance to GO", "Move to Start and collect 2,000,000 VND."),
        108 => ("Metro Project Bonus", "Collect 1,500,000 VND from contract dividend."),
        109 => ("Trip to Saigon Station", "Take a train to Saigon Railway Station."),
        110 => ("Phan Xich Long Food Tour", "Advance to Phan Xich Long."),
        111 => ("Shopping on An Duong Vuong", "Advance to An Duong Vuong."),
        112 => ("Metro Line 1 Ride", "Advance to Ben Thanh Metro."),
        113 => ("Visit Dong Khoi", "Advance to Dong Khoi Street."),
        114 => ("Neighborhood Chair", "Pay each player 500,000 VND."),
        115 => ("Thu Thiem Tunnel Fine", "Pay 150,000 VND speeding fine."),
        116 => ("Savings Interest", "Collect 500,000 VND bank interest."),
        117 => ("Construction Loan Payout", "Collect 1,500,000 VND."),
        118 => ("Walk in Thao Dien", "Advance to Xuan Thuy."),
        201 => ("Lunar New Year Lucky Money", "Collect 200,000 VND from every player."),
        202 => ("Zero-VND Kitchen Charity", "Donate 500,000 VND to charity."),
        203 => ("Year-End Bank Dividend", "Collect 1,000,000 VND."),
        204 => ("Property Insurance Payout", "Collect 1,000,000 VND."),
        205 => ("Tax Evasion Arrest", "Go directly to Jail. Do not pass GO."),
        206 => ("Bail Release Card", "Get Out of Jail Free."),
        207 => ("Housewarming Party", "Pay 500,000 VND."),
        208 => ("Utility Bills", "Pay 250,000 VND for water and power."),
        209 => ("Bank Error in Your Favor", "Collect 2,000,000 VND."),
        210 => ("Income Tax Refund", "Collect 200,000 VND."),
        211 => ("Health Checkup", "Pay 500,000 VND medical fee."),
        212 => ("Tuition Fee", "Pay 500,000 VND."),
        213 => ("Family Inheritance", "Collect 1,000,000 VND."),
        214 => ("Stock Profit", "Collect 500,000 VND."),
        215 => ("Term Deposit Maturity", "Collect 1,000,000 VND."),
        216 => ("Alleyway Upgrade Contribution", "Pay 400,000 VND per house and 1,150,000 VND per hotel."),
        217 => ("Opera House Concert", "Pay 500,000 VND for tickets."),
        218 => ("Saigon Photo Contest Prize", "Collect 100,000 VND."),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tiles_and_cards_resolve_in_vi_and_en() {
        for idx in 0..40u8 {
            assert!(!LocaleCatalog::tile_name(Locale::Vi, idx).is_empty());
            assert!(!LocaleCatalog::tile_name(Locale::En, idx).is_empty());
        }
        for id in (101..=118u16).chain(201..=218u16) {
            let (t_vi, _) = LocaleCatalog::card_text(Locale::Vi, CardId(id));
            let (t_en, _) = LocaleCatalog::card_text(Locale::En, CardId(id));
            assert!(!t_vi.is_empty());
            assert!(!t_en.is_empty());
        }
        assert!(!LocaleCatalog::message(Locale::En, &MessageKey::Timeout).is_empty());
        assert!(!LocaleCatalog::message(Locale::Vi, &MessageKey::Timeout).is_empty());
    }
}
