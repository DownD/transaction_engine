use std::collections::HashMap;
use std::collections::BTreeMap;
use log::trace;
use crate::csv_handler::TransactionRaw;
use crate::csv_handler::TransactionTypeRaw;

type ClientID = u16;
type TransactionID = u32;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    Disputed,
    ChargedBack
}

#[derive(Debug)]
struct Transaction {
    state: State,
    amount: f64, // Negative if it's a withdrawal and positive if it's a deposit
}

#[derive(Debug)]
struct ClientFunds {
    available: f64,
    held: f64,
    locked: bool,
    transactions: BTreeMap<TransactionID, Transaction>
}

impl Default for ClientFunds {
    fn default() -> Self {
        ClientFunds {
            available: 0.0,
            held: 0.0,
            locked: false,
            transactions: BTreeMap::new()
        }
    }
}

impl ClientFunds {
    #[inline]
    pub fn load_deposit(&mut self, amount: f64, transaction_id: u32) {
        self.available += amount;

        self.transactions.insert(transaction_id, Transaction {
            state: State::Normal,
            amount
        });
    }

    #[inline]
    pub fn load_withdrawal(&mut self, client_id: u16, amount: f64, transaction_id: u32) {
        if self.available < amount {
            trace!("Client {} has insufficient funds for withdrawal of amount {}. Available: {}", client_id, amount, self.available);
            return;
        }
        self.available -= amount;

        self.transactions.insert(transaction_id, Transaction {
            state: State::Normal,
            amount: -amount
        });
    }

    #[inline]
    pub fn load_dispute(&mut self, client_id: u16, ref_transaction_id: u32) {
        if let Some(transaction) = self.transactions.get_mut(&ref_transaction_id) {
            if transaction.state != State::Normal {
                trace!("Transaction {} for client {} is not in a normal state and cannot be disputed.", ref_transaction_id, client_id);
                return;
            }

            if transaction.amount < 0.0 {
                trace!("Transaction {} for client {} is a withdrawal and cannot be disputed.", ref_transaction_id, client_id);
                return;
            }
            
            if transaction.amount > self.available {
                trace!("Client {} has insufficient available funds to dispute transaction {}. Available: {}, Transaction Amount: {}", client_id, ref_transaction_id, self.available, transaction.amount);
                return;
            }
            transaction.state = State::Disputed;
            self.available -= transaction.amount;
            self.held += transaction.amount;
        } else {
            trace!("Transaction {} for client {} not found for dispute.", ref_transaction_id, client_id);
        }
    }

    #[inline]
    pub fn load_resolve(&mut self, client_id: u16, ref_transaction_id: u32) {
        if let Some(transaction) = self.transactions.get_mut(&ref_transaction_id) {
            if transaction.state != State::Disputed {
                trace!("Transaction {} for client {} is not in a disputed state and cannot be resolved.", ref_transaction_id, client_id);
                return;
            }
            transaction.state = State::Normal;
            self.available += transaction.amount;
            self.held -= transaction.amount;
        } else {
            trace!("Transaction {} for client {} not found for resolve.", ref_transaction_id, client_id);
        }
    }

    #[inline]
    pub fn load_chargeback(&mut self, client_id: u16, ref_transaction_id: u32) {
        if let Some(transaction) = self.transactions.get_mut(&ref_transaction_id) {
            if transaction.state != State::Disputed {
                trace!("Transaction {} for client {} is not in a disputed state and cannot be chargebacked.", ref_transaction_id, client_id);
                return;
            }
            transaction.state = State::ChargedBack;
            self.held -= transaction.amount;
            self.locked = true;
        } else {
            trace!("Transaction {} for client {} not found for chargeback.", ref_transaction_id, client_id);
        }
    }
}

#[derive(Debug)]
pub struct ClientInfo {
    pub client_id: ClientID,
    pub total: f64,
    pub available: f64,
    pub held: f64,
    pub locked: bool
}

/// The transaction engine, responsible for processing transactions
/// and maintaining client states and balances.
#[derive(Debug, Default)]
pub struct TransactionEngine {
    clients: HashMap<ClientID, ClientFunds>,
}

impl TransactionEngine {

    pub fn load_transactions(&mut self, transactions: impl Iterator<Item = TransactionRaw>) {
        for transaction in transactions {
            let client_funds = self.clients.entry(transaction.client).or_default();
            if client_funds.locked {
                trace!("Client {} is locked. Skipping transaction {}.", transaction.client, transaction.tx);
                continue;
            }
            match transaction.transaction_type {
                TransactionTypeRaw::Deposit => {
                    if let Some(amount) = transaction.amount {
                        client_funds.load_deposit(amount, transaction.tx);
                    } else {
                        trace!("Deposit transaction {} for client {} is missing an amount.", transaction.tx, transaction.client);
                    }
                },
                TransactionTypeRaw::Withdrawal => {
                    if let Some(amount) = transaction.amount {
                        client_funds.load_withdrawal(transaction.client, amount, transaction.tx);
                    } else {
                        trace!("Withdrawal transaction {} for client {} is missing an amount.", transaction.tx, transaction.client);
                    }
                },
                TransactionTypeRaw::Dispute => {
                    client_funds.load_dispute(transaction.client, transaction.tx);
                },
                TransactionTypeRaw::Resolve => {
                    client_funds.load_resolve(transaction.client, transaction.tx);
                },
                TransactionTypeRaw::Chargeback => {
                    client_funds.load_chargeback(transaction.client, transaction.tx);
                },
            }
        }
    }

    pub fn clients(&self) -> impl Iterator<Item = ClientInfo> + '_ {
        self.clients.iter().map(|(&client_id, funds)| ClientInfo {
            client_id,
            available: funds.available,
            held: funds.held,
            total: funds.available + funds.held,
            locked: funds.locked
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispute_valid() {
        let mut client_funds = ClientFunds::default();
        client_funds.load_deposit(100.0, 1);
        client_funds.load_dispute(1, 1);

        assert_eq!(client_funds.available, 0.0);
        assert_eq!(client_funds.held, 100.0);
        assert_eq!(client_funds.locked, false);
    }

    #[test]
    fn test_dispute_invalid_after_withdrawal() {
        let mut client_funds = ClientFunds::default();
        client_funds.load_deposit(100.0, 1);
        client_funds.load_withdrawal(1, 50.0, 2);
        client_funds.load_dispute(1, 1);

        assert_eq!(client_funds.available, 50.0);
        assert_eq!(client_funds.held, 0.0);
        assert_eq!(client_funds.locked, false);
    }

    #[test]
    fn test_dispute_invalid_transaction() {
        let mut client_funds = ClientFunds::default();
        client_funds.load_deposit(100.0, 1);
        client_funds.load_dispute(1, 2);
        
        assert_eq!(client_funds.available, 100.0);
        assert_eq!(client_funds.held, 0.0);
        assert_eq!(client_funds.locked, false);
    }

    #[test]
    fn test_dispute_invalid_state() {
        let mut client_funds = ClientFunds::default();
        client_funds.load_deposit(100.0, 1);
        client_funds.load_dispute(1, 1);
        client_funds.load_dispute(1, 1);
        
        assert_eq!(client_funds.available, 0.0);
        assert_eq!(client_funds.held, 100.0);
        assert_eq!(client_funds.locked, false);
    }

    #[test]
    fn test_resolve_valid() {
        let mut client_funds = ClientFunds::default();
        client_funds.load_deposit(100.0, 1);
        client_funds.load_dispute(1, 1);
        client_funds.load_resolve(1, 1);

        assert_eq!(client_funds.available, 100.0);
        assert_eq!(client_funds.held, 0.0);
        assert_eq!(client_funds.locked, false);
    }

    #[test]
    fn test_resolve_invalid_transaction() {
        let mut client_funds = ClientFunds::default();
        client_funds.load_deposit(100.0, 1);
        client_funds.load_dispute(1, 1);
        client_funds.load_resolve(1, 2);
        
        assert_eq!(client_funds.available, 0.0);
        assert_eq!(client_funds.held, 100.0);
        assert_eq!(client_funds.locked, false);
    }

    #[test]
    fn test_resolve_invalid_state() {
        let mut client_funds = ClientFunds::default();
        client_funds.load_deposit(100.0, 1);
        client_funds.load_resolve(1, 1);

        assert_eq!(client_funds.available, 100.0);
        assert_eq!(client_funds.held, 0.0);
        assert_eq!(client_funds.locked, false);
    }

    #[test]
    fn test_chargeback_valid() {
        let mut client_funds = ClientFunds::default();
        client_funds.load_deposit(100.0, 1);
        client_funds.load_dispute(1, 1);
        client_funds.load_chargeback(1, 1);
        
        assert_eq!(client_funds.available, 0.0);
        assert_eq!(client_funds.held, 0.0);
        assert_eq!(client_funds.locked, true);
    }

    #[test]
    fn test_chargeback_invalid_transaction() {
        let mut client_funds = ClientFunds::default();
        client_funds.load_deposit(100.0, 1);
        client_funds.load_dispute(1, 1);
        client_funds.load_chargeback(1, 2);

        assert_eq!(client_funds.available, 0.0);
        assert_eq!(client_funds.held, 100.0);
        assert_eq!(client_funds.locked, false);
    }

    #[test]
    fn test_chargeback_invalid_state() {
        let mut client_funds = ClientFunds::default();
        client_funds.load_deposit(100.0, 1);
        client_funds.load_chargeback(1, 1);

        assert_eq!(client_funds.available, 100.0);
        assert_eq!(client_funds.held, 0.0);
        assert_eq!(client_funds.locked, false);
    }

    #[test]
    fn test_locked_account_blocks_transactions() {
        let mut engine = TransactionEngine::default();
        
        // Create transactions for client 1
        let transactions = vec![
            TransactionRaw {
                transaction_type: TransactionTypeRaw::Deposit,
                client: 1,
                tx: 1,
                amount: Some(100.0),
            },
            TransactionRaw {
                transaction_type: TransactionTypeRaw::Dispute,
                client: 1,
                tx: 1,
                amount: None,
            },
            TransactionRaw {
                transaction_type: TransactionTypeRaw::Chargeback,
                client: 1,
                tx: 1,
                amount: None,
            },
            // These should be blocked because account is locked
            TransactionRaw {
                transaction_type: TransactionTypeRaw::Deposit,
                client: 1,
                tx: 2,
                amount: Some(50.0),
            },
            TransactionRaw {
                transaction_type: TransactionTypeRaw::Withdrawal,
                client: 1,
                tx: 3,
                amount: Some(25.0),
            },
        ];
        
        engine.load_transactions(transactions.into_iter());
        
        // Get client info
        let client_info: Vec<_> = engine.clients().collect();
        assert_eq!(client_info.len(), 1);
        
        let client = &client_info[0];
        assert_eq!(client.client_id, 1);
        assert_eq!(client.available, 0.0); // Should remain 0 after chargeback
        assert_eq!(client.held, 0.0);
        assert_eq!(client.total, 0.0);
        assert_eq!(client.locked, true);
    }
}