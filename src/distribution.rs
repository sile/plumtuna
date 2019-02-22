#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Distribution {
    Uniform { low: f64, high: f64 },
    LogUniform { low: f64, high: f64 },
    DiscreteUniform { low: f64, high: f64, q: f64 },
    IntUniform { low: i64, high: i64 },
    Categorical { choices: Vec<Category> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Category {
    Str(String),
    Float(f64),
}
