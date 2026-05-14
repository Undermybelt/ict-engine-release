"""
TomacNQ_RegimeLiquiditySweepReclaim — NQ 1h sweep-and-reclaim mean-reversion candidate.

Paradigm: regime-cluster mean-reversion / liquidity sweep
Hypothesis: A clean stop-run below the prior 12h low followed by an immediate close-back above that low is a different return source than the existing trend-continuation pack. This shape feeds Layer 1 setup-quality and Layer 4 regime-clustering with a convex small-loss / asymmetric-winner payoff that the existing breakout/persistence/transition variants do not produce, and therefore fits the post-regime portfolio-diversity rule (orthogonal source, not just a stronger trend variant).
Parent: TomacNQ_KillzoneBreakout
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeLiquiditySweepReclaim(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.022

    trailing_stop = True
    trailing_stop_positive = 0.006
    trailing_stop_positive_offset = 0.013
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 280

    @informative("4h")
    def populate_indicators_4h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema21"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["low_12h"] = dataframe["low"].rolling(12).min().shift(1)
        dataframe["low_2h"] = dataframe["low"].rolling(2).min()
        dataframe["sweep_below"] = dataframe["low_2h"] < dataframe["low_12h"]
        dataframe["reclaim_close"] = dataframe["close"] > dataframe["low_12h"]
        dataframe["body_strength"] = (dataframe["close"] - dataframe["open"]) / dataframe["atr"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 12) & (dataframe["hour_utc"] <= 21)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        clean_reclaim = dataframe["sweep_below"] & dataframe["reclaim_close"]
        confirmation_body = dataframe["body_strength"] > 0.4
        not_already_extended = dataframe["close"] < dataframe["ema21"] * 1.012
        dataframe.loc[
            liquid_window & higher_trend & clean_reclaim & confirmation_body & not_already_extended,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_reclaim = dataframe["close"] < dataframe["low_12h"]
        upper_overshoot = dataframe["close"] > dataframe["ema21"] * 1.025
        trend_break_4h = dataframe["ema_fast_4h"] < dataframe["ema_slow_4h"]
        dataframe.loc[failed_reclaim | upper_overshoot | trend_break_4h, "exit_long"] = 1
        return dataframe
