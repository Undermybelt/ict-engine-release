"""
TomacNQ_KillzoneBreakout — single-pair NQ futures strategy on synthetic NQ/USD feather data.

Paradigm: breakout
Hypothesis: On NQ continuous-rolled 1h futures bars, breakouts above the prior 24h high concentrated in the US AM session (UTC 13:30-15:30 ≈ NY 09:30-11:30) deliver follow-through when the 4h trend (EMA21>EMA89) agrees, mirroring Tomac's literal AM killzone exploit on its native asset class.
Parent: TomacKillzoneBreakout (crypto generalisation, autoresearch/apr26)
Created: pending-first-commit
Status: active
Uses MTF: yes

Provenance: idea originally seeded by /Users/thrill3r/Downloads/Tomac/ultimate_ict_strategy.py and 90wr1.5rrr_strategy.py (read-only). Source data: /Users/thrill3r/Downloads/Tomac/nq future 2021-2025/NQ_1min_Continuous_Shifted_2836.csv resampled by prepare_external.py.
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_KillzoneBreakout(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.02

    trailing_stop = True
    trailing_stop_positive = 0.005
    trailing_stop_positive_offset = 0.01
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 250

    @informative("4h")
    def populate_indicators_4h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["high_24h"] = dataframe["high"].rolling(24).max().shift(1)
        dataframe["low_24h"] = dataframe["low"].rolling(24).min().shift(1)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        am_killzone = (dataframe["hour_utc"] >= 13) & (dataframe["hour_utc"] <= 15)
        breakout = dataframe["close"] > dataframe["high_24h"]
        trend_4h = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        dataframe.loc[
            am_killzone & breakout & trend_4h,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        breakdown = dataframe["close"] < dataframe["low_24h"]
        trend_break_4h = dataframe["ema_fast_4h"] < dataframe["ema_slow_4h"]
        dataframe.loc[breakdown | trend_break_4h, "exit_long"] = 1
        return dataframe
