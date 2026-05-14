"""
TomacNQ_RegimeCompressionReleaseDense — density-focused compression-release candidate.

Paradigm: regime-cluster rhythm release
Hypothesis: A looser compression/release definition should test whether rhythm-state descriptors can move from sparse anecdotes into a thin-but-usable regime-clustering sample.
Parent: TomacNQ_RegimeCompressionReleaseWide
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeCompressionReleaseDense(IStrategy):
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
        dataframe["ema34"] = ta.EMA(dataframe, timeperiod=34)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        bands = ta.BBANDS(dataframe, timeperiod=20, nbdevup=2.0, nbdevdn=2.0)
        dataframe["bb_upper"] = bands["upperband"]
        dataframe["bb_middle"] = bands["middleband"]
        dataframe["bb_lower"] = bands["lowerband"]
        dataframe["bb_width"] = (dataframe["bb_upper"] - dataframe["bb_lower"]) / dataframe["bb_middle"]
        dataframe["bb_width_p70"] = dataframe["bb_width"].rolling(120).quantile(0.70)
        dataframe["recent_compression"] = (
            dataframe["bb_width"].rolling(18).min() < dataframe["bb_width_p70"].rolling(18).min()
        )
        dataframe["high_3h"] = dataframe["high"].rolling(3).max().shift(1)
        dataframe["rsi"] = ta.RSI(dataframe, timeperiod=14)
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 8) & (dataframe["hour_utc"] <= 23)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        release = (dataframe["close"] > dataframe["bb_middle"]) & (dataframe["close"] > dataframe["high_3h"])
        trend_context = dataframe["close"] > dataframe["ema89"]
        not_exhausted = dataframe["rsi"] <= 78
        dataframe.loc[
            liquid_window & higher_trend & dataframe["recent_compression"] & release & trend_context & not_exhausted,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_release = dataframe["close"] < dataframe["ema34"]
        exhausted = dataframe["rsi"] > 82
        dataframe.loc[failed_release | exhausted, "exit_long"] = 1
        return dataframe
