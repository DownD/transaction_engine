use crate::transaction_engine::TransactionEngine;

mod csv_handler;
mod transaction_engine;

fn main() {
    env_logger::init();
    let path = std::env::args().nth(1).expect("Please provide a file path as the first argument");
    let file = std::fs::File::open(&path).expect("Failed to open file");

    let trasactions = csv_handler::load_csv_file(file);
    let mut transaction_engine = TransactionEngine::default();
    transaction_engine.load_transactions(trasactions);
    csv_handler::write_clients_csv(&transaction_engine);
}
