pub mod allin1;
pub mod ssm;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectProgress {
    pub stage: String,
    pub percent: f64,
}
