use log::warn;
use serde::Deserialize;
use std::fs::File;
use crate::transaction_engine::TransactionEngine;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionTypeRaw {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize)]
pub struct TransactionRaw {
    #[serde(rename = "type")]
    pub transaction_type: TransactionTypeRaw,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<f64>,
}

/// Loads transactions from a CSV file and applies them to the transaction engine.
pub fn load_csv_file(file: File) -> impl Iterator<Item = TransactionRaw> {
    let reader: csv::DeserializeRecordsIntoIter<File, TransactionRaw> = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(file)
        .into_deserialize();
    reader.into_iter().filter_map(|result| {
        match result {
            Ok(transaction) => Some(transaction),
            Err(e) => {
                warn!("Failed to parse a transaction from the CSV file: {}. Skipping invalid record.", e);
                None
            }
        }
    })
}

/// Writes the current state of all clients to standard output in CSV format.
pub fn write_clients_csv(engine: &TransactionEngine) {
    println!("client, available, held, total, locked");
    for client_info in engine.clients() {
        let client_id = client_info.client_id;
        println!("{}, {:.4}, {:.4}, {:.4}, {}", client_id, client_info.available, client_info.held, client_info.total, client_info.locked);
    }
}
