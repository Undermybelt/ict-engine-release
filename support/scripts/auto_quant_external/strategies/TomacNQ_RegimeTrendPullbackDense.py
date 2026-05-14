"""
TomacNQ_RegimeTrendPullbackDense — density-focused NQ pullback-regime candidate.

Paradigm: regime-cluster trend pullback
Hypothesis: The wide pullback fork reached probe density; a denser variant should test whether the same regime descriptor can cross the thin evidence floor while keeping trend-context and pullback-state semantics.
Parent: TomacNQ_RegimeTrendPullbackWide
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeTrendPullbackDense(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.03

    trailing_stop = True
    trailing_stop_positive = 0.005
    trailing_stop_positive_offset = 0.012
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 240

    @informative("4h")
    def populate_indicators_4h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema21"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema55"] = ta.EMA(dataframe, timeperiod=55)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["rsi"] = ta.RSI(dataframe, timeperiod=14)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["near_ema21"] = (dataframe["close"] - dataframe["ema21"]).abs() / dataframe["atr"]
        dataframe["body_green"] = dataframe["close"] > dataframe["open"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 8) & (dataframe["hour_utc"] <= 23)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        local_trend = (dataframe["ema21"] > dataframe["ema89"]) & (dataframe["close"] > dataframe["ema89"])
        pullback_zone = dataframe["near_ema21"] <= 2.4
        reacceleration = dataframe["body_green"] | (dataframe["close"] > dataframe["close"].shift(1))
        not_exhausted = (dataframe["rsi"] >= 35) & (dataframe["rsi"] <= 74)
        dataframe.loc[
            liquid_window & (higher_trend | local_trend) & pullback_zone & reacceleration & not_exhausted,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        regime_break = dataframe["close"] < dataframe["ema89"]
        exhausted = dataframe["rsi"] > 82
        dataframe.loc[regime_break | exhausted, "exit_long"] = 1
        return dataframe
