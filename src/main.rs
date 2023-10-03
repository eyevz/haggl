mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Inspect(commands::inspect::Args),
    Gui(commands::gui::Args),
}

fn main() {
    let args = Args::parse();
    match args.command {
        Commands::Gui(args) => commands::gui::entry_point(args),
        Commands::Inspect(args) => commands::inspect::entry_point(args),
    };
}

mod helpers {
    use clap::ValueEnum;
    use haggl::types::Symbol;

    #[derive(Copy, Clone, ValueEnum)]
    #[allow(clippy::upper_case_acronyms)]
    pub enum ValidBaseSymInputs {
        BTC,
        ETH,
        SOL,
        XRP,
    }

    impl From<ValidBaseSymInputs> for Symbol {
        fn from(value: ValidBaseSymInputs) -> Self {
            match value {
                ValidBaseSymInputs::BTC => Symbol::BTC,
                ValidBaseSymInputs::ETH => Symbol::ETH,
                ValidBaseSymInputs::SOL => Symbol::SOL,
                ValidBaseSymInputs::XRP => Symbol::XRP,
            }
        }
    }
}
