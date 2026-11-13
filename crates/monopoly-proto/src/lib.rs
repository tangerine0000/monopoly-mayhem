use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RoomId(pub String);

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PlayerId(pub Uuid);

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionToken(pub String);

impl fmt::Display for SessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
pub struct Money(pub i64); // 1 unit = 1,000 VNĐ

impl Money {
    pub const ZERO: Money = Money(0);

    pub fn from_vnd(vnd: i64) -> Self {
        Money(vnd / 1_000)
    }

    pub fn as_vnd(&self) -> i64 {
        self.0 * 1_000
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vnd = self.as_vnd();
        let is_negative = vnd < 0;
        let abs_val = vnd.unsigned_abs();
        let s = abs_val.to_string();

        let mut formatted = String::new();
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();

        for (i, ch) in chars.iter().enumerate() {
            if i > 0 && (len - i).is_multiple_of(3) {
                formatted.push('.');
            }
            formatted.push(*ch);
        }

        if is_negative {
            write!(f, "-{} đ", formatted)
        } else {
            write!(f, "{} đ", formatted)
        }
    }
}

impl std::ops::Add for Money {
    type Output = Money;
    fn add(self, rhs: Self) -> Self::Output {
        Money(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Self) -> Self::Output {
        Money(self.0 - rhs.0)
    }
}

impl std::ops::AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl std::ops::SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    JoinRoom {
        player_name: String,
        session_token: Option<SessionToken>,
    },
    RollDice,
    BuyProperty,
    DeclineProperty,
    PlaceBid {
        amount: Money,
    },
    PassAuction,
    BuildHouse {
        tile_index: u8,
    },
    SellHouse {
        tile_index: u8,
    },
    MortgageProperty {
        tile_index: u8,
    },
    UnmortgageProperty {
        tile_index: u8,
    },
    PayJailBail,
    UseJailCard,
    DeclareBankruptcy,
    EndTurn,
    ConfigureGame {
        enable_auction: bool,
        #[serde(default)]
        max_players: Option<u8>,
    },
    StartGame,
    AddBot,
    /// Trả khoản nợ đang treo (sau khi đã bán nhà / thế chấp đủ tiền).
    PayDebt,
    /// Người đang tới lượt đề nghị trao đổi đất và tiền với một người chơi khác.
    ProposeTrade {
        to: PlayerId,
        give_tiles: Vec<u8>,
        give_cash: Money,
        take_tiles: Vec<u8>,
        take_cash: Money,
    },
    /// Người nhận đề nghị đồng ý hoặc từ chối.
    RespondTrade {
        accept: bool,
    },
    /// Người đề nghị rút lại đề nghị.
    CancelTrade,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    RoomJoined {
        room_id: RoomId,
        player_id: PlayerId,
        session_token: SessionToken,
        seat_index: u8,
    },
    GameStarted {
        turn_order: Vec<PlayerId>,
    },
    TurnStarted {
        active_player: PlayerId,
        time_limit_seconds: u32,
    },
    TimerTick {
        seconds_remaining: u32,
    },
    DiceRolled {
        player_id: PlayerId,
        dice: (u8, u8),
        new_tile_index: u8,
        is_doubles: bool,
    },
    PropertyAcquired {
        player_id: PlayerId,
        tile_index: u8,
        cost: Money,
    },
    RentPaid {
        payer: PlayerId,
        recipient: PlayerId,
        tile_index: u8,
        amount: Money,
    },
    AuctionStarted {
        tile_index: u8,
        opening_bid: Money,
    },
    BidPlaced {
        bidder: PlayerId,
        amount: Money,
    },
    AuctionConcluded {
        winner: Option<PlayerId>,
        final_price: Money,
    },
    HouseModified {
        tile_index: u8,
        house_count: u8,
    },
    PropertyMortgageChanged {
        tile_index: u8,
        is_mortgaged: bool,
    },
    CardDrawn {
        player_id: PlayerId,
        deck: String, // "Cơ Hội" | "Khí Vận"
        description: String,
        effect_summary: String,
    },
    PlayerBankrupt {
        player_id: PlayerId,
    },
    BotAssumedControl {
        player_id: PlayerId,
        reason: String, // "Timeout" | "Disconnected"
    },
    PlayerReconnected {
        player_id: PlayerId,
    },
    SyncSnapshot {
        state: serde_json::Value,
    },
    ActionRejected {
        reason: String,
    },
    /// Người chơi không đủ tiền mặt nhưng còn tài sản để gom.
    DebtIncurred {
        debtor: PlayerId,
        creditor: Option<PlayerId>,
        amount: Money,
    },
    TradeProposed {
        from: PlayerId,
        to: PlayerId,
    },
    TradeResolved {
        from: PlayerId,
        to: PlayerId,
        accepted: bool,
    },
    /// Đồng hồ phiên đấu giá (đặt lại mỗi lần có người trả giá).
    AuctionTimer {
        seconds_remaining: u32,
    },
}
