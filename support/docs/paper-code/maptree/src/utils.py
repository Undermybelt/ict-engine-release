# 决策树工具函数

import numpy as np
from model import TreeNode


def print_tree(node: TreeNode, indent: str = "", feature_names=None) -> str:
    """打印树结构"""
    lines = []
    if node.is_leaf:
        lines.append(f"{indent}Leaf: predict={node.prediction:.3f} (n={node.n_samples})")
    else:
        fname = (feature_names[node.feature_idx] if feature_names 
                 else f"X[{node.feature_idx}]")
        lines.append(f"{indent}if {fname} <= {node.threshold:.4f}:")
        lines.append(print_tree(node.left, indent + "  ", feature_names))
        lines.append(f"{indent}else:")
        lines.append(print_tree(node.right, indent + "  ", feature_names))
    return "\n".join(lines)


def tree_to_dict(node: TreeNode) -> dict:
    """树 → 字典（JSON 可序列化）"""
    if node.is_leaf:
        return {
            "leaf": True,
            "prediction": node.prediction,
            "n_samples": node.n_samples,
            "n_positive": node.n_positive,
        }
    return {
        "leaf": False,
        "feature_idx": node.feature_idx,
        "threshold": node.threshold,
        "n_samples": node.n_samples,
        "left": tree_to_dict(node.left),
        "right": tree_to_dict(node.right),
    }


def depth(node: TreeNode) -> int:
    """树深度"""
    if node is None or node.is_leaf:
        return 0
    return 1 + max(depth(node.left), depth(node.right))


def count_nodes(node: TreeNode) -> int:
    """总节点数"""
    if node is None:
        return 0
    if node.is_leaf:
        return 1
    return 1 + count_nodes(node.left) + count_nodes(node.right)
