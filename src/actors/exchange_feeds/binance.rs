use actix::io::SinkWrite;
use actix::{io::WriteHandler, prelude::*};
use actix_codec::Framed;
use anyhow::{anyhow, Result};
use awc::ws::{Codec, Message};
use awc::{error::WsProtocolError, ws::Frame};
use awc::{http, BoxedSocket};
use chrono::NaiveDateTime;
use futures::stream::SplitSink;
use futures::StreamExt;
use serde::Deserialize;

use crate::actors::{brokers::MarketDataMsg, MarketDataBrokerActor};
use crate::types::{BaseSymbol, QuoteSymbol, SpotTrade, Symbol, Taker, Venue};

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct BinanceAggTradeRecord {
    #[serde(skip_deserializing)]
    _e: String,
    #[serde(skip_deserializing)]
    _E: u64,
    #[serde(skip_deserializing)]
    _s: String,
    #[serde(skip_deserializing)]
    _a: u64,
    p: String,
    q: String,
    #[serde(skip_deserializing)]
    _f: u64,
    #[serde(skip_deserializing)]
    _l: u64,
    T: u64,
    m: bool,
    #[serde(skip_deserializing)]
    _M: bool,
}

impl TryFrom<(&BinanceFeedActor, BinanceAggTradeRecord)> for SpotTrade {
    type Error = anyhow::Error;

    fn try_from(value: (&BinanceFeedActor, BinanceAggTradeRecord)) -> Result<Self, Self::Error> {
        Ok(Self {
            venue: Venue::BinanceSpot,
            taker: if value.1.m {
                Taker::Seller
            } else {
                Taker::Buyer
            },
            base_sym: value.0.base_sym,
            quote_sym: value.0.quote_sym,
            base_qty: value.1.q.parse()?,
            price: value.1.p.parse()?,
            ts: NaiveDateTime::from_timestamp_millis(value.1.T.try_into()?)
                .ok_or_else(|| anyhow!("Unexpected value error"))?,
        })
    }
}

type WsTx = SplitSink<Framed<BoxedSocket, Codec>, Message>;
pub struct BinanceFeedActor {
    broker_addr: Addr<MarketDataBrokerActor>,
    base_sym: Symbol,
    quote_sym: Symbol,
    ws: Option<SinkWrite<Message, WsTx>>,
}

impl BinanceFeedActor {
    pub fn new(
        broker_addr: Addr<MarketDataBrokerActor>,
        base_sym: BaseSymbol,
        quote_sym: QuoteSymbol,
    ) -> Self {
        Self {
            broker_addr,
            base_sym: base_sym.0,
            quote_sym: quote_sym.0,
            ws: None,
        }
    }

    fn feed_url(&self) -> String {
        // TODO Review feed type
        //  Aggregated trades should be okay for almost all purposes. Raw trades
        // might be preferable if we end up forming volume bars, where they
        // alleviate (but do not solve) the problem of over- or under- flowing bars.
        let feed_type = "aggTrade"; // "trade" (fills really) or "aggTrade"
        let base_sym = self.base_sym.as_str().to_lowercase();
        let quote_sym = self.quote_sym.as_str().to_lowercase();

        format!("wss://stream.binance.com:443/ws/{base_sym}{quote_sym}@{feed_type}")
    }

    fn try_publish(&self, msg: SpotTrade) -> Result<()> {
        self.broker_addr.try_send(MarketDataMsg::SpotTrade(msg))?;

        Ok(())
    }
}

impl Actor for BinanceFeedActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // TODO Double-check this
        //  Is it possible in the below for messages to be missed between
        // the response when the connection is established and the registering
        // of the stream with add_stream? This is not serious for a trade stream,
        // but orderbook streams may contain important initial snapshots.
        awc::ClientBuilder::new()
            .max_http_version(http::Version::HTTP_11)
            .finish()
            .ws(self.feed_url())
            .connect()
            .into_actor(self)
            .map(|outcome, actor, ctx| match outcome {
                Ok((response, connection)) => {
                    dbg!(response); // TODO Handle appropriately
                    let (sink, stream) = connection.split();
                    actor.ws = Some(SinkWrite::new(sink, ctx));
                    ctx.add_stream(stream);
                }
                Err(e) => {
                    dbg!(e); // TODO Handle appropriately
                }
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        if let Some(mut ws) = self.ws.take() {
            ws.close();
        }

        Running::Stop
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for BinanceFeedActor {
    fn handle(&mut self, item: Result<Frame, WsProtocolError>, _ctx: &mut Self::Context) {
        match item {
            Ok(frame) => match frame {
                Frame::Text(bytes) => {
                    let record: BinanceAggTradeRecord = serde_json::from_slice(&bytes).unwrap();
                    let trade = (&*self, record).try_into().unwrap();
                    if let Err(error) = self.try_publish(trade) {
                        // TODO Report this error to a supervisor - the broker is likely broken
                        dbg!(error);
                    }
                }
                Frame::Ping(bytes) => {
                    // TODO Handle errors
                    // * When self.ws is None (should be impossible, make that clear in code)
                    // * Write error
                    let sink = self.ws.as_mut().unwrap();
                    if let Err(error) = sink.write(Message::Pong(bytes)) {
                        dbg!(error);
                    }
                }
                Frame::Close(reason) => {
                    // TODO Handle appropriately
                    // TODO Detect when the stream is broken, even in the absence of this message
                    dbg!(reason);
                    if let Some(mut ws) = self.ws.take() {
                        ws.close();
                    }
                }
                f => {
                    dbg!(f);
                }
            },
            Err(e) => {
                dbg!(e);
            }
        }
    }
}

impl WriteHandler<WsProtocolError> for BinanceFeedActor {}
