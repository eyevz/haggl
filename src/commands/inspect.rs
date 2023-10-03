use std::{sync::mpsc::channel, thread};

use actix::{Actor, Arbiter, System};
use clap::Parser;
use haggl::{
    actors::{
        brokers::{MarketDataBrokerActor, MarketDataMsg},
        exchange_feeds::BinanceFeedActor,
    },
    types::{BaseSymbol, QuoteSymbol, Symbol},
};

#[derive(Parser)]
pub(crate) struct Args {
    #[arg(value_enum)]
    base_symbol: crate::helpers::ValidBaseSymInputs,
}

pub(crate) fn entry_point(args: Args) {
    inspect(BaseSymbol(args.base_symbol.into()))
}

fn inspect(base_sym: BaseSymbol) {
    let quote_sym = QuoteSymbol(Symbol::USDT);

    let (tx, rx) = channel();

    let system_handle = thread::spawn(move || {
        let system = System::new();
        let arbiter = Arbiter::current();
        arbiter.spawn(async move {
            let mut broker = MarketDataBrokerActor::default();
            broker.subscribe_sender(tx);
            let broker_addr = broker.start();
            BinanceFeedActor::new(broker_addr, base_sym, quote_sym).start();
        });

        system.run().unwrap();
    });

    loop {
        match rx.recv() {
            Ok(MarketDataMsg::SpotTrade(trade)) => {
                println!("{trade:?}");
            }
            Err(e) => {
                dbg!(e);
                break;
            }
        }
    }

    system_handle.join().unwrap();
}
