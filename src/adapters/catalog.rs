use crate::adapters::contract::{ToolCatalogEntry, ToolCatalogParameter};

pub fn read_only_market_data_stub_catalog() -> Vec<ToolCatalogEntry> {
    vec![
        ToolCatalogEntry {
            name: "stub-ticker".to_string(),
            provider: "stub".to_string(),
            operation: "ticker.fetch".to_string(),
            description: "Read-only ticker snapshot from stub provider.".to_string(),
            auth_required: false,
            dangerous: false,
            asset_classes: vec!["crypto".to_string(), "forex".to_string()],
            parameters: vec![ToolCatalogParameter {
                name: "symbol".to_string(),
                param_type: "string".to_string(),
                required: true,
                description: "Market symbol".to_string(),
            }],
            notes: vec![
                "stdout_json_only".to_string(),
                "read_only".to_string(),
                "safe_for_agent_testing".to_string(),
            ],
        },
        ToolCatalogEntry {
            name: "stub-ohlc".to_string(),
            provider: "stub".to_string(),
            operation: "ohlc.fetch".to_string(),
            description: "Read-only OHLC snapshot from stub provider.".to_string(),
            auth_required: false,
            dangerous: false,
            asset_classes: vec!["crypto".to_string(), "forex".to_string()],
            parameters: vec![
                ToolCatalogParameter {
                    name: "symbol".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Market symbol".to_string(),
                },
                ToolCatalogParameter {
                    name: "interval".to_string(),
                    param_type: "integer".to_string(),
                    required: false,
                    description: "Bar interval in minutes".to_string(),
                },
            ],
            notes: vec![
                "stdout_json_only".to_string(),
                "read_only".to_string(),
                "snapshot_or_replay_equivalent".to_string(),
            ],
        },
        ToolCatalogEntry {
            name: "stub-orderbook".to_string(),
            provider: "stub".to_string(),
            operation: "orderbook.fetch".to_string(),
            description: "Read-only order book snapshot from stub provider.".to_string(),
            auth_required: false,
            dangerous: false,
            asset_classes: vec!["crypto".to_string()],
            parameters: vec![
                ToolCatalogParameter {
                    name: "symbol".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Market symbol".to_string(),
                },
                ToolCatalogParameter {
                    name: "depth".to_string(),
                    param_type: "integer".to_string(),
                    required: false,
                    description: "Order book depth".to_string(),
                },
            ],
            notes: vec![
                "stdout_json_only".to_string(),
                "read_only".to_string(),
                "for_adapter_contract_validation".to_string(),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_catalog_is_read_only_and_safe() {
        let catalog = read_only_market_data_stub_catalog();
        assert!(!catalog.is_empty());
        assert!(catalog.iter().all(|entry| !entry.auth_required));
        assert!(catalog.iter().all(|entry| !entry.dangerous));
        assert!(catalog.iter().all(|entry| entry.operation.ends_with(".fetch")));
    }
}
