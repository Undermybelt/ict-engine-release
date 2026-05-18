"""
TomacNQ_RegimeTrendPullback — NQ 1h regime-persistence pullback candidate.

Paradigm: regime-cluster trend pullback
Hypothesis: A pullback that holds above the 1h EMA55 while the 4h and 1d trend regimes stay positive should produce denser Layer 4 regime-persistence evidence than the sparse killzone breakout seed.
Parent: TomacNQ_KillzoneBreakout
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeTrendPullback(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.025

    trailing_stop = True
    trailing_stop_positive = 0.006
    trailing_stop_positive_offset = 0.013
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 320

    @informative("4h")
    def populate_indicators_4h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["adx"] = ta.ADX(dataframe, timeperiod=14)
        return dataframe

    @informative("1d")
    def populate_indicators_1d(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema50"] = ta.EMA(dataframe, timeperiod=50)
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema21"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema55"] = ta.EMA(dataframe, timeperiod=55)
        dataframe["rsi"] = ta.RSI(dataframe, timeperiod=14)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        dataframe["pullback_depth"] = (dataframe["ema21"] - dataframe["close"]).abs() / dataframe["atr"]
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 13) & (dataframe["hour_utc"] <= 20)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        daily_trend = dataframe["close"] > dataframe["ema50_1d"]
        controlled_pullback = (dataframe["close"] > dataframe["ema55"]) & (dataframe["pullback_depth"] <= 1.15)
        persistence = (dataframe["rsi"] >= 42) & (dataframe["rsi"] <= 64) & (dataframe["adx_4h"] >= 14)
        dataframe.loc[
            liquid_window & higher_trend & daily_trend & controlled_pullback & persistence,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        trend_break = dataframe["close"] < dataframe["ema55"]
        overextended = dataframe["rsi"] > 74
        higher_break = dataframe["ema_fast_4h"] < dataframe["ema_slow_4h"]
        dataframe.loc[trend_break | overextended | higher_break, "exit_long"] = 1
        return dataframe
