use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StyleNode {
    pub radius: f32,
}

impl Default for StyleNode {
    fn default() -> Self {
        Self { radius: 5. }
    }
}
