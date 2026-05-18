use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::state::{load_state_or_default, save_state, TRADE_HISTORY_FILE};

/// File-backed trade history store.
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new(path: &str) -> Self {
        Self {
            path: PathBuf::from(path),
        }
    }

    /// Initialize database directory.
    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.path)?;
        Ok(())
    }

    /// Save trade record into symbol-scoped state.
    pub fn save_trade(&self, trade: &crate::types::TradeRecord) -> Result<()> {
        self.init()?;
        let symbol = format!("{:?}", trade.symbol);
        let mut trades: Vec<crate::types::TradeRecord> =
            load_state_or_default(&self.path, &symbol, TRADE_HISTORY_FILE)?;
        trades.push(trade.clone());
        save_state(&self.path, &symbol, TRADE_HISTORY_FILE, &trades)
    }

    /// Get trade history for a symbol.
    pub fn get_trades(&self, symbol: &str) -> Result<Vec<crate::types::TradeRecord>> {
        self.init()?;
        load_state_or_default(&self.path, symbol, TRADE_HISTORY_FILE)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
