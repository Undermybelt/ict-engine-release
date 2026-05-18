"""
TomacNQ_RegimeFVGRetrace — NQ 1h fair-value-gap retrace candidate.

Paradigm: regime-cluster structural retrace
Hypothesis: A bullish Fair Value Gap (high[t-2] < low[t]) is an unfilled price imbalance whose later retest under an aligned 4h trend is a structurally different setup than every breakout / persistence / transition candidate already in the pack. The retrace-and-reject geometry produces a tight-stop / asymmetric-target payoff and supplies Layer 1 setup-quality and Layer 3 evidence-quality material that breakout-shaped candidates cannot expose. Intended as a portfolio-diversification orthogonal source, not a stronger trend variant.
Parent: TomacNQ_KillzoneBreakout
Created: 2026-05-07
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeFVGRetrace(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.020

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
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["fvg_lower"] = dataframe["high"].shift(6)
        dataframe["fvg_upper"] = dataframe["low"].shift(4)
        dataframe["fvg_width"] = dataframe["fvg_upper"] - dataframe["fvg_lower"]
        dataframe["fvg_existed"] = (dataframe["fvg_lower"] < dataframe["fvg_upper"]) & (
            dataframe["fvg_width"] > 0.0010 * dataframe["close"]
        )
        dataframe["body_strength"] = (dataframe["close"] - dataframe["open"]) / dataframe["atr"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 12) & (dataframe["hour_utc"] <= 21)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        retraced_into_gap = (dataframe["low"] <= dataframe["fvg_upper"]) & (
            dataframe["low"] >= dataframe["fvg_lower"] * 0.998
        )
        rejected_back_above = dataframe["close"] > dataframe["fvg_lower"]
        bullish_close = dataframe["body_strength"] > 0.25
        not_extended = dataframe["close"] < dataframe["ema21"] * 1.012
        dataframe.loc[
            liquid_window
            & higher_trend
            & dataframe["fvg_existed"]
            & retraced_into_gap
            & rejected_back_above
            & bullish_close
            & not_extended,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        gap_failed = dataframe["close"] < dataframe["fvg_lower"]
        upper_target = dataframe["close"] > dataframe["ema21"] * 1.022
        trend_break_4h = dataframe["ema_fast_4h"] < dataframe["ema_slow_4h"]
        dataframe.loc[gap_failed | upper_target | trend_break_4h, "exit_long"] = 1
        return dataframe
