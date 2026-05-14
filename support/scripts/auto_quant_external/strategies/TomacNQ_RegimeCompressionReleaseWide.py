"""
TomacNQ_RegimeCompressionReleaseWide — denser NQ compression-release regime candidate.

Paradigm: regime-cluster rhythm release
Hypothesis: The first compression-release candidate produced only probe density; loosening compression and release thresholds should test whether the rhythm-state factor can reach usable cluster density without losing the regime hypothesis.
Parent: TomacNQ_RegimeCompressionRelease
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeCompressionReleaseWide(IStrategy):
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
        dataframe["ema34"] = ta.EMA(dataframe, timeperiod=34)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        bands = ta.BBANDS(dataframe, timeperiod=20, nbdevup=2.0, nbdevdn=2.0)
        dataframe["bb_upper"] = bands["upperband"]
        dataframe["bb_middle"] = bands["middleband"]
        dataframe["bb_lower"] = bands["lowerband"]
        dataframe["bb_width"] = (dataframe["bb_upper"] - dataframe["bb_lower"]) / dataframe["bb_middle"]
        dataframe["bb_width_p55"] = dataframe["bb_width"].rolling(140).quantile(0.55)
        dataframe["was_compressed"] = (
            dataframe["bb_width"].rolling(14).min() < dataframe["bb_width_p55"].rolling(14).min()
        )
        dataframe["high_4h"] = dataframe["high"].rolling(4).max().shift(1)
        dataframe["rsi"] = ta.RSI(dataframe, timeperiod=14)
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 10) & (dataframe["hour_utc"] <= 22)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        release = (dataframe["close"] > dataframe["bb_middle"]) & (dataframe["close"] > dataframe["high_4h"])
        trend_context = dataframe["close"] > dataframe["ema89"]
        not_exhausted = dataframe["rsi"] <= 74
        dataframe.loc[
            liquid_window & higher_trend & dataframe["was_compressed"] & release & trend_context & not_exhausted,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_release = dataframe["close"] < dataframe["ema34"]
        exhausted = dataframe["rsi"] > 78
        dataframe.loc[failed_release | exhausted, "exit_long"] = 1
        return dataframe
