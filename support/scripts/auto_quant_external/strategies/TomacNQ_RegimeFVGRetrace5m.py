"""
TomacNQ_RegimeFVGRetrace5m — 5m base variant of the bullish FVG retrace candidate, with 15m/1h/4h informative resonance stack.

Paradigm: regime-cluster structural retrace, multi-timeframe resonance
Hypothesis: The same FVG-retest geometry as the 1h base candidate becomes more selective and provides denser intraday trade evidence when run on a 5m base with three informative timeframes (15m, 1h, 4h) gating the entry. The minimum resonance stack mandated by the TODO for a 5m base is `15m, 1h, 4h, 1d`; this candidate covers `15m, 1h, 4h` directly through informatives and treats the 1h trend as the strongest single-vote (a contradicted 1h trend invalidates the entry). Intended to grow the timeframe coverage of Family A and to surface 5m-resolution regime evidence for Layer 4 clustering and Layer 1 setup-quality.
Parent: TomacNQ_RegimeFVGRetrace
Created: 2026-05-07
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeFVGRetrace5m(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "5m"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.012

    trailing_stop = True
    trailing_stop_positive = 0.003
    trailing_stop_positive_offset = 0.007
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 320

    @informative("15m")
    def populate_indicators_15m(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=55)
        return dataframe

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
        dataframe["ema55"] = ta.EMA(dataframe, timeperiod=55)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["fvg_lower"] = dataframe["high"].shift(6)
        dataframe["fvg_upper"] = dataframe["low"].shift(4)
        dataframe["fvg_width"] = dataframe["fvg_upper"] - dataframe["fvg_lower"]
        dataframe["fvg_existed"] = (dataframe["fvg_lower"] < dataframe["fvg_upper"]) & (
            dataframe["fvg_width"] > 0.0006 * dataframe["close"]
        )
        dataframe["body_strength"] = (dataframe["close"] - dataframe["open"]) / dataframe["atr"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 12) & (dataframe["hour_utc"] <= 21)
        resonance_15m = dataframe["ema_fast_15m"] > dataframe["ema_slow_15m"]
        resonance_1h = dataframe["ema_fast_1h"] > dataframe["ema_slow_1h"]
        resonance_4h = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        retraced_into_gap = (dataframe["low"] <= dataframe["fvg_upper"]) & (
            dataframe["low"] >= dataframe["fvg_lower"] * 0.998
        )
        rejected_back_above = dataframe["close"] > dataframe["fvg_lower"]
        bullish_close = dataframe["body_strength"] > 0.20
        not_extended = dataframe["close"] < dataframe["ema21"] * 1.006
        dataframe.loc[
            liquid_window
            & resonance_15m
            & resonance_1h
            & resonance_4h
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
        upper_target = dataframe["close"] > dataframe["ema21"] * 1.012
        resonance_lost_1h = dataframe["ema_fast_1h"] < dataframe["ema_slow_1h"]
        dataframe.loc[
            gap_failed | upper_target | resonance_lost_1h,
            "exit_long",
        ] = 1
        return dataframe
