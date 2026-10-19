// SPDX-License-Identifier: Apache-2.0

//! LCSC distributor API adapter.

use crate::distributor::Distributor;
use crate::types::{
    normalize_part_number, AssemblyEligibility, LifecycleStatus, PriceTier, PurchaseConstraints,
    StockStatus, SupplyError, SupplyStatus,
};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;

pub const DEFAULT_LCSC_ENDPOINT: &str = "https://wwwapi.lcsc.com/v1/search/search-product-list";

/// LCSC API client.
#[derive(Debug, Clone)]
pub struct LcscDistributor {
    client: Client,
    endpoint: String,
}

impl LcscDistributor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Synth-EDA/0.0.1 (supply-chain-query)")
                .build()
                .unwrap_or_default(),
            endpoint: DEFAULT_LCSC_ENDPOINT.to_string(),
        }
    }

    #[must_use]
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .user_agent("Synth-EDA/0.0.1 (supply-chain-query)")
                .build()
                .unwrap_or_default(),
            endpoint: endpoint.into(),
        }
    }
}

impl Default for LcscDistributor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct LcscResponse {
    code: Option<i32>,
    result: Option<LcscResult>,
}

#[derive(Debug, Deserialize)]
struct LcscResult {
    #[serde(rename = "productList")]
    product_list: Option<Vec<LcscProduct>>,
}

#[derive(Debug, Deserialize)]
struct LcscProduct {
    #[serde(rename = "productCode")]
    product_code: Option<String>,
    #[serde(rename = "stockNumber")]
    stock_number: Option<u64>,
    #[serde(rename = "minBuyNumber")]
    min_buy_number: Option<u32>,
    #[serde(rename = "orderMultiple")]
    order_multiple: Option<u32>,
    #[serde(rename = "productStatus")]
    product_status: Option<String>,
    #[serde(rename = "productPriceList")]
    product_price_list: Option<Vec<LcscPrice>>,
}

#[derive(Debug, Deserialize)]
struct LcscPrice {
    #[serde(rename = "productPrice")]
    product_price: Option<f64>,
    #[serde(rename = "startNumber", alias = "qFrom")]
    minimum_quantity: Option<u32>,
    #[serde(rename = "endNumber", alias = "qTo")]
    maximum_quantity: Option<u32>,
}

#[async_trait]
impl Distributor for LcscDistributor {
    fn name(&self) -> &'static str {
        "LCSC"
    }

    #[allow(clippy::too_many_lines)]
    async fn query(&self, part_number: &str) -> Result<Option<SupplyStatus>, SupplyError> {
        let pn = normalize_part_number(part_number);
        if pn.is_empty() {
            return Ok(None);
        }

        let url = format!("{}?keyword={}", self.endpoint, urlencoding::encode(&pn));
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(SupplyError::ProviderUnavailable {
                distributor: "LCSC".to_string(),
                reason: format!("HTTP {}", resp.status()),
            });
        }

        let data: LcscResponse = resp.json().await?;
        if data.code != Some(200) {
            return Err(SupplyError::ProviderUnavailable {
                distributor: "LCSC".to_string(),
                reason: format!("API returned code {:?}", data.code),
            });
        }

        let products = match data.result.and_then(|r| r.product_list) {
            Some(list) if !list.is_empty() => list,
            _ => {
                return Err(SupplyError::NoExactMatch {
                    distributor: "LCSC".to_string(),
                    part_number: pn,
                })
            }
        };

        // A search result is not an identity match. Never silently select the
        // first candidate; unresolved candidates must be reviewed explicitly.
        let product = match products.iter().find(|p| {
            p.product_code
                .as_deref()
                .unwrap_or("")
                .eq_ignore_ascii_case(&pn)
        }) {
            Some(product) => product,
            None if products.len() > 1 => {
                return Err(SupplyError::AmbiguousMatch {
                    distributor: "LCSC".to_string(),
                    part_number: pn,
                    candidate_count: products.len(),
                })
            }
            None => {
                return Err(SupplyError::NoExactMatch {
                    distributor: "LCSC".to_string(),
                    part_number: pn,
                })
            }
        };

        let stock_qty = product.stock_number.unwrap_or(0);
        let stock_status = match product.stock_number {
            Some(quantity) if quantity > 0 => StockStatus::InStock,
            Some(_) => StockStatus::OutOfStock,
            None => StockStatus::Unknown,
        };
        let moq = product.min_buy_number.unwrap_or(0);
        let price_tiers = product
            .product_price_list
            .as_ref()
            .map(|prices| {
                prices
                    .iter()
                    .filter_map(|price| {
                        price.product_price.map(|unit_price_usd| PriceTier {
                            minimum_quantity: price.minimum_quantity.unwrap_or(moq.max(1)),
                            maximum_quantity: price.maximum_quantity,
                            unit_price_usd,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let unit_price_usd = product
            .product_price_list
            .as_ref()
            .and_then(|list| list.first())
            .and_then(|p| p.product_price);

        let status_str = product.product_status.as_deref().unwrap_or("");
        let lifecycle = match status_str.to_lowercase().as_str() {
            "discontinued" | "obsolete" | "eol" => LifecycleStatus::Obsolete,
            "not recommended" | "nrnd" => LifecycleStatus::Nrnd,
            "active" | "available" => LifecycleStatus::Active,
            _ => LifecycleStatus::Unknown,
        };

        Ok(Some(SupplyStatus {
            part_number: product.product_code.clone().unwrap_or(pn),
            distributor: "LCSC".to_string(),
            in_stock: matches!(stock_status, StockStatus::InStock),
            stock_qty,
            stock_status,
            assembly_eligibility: AssemblyEligibility::Unknown,
            moq,
            purchase: PurchaseConstraints {
                moq: product.min_buy_number,
                order_multiple: product.order_multiple,
                price_tiers,
            },
            unit_price_usd,
            lifecycle,
            fetched_at: Utc::now().to_rfc3339(),
            snapshot_id: None,
        }))
    }
}
