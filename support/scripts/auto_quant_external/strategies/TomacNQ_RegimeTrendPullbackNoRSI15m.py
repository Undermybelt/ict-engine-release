"""
TomacNQ_RegimeTrendPullbackNoRSI15m — Simple15m with the RSI not-exhausted gate dropped.

Paradigm: regime-cluster trend pullback, RSI-gate-free
Hypothesis: Slice 101's no-informative Simple15m fired entries through 2018-2023 but went silent in 2024-2025. The diagnosis isolated the cause to the `RSI 35-74` not-exhausted gate: in the strong post-2023 NQ uptrend, RSI(14) on 15m bars stayed elevated above 74 for extended periods, systematically blocking entries. Dropping the RSI gate entirely keeps the pullback-zone (close within 2.4 ATR of EMA21) as the sole exhaustion guard — if price has retraced into EMA21 vicinity, by definition the move is not extended. This variant tests whether removing the structurally-fragile RSI gate restores 2024-2025 trade density to the ~500-600/year rate seen in 2018-2020.
Parent: TomacNQ_RegimeTrendPullbackSimple15m
Created: 2026-05-07
Status: drought-resolution diagnostic candidate
Uses MTF: NO
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy
from pandas import DataFrame


class TomacNQ_RegimeTrendPullbackNoRSI15m(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "15m"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.022

    trailing_stop = True
    trailing_stop_positive = 0.004
    trailing_stop_positive_offset = 0.010
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 700

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema21"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["ema200"] = ta.EMA(dataframe, timeperiod=200)
        dataframe["ema600"] = ta.EMA(dataframe, timeperiod=600)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["near_ema21"] = (dataframe["close"] - dataframe["ema21"]).abs() / dataframe["atr"]
        dataframe["body_green"] = dataframe["close"] > dataframe["open"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 8) & (dataframe["hour_utc"] <= 23)
        long_trend = dataframe["ema200"] > dataframe["ema600"]
        local_trend = (dataframe["ema21"] > dataframe["ema89"]) & (dataframe["close"] > dataframe["ema89"])
        pullback_zone = dataframe["near_ema21"] <= 2.4
        reacceleration = dataframe["body_green"] | (dataframe["close"] > dataframe["close"].shift(1))
        dataframe.loc[
            liquid_window
            & (long_trend | local_trend)
            & pullback_zone
            & reacceleration,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        regime_break = dataframe["close"] < dataframe["ema200"]
        dataframe.loc[regime_break, "exit_long"] = 1
        return dataframe
