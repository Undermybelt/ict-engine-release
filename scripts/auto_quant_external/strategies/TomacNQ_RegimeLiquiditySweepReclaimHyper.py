"""
TomacNQ_RegimeLiquiditySweepReclaimHyper — structurally widened variant of TomacNQ_RegimeLiquiditySweepReclaim.

Paradigm: regime-cluster mean-reversion / liquidity sweep, density-widened
Hypothesis: The Slice 81 backtest showed `LiquiditySweepReclaim` produced profit factor 7.53 / win rate 75% on `NQ/USD 1h ~3Y` but only 4 trades over 3 years — exactly the narrow-high-win-rate density failure the TODO Trade-Density Rule warns against. The Hyper variant drops two stacked conditions (`body_strength > 0.4` and `not_already_extended` test) and softens the sweep depth requirement, while keeping the core sweep-and-reclaim geometry plus the 4h-trend filter. The intent is `~4-10x` trade count with the convex-payoff geometry intact, exposing whether the original candidate's edge is robust under wider activation or was an over-fit narrow window.
Parent: TomacNQ_RegimeLiquiditySweepReclaim
Created: 2026-05-07
Status: density-widening probe
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeLiquiditySweepReclaimHyper(IStrategy):
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
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["low_8h"] = dataframe["low"].rolling(8).min().shift(1)
        dataframe["sweep_below"] = dataframe["low"] < dataframe["low_8h"]
        dataframe["reclaim_close"] = dataframe["close"] > dataframe["low_8h"]
        dataframe["bullish_body"] = dataframe["close"] > dataframe["open"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 12) & (dataframe["hour_utc"] <= 22)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        clean_reclaim = dataframe["sweep_below"] & dataframe["reclaim_close"]
        dataframe.loc[
            liquid_window & higher_trend & clean_reclaim & dataframe["bullish_body"],
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_reclaim = dataframe["close"] < dataframe["low_8h"]
        upper_overshoot = dataframe["close"] > dataframe["ema21"] * 1.03
        trend_break_4h = dataframe["ema_fast_4h"] < dataframe["ema_slow_4h"]
        dataframe.loc[failed_reclaim | upper_overshoot | trend_break_4h, "exit_long"] = 1
        return dataframe
