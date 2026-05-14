"""
TomacNQ_RegimeVVIXDivergence15m — vol-of-vol divergence entry, orthogonal regime axis.

Paradigm: regime-cluster vol-of-vol divergence (different regime axis than VIXShockReversal)
Hypothesis: When daily VVIX rises sharply (`vvix_z20 > 1.0`) while spot VIX stays stable (`vix_z20 < 0.5`), the market is pricing in higher future volatility without realizing it yet — a common precursor to a vol shock that resolves either through actual VIX expansion (negative for risk assets) or through VVIX normalization (relief rally for risk assets). This candidate takes the relief-rally side: counter-positioned long entry on bullish 15m candles when the divergence is observed and NQ is not collapsing. The trigger uses VVIX data that no existing pack member uses; it fires on entirely different days than VIXShockReversal (which uses `vix_z20`), expanding the basket's regime-feature dimensionality rather than thickening existing coverage.
Parent: orthogonal new geometry (no direct parent in pack)
Created: 2026-05-07
Status: orthogonal-source probe via vol-of-vol divergence
Uses MTF: yes
External data: /tmp/ict-engine-ibkr-probe/vvix.1d.10y.csv and vix.1d.10y.csv (IBKR daily 2018-2026)
"""
from __future__ import annotations

from pathlib import Path

import pandas as pd
import talib.abstract as ta
from freqtrade.strategy import IStrategy, informative
from pandas import DataFrame

_VIX_CSV = Path("/tmp/ict-engine-ibkr-probe/vix.1d.10y.csv")
_VVIX_CSV = Path("/tmp/ict-engine-ibkr-probe/vvix.1d.10y.csv")
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


def _z20(series: pd.Series) -> pd.Series:
    mean20 = series.rolling(20, min_periods=10).mean()
    std20 = series.rolling(20, min_periods=10).std()
    return (series - mean20) / std20.where(std20 > 1e-9)


class TomacNQ_RegimeVVIXDivergence15m(IStrategy):
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
        vix_z = _z20(_load_close_series(_VIX_CSV, "vix"))
        vvix_z = _z20(_load_close_series(_VVIX_CSV, "vvix"))
        candle_dates = pd.to_datetime(dataframe["date"], utc=True).dt.normalize()
        dataframe["vix_z20"] = candle_dates.map(vix_z).ffill()
        dataframe["vvix_z20"] = candle_dates.map(vvix_z).ffill()
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["enter_long"] = 0
        liquid_window = (dataframe["hour_utc"] >= 13) & (dataframe["hour_utc"] <= 21)
        vvix_rising = dataframe["vvix_z20"] > 1.0
        vix_stable = dataframe["vix_z20"] < 0.5
        not_collapsing = dataframe["close"] > dataframe["ema89"] * 0.97
        higher_trend_4h = dataframe["ema_fast_4h"] > dataframe["ema_slow_4h"]
        dataframe.loc[
            liquid_window
            & vvix_rising
            & vix_stable
            & not_collapsing
            & dataframe["bullish_body"]
            & higher_trend_4h,
            "enter_long",
        ] = 1
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["exit_long"] = 0
        vvix_normalized = dataframe["vvix_z20"] < 0.3
        vix_spiked = dataframe["vix_z20"] > 1.5
        regime_break = dataframe["close"] < dataframe["ema89"] * 0.97
        upper_target = dataframe["close"] > dataframe["ema21"] * 1.020
        dataframe.loc[
            vvix_normalized | vix_spiked | regime_break | upper_target,
            "exit_long",
        ] = 1
        return dataframe
