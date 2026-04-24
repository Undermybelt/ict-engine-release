# §Analysis of Results — Stylized Facts 检验
# 论文：Phase Transitions in Financial Markets Using the Ising Model (arXiv 2504.19050)
#
# 检验 Ising 模拟是否复现 S&P 500 的 5 个 stylized facts：
# 1. Volatility clustering (§Volatility Clustering)
# 2. Negative skewness (§Inferential Statistics)
# 3. Heavy tails / leptokurtic (§Inferential Statistics)
# 4. No autocorrelation in returns (§Autocorrelation of Returns)
# 5. Slow-decaying autocorrelation in absolute returns (§Autocorrelation of Returns)

import numpy as np
from typing import Optional


def volatility_clustering_test(returns: np.ndarray, window: int = 100) -> dict:
    """§Volatility Clustering: "large price changes tend to be followed by large changes"
    
    检验：大收益后是否跟大收益（绝对收益自相关）
    """
    abs_returns = np.abs(returns)
    # 计算滑动窗口波动率
    if len(returns) < window * 2:
        return {"test": "volatility_clustering", "passed": False, "reason": "insufficient data"}
    
    rolling_vol = np.array([
        returns[i:i+window].std() for i in range(len(returns) - window)
    ])
    # 波动率自相关
    if len(rolling_vol) > 1:
        vol_autocorr = np.corrcoef(rolling_vol[:-1], rolling_vol[1:])[0, 1]
    else:
        vol_autocorr = 0.0
    
    return {
        "test": "volatility_clustering",
        "vol_autocorr_lag1": float(vol_autocorr),
        "passed": vol_autocorr > 0.3,  # §Volatility Clustering: 应有显著正自相关
        "interpretation": f"Vol autocorr={vol_autocorr:.3f} (>0.3 = clustering present)"
    }


def autocorrelation_test(returns: np.ndarray, max_lag: int = 150) -> dict:
    """§Autocorrelation of Returns: "autocorrelation in the Ising model data
    gradually approaches that of the S&P 500"
    
    论文用 τ={0,...,150}
    """
    n = len(returns)
    if n < max_lag + 10:
        return {"test": "autocorrelation", "passed": False, "reason": "insufficient data"}
    
    # 标准化收益
    r = (returns - returns.mean()) / (returns.std() + 1e-10)
    
    # 计算自相关
    autocorrs = []
    for lag in range(1, min(max_lag + 1, n // 2)):
        if lag < n:
            corr = np.corrcoef(r[:-lag], r[lag:])[0, 1]
            autocorrs.append(corr)
    
    autocorrs = np.array(autocorrs)
    # §Autocorrelation of Returns: 收益自相关应接近零（无显著自相关）
    mean_autocorr = np.abs(autocorrs).mean()
    
    return {
        "test": "return_autocorrelation",
        "lags_computed": len(autocorrs),
        "mean_abs_autocorr": float(mean_autocorr),
        "passed": mean_autocorr < 0.05,  # §Autocorrelation: 应接近零
        "interpretation": f"Mean |autocorr|={mean_autocorr:.4f} (<0.05 = no significant autocorr)"
    }


def absolute_return_autocorrelation(returns: np.ndarray, max_lag: int = 150) -> dict:
    """§Autocorrelation of Returns: "autocorrelation of absolute returns
    remains positive and decays slowly... power-law function ρ_A(τ)=A·τ^{-η}"
    
    §Autocorrelation of Returns: η 在 [0.2, 0.4] 范围内
    论文发现 Ising 模型 η≈0.3
    """
    n = len(returns)
    if n < max_lag + 10:
        return {"test": "abs_return_autocorrelation", "passed": False, "reason": "insufficient data"}
    
    abs_r = np.abs(returns)
    abs_r_centered = abs_r - abs_r.mean()
    
    autocorrs = []
    lags = []
    for lag in range(1, min(max_lag + 1, n // 2)):
        if lag < n:
            corr = np.corrcoef(abs_r_centered[:-lag], abs_r_centered[lag:])[0, 1]
            autocorrs.append(corr)
            lags.append(lag)
    
    autocorrs = np.array(autocorrs)
    lags = np.array(lags)
    
    # Power-law fit: ρ(τ) = A · τ^{-η}
    # log(ρ) = log(A) - η·log(τ)
    # [UNSPECIFIED] 论文未给出具体拟合方法
    # Using: OLS on log-log scale (标准做法)
    positive_mask = autocorrs > 0
    if positive_mask.sum() > 5:
        log_lags = np.log(lags[positive_mask])
        log_acf = np.log(autocorrs[positive_mask])
        # 线性回归
        A_mat = np.vstack([log_lags, np.ones(len(log_lags))]).T
        eta, log_A = np.linalg.lstsq(A_mat, log_acf, rcond=None)[0]
        eta = -eta  # ρ = A·τ^{-η}，斜率 = -η
    else:
        eta = 0.0
        log_A = 0.0
    
    eta_in_range = 0.2 <= eta <= 0.4  # §Autocorrelation: "η empirically found to lie within [0.2,0.4]"
    
    return {
        "test": "abs_return_power_law",
        "eta": float(eta),
        "eta_in_empirical_range": eta_in_range,
        "first_lag_autocorr": float(autocorrs[0]) if len(autocorrs) > 0 else 0.0,
        "passed": eta_in_range and (autocorrs[0] > 0.1 if len(autocorrs) > 0 else False),
        "interpretation": f"η={eta:.3f} (empirical range [0.2,0.4], paper finds η≈0.3)"
    }


def normality_tests(returns: np.ndarray) -> dict:
    """§Inferential Statistics: "Shapiro-Wilk and Jarque-Bera tests... p-values
    well below the 0.05 threshold, leading to rejection of the null hypothesis"
    
    §Inferential Statistics: "p-value (Ising)≈0.0"
    """
    from scipy import stats
    
    skew = float(stats.skew(returns))
    kurt = float(stats.kurtosis(returns))  # excess kurtosis
    
    # Jarque-Bera test
    jb_stat, jb_p = stats.jarque_bera(returns)
    
    # Shapiro-Wilk (需要降采样，因为大样本会拒绝一切)
    sample = returns[np.random.choice(len(returns), min(5000, len(returns)), replace=False)]
    sw_stat, sw_p = stats.shapiro(sample)
    
    return {
        "test": "normality",
        "skewness": skew,
        "excess_kurtosis": kurt,
        "jarque_bera": {"statistic": float(jb_stat), "p_value": float(jb_p)},
        "shapiro_wilk": {"statistic": float(sw_stat), "p_value": float(sw_p)},
        "is_leptokurtic": kurt > 0,  # §Inferential Statistics: "leptokurtic, exhibiting heavy tails"
        "is_negatively_skewed": skew < 0,  # §Inferential Statistics: "negative skewness"
        "normality_rejected": jb_p < 0.05 and sw_p < 0.05,
        "passed": jb_p < 0.05,  # §Inferential Statistics: "p-value≈0.0"
        "interpretation": f"skew={skew:.3f}, kurt={kurt:.3f}, JB-p={jb_p:.2e}"
    }


def full_stylized_facts_analysis(returns: np.ndarray) -> dict:
    """完整 stylized facts 检验套件
    
    §Analysis of Results: 检验所有 5 个 stylized facts
    """
    results = {
        "volatility_clustering": volatility_clustering_test(returns),
        "return_autocorrelation": autocorrelation_test(returns),
        "abs_return_autocorrelation": absolute_return_autocorrelation(returns),
        "normality": normality_tests(returns),
    }
    
    passed = sum(1 for r in results.values() if r.get("passed", False))
    total = len(results)
    
    results["summary"] = {
        "tests_passed": passed,
        "tests_total": total,
        "all_passed": passed == total,
        "stylized_facts_replicated": passed >= 3,  # 论文说"majority"
    }
    
    return results


if __name__ == "__main__":
    from model import simulate, IsingConfig

    print("Running Ising simulation + stylized facts analysis...")
    config = IsingConfig()
    # [UNSPECIFIED] 完整模拟 1M 步很慢，demo 用小规模
    # Using: 200k iterations for quick test
    # Alternatives: 1M (paper default), 500k (compromise)
    config.n_iterations = 200_000
    config.warmup = 20_000
    
    result = simulate(config)
    returns = result["returns"]
    
    print(f"\nReturns: {len(returns)} samples")
    print(f"Mean: {returns.mean():.6f}, Std: {returns.std():.6f}")
    
    analysis = full_stylized_facts_analysis(returns)
    
    print("\n=== Stylized Facts Analysis (§Analysis of Results) ===")
    for test_name, test_result in analysis.items():
        if test_name == "summary":
            continue
        status = "✓" if test_result.get("passed") else "✗"
        interp = test_result.get("interpretation", "")
        print(f"  {status} {test_name}: {interp}")
    
    summary = analysis["summary"]
    print(f"\nPassed: {summary['tests_passed']}/{summary['tests_total']}")
    print(f"Stylized facts replicated: {summary['stylized_facts_replicated']}")
