"""
TomacNQ_RegimeSweepHighVIX15m — VIX-gated high-volatility sub-regime of SweepReclaim15mWide.

Paradigm: regime-cluster mean-reversion / liquidity sweep, vol-regime-gated (high VIX)
Hypothesis: Mirror of TomacNQ_RegimeSweepLowVIX15m. Gates entry to days when daily VIX close is `>= 22`, capturing intraday sweeps in high-stress environments where liquidity grabs may be more meaningful (genuine forced liquidations rather than noise) but reversal odds may be lower (stress regimes also produce sustained continuation moves). Together with the LowVIX sibling this partitions the parent's 62 trades into two regime sub-buckets; if Sharpe in one bucket is materially higher than the parent's 2.68, the gate isolates a stronger sub-edge. If both buckets are comparable to each other but lower than parent, neither is the answer.
Parent: TomacNQ_RegimeLiquiditySweepReclaim15mWide
Created: 2026-05-07
Status: vol-regime-gated probe
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


class TomacNQ_RegimeSweepHighVIX15m(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "15m"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.022

    trailing_stop = True
    trailing_stop_positive = 0.005
    trailing_stop_positive_offset = 0.012
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 280

    @informative("1h")
    def populate_indicators_1h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        return dataframe

    @informative("4h")
    def populate_indicators_4h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema21"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["low_12bar"] = dataframe["low"].rolling(12).min().shift(1)
        dataframe["low_2bar"] = dataframe["low"].rolling(2).min()
        dataframe["sweep_below"] = dataframe["low_2bar"] < dataframe["low_12bar"]
        dataframe["reclaim_close"] = dataframe["close"] > dataframe["low_12bar"]
        dataframe["body_strength"] = (dataframe["close"] - dataframe["open"]) / dataframe["atr"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        vix = _load_vix_series()
        candle_dates = pd.to_datetime(dataframe["date"], utc=True).dt.normalize()
        dataframe["vix_close"] = candle_dates.map(vix).ffill()
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 8) & (dataframe["hour_utc"] <= 22)
        higher_trend_4h = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        higher_trend_1h = dataframe["ema_fast_1h"] > dataframe["ema_slow_1h"]
        clean_reclaim = dataframe["sweep_below"] & dataframe["reclaim_close"]
        confirmation_body = dataframe["body_strength"] > 0.25
        high_vix_regime = dataframe["vix_close"] >= 22.0
        dataframe.loc[
            liquid_window
            & (higher_trend_4h | higher_trend_1h)
            & clean_reclaim
            & confirmation_body
            & high_vix_regime,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_reclaim = dataframe["close"] < dataframe["low_12bar"]
        upper_overshoot = dataframe["close"] > dataframe["ema21"] * 1.030
        trend_break_4h = dataframe["ema_fast_4h"] < dataframe["ema_slow_4h"]
        dataframe.loc[failed_reclaim | upper_overshoot | trend_break_4h, "exit_long"] = 1
        return dataframe
