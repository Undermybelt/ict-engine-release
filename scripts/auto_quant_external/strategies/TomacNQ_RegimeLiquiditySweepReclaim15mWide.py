"""
TomacNQ_RegimeLiquiditySweepReclaim15mWide — structurally widened 15m base sweep-reclaim variant.

Paradigm: regime-cluster mean-reversion / liquidity sweep, density-widened on 15m base
Hypothesis: Slice 84's `LiquiditySweepReclaim15m` reached 13 trades on NQ/USD 15m 3Y with PF 1.57 (probe-only). The Wide variant drops the `not_already_extended (close < ema21 * 1.008)` gate that suppressed roughly half the entries in the original 1h shape, and softens the body-strength threshold from `0.4` to `0.25` ATR. The intent is `~3x` more entries on top of Slice 84's port, putting the candidate above the dense floor while keeping the sweep + reclaim core geometry.
Parent: TomacNQ_RegimeLiquiditySweepReclaim15m
Created: 2026-05-07
Status: density-widening probe on 15m base
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeLiquiditySweepReclaim15mWide(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "15m"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.018

    trailing_stop = True
    trailing_stop_positive = 0.004
    trailing_stop_positive_offset = 0.010
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 280

    @informative("1h")
    def populate_indicators_1h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        return dataframe

    @informative("4h")
    def populate_indicators_4h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema21"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["low_12bar"] = dataframe["low"].rolling(12).min().shift(1)
        dataframe["low_2bar"] = dataframe["low"].rolling(2).min()
        dataframe["sweep_below"] = dataframe["low_2bar"] < dataframe["low_12bar"]
        dataframe["reclaim_close"] = dataframe["close"] > dataframe["low_12bar"]
        dataframe["body_strength"] = (dataframe["close"] - dataframe["open"]) / dataframe["atr"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 8) & (dataframe["hour_utc"] <= 22)
        higher_trend_4h = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        higher_trend_1h = dataframe["ema_fast_1h"] > dataframe["ema_slow_1h"]
        clean_reclaim = dataframe["sweep_below"] & dataframe["reclaim_close"]
        confirmation_body = dataframe["body_strength"] > 0.25
        dataframe.loc[
            liquid_window
            & (higher_trend_4h | higher_trend_1h)
            & clean_reclaim
            & confirmation_body,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_reclaim = dataframe["close"] < dataframe["low_12bar"]
        upper_overshoot = dataframe["close"] > dataframe["ema21"] * 1.025
        trend_break_4h = dataframe["ema_fast_4h"] < dataframe["ema_slow_4h"]
        dataframe.loc[failed_reclaim | upper_overshoot | trend_break_4h, "exit_long"] = 1
        return dataframe
