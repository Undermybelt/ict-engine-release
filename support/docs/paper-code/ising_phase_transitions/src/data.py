# S&P 500 数据加载
# §Monte Carlo simulation: "daily financial data spanning the past forty years,
# specifically from February 1982 to February 2022"

import numpy as np
from pathlib import Path


def load_sp500_returns(filepath: str = None) -> np.ndarray:
    """加载 S&P 500 日收益数据
    
    §Monte Carlo simulation: "Adjusted Close prices of the S&P 500 index"
    §Monte Carlo simulation: "r_Δt(t) = ln(P_t) - ln(P_{t-Δt})"
    
    [UNSPECIFIED] 论文未提供数据下载链接
    Using: 从 CSV 加载（如有），否则生成模拟数据
    Alternatives: yfinance API, pandas_datareader
    """
    if filepath and Path(filepath).exists():
        import csv
        prices = []
        with open(filepath) as f:
            reader = csv.DictReader(f)
            for row in reader:
                prices.append(float(row.get('Adj Close', row.get('Close', 0))))
        prices = np.array(prices)
        returns = np.log(prices[1:]) - np.log(prices[:-1])
        return returns
    else:
        # [UNSPECIFIED] 论文要求真实数据但未提供
        # 生成模拟 S&P 500 收益（正态 + 厚尾混合）
        print("[UNSPECIFIED] No S&P 500 data file provided.")
        print("Using: synthetic returns with realistic stylized facts")
        print("To use real data: pass filepath='path/to/sp500.csv'")
        np.random.seed(42)
        n = 10_000
        # 混合分布：95% 正态 + 5% 厚尾
        normal_part = np.random.normal(0.0003, 0.01, int(n * 0.95))
        fat_tail_part = np.random.standard_t(df=3, size=int(n * 0.05)) * 0.02
        returns = np.concatenate([normal_part, fat_tail_part])
        np.random.shuffle(returns)
        return returns


def compute_log_returns(prices: np.ndarray, delta_t: int = 1) -> np.ndarray:
    """§Monte Carlo simulation: r_Δt(t) = ln(P_t) - ln(P_{t-Δt})"""
    if len(prices) <= delta_t:
        return np.array([])
    return np.log(prices[delta_t:]) - np.log(prices[:-delta_t])
