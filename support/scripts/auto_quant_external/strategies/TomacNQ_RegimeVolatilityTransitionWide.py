"""
TomacNQ_RegimeVolatilityTransitionWide — denser NQ volatility-transition candidate.

Paradigm: regime-cluster volatility transition
Hypothesis: The strict low-to-high ATR transition was too rare; a wider above-median volatility expansion should test whether volatility-regime descriptors can produce enough observations for clustering.
Parent: TomacNQ_RegimeVolatilityTransition
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeVolatilityTransitionWide(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.029

    trailing_stop = True
    trailing_stop_positive = 0.006
    trailing_stop_positive_offset = 0.014
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
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["atr_pct"] = dataframe["atr"] / dataframe["close"]
        dataframe["atr_p50"] = dataframe["atr_pct"].rolling(120).quantile(0.50)
        dataframe["high_6h"] = dataframe["high"].rolling(6).max().shift(1)
        dataframe["rsi"] = ta.RSI(dataframe, timeperiod=14)
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 10) & (dataframe["hour_utc"] <= 22)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        volatility_active = dataframe["atr_pct"] > dataframe["atr_p50"]
        breakout = dataframe["close"] > dataframe["high_6h"]
        trend_context = dataframe["close"] > dataframe["ema89"]
        not_exhausted = dataframe["rsi"] <= 74
        dataframe.loc[
            liquid_window & higher_trend & volatility_active & breakout & trend_context & not_exhausted,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_breakout = dataframe["close"] < dataframe["ema21"]
        exhausted = dataframe["rsi"] > 80
        dataframe.loc[failed_breakout | exhausted, "exit_long"] = 1
        return dataframe
