"""
TomacNQ_RegimeTransitionHazard — NQ 1h trend-transition hazard candidate.

Paradigm: regime-cluster transition hazard
Hypothesis: A rising EMA gap after a low-gap state marks transition-hazard clusters that can distinguish continuation from fragile phase transition for the execution tree.
Parent: TomacNQ_KillzoneBreakout
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeTransitionHazard(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.027

    trailing_stop = True
    trailing_stop_positive = 0.006
    trailing_stop_positive_offset = 0.014
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
        dataframe["rsi"] = ta.RSI(dataframe, timeperiod=14)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["ema_gap"] = (dataframe["ema21"] - dataframe["ema89"]) / dataframe["atr"]
        dataframe["gap_floor"] = dataframe["ema_gap"].rolling(80).quantile(0.25)
        dataframe["gap_slope"] = dataframe["ema_gap"].diff(5)
        dataframe["high_6h"] = dataframe["high"].rolling(6).max().shift(1)
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 12) & (dataframe["hour_utc"] <= 21)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        transition_from_low_gap = (
            (dataframe["ema_gap"].shift(8) <= dataframe["gap_floor"].shift(8))
            & (dataframe["ema_gap"] > 0.0)
            & (dataframe["gap_slope"] > 0.18)
        )
        price_confirmation = dataframe["close"] > dataframe["high_6h"]
        not_late = dataframe["rsi"] <= 72
        dataframe.loc[
            liquid_window & higher_trend & transition_from_low_gap & price_confirmation & not_late,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_transition = dataframe["ema_gap"] < 0.0
        exhaustion = dataframe["rsi"] > 78
        dataframe.loc[failed_transition | exhaustion, "exit_long"] = 1
        return dataframe
