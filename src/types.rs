use chrono::NaiveDateTime;

#[derive(Copy, Clone, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum Symbol {
    BTC,
    ETH,
    SOL,
    USDT,
    XRP,
}

impl Symbol {
    pub fn as_str(&self) -> &str {
        match self {
            Symbol::BTC => "BTC",
            Symbol::ETH => "ETH",
            Symbol::SOL => "SOL",
            Symbol::USDT => "USDT",
            Symbol::XRP => "XRP",
        }
    }
}

pub struct BaseSymbol(pub Symbol);
pub struct QuoteSymbol(pub Symbol);

#[derive(Copy, Clone, Debug)]
pub enum Venue {
    BinanceSpot,
}

#[derive(Copy, Clone, Debug)]
pub enum Taker {
    Buyer,
    Seller,
}

#[derive(Copy, Clone, Debug)]
pub struct SpotTrade {
    pub venue: Venue,
    pub taker: Taker,
    pub base_sym: Symbol,
    pub quote_sym: Symbol,
    pub base_qty: f64,
    pub price: f64,
    pub ts: NaiveDateTime,
}

impl SpotTrade {
    pub fn quote_qty(&self) -> f64 {
        self.base_qty * self.price
    }
}
