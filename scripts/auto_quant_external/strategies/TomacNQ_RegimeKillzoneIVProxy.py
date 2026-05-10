"""
TomacNQ_RegimeKillzoneIVProxy — NQ 1h AM-killzone breakout gated by an in-asset IV-term-structure proxy.

Paradigm: regime-cluster session-quality + volatility-regime gate
Hypothesis: AM-killzone breakouts are higher quality when the in-asset volatility term-structure proxy (ATR(5) / ATR(60)) is in a flat-to-contango band, i.e., short-term realized volatility is at or below long-term realized volatility. This mimics what a flat / mildly contango VIX9D-VIX1Y term structure would say about regime stability and is a Layer 4 vol-regime gate added on top of a Family H session entry. The intent is to harvest the same session breakout edge as the parent only when the vol-regime backdrop is supportive, producing a Layer 1 + Layer 4 combined factor that is structurally different from any existing candidate.
Parent: TomacNQ_KillzoneBreakout
Created: 2026-05-07
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeKillzoneIVProxy(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.019

    trailing_stop = True
    trailing_stop_positive = 0.005
    trailing_stop_positive_offset = 0.011
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 320

    @informative("4h")
    def populate_indicators_4h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema21"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema89"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["atr_short"] = ta.ATR(dataframe, timeperiod=5)
        dataframe["atr_long"] = ta.ATR(dataframe, timeperiod=60)
        dataframe["term_ratio"] = dataframe["atr_short"] / dataframe["atr_long"]
        dataframe["term_ratio_smoothed"] = dataframe["term_ratio"].rolling(6).mean()
        atr_pct = dataframe["atr"] / dataframe["close"]
        atr_pct_mean = atr_pct.rolling(240).mean()
        atr_pct_std = atr_pct.rolling(240).std()
        dataframe["atr_pct_z240"] = (atr_pct - atr_pct_mean) / atr_pct_std
        dataframe["high_24h"] = dataframe["high"].rolling(24).max().shift(1)
        dataframe["low_24h"] = dataframe["low"].rolling(24).min().shift(1)
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        am_killzone = (dataframe["hour_utc"] >= 13) & (dataframe["hour_utc"] <= 15)
        higher_trend = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        breakout = dataframe["close"] > dataframe["high_24h"]
        flat_or_contango_term = (dataframe["term_ratio_smoothed"] >= 0.55) & (
            dataframe["term_ratio_smoothed"] <= 0.95
        )
        not_vol_spike = dataframe["atr_pct_z240"] < 1.2
        trend_context = dataframe["close"] > dataframe["ema89"]
        dataframe.loc[
            am_killzone
            & higher_trend
            & breakout
            & flat_or_contango_term
            & not_vol_spike
            & trend_context,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        breakdown = dataframe["close"] < dataframe["low_24h"]
        vol_regime_break = dataframe["term_ratio_smoothed"] > 1.30
        vol_spike = dataframe["atr_pct_z240"] > 1.5
        trend_break_4h = dataframe["ema_fast_4h"] < dataframe["ema_slow_4h"]
        dataframe.loc[
            breakdown | vol_regime_break | vol_spike | trend_break_4h,
            "exit_long",
        ] = 1
        return dataframe
