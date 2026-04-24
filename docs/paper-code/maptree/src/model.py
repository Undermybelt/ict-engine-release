# MAPTree — Bayesian Decision Trees via AND/OR Search
# 论文：MAPTree: Beating "Optimal" Decision Trees with Bayesian Decision Trees (arXiv 2309.15312)
#
# §3.2 BCART 后验：P(T|Y,X) ∝ P(Y|X,T) · P(T|X)
# §3.2 边际似然：ℓ_leaf(c^1,c^0) := B(c^1+ρ^1, c^0+ρ^0) / B(ρ^1, ρ^0)
# §3.2 树先验：p_split(d) = α(1+d)^{-β}
# §5 Algorithm 1-5: AND/OR search with bounds

import numpy as np
import math
from dataclasses import dataclass, field
from typing import Optional, List, Dict

try:
    from scipy.special import betaln
except ImportError:
    def betaln(a, b):
        return math.lgamma(a) + math.lgamma(b) - math.lgamma(a + b)


@dataclass
class TreeNode:
    feature_idx: Optional[int] = None
    threshold: Optional[float] = None
    prediction: Optional[float] = None
    left: Optional['TreeNode'] = None
    right: Optional['TreeNode'] = None
    n_samples: int = 0
    n_positive: int = 0
    depth: int = 0

    @property
    def is_leaf(self) -> bool:
        return self.feature_idx is None

    @property
    def n_negative(self) -> int:
        return self.n_samples - self.n_positive


@dataclass
class BCARTConfig:
    """§3.2 BCART 参数"""
    rho_1: float = 1.0   # Beta(ρ^1, ρ^0) 先验
    rho_0: float = 1.0
    split_alpha: float = 0.95  # p_split(d) = α(1+d)^{-β}
    split_beta: float = 1.0
    max_depth: int = 10
    min_samples_leaf: int = 5


# ── §3.2 核心公式 ────────────────────────────────────────

def log_leaf_marginal(c1: int, c0: int, config: BCARTConfig) -> float:
    """§3.2: ℓ_leaf(c^1,c^0) := B(c^1+ρ^1, c^0+ρ^0) / B(ρ^1, ρ^0)"""
    return betaln(c1 + config.rho_1, c0 + config.rho_0) - betaln(config.rho_1, config.rho_0)


def p_split(d: int, config: BCARTConfig) -> float:
    """§3.2: p_split(d) = α(1+d)^{-β}"""
    return config.split_alpha * (1 + d) ** (-config.split_beta)


def log_posterior_leaf(c1: int, c0: int, d: int, config: BCARTConfig) -> float:
    """log 后验 for 叶节点 = log ℓ_leaf + log(1 - p_split(d))"""
    return log_leaf_marginal(c1, c0, config) + np.log(1 - p_split(d, config))


def log_posterior_split(c1_left: int, c0_left: int, c1_right: int, c0_right: int,
                         d: int, n_features: int, config: BCARTConfig) -> float:
    """log 后验 for 分裂节点 = log p_inner + log ℓ_leaf_left + log ℓ_leaf_right"""
    if n_features == 0:
        return -np.inf
    return (np.log(p_split(d, config)) - np.log(n_features)
            + log_leaf_marginal(c1_left, c0_left, config)
            + log_leaf_marginal(c1_right, c0_right, config))


# ── §5 启发式 ─────────────────────────────────────────────

def h_or(c1: int, c0: int, d: int, n_feat: int, config: BCARTConfig) -> float:
    """§5: h(o) = -max{log ℓ_leaf, log p_split + log ℓ_leaf(c^1,0) + log ℓ_leaf(0,c^0)}"""
    log_leaf = log_leaf_marginal(c1, c0, config)
    if n_feat > 0:
        log_split = (np.log(p_split(d, config))
                     + log_leaf_marginal(c1, 0, config)
                     + log_leaf_marginal(0, c0, config))
    else:
        log_split = -np.inf
    return -max(log_leaf, log_split)


class MAPTreeClassifier:
    """§5 MAPTree: BCART MAP 推断 via AND/OR 搜索
    
    用递归分支定界：对每个 OR 节点，比较"不分裂(叶)" vs "最优分裂"。
    如果分裂更优，递归构建左右子树。
    """

    def __init__(self, config: Optional[BCARTConfig] = None):
        self.config = config or BCARTConfig()
        self.tree_: Optional[TreeNode] = None
        self.best_log_posterior_: float = -np.inf

    def fit(self, X: np.ndarray, y: np.ndarray) -> 'MAPTreeClassifier':
        X = np.asarray(X, dtype=np.float64)
        y = np.asarray(y, dtype=np.int64)
        indices = np.arange(len(y))
        self.tree_ = self._search(X, y, indices, depth=0)
        self.best_log_posterior_ = self._tree_log_posterior(self.tree_)
        return self

    def _search(self, X: np.ndarray, y: np.ndarray,
                indices: np.ndarray, depth: int) -> TreeNode:
        """§5 递归 AND/OR 搜索
        
        OR 节点：叶节点 vs 最优分裂
        AND 节点：枚举所有 (特征, 阈值)，选后验最高的
        """
        n = len(indices)
        c1 = int(y[indices].sum())
        c0 = n - c1
        n_feat = X.shape[1]

        # --- 叶节点候选（OR 的"不分裂"选项）---
        leaf = TreeNode(
            n_samples=n, n_positive=c1, depth=depth,
            prediction=c1 / n if n > 0 else 0.5,
        )
        leaf_post = log_posterior_leaf(c1, c0, depth, self.config)

        # 终止条件
        if (c1 == 0 or c0 == 0 or
            depth >= self.config.max_depth or
            n < 2 * self.config.min_samples_leaf):
            return leaf

        # --- AND 节点：搜索最优分裂 ---
        best_split_node = None
        best_split_post = leaf_post  # 至少要比叶节点好

        for feat_idx in range(n_feat):
            values = X[indices, feat_idx]
            unique_vals = np.unique(values)
            if len(unique_vals) < 2:
                continue

            # §5: 遍历所有候选阈值
            thresholds = (unique_vals[:-1] + unique_vals[1:]) / 2.0

            for threshold in thresholds:
                left_mask = values <= threshold
                right_mask = ~left_mask
                left_idx = indices[left_mask]
                right_idx = indices[right_mask]

                if (len(left_idx) < self.config.min_samples_leaf or
                    len(right_idx) < self.config.min_samples_leaf):
                    continue

                lc1 = int(y[left_idx].sum())
                lc0 = len(left_idx) - lc1
                rc1 = int(y[right_idx].sum())
                rc0 = len(right_idx) - rc1

                # §5: 分裂的 log 后验
                split_post = log_posterior_split(
                    lc1, lc0, rc1, rc0, depth, n_feat, self.config
                )

                if split_post > best_split_post:
                    # 递归构建子树（AND 的两个 OR 子节点）
                    left_child = self._search(X, y, left_idx, depth + 1)
                    right_child = self._search(X, y, right_idx, depth + 1)

                    best_split_post = split_post
                    best_split_node = TreeNode(
                        feature_idx=feat_idx, threshold=threshold,
                        n_samples=n, n_positive=c1, depth=depth,
                        left=left_child, right=right_child,
                    )

        # OR 决策：叶节点 vs 最优分裂
        if best_split_node is not None:
            return best_split_node
        return leaf

    def _tree_log_posterior(self, node: TreeNode) -> float:
        """计算完整树的 log 后验"""
        if node.is_leaf:
            return log_posterior_leaf(node.n_positive, node.n_negative,
                                      node.depth, self.config)
        if node.left is None or node.right is None:
            return -np.inf
        return (self._tree_log_posterior(node.left) +
                self._tree_log_posterior(node.right))

    def predict(self, X: np.ndarray) -> np.ndarray:
        X = np.asarray(X, dtype=np.float64)
        return np.array([self._predict_one(x) for x in X])

    def _predict_one(self, x: np.ndarray) -> float:
        node = self.tree_
        while not node.is_leaf and node.left is not None:
            if x[node.feature_idx] <= node.threshold:
                node = node.left
            else:
                node = node.right
        return node.prediction

    def predict_class(self, X: np.ndarray, threshold: float = 0.5) -> np.ndarray:
        return (self.predict(X) >= threshold).astype(int)

    def tree_depth(self) -> int:
        def _d(n):
            if n is None or n.is_leaf:
                return 0
            return 1 + max(_d(n.left), _d(n.right))
        return _d(self.tree_)

    def n_leaves(self) -> int:
        def _c(n):
            if n is None:
                return 0
            if n.is_leaf:
                return 1
            return _c(n.left) + _c(n.right)
        return _c(self.tree_)


if __name__ == "__main__":
    np.random.seed(42)
    X = np.random.randn(200, 5)
    y = (X[:, 0] + X[:, 1] > 0).astype(float)

    model = MAPTreeClassifier(BCARTConfig(max_depth=4, min_samples_leaf=10))
    model.fit(X, y)

    acc = (model.predict_class(X) == y).mean()
    print(f"MAPTree: depth={model.tree_depth()}, leaves={model.n_leaves()}")
    print(f"Train acc: {acc:.3f}, log_post: {model.best_log_posterior_:.2f}")
