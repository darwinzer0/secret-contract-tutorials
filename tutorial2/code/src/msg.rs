use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{HumanAddr,};
use crate::viewing_key::ViewingKey;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// Maximum size of a reminder message in bytes
    pub max_size: i32,
    /// User supplied entropy string for pseudorandom number generator seed
    pub prng_seed: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Records a new reminder for the sender
    Record {
        reminder: String,
    },
    /// Requests the current reminder for the sender
    Read { },
    /// Generates a new viewing key with user supplied entropy
    GenerateViewingKey {
        entropy: String,
        padding: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Gets basic statistics about the use of the contract
    Stats { },
    /// Read implemented as an authenticated query
    Read {
        address: HumanAddr,
        key: String,
    }
}

impl QueryMsg {
    pub fn get_validation_params(&self) -> (Vec<&HumanAddr>, ViewingKey) {
        match self {
            Self::Read { address, key, .. } => (vec![address], ViewingKey(key.clone())),
            _ => panic!("This query type does not require authentication"),
        }
    }
}

/// Responses from handle functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    /// Return a status message to let the user know if it succeeded or failed
    Record {
        status: String,
    },
    /// Return a status message and the current reminder and its timestamp, if it exists
    Read {
        status: String,
        reminder: Option<String>,
        timestamp: Option<u64>,
    },
    /// Return the generated key
    GenerateViewingKey {
        key: ViewingKey,
    },
}

/// Responses from query functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    /// Return basic statistics about contract
    Stats {
        reminder_count: u64,
    },
    /// Return a status message and the current reminder and its timestamp, if it exists
    Read {
        status: String,
        reminder: Option<String>,
        timestamp: Option<u64>,
    },
}

