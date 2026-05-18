"""
TomacNQ_RegimeVRPCarry — NQ 1h volatility-risk-premium / carry-shape candidate.

Paradigm: regime-cluster volatility-risk-premium proxy
Hypothesis: When realized-vol z-score sits in a compression band and price oscillates inside a value zone with weak directional pressure, a long-only carry-shape entry produces small consistent gains and a tail-risk loss profile, mirroring the payoff shape of an IV/HV vol-risk-premium harvest without requiring options data. This is deliberately orthogonal to the trend-continuation, breakout, and sweep families already in the pack; its purpose is portfolio-diversity, not standalone Sharpe maximization.
Parent: TomacNQ_KillzoneBreakout
Created: 2026-05-06
Status: active external candidate
Uses MTF: yes
"""
from __future__ import annotations

import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame


class TomacNQ_RegimeVRPCarry(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "1h"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.018

    trailing_stop = True
    trailing_stop_positive = 0.004
    trailing_stop_positive_offset = 0.009
    trailing_only_offset_is_reached = True

    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    startup_candle_count: int = 360

    @informative("4h")
    def populate_indicators_4h(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=89)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        return dataframe

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema21"] = ta.EMA(dataframe, timeperiod=21)
        dataframe["ema55"] = ta.EMA(dataframe, timeperiod=55)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=14)
        dataframe["atr_pct"] = dataframe["atr"] / dataframe["close"]
        atr_pct_mean = dataframe["atr_pct"].rolling(240).mean()
        atr_pct_std = dataframe["atr_pct"].rolling(240).std()
        dataframe["atr_pct_z240"] = (dataframe["atr_pct"] - atr_pct_mean) / atr_pct_std
        dataframe["atr_short"] = ta.ATR(dataframe, timeperiod=5)
        dataframe["atr_long"] = ta.ATR(dataframe, timeperiod=60)
        dataframe["term_ratio"] = dataframe["atr_short"] / dataframe["atr_long"]
        dataframe["realized_vol_60"] = dataframe["close"].pct_change().rolling(60).std()
        dataframe["realized_vol_240"] = dataframe["close"].pct_change().rolling(240).std()
        dataframe["vrp_proxy"] = dataframe["realized_vol_240"] / dataframe["realized_vol_60"].clip(lower=1e-9)
        dataframe["value_zone_low"] = dataframe[["ema21", "ema55"]].min(axis=1) * 0.998
        dataframe["value_zone_high"] = dataframe[["ema21", "ema55"]].max(axis=1) * 1.002
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 12) & (dataframe["hour_utc"] <= 21)
        compressed_realized = dataframe["atr_pct_z240"] < -0.4
        flat_term_structure = (dataframe["term_ratio"] > 0.85) & (dataframe["term_ratio"] < 1.15)
        vrp_carry_regime = dataframe["vrp_proxy"] > 1.05
        in_value_zone = (dataframe["close"] >= dataframe["value_zone_low"]) & (
            dataframe["close"] <= dataframe["value_zone_high"]
        )
        ema_4h_band = (dataframe["ema_fast_4h"] - dataframe["ema_slow_4h"]).abs() < (
            0.4 * dataframe["atr_4h"]
        )
        dataframe.loc[
            liquid_window
            & compressed_realized
            & flat_term_structure
            & vrp_carry_regime
            & in_value_zone
            & ema_4h_band,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        regime_break_up = dataframe["atr_pct_z240"] > 0.9
        regime_break_term = dataframe["term_ratio"] > 1.35
        zone_break_down = dataframe["close"] < dataframe["value_zone_low"]
        dataframe.loc[
            regime_break_up | regime_break_term | zone_break_down,
            "exit_long",
        ] = 1
        return dataframe
