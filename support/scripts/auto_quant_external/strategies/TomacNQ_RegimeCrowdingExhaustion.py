"""
TomacNQ_RegimeCrowdingExhaustion — NQ 1h crowded-selling exhaustion absorption candidate.

Paradigm: regime-cluster crowding / herding exhaustion (Family E)
Hypothesis: Three consecutive declining bars near a recent swing low followed by a high-volume bullish-body bar that closes back above the prior bar's high is a Family E exhaustion-and-absorption signature: the herd has been forced out and a counter-side participant has stepped in. This setup feeds Layer 1 execution-feature enrichment (crowding pressure relief) and Layer 4 regime-clustering (exhaustion regime detector) with a payoff geometry that no breakout / persistence / transition / FVG / sweep candidate already in the pack can produce. It is intentionally counter-regime: the 4h trend can still be down; we are buying exhaustion at a level rather than continuation.
Parent: TomacNQ_KillzoneBreakout
Created: 2026-05-07
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeCrowdingExhaustion(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.024

    trailing_stop = True
    trailing_stop_positive = 0.006
    trailing_stop_positive_offset = 0.014
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
        dataframe["volume_ma20"] = dataframe["volume"].rolling(20).mean()
        dataframe["swing_low_50"] = dataframe["low"].rolling(50).min()
        dataframe["bar_range"] = (dataframe["high"] - dataframe["low"]).clip(lower=1e-9)
        dataframe["body"] = dataframe["close"] - dataframe["open"]
        dataframe["body_ratio"] = dataframe["body"] / dataframe["bar_range"]
        dataframe["three_declining"] = (
            (dataframe["close"].shift(1) < dataframe["close"].shift(2))
            & (dataframe["close"].shift(2) < dataframe["close"].shift(3))
            & (dataframe["close"].shift(3) < dataframe["close"].shift(4))
        )
        dataframe["near_swing_low"] = (
            dataframe["low"] <= dataframe["swing_low_50"] + 0.6 * dataframe["atr"]
        )
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 12) & (dataframe["hour_utc"] <= 21)
        forced_selling = dataframe["volume"] > dataframe["volume_ma20"] * 1.35
        bullish_absorption = (dataframe["body"] > 0) & (dataframe["body_ratio"] > 0.40)
        rejection_close = dataframe["close"] > dataframe["high"].shift(1)
        not_already_recovered = dataframe["close"] < dataframe["ema21"]
        regime_not_collapsing = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"] * 0.985
        dataframe.loc[
            liquid_window
            & dataframe["three_declining"]
            & dataframe["near_swing_low"]
            & forced_selling
            & bullish_absorption
            & rejection_close
            & not_already_recovered
            & regime_not_collapsing,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        failed_reversal = dataframe["close"] < dataframe["swing_low_50"]
        target_recovery = dataframe["close"] > dataframe["ema21"] * 1.018
        regime_collapse = dataframe["ema_fast_4h"] < dataframe["ema_slow_4h"] * 0.95
        dataframe.loc[
            failed_reversal | target_recovery | regime_collapse,
            "exit_long",
        ] = 1
        return dataframe
