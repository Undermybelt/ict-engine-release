"""
TomacNQ_RegimeVIXBackwardation15m — VIX term-structure inversion entry, orthogonal regime axis.

Paradigm: regime-cluster term-structure inversion (different regime axis than VIXShockReversal and VVIXDivergence)
Hypothesis: When daily `VIX9D / VIX3M > 1.0`, the front-month vol exceeds the 3-month vol — a classic backwardation regime signal that occurs during stress periods. If during this stress regime NQ holds above a recent support level (close > rolling 5d low + small buffer), the stress is being priced in by vol markets but rejected by index price, suggesting upside reversion as the term-structure normalizes. This candidate uses VIX9D and VIX3M data that no existing pack member uses, on a different regime axis (term-structure vs spot vol level vs vol-of-vol). Combined with VIXShockReversal (vix_z20) and VVIXDivergence (vvix_z20), the basket spans three orthogonal regime feature dimensions.
Parent: orthogonal new geometry (no direct parent in pack)
Created: 2026-05-07
Status: orthogonal-source probe via VIX term-structure inversion
Uses MTF: yes
External data: /tmp/ict-engine-ibkr-probe/vix9d.1d.10y.csv and vix3m.1d.10y.csv (IBKR daily 2018-2026)
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


class TomacNQ_RegimeVIXBackwardation15m(IStrategy):
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
        dataframe["low_5d_15m"] = dataframe["low"].rolling(96 * 5).min().shift(1)
        dataframe["holds_support"] = dataframe["close"] > dataframe["low_5d_15m"] * 1.005
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
        backwardation = dataframe["term_ratio"] > 1.0
        not_collapsing = dataframe["close"] > dataframe["ema89"] * 0.97
        dataframe.loc[
            liquid_window
            & backwardation
            & dataframe["holds_support"]
            & not_collapsing
            & dataframe["bullish_body"],
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        term_normalized = dataframe["term_ratio"] < 0.95
        regime_break = dataframe["close"] < dataframe["ema89"] * 0.97
        upper_target = dataframe["close"] > dataframe["ema21"] * 1.022
        dataframe.loc[
            term_normalized | regime_break | upper_target,
            "exit_long",
        ] = 1
        return dataframe
