"""
TomacNQ_RegimePersistenceClusterDense — density-focused NQ persistence-cluster candidate.

Paradigm: regime-cluster persistence
Hypothesis: A denser persistence fork should reveal whether EMA-stack persistence can produce thin-or-better observations while still tagging the Layer 4 trend-state regime.
Parent: TomacNQ_RegimePersistenceClusterWide
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimePersistenceClusterDense(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.029

    trailing_stop = True
    trailing_stop_positive = 0.005
    trailing_stop_positive_offset = 0.012
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 220

    @informative("4h")
    def populate_indicators_4h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema13"] = ta.EMA(dataframe, timeperiod=13)
        dataframe["ema34"] = ta.EMA(dataframe, timeperiod=34)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["rsi"] = ta.RSI(dataframe, timeperiod=14)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["ema13_slope"] = dataframe["ema13"].diff(3) / dataframe["atr"]
        dataframe["above_ema13_count"] = (
            (dataframe["close"] > dataframe["ema13"]).rolling(5).sum()
        )
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 8) & (dataframe["hour_utc"] <= 23)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        local_stack = (dataframe["ema13"] > dataframe["ema34"]) & (dataframe["ema34"] > dataframe["ema89"])
        persistence = (dataframe["above_ema13_count"] >= 2) & (dataframe["ema13_slope"] > -0.12)
        not_exhausted = (dataframe["rsi"] >= 43) & (dataframe["rsi"] <= 78)
        dataframe.loc[
            liquid_window & (higher_trend | local_stack) & local_stack & persistence & not_exhausted,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        lost_persistence = dataframe["close"] < dataframe["ema55"] if "ema55" in dataframe else dataframe["close"] < dataframe["ema89"]
        exhausted = dataframe["rsi"] > 82
        dataframe.loc[lost_persistence | exhausted, "exit_long"] = 1
        return dataframe
