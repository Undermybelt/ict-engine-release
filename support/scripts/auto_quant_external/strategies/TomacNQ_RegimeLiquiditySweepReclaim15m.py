"""
TomacNQ_RegimeLiquiditySweepReclaim15m — 15m base port of LiquiditySweepReclaim.

Paradigm: regime-cluster mean-reversion / liquidity sweep, lower-timeframe density variant
Hypothesis: The 1h base LiquiditySweepReclaim produced the strongest convex payoff in the pack — profit factor 7.53, win rate 75%, PF Calmar 11.53 — but only 4 trades over 3 years on NQ/USD. Porting the same condition stack to a 15m base with `1h` and `4h` informative resonance should give a `~4x` natural density rise. The sweep window scales from 12h on 1h base to 12 bars on 15m (3h, intraday-sweep semantics) so the candidate captures intraday liquidity grabs rather than overnight sweeps; this is a deliberate regime shift that pairs naturally with the 15m base.
Parent: TomacNQ_RegimeLiquiditySweepReclaim
Created: 2026-05-07
Status: density-via-timeframe probe
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeLiquiditySweepReclaim15m(IStrategy):
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
        liquid_window = (dataframe["hour_utc"] >= 12) & (dataframe["hour_utc"] <= 21)
        higher_trend_4h = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        higher_trend_1h = dataframe["ema_fast_1h"] > dataframe["ema_slow_1h"]
        clean_reclaim = dataframe["sweep_below"] & dataframe["reclaim_close"]
        confirmation_body = dataframe["body_strength"] > 0.4
        not_already_extended = dataframe["close"] < dataframe["ema21"] * 1.008
        dataframe.loc[
            liquid_window
            & (higher_trend_4h | higher_trend_1h)
            & clean_reclaim
            & confirmation_body
            & not_already_extended,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_reclaim = dataframe["close"] < dataframe["low_12bar"]
        upper_overshoot = dataframe["close"] > dataframe["ema21"] * 1.018
        trend_break_4h = dataframe["ema_fast_4h"] < dataframe["ema_slow_4h"]
        dataframe.loc[failed_reclaim | upper_overshoot | trend_break_4h, "exit_long"] = 1
        return dataframe
