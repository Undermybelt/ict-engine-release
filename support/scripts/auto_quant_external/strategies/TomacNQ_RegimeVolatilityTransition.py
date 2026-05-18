"""
TomacNQ_RegimeVolatilityTransition — NQ 1h low-to-high volatility regime candidate.

Paradigm: regime-cluster volatility transition
Hypothesis: Breakouts after a local ATR-percentile expansion should identify transition clusters that feed HMM regime-change and execution-readiness evidence more directly than static structure filters.
Parent: TomacNQ_KillzoneBreakout
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeVolatilityTransition(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.028

    trailing_stop = True
    trailing_stop_positive = 0.007
    trailing_stop_positive_offset = 0.015
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 300

    @informative("4h")
    def populate_indicators_4h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema21"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["atr_pct"] = dataframe["atr"] / dataframe["close"]
        dataframe["atr_p35"] = dataframe["atr_pct"].rolling(160).quantile(0.35)
        dataframe["atr_p65"] = dataframe["atr_pct"].rolling(160).quantile(0.65)
        dataframe["high_12h"] = dataframe["high"].rolling(12).max().shift(1)
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 12) & (dataframe["hour_utc"] <= 21)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        volatility_transition = (dataframe["atr_pct"] > dataframe["atr_p65"]) & (
            dataframe["atr_pct"].shift(6) < dataframe["atr_p35"].shift(6)
        )
        breakout = dataframe["close"] > dataframe["high_12h"]
        trend_context = dataframe["close"] > dataframe["ema89"]
        dataframe.loc[
            liquid_window & higher_trend & volatility_transition & breakout & trend_context,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_breakout = dataframe["close"] < dataframe["ema21"]
        volatility_faded = dataframe["atr_pct"] < dataframe["atr_p35"]
        dataframe.loc[failed_breakout | volatility_faded, "exit_long"] = 1
        return dataframe
