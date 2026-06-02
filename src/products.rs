use crate::error::ScheduleError;
use chrono::NaiveDate;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct ProductMapFile {
    pub coding_system: String,
    pub coding_system_url: Option<String>,
    pub last_verified: Option<NaiveDate>,
    #[serde(default)]
    pub product: Vec<Product>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Product {
    pub code: String,
    pub display: String,
    pub antigens: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProductMap {
    pub coding_system: String,
    by_code: HashMap<String, Product>,
}

impl ProductMap {
    pub fn antigens_for(&self, code: &str) -> Option<&[String]> {
        self.by_code.get(code).map(|p| p.antigens.as_slice())
    }

    pub fn display_for(&self, code: &str) -> Option<&str> {
        self.by_code.get(code).map(|p| p.display.as_str())
    }
}

pub fn load_product_map(path: &Path) -> Result<ProductMap, ScheduleError> {
    let raw = fs::read_to_string(path)?;
    let file: ProductMapFile = toml::from_str(&raw)?;
    let by_code = file
        .product
        .into_iter()
        .map(|p| (p.code.clone(), p))
        .collect();
    Ok(ProductMap {
        coding_system: file.coding_system,
        by_code,
    })
}
