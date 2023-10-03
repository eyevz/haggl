use std::sync::mpsc::Sender;

use actix::prelude::*;

use crate::types::SpotTrade;

#[derive(Message, Copy, Clone, Debug)]
#[rtype(result = "()")]
pub enum MarketDataMsg {
    SpotTrade(SpotTrade),
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Subscribe(pub Recipient<MarketDataMsg>);

#[derive(Default)]
pub struct MarketDataBrokerActor {
    recipient_subs: Vec<Recipient<MarketDataMsg>>,
    sender_subs: Vec<Sender<MarketDataMsg>>,
}

impl MarketDataBrokerActor {
    pub fn subscribe_recipient(&mut self, subscriber: Recipient<MarketDataMsg>) {
        if !self.recipient_subs.contains(&subscriber) {
            self.recipient_subs.push(subscriber);
        }
    }

    pub fn subscribe_sender(&mut self, subscriber: Sender<MarketDataMsg>) {
        // TODO It is possible to help clients avoid multiple subs on the same channel?
        self.sender_subs.push(subscriber);
    }
}

impl Actor for MarketDataBrokerActor {
    type Context = Context<Self>;
}

impl Handler<MarketDataMsg> for MarketDataBrokerActor {
    type Result = ();

    fn handle(&mut self, msg: MarketDataMsg, _ctx: &mut Context<Self>) -> Self::Result {
        // TODO Handle the errors below appropriately

        for subscriber in &self.recipient_subs {
            if let Err(e) = subscriber.try_send(msg) {
                dbg!(e);
            }
        }

        for subscriber in &self.sender_subs {
            if let Err(e) = subscriber.send(msg) {
                dbg!(e);
            }
        }
    }
}

impl Handler<Subscribe> for MarketDataBrokerActor {
    type Result = ();

    fn handle(&mut self, msg: Subscribe, _ctx: &mut Context<Self>) -> Self::Result {
        self.subscribe_recipient(msg.0);
    }
}
