"""
TomacNQ_RegimeVIXShockReversalWide15m — structurally widened VIXShockReversal candidate.

Paradigm: regime-cluster vol-shock mean reversion, density-widened orthogonal entry
Hypothesis: Slice 88's `VIXShockReversal15m` produced exceptional per-trade quality (Sharpe 1.80 / PF 3.72 / WR 85.7% / Sortino infinity) on only 7 trades, with correlation 0.030 to the existing SweepReclaim parent — orthogonal but anecdotal. The Wide variant lowers each gate by roughly one selectivity step: vix_z20 threshold 1.2 -> 0.8 (rare-shock -> elevated-shock, captures ~2x more days), pullback threshold -0.5% -> -0.2% (any correction qualifies), and drops the "first bullish bar after shock" first-fire requirement so that subsequent days while the regime persists can also enter. If most of the parent's edge survives at higher density, the candidate becomes the second high-Sharpe orthogonal candidate the basket needs to cross best-standalone.
Parent: TomacNQ_RegimeVIXShockReversal15m
Created: 2026-05-07
Status: density-widening probe of orthogonal vol-shock entry
Uses MTF: yes
External data: /tmp/ict-engine-ibkr-probe/vix.1d.10y.csv (IBKR daily VIX, 2018-2026)
"""
from __future__ import annotations

from pathlib import Path

import pandas as pd
import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame

_VIX_CSV = Path("/tmp/ict-engine-ibkr-probe/vix.1d.10y.csv")
_VIX_SERIES_CACHE: pd.Series | None = None


def _load_vix_series() -> pd.Series:
    global _VIX_SERIES_CACHE
    if _VIX_SERIES_CACHE is None:
        df = pd.read_csv(_VIX_CSV)
        df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
        df = df.dropna(subset=["ts", "close"])
        df["date"] = df["ts"].dt.normalize()
        s = df.set_index("date")["close"].astype(float)
        _VIX_SERIES_CACHE = s[~s.index.duplicated(keep="last")].sort_index()
    return _VIX_SERIES_CACHE


def _vix_z20_series() -> pd.Series:
    vix = _load_vix_series()
    mean20 = vix.rolling(20, min_periods=10).mean()
    std20 = vix.rolling(20, min_periods=10).std()
    return ((vix - mean20) / std20.where(std20 > 1e-9)).rename("vix_z20")


class TomacNQ_RegimeVIXShockReversalWide15m(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "15m"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.025

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
        dataframe["high_5d_15m"] = dataframe["high"].rolling(96 * 5).max().shift(1)
        dataframe["pullback_pct"] = (
            dataframe["close"] / dataframe["high_5d_15m"] - 1.0
        )
        dataframe["bullish_body"] = dataframe["close"] > dataframe["open"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        vix_z20 = _vix_z20_series()
        candle_dates = pd.to_datetime(dataframe["date"], utc=True).dt.normalize()
        dataframe["vix_z20"] = candle_dates.map(vix_z20).ffill()
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 13) & (dataframe["hour_utc"] <= 21)
        vix_elevated = dataframe["vix_z20"] > 0.8
        nq_corrected = dataframe["pullback_pct"] < -0.002
        not_collapsing = dataframe["close"] > dataframe["ema89"] * 0.97
        dataframe.loc[
            liquid_window
            & vix_elevated
            & nq_corrected
            & not_collapsing
            & dataframe["bullish_body"],
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        vix_normalized = dataframe["vix_z20"] < 0.3
        regime_break = dataframe["close"] < dataframe["ema89"] * 0.97
        upper_target = dataframe["close"] > dataframe["ema21"] * 1.025
        dataframe.loc[
            vix_normalized | regime_break | upper_target,
            "exit_long",
        ] = 1
        return dataframe
