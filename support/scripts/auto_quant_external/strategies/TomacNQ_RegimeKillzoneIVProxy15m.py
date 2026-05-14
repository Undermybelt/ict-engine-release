"""
TomacNQ_RegimeKillzoneIVProxy15m — 15m base port of KillzoneIVProxy with 1h+4h informative resonance.

Paradigm: regime-cluster session-quality + volatility-regime gate, lower-timeframe density variant
Hypothesis: The 1h base KillzoneIVProxy produced 2 trades / profit factor 3.45 on NQ/USD 3Y — too thin to evaluate but with a high-quality payoff signature. The 15m port keeps the AM-killzone gate, the breakout above prior 24h high (now expressed as 96 15m bars), and the in-asset volatility-term-structure proxy (`ATR(5) / ATR(60)`) which on 15m base becomes a `~1.25h vs 15h` comparison instead of `5h vs 60h` — semantically still a short-vs-long-window vol comparison. Density should rise `~4x` on the same condition geometry; the candidate keeps its Family H + Layer 4 vol-regime gate semantics.
Parent: TomacNQ_RegimeKillzoneIVProxy
Created: 2026-05-07
Status: density-via-timeframe probe
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeKillzoneIVProxy15m(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "15m"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.016

    trailing_stop = True
    trailing_stop_positive = 0.004
    trailing_stop_positive_offset = 0.009
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 320

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
        dataframe["atr_short"] = ta.ATR(dataframe, timeperiod=5)
        dataframe["atr_long"] = ta.ATR(dataframe, timeperiod=60)
        dataframe["term_ratio"] = dataframe["atr_short"] / dataframe["atr_long"]
        dataframe["term_ratio_smoothed"] = dataframe["term_ratio"].rolling(6).mean()
        atr_pct = dataframe["atr"] / dataframe["close"]
        atr_pct_mean = atr_pct.rolling(240).mean()
        atr_pct_std = atr_pct.rolling(240).std()
        dataframe["atr_pct_z240"] = (atr_pct - atr_pct_mean) / atr_pct_std
        dataframe["high_96bar"] = dataframe["high"].rolling(96).max().shift(1)
        dataframe["low_96bar"] = dataframe["low"].rolling(96).min().shift(1)
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        am_killzone = (dataframe["hour_utc"] >= 13) & (dataframe["hour_utc"] <= 15)
        higher_trend_4h = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        higher_trend_1h = dataframe["ema_fast_1h"] > dataframe["ema_slow_1h"]
        breakout = dataframe["close"] > dataframe["high_96bar"]
        flat_or_contango_term = (dataframe["term_ratio_smoothed"] >= 0.55) & (
            dataframe["term_ratio_smoothed"] <= 0.95
        )
        not_vol_spike = dataframe["atr_pct_z240"] < 1.2
        trend_context = dataframe["close"] > dataframe["ema89"]
        dataframe.loc[
            am_killzone
            & (higher_trend_4h | higher_trend_1h)
            & breakout
            & flat_or_contango_term
            & not_vol_spike
            & trend_context,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        breakdown = dataframe["close"] < dataframe["low_96bar"]
        vol_regime_break = dataframe["term_ratio_smoothed"] > 1.30
        vol_spike = dataframe["atr_pct_z240"] > 1.5
        trend_break_4h = dataframe["ema_fast_4h"] < dataframe["ema_slow_4h"]
        dataframe.loc[
            breakdown | vol_regime_break | vol_spike | trend_break_4h,
            "exit_long",
        ] = 1
        return dataframe
