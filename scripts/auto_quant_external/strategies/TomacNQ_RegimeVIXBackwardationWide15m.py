"""
TomacNQ_RegimeVIXBackwardationWide15m — structurally widened backwardation entry.

Paradigm: regime-cluster term-structure inversion, density-widened
Hypothesis: Slice 90's `VIXBackwardation15m` produced 13 trades / Sharpe 1.76 / WR 76.9% / PF 2.47 on `NQ/USD 15m 3Y`. The Wide variant lowers the term-ratio threshold from `> 1.0` (strict backwardation) to `> 0.97` (near-flat term structure, capturing the regime margin where stress is building but not yet inverted), and drops the `holds_support` price-position gate so any regime-not-collapsing bullish 15m bar in the elevated-term-ratio window can fire. Target `~30-50 trades` while preserving the bulk of the parent's edge; if Sharpe stays above `1.5`, the candidate becomes the second high-Sharpe orthogonal-axis candidate above probe density and the basket should lift further.
Parent: TomacNQ_RegimeVIXBackwardation15m
Created: 2026-05-07
Status: density-widening probe of orthogonal term-structure entry
Uses MTF: yes
External data: /tmp/ict-engine-ibkr-probe/{vix9d,vix3m}.1d.10y.csv
"""
from __future__ import annotations

from pathlib import Path

import pandas as pd
import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame

_VIX9D_CSV = Path("/tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv")
_VIX3M_CSV = Path("/tmp/ict-engine-ibkr-probe/vix3m.1d.10y.csv")
_SERIES_CACHE: dict[str, pd.Series] = {}


def _load_close_series(csv_path: Path, key: str) -> pd.Series:
    if key not in _SERIES_CACHE:
        df = pd.read_csv(csv_path)
        df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
        df = df.dropna(subset=["ts", "close"])
        df["date"] = df["ts"].dt.normalize()
        s = df.set_index("date")["close"].astype(float)
        _SERIES_CACHE[key] = s[~s.index.duplicated(keep="last")].sort_index()
    return _SERIES_CACHE[key]


class TomacNQ_RegimeVIXBackwardationWide15m(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "15m"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.024

    trailing_stop = True
    trailing_stop_positive = 0.005
    trailing_stop_positive_offset = 0.012
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 280

    @informative("4h")
    def populate_indicators_4h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema21"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["bullish_body"] = dataframe["close"] > dataframe["open"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        vix9d = _load_close_series(_VIX9D_CSV, "vix9d")
        vix3m = _load_close_series(_VIX3M_CSV, "vix3m")
        term_ratio_series = (vix9d / vix3m.where(vix3m > 1e-9)).rename("term_ratio")
        candle_dates = pd.to_datetime(dataframe["date"], utc=True).dt.normalize()
        dataframe["term_ratio"] = candle_dates.map(term_ratio_series).ffill()
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 13) & (dataframe["hour_utc"] <= 21)
        elevated_term = dataframe["term_ratio"] > 0.97
        not_collapsing = dataframe["close"] > dataframe["ema89"] * 0.97
        dataframe.loc[
            liquid_window
            & elevated_term
            & not_collapsing
            & dataframe["bullish_body"],
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        term_normalized = dataframe["term_ratio"] < 0.90
        regime_break = dataframe["close"] < dataframe["ema89"] * 0.97
        upper_target = dataframe["close"] > dataframe["ema21"] * 1.022
        dataframe.loc[
            term_normalized | regime_break | upper_target,
            "exit_long",
        ] = 1
        return dataframe
