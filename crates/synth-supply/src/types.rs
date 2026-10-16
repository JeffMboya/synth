// SPDX-License-Identifier: Apache-2.0

//! Data types for supply-chain stock, pricing, and lifecycle queries.

use serde::{Deserialize, Serialize};

/// Canonicalize a supplier part number for cache keys and request
/// de-duplication. Provider product codes are case-insensitive in this layer.
#[must_use]
pub fn normalize_part_number(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

#[must_use]
pub fn looks_like_lcsc_part_number(value: &str) -> bool {
    let normalized = normalize_part_number(value);
    let digits = normalized.strip_prefix('C').unwrap_or("");
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

/// Component manufacturing lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStatus {
    #[default]
    Active,
    /// Not Recommended for New Designs
    Nrnd,
    /// End-of-Life / Discontinued
    Obsolete,
    Unknown,
}

/// Whether the provider explicitly reported an inventory quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StockStatus {
    InStock,
    OutOfStock,
    #[default]
    Unknown,
}

/// Whether a provider has confirmed that the part can be assembled by the
/// selected manufacturing service. Inventory alone does not establish this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssemblyEligibility {
    Eligible,
    Ineligible,
    #[default]
    Unknown,
}

/// One quantity price break supplied by a distributor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriceTier {
    pub minimum_quantity: u32,
    #[serde(default)]
    pub maximum_quantity: Option<u32>,
    pub unit_price_usd: f64,
}

/// Purchasing constraints that must be known before calculating a purchase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PurchaseConstraints {
    #[serde(default)]
    pub moq: Option<u32>,
    #[serde(default)]
    pub order_multiple: Option<u32>,
    #[serde(default)]
    pub price_tiers: Vec<PriceTier>,
}

impl LifecycleStatus {
    #[must_use]
    pub fn is_problematic(self) -> bool {
        matches!(self, Self::Nrnd | Self::Obsolete)
    }
}

/// Consolidated supply status for a part from a specific distributor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SupplyStatus {
    /// Part number queried (LCSC PN or MPN).
    pub part_number: String,
    /// Distributor name (e.g. `"LCSC"`, `"Nexar"`, `"Mouser"`, `"DigiKey"`).
    pub distributor: String,
    /// True if available stock quantity > 0.
    pub in_stock: bool,
    /// Total available stock quantity.
    pub stock_qty: u64,
    /// Explicit inventory confidence. `stock_qty == 0` is not sufficient to
    /// distinguish confirmed zero stock from an omitted provider field.
    #[serde(default)]
    pub stock_status: StockStatus,
    /// Assembly eligibility is deliberately independent from distributor stock.
    #[serde(default)]
    pub assembly_eligibility: AssemblyEligibility,
    /// Minimum order quantity.
    pub moq: u32,
    /// Quantity constraints and all known price breaks.
    #[serde(default)]
    pub purchase: PurchaseConstraints,
    /// Unit price in USD at minimum order quantity (if available).
    #[serde(default)]
    pub unit_price_usd: Option<f64>,
    /// Component lifecycle status.
    pub lifecycle: LifecycleStatus,
    /// Timestamp when this record was fetched or cached (ISO-8601).
    pub fetched_at: String,
    /// Provider snapshot identity, when the result came from an imported or
    /// atomically persisted catalog snapshot.
    #[serde(default)]
    pub snapshot_id: Option<String>,
}

/// Request payload item when querying supply for a design's BOM.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BomQueryEntry {
    /// Component reference designator (e.g. `"U1"`, `"C5"`).
    pub ref_des: String,
    /// Synth registry Part ID (e.g. `"rp2350"`).
    pub part_id: String,
    /// Manufacturer Part Number (e.g. `"RP2350A"`).
    #[serde(default)]
    pub mpn: Option<String>,
    /// LCSC part number (e.g. `"C14663"`).
    #[serde(default)]
    pub lcsc_pn: Option<String>,
}

/// Error type returned by supply-chain queries.
#[derive(Debug, thiserror::Error)]
pub enum SupplyError {
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Database cache error: {0}")]
    Cache(#[from] rusqlite::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{distributor} provider unavailable: {reason}")]
    ProviderUnavailable { distributor: String, reason: String },

    #[error("no exact {distributor} part matched `{part_number}`")]
    NoExactMatch {
        distributor: String,
        part_number: String,
    },

    #[error("ambiguous {distributor} search for `{part_number}` ({candidate_count} candidates)")]
    AmbiguousMatch {
        distributor: String,
        part_number: String,
        candidate_count: usize,
    },

    #[error("invalid part number `{0}`")]
    InvalidPartNumber(String),

    #[error("No distributor configured or part number empty")]
    InvalidQuery,
}
