"""
TomacNQ_RegimeTrendPullbackDense15m — 15m base port of the TrendPullbackDense candidate.

Paradigm: regime-cluster trend pullback, lower-timeframe density variant
Hypothesis: The 1h base TrendPullbackDense produced 57 trades / Sharpe 0.19 / +8.8% / DD -5.0% / PF 1.58 over 3 years on NQ/USD — currently the density+quality leader of the pack but still only `thin` (30-79). Porting the same condition stack to a 15m base with `1h` and `4h` informatives should give a `~4x` natural density rise on the same edge geometry, taking the candidate past the `dense (>= 80)` floor, while the dual `1h + 4h` informative resonance preserves the higher-timeframe trend context that drove the original's edge.
Parent: TomacNQ_RegimeTrendPullbackDense
Created: 2026-05-07
Status: density-via-timeframe probe
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeTrendPullbackDense15m(IStrategy):
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

    startup_candle_count: int = 240

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
        dataframe["rsi"] = ta.RSI(dataframe, timeperiod=14)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["near_ema21"] = (dataframe["close"] - dataframe["ema21"]).abs() / dataframe["atr"]
        dataframe["body_green"] = dataframe["close"] > dataframe["open"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 8) & (dataframe["hour_utc"] <= 23)
        higher_trend_4h = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        higher_trend_1h = dataframe["ema_fast_1h"] > dataframe["ema_slow_1h"]
        local_trend = (dataframe["ema21"] > dataframe["ema89"]) & (dataframe["close"] > dataframe["ema89"])
        pullback_zone = dataframe["near_ema21"] <= 2.4
        reacceleration = dataframe["body_green"] | (dataframe["close"] > dataframe["close"].shift(1))
        not_exhausted = (dataframe["rsi"] >= 35) & (dataframe["rsi"] <= 74)
        dataframe.loc[
            liquid_window
            & (higher_trend_4h | higher_trend_1h | local_trend)
            & pullback_zone
            & reacceleration
            & not_exhausted,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        regime_break = dataframe["close"] < dataframe["ema89"]
        exhausted = dataframe["rsi"] > 82
        dataframe.loc[regime_break | exhausted, "exit_long"] = 1
        return dataframe
