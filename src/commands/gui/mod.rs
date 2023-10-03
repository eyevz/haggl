mod app;

use clap::Parser;

use app::HagglApp;
use haggl::types::{BaseSymbol, QuoteSymbol, Symbol};

#[derive(Parser)]
pub(crate) struct Args {
    #[arg(value_enum)]
    base_symbol: crate::helpers::ValidBaseSymInputs,
}

pub(crate) fn entry_point(args: Args) {
    gui(BaseSymbol(args.base_symbol.into()));
}

fn gui(base_sym: BaseSymbol) {
    let quote_sym = QuoteSymbol(Symbol::USDT);

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(16.0 * 100.0, 9.0 * 100.0)),
        ..Default::default()
    };

    eframe::run_native(
        "haggl",
        options,
        Box::new(|cc| Box::new(HagglApp::new(cc, base_sym, quote_sym))),
    )
    .unwrap();
}
