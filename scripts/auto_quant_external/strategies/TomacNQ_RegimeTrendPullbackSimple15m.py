"""
TomacNQ_RegimeTrendPullbackSimple15m — diagnostic no-informative variant of TrendPullbackDense15m.

Paradigm: regime-cluster trend pullback, single-timeframe diagnostic
Hypothesis: The Slice 100 entry-drought diagnostic found that TrendPullbackDense15m's entry conditions appear satisfied ~24% of 15m bars across 2018-2025 (per pandas reproduction), but freqtrade fires zero entries in 2024-2025 and only 8 in 2021. The discrepancy may be caused by freqtrade's informative_pairs path: 1h and 4h reindex onto 15m base, NaN propagation across session boundaries, or TA-Lib vs pandas indicator drift in informative columns. This variant strips the 1h and 4h informatives entirely and uses LONGER-PERIOD 15m EMAs (EMA200, EMA600) as in-asset trend proxies. EMA600 on 15m equals roughly 6 trading days of memory, similar to 4h EMA89 (~14 days but still in same regime regime). If this variant fires consistently across 2018-2025, the drought is informative-pairs-related. If it has the same gaps, the issue is deeper.
Parent: TomacNQ_RegimeTrendPullbackDense15m
Created: 2026-05-07
Status: drought-diagnostic candidate
Uses MTF: NO (intentionally — diagnostic isolation)
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy
from pandas import DataFrame


class TomacNQ_RegimeTrendPullbackSimple15m(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "15m"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.022

    trailing_stop = True
    trailing_stop_positive = 0.004
    trailing_stop_positive_offset = 0.010
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 700

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema21"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["ema200"] = ta.EMA(dataframe, timeperiod=200)
        dataframe["ema600"] = ta.EMA(dataframe, timeperiod=600)
        dataframe["rsi"] = ta.RSI(dataframe, timeperiod=14)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["near_ema21"] = (dataframe["close"] - dataframe["ema21"]).abs() / dataframe["atr"]
        dataframe["body_green"] = dataframe["close"] > dataframe["open"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 8) & (dataframe["hour_utc"] <= 23)
        long_trend = dataframe["ema200"] > dataframe["ema600"]
        local_trend = (dataframe["ema21"] > dataframe["ema89"]) & (dataframe["close"] > dataframe["ema89"])
        pullback_zone = dataframe["near_ema21"] <= 2.4
        reacceleration = dataframe["body_green"] | (dataframe["close"] > dataframe["close"].shift(1))
        not_exhausted = (dataframe["rsi"] >= 35) & (dataframe["rsi"] <= 74)
        dataframe.loc[
            liquid_window
            & (long_trend | local_trend)
            & pullback_zone
            & reacceleration
            & not_exhausted,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        regime_break = dataframe["close"] < dataframe["ema200"]
        exhausted = dataframe["rsi"] > 82
        dataframe.loc[regime_break | exhausted, "exit_long"] = 1
        return dataframe
