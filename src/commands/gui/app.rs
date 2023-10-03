use std::{
    sync::{mpsc::channel, Arc, Mutex},
    thread::{self, JoinHandle},
};

use actix::{Actor, System};
use chrono::{Local, TimeZone, Utc};
use egui::{CentralPanel, Color32};
use egui_plot::{GridInput, GridMark, Plot, Points};

use haggl::{
    actors::{
        brokers::{MarketDataBrokerActor, MarketDataMsg},
        exchange_feeds::BinanceFeedActor,
    },
    types::{BaseSymbol, QuoteSymbol, SpotTrade, Taker},
};

pub(super) struct HagglApp {
    data: PresentationData,
    system_addr: Option<System>,
    system_thread: Option<JoinHandle<()>>,
    data_thread: Option<JoinHandle<()>>,
}

#[derive(Clone, Default)]
struct RawData {
    spot_trades: Arc<Mutex<Vec<SpotTrade>>>,
}

impl RawData {
    fn add_spot_trade(&self, trade: SpotTrade) {
        self.spot_trades.lock().unwrap().push(trade);
    }
}

#[derive(Clone, Default)]
struct PresentationData {
    spot_buys: Arc<Mutex<Vec<[f64; 2]>>>,
    spot_sells: Arc<Mutex<Vec<[f64; 2]>>>,
}

impl PresentationData {
    #[inline]
    fn add_spot_trade(&self, trade: SpotTrade) {
        match trade.taker {
            Taker::Buyer => &self.spot_buys,
            Taker::Seller => &self.spot_sells,
        }
        .lock()
        .unwrap()
        .push([trade.ts.timestamp_millis() as f64, trade.price]);
    }

    #[inline]
    fn spot_buys(&self) -> Points {
        Points::new(self.spot_buys.lock().unwrap().clone()).color(Color32::GREEN)
    }

    #[inline]
    fn spot_sells(&self) -> Points {
        Points::new(self.spot_sells.lock().unwrap().clone()).color(Color32::RED)
    }
}

impl HagglApp {
    pub fn new(cc: &eframe::CreationContext, base_sym: BaseSymbol, quote_sym: QuoteSymbol) -> Self {
        let raw_data = RawData::default();
        let presentation_data = PresentationData::default();
        let (broker_tx, data_rx) = channel();

        let (system_thread, system_addr) = {
            let (tx, rx) = channel();
            let handle = thread::spawn(move || {
                let runner = System::new();
                let system = System::current();
                let arbiter = system.arbiter();
                arbiter.spawn(async move {
                    let mut broker = MarketDataBrokerActor::default();
                    broker.subscribe_sender(broker_tx);
                    let broker_addr = broker.start();
                    BinanceFeedActor::new(broker_addr, base_sym, quote_sym).start();
                });
                tx.send(system).unwrap(); // We'll need this later to shut down cleanly
                runner.run().unwrap();

                eprintln!("system_thread terminating");
            });

            let system_addr = rx.recv().unwrap();

            (handle, system_addr)
        };

        let data_thread = {
            let presentation_data = presentation_data.clone();
            let egui_ctx = cc.egui_ctx.clone();
            thread::spawn(move || {
                while let Ok(MarketDataMsg::SpotTrade(trade)) = data_rx.recv() {
                    raw_data.add_spot_trade(trade);
                    presentation_data.add_spot_trade(trade);
                    egui_ctx.request_repaint();
                }
                eprintln!("data_thread terminating");
            })
        };

        Self {
            data: presentation_data,
            system_addr: Some(system_addr),
            system_thread: Some(system_thread),
            data_thread: Some(data_thread),
        }
    }

    fn shut_down(&mut self) {
        if let Some(system) = self.system_addr.take() {
            system.stop();
        }
        if let Some(handle) = self.system_thread.take() {
            handle.join().unwrap();
        }
        if let Some(handle) = self.data_thread.take() {
            handle.join().unwrap();
        }
    }
}

impl eframe::App for HagglApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.shut_down();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // TODO Review the x-axis spacer and formatter with larger data ranges.
        // TODO Read up: Since `update()` is a critical path, is better to use free
        //  functions instead of closures (even if they don't capture anything)?

        let grid_spacer = |input: GridInput| -> Vec<GridMark> {
            let range = input.bounds.1 - input.bounds.0;
            // The idea is to choose the smallest "natural" spacing
            // that does not lead to too many ticks.
            let max_ticks = 10.0; // TODO Should depend on plot width and/or config
            let step_size = match range / 1_000.0 {
                r if r <= 0.1 * max_ticks => 0.1,
                r if r <= 1.0 * max_ticks => 1.0,
                r if r <= 5.0 * max_ticks => 5.0,
                r if r <= 15.0 * max_ticks => 15.0,
                r if r <= 30.0 * max_ticks => 30.0,
                r if r <= 60.0 * max_ticks => 60.0,
                r if r <= 5.0 * 60.0 * max_ticks => 5.0 * 60.0,
                r if r <= 15.0 * 60.0 * max_ticks => 15.0 * 60.0,
                r if r <= 30.0 * 60.0 * max_ticks => 30.0 * 60.0,
                // If we fall through to the default the spacing will become
                // unnatural - not aligned neatly on minutes / seconds / whatever.
                // Right now we don't ingest historical data so a user will have
                // to collect live data for at least five hours to fall through.
                // TODO Add more arms
                _ => range / max_ticks,
            } * 1_000.0;

            let mut marks = Vec::new();
            let mut value = ((input.bounds.0 / step_size).ceil()) * step_size;
            while value <= input.bounds.1 {
                marks.push(GridMark { value, step_size });
                value += step_size;
            }

            marks
        };

        let axis_formatter = |value, _max_size, _range: &_| {
            let epoch_millis = value as i64;

            Utc.timestamp_millis_opt(epoch_millis)
                .single()
                .unwrap()
                .with_timezone(&Local)
                .format("%e %b %y %T")
                .to_string()
        };

        CentralPanel::default().show(ctx, |ui| {
            Plot::new("spot-trades")
                .x_grid_spacer(grid_spacer)
                .x_axis_formatter(axis_formatter)
                .show(ui, |plot_ui| {
                    plot_ui.points(self.data.spot_buys());
                    plot_ui.points(self.data.spot_sells());
                });
        });
    }
}
