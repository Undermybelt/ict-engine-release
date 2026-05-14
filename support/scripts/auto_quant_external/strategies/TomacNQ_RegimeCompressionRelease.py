"""
TomacNQ_RegimeCompressionRelease — NQ 1h compression-to-release rhythm candidate.

Paradigm: regime-cluster rhythm release
Hypothesis: A rolling Bollinger-width compression followed by directional release supplies rhythm-state and transition evidence for Layer 4 regime clustering while widening trade density beyond strict killzone breakouts.
Parent: TomacNQ_KillzoneBreakout
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeCompressionRelease(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.026

    trailing_stop = True
    trailing_stop_positive = 0.006
    trailing_stop_positive_offset = 0.014
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
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema55"] = ta.EMA(dataframe, timeperiod=55)
        bands = ta.BBANDS(dataframe, timeperiod=20, nbdevup=2.0, nbdevdn=2.0)
        dataframe["bb_upper"] = bands["upperband"]
        dataframe["bb_middle"] = bands["middleband"]
        dataframe["bb_lower"] = bands["lowerband"]
        dataframe["bb_width"] = (dataframe["bb_upper"] - dataframe["bb_lower"]) / dataframe["bb_middle"]
        dataframe["bb_width_p35"] = dataframe["bb_width"].rolling(180).quantile(0.35)
        dataframe["was_compressed"] = (
            dataframe["bb_width"].rolling(10).min() < dataframe["bb_width_p35"].rolling(10).min()
        )
        dataframe["high_8h"] = dataframe["high"].rolling(8).max().shift(1)
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 12) & (dataframe["hour_utc"] <= 21)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        release = (dataframe["close"] > dataframe["bb_upper"]) | (dataframe["close"] > dataframe["high_8h"])
        trend_context = dataframe["close"] > dataframe["ema55"]
        dataframe.loc[
            liquid_window & higher_trend & dataframe["was_compressed"] & release & trend_context,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        mean_revert = dataframe["close"] < dataframe["bb_middle"]
        trend_break = dataframe["close"] < dataframe["ema55"]
        dataframe.loc[mean_revert | trend_break, "exit_long"] = 1
        return dataframe
