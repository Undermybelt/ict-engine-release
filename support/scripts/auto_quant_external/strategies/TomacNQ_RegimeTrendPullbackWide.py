"""
TomacNQ_RegimeTrendPullbackWide — denser NQ pullback-regime candidate.

Paradigm: regime-cluster trend pullback
Hypothesis: The first pullback candidate produced zero trades because the 1d and pullback filters were too strict; a wider 4h-trend pullback should test whether the concept has any reachable density.
Parent: TomacNQ_RegimeTrendPullback
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeTrendPullbackWide(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.027

    trailing_stop = True
    trailing_stop_positive = 0.005
    trailing_stop_positive_offset = 0.012
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 260

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
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 10) & (dataframe["hour_utc"] <= 22)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        trend_context = dataframe["close"] > dataframe["ema89"]
        pullback_zone = dataframe["near_ema21"] <= 1.7
        reacceleration = dataframe["close"] > dataframe["close"].shift(1)
        not_exhausted = (dataframe["rsi"] >= 38) & (dataframe["rsi"] <= 68)
        dataframe.loc[
            liquid_window & higher_trend & trend_context & pullback_zone & reacceleration & not_exhausted,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_pullback = dataframe["close"] < dataframe["ema55"]
        exhausted = dataframe["rsi"] > 76
        dataframe.loc[failed_pullback | exhausted, "exit_long"] = 1
        return dataframe
