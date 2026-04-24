# MAPTree 对比评估
# 论文：MAPTree: Beating "Optimal" Decision Trees with Bayesian Decision Trees (arXiv 2309.15312)

import numpy as np
from model import MAPTreeClassifier, BCARTConfig


def compare_trees(X_train, y_train, X_test, y_test, 
                  max_depth=5, min_samples_leaf=5) -> dict:
    """对比 MAPTree vs sklearn CART vs Random Forest"""
    from sklearn.tree import DecisionTreeClassifier
    from sklearn.ensemble import RandomForestClassifier
    
    results = {}
    
    # MAPTree
    config = BCARTConfig(max_depth=max_depth, min_samples_leaf=min_samples_leaf)
    maptree = MAPTreeClassifier(config)
    maptree.fit(X_train, y_train)
    results["maptree"] = {
        "train_acc": float((maptree.predict_class(X_train) == y_train).mean()),
        "test_acc": float((maptree.predict_class(X_test) == y_test).mean()),
        "depth": maptree.tree_depth(),
        "leaves": maptree.n_leaves(),
        "log_posterior": float(maptree.best_log_posterior_),
    }
    
    # Sklearn CART
    cart = DecisionTreeClassifier(max_depth=max_depth, min_samples_leaf=min_samples_leaf)
    cart.fit(X_train, y_train)
    results["sklearn_cart"] = {
        "train_acc": float(cart.score(X_train, y_train)),
        "test_acc": float(cart.score(X_test, y_test)),
        "depth": cart.get_depth(),
        "leaves": cart.get_n_leaves(),
    }
    
    # Random Forest (ensemble baseline)
    rf = RandomForestClassifier(n_estimators=100, max_depth=max_depth, random_state=42)
    rf.fit(X_train, y_train)
    results["random_forest"] = {
        "train_acc": float(rf.score(X_train, y_train)),
        "test_acc": float(rf.score(X_test, y_test)),
    }
    
    return results


def noise_robustness_test(n_samples=500, noise_levels=[0.0, 0.1, 0.2, 0.3]) -> dict:
    """§论文：MAPTree demonstrates greater robustness to noise"""
    from sklearn.datasets import make_classification
    from sklearn.model_selection import train_test_split
    
    results = {}
    for noise in noise_levels:
        X, y = make_classification(
            n_samples=n_samples, n_features=10, n_informative=5,
            flip_y=noise, random_state=42
        )
        X_train, X_test, y_train, y_test = train_test_split(
            X, y, test_size=0.3, random_state=42
        )
        
        comp = compare_trees(X_train, y_train, X_test, y_test, max_depth=5)
        results[f"noise_{noise}"] = comp
    
    return results


if __name__ == "__main__":
    from sklearn.datasets import make_classification
    from sklearn.model_selection import train_test_split
    
    print("MAPTree vs Baselines Comparison")
    print("=" * 50)
    
    X, y = make_classification(
        n_samples=500, n_features=10, n_informative=5, random_state=42
    )
    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.3, random_state=42
    )
    
    results = compare_trees(X_train, y_train, X_test, y_test)
    
    for name, metrics in results.items():
        print(f"\n{name}:")
        for k, v in metrics.items():
            if isinstance(v, float):
                print(f"  {k}: {v:.4f}")
            else:
                print(f"  {k}: {v}")
    
    print("\n--- Noise Robustness ---")
    noise_results = noise_robustness_test()
    for noise_level, methods in noise_results.items():
        print(f"\n{noise_level}:")
        for method, metrics in methods.items():
            acc = metrics.get("test_acc", 0)
            print(f"  {method}: test_acc={acc:.3f}")
