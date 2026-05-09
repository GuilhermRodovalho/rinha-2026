use std::{collections::HashMap, sync::LazyLock};

use serde::{Deserialize, Serialize};

use crate::api::Payload;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProccessedTransactionData {
    pub vector: [f32; 14],
    pub label: String,
}

static MCC: LazyLock<HashMap<&str, f32>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("5411", 0.15);
    m.insert("5812", 0.30);
    m.insert("5912", 0.20);
    m.insert("5944", 0.45);
    m.insert("7801", 0.80);
    m.insert("7802", 0.75);
    m.insert("7995", 0.85);
    m.insert("4511", 0.35);
    m.insert("5311", 0.25);
    m.insert("5999", 0.5);
    m
});

impl Payload {
    pub fn vectorize(&self) -> [f32; 14] {
        let mut vector: [f32; 14] = [0.0; 14];

        vector[0] = (self.transaction.amount / 10000.0).clamp(0.0, 1.0);
        vector[1] = (self.transaction.installments as f32 / 12.0).clamp(0.0, 1.0);
        vector[2] = ((self.transaction.amount / self.customer.avg_amount) / 10.0).clamp(0.0, 1.0);
        vector[3] = self.transaction.requested_at.hour() as f32 / 23.0;
        vector[4] = self
            .transaction
            .requested_at
            .weekday()
            .number_days_from_monday() as f32
            / 6.0;

        vector[5] = self.last_transaction.as_ref().map_or(-1.0, |tr| {
            (self.transaction.requested_at - tr.timestamp).whole_minutes() as f32 / 1440.0
        });
        vector[6] = self
            .last_transaction
            .as_ref()
            .map_or(-1.0, |tr| (tr.km_from_current / 1000.0).clamp(0.0, 1.0));
        vector[7] = (self.terminal.km_from_home / 1000.0).clamp(0.0, 1.0);
        vector[8] = (self.customer.tx_count_24h as f32 / 20.0).clamp(0.0, 1.0);
        vector[9] = if self.terminal.is_online { 1.0 } else { 0.0 };
        vector[10] = if self.terminal.card_present { 1.0 } else { 0.0 };
        vector[11] = if !self.customer.known_merchants.contains(&self.merchant.id) {
            1.0
        } else {
            0.0
        };

        vector[12] = match MCC.get(self.merchant.mcc.as_str()) {
            Some(v) => *v,
            None => 0.5,
        };
        vector[13] = self.merchant.avg_amount / 10000.0;

        vector
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vectorize_given_example() {
        let example = r#"{
              "id": "tx-1329056812",
              "transaction":      { "amount": 41.12, "installments": 2, "requested_at": "2026-03-11T18:45:53Z" },
              "customer":         { "avg_amount": 82.24, "tx_count_24h": 3, "known_merchants": ["MERC-003", "MERC-016"] },
              "merchant":         { "id": "MERC-016", "mcc": "5411", "avg_amount": 60.25 },
              "terminal":         { "is_online": false, "card_present": true, "km_from_home": 29.23 },
              "last_transaction": null
            }"#;

        let example_payload = serde_json::from_str::<Payload>(example).unwrap();

        assert!(!example_payload.vectorize().is_empty())
    }
}
