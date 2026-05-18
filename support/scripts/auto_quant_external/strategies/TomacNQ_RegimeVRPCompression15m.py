"""
TomacNQ_RegimeVRPCompression15m — IV-HV compression-regime entry, orthogonal regime axis.

Paradigm: regime-cluster vol-regime compression (different axis than VIXShock / VIXBackwardation)
Hypothesis: When QQQ IV percentile-rank over a 252-day window is in the bottom quartile (vol cheap by long-term standards) AND QQQ HV percentile-rank is also in the bottom quartile (realized vol equally low), the market is in a compressed-vol regime — historically a precursor to vol expansion. The expansion direction is the open question; this candidate takes the upside view: if NQ holds above EMA89 (no underlying weakness) and the next 15m bar is bullish during the liquid window, take a long. Different regime axis from VIXShockReversal (active spike) and VIXBackwardation (stress / term inversion); spans the BOTTOM of the vol distribution rather than the top.
Parent: orthogonal new geometry (no direct parent in pack)
Created: 2026-05-07
Status: orthogonal-source probe via IV-HV compression
Uses MTF: yes
External data: /tmp/ict-engine-ibkr-probe/{qqq.iv,qqq.hv}.1d.10y.csv (IBKR daily 2016-2026, ~10Y)
"""
from __future__ import annotations

from pathlib import Path

import pandas as pd
import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame

_QQQ_IV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.iv.1d.10y.csv")
_QQQ_HV_CSV = Path("/tmp/ict-engine-ibkr-probe/qqq.hv.1d.10y.csv")
_SERIES_CACHE: dict[str, pd.Series] = {}


def _load_close_series(csv_path: Path, key: str) -> pd.Series:
    if key not in _SERIES_CACHE:
        df = pd.read_csv(csv_path)
        df["ts"] = pd.to_datetime(df["ts"], utc=True, errors="coerce")
        df = df.dropna(subset=["ts", "close"])
        df["date"] = df["ts"].dt.normalize()
        s = df.set_index("date")["close"].astype(float)
        _SERIES_CACHE[key] = s[~s.index.duplicated(keep="last")].sort_index()
    return _SERIES_CACHE[key]


def _pct_rank_252(series: pd.Series) -> pd.Series:
    return series.rolling(252, min_periods=128).rank(pct=True)


class TomacNQ_RegimeVRPCompression15m(IStrategy):
    INTERFACE_VERSION = 3

    timeframe = "15m"
    can_short = False

    minimal_roi = {"0": 100}
    stoploss = -0.022

    trailing_stop = True
    trailing_stop_positive = 0.005
    trailing_stop_positive_offset = 0.011
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
        dataframe["bullish_body"] = dataframe["close"] > dataframe["open"]
        dataframe["hour_utc"] = dataframe["date"].dt.hour
        iv_pr = _pct_rank_252(_load_close_series(_QQQ_IV_CSV, "qqq_iv"))
        hv_pr = _pct_rank_252(_load_close_series(_QQQ_HV_CSV, "qqq_hv"))
        candle_dates = pd.to_datetime(dataframe["date"], utc=True).dt.normalize()
        dataframe["iv_pct_rank_252"] = candle_dates.map(iv_pr).ffill()
        dataframe["hv_pct_rank_252"] = candle_dates.map(hv_pr).ffill()
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 13) & (dataframe["hour_utc"] <= 21)
        iv_compressed = dataframe["iv_pct_rank_252"] < 0.30
        hv_compressed = dataframe["hv_pct_rank_252"] < 0.30
        not_collapsing = dataframe["close"] > dataframe["ema89"]
        higher_trend_4h = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        dataframe.loc[
            liquid_window
            & iv_compressed
            & hv_compressed
            & not_collapsing
            & higher_trend_4h
            & dataframe["bullish_body"],
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        vol_expanding = dataframe["iv_pct_rank_252"] > 0.55
        regime_break = dataframe["close"] < dataframe["ema89"]
        upper_target = dataframe["close"] > dataframe["ema21"] * 1.020
        dataframe.loc[
            vol_expanding | regime_break | upper_target,
            "exit_long",
        ] = 1
        return dataframe
