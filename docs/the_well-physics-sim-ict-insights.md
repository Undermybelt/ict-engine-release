# The Well: 15TB 物理模拟数据集 - ICT 启示

**来源**: [PolymathicAI/the_well](https://github.com/PolymathicAI/the_well)  
**日期**: 2026-04-21  
**用途**: 为 ICT 引擎寻找可借鉴的算法、公式和架构模式

## 概述

The Well 是一个大规模物理模拟数据集合，包含 16 个不同领域的数据集，总计 15TB。涵盖流体动力学、磁流体动力学、声学散射、活性物质、超新星爆炸等。用于训练代理模型（surrogate models）以加速科学计算。

## 核心算法与架构

### 1. 傅里叶神经算子 (FNO)
**文件**: `the_well/benchmark/models/fno/__init__.py`

**核心思想**: 在频域学习偏微分方程的解算子，而非学习单个解。

**ICT 启示**:
- **信号处理**: 频域学习可应用于通信信道建模、信号预测
- **时序预测**: 金融时间序列的频域特征学习
- **参数**: `modes1`, `modes2`, `modes3` 控制频域分辨率

```python
# 关键参数
n_modes = (modes1, modes2, modes3)  # 频域模式数
hidden_channels = 64                 # 隐藏层通道
```

### 2. Tucker 分解傅里叶神经算子 (TFNO)
**文件**: `the_well/benchmark/models/tfno/__init__.py`

**核心思想**: 使用 Tucker 张量分解压缩 FNO 的参数量。

**ICT 启示**:
- **高维数据压缩**: 适用于多资产、多时间框架的金融数据
- **参数效率**: 减少模型大小，适合边缘部署
- **张量分解**: 可用于因子分解、降维

### 3. 自适应傅里叶神经算子 (AFNO)
**文件**: `the_well/benchmark/models/afno/__init__.py`

**核心思想**: 
- 使用复杂数 MLP 在频域处理
- 稀疏阈值 (softshrink) 过滤噪声
- 块对角权重矩阵减少参数

**ICT 启示**:
- **自适应滤波**: 稀疏阈值可用于噪声过滤、特征选择
- **复数处理**: 适用于相位信息、调制信号
- **块对角结构**: 可用于分块处理多市场数据

```python
# 关键公式
x = torch.fft.rfftn(x, dim=spatial_dims, norm="ortho")  # 频域变换
x = F.softshrink(x, lambd=self.sparsity_threshold)       # 稀疏阈值
x = torch.fft.irfftn(x, s=resolution, dim=spatial_dims, norm="ortho")  # 逆变换
```

### 4. 轴向注意力视觉 Transformer (AViT)
**文件**: `the_well/benchmark/models/avit/__init__.py`

**核心思想**:
- 使用轴向注意力 (axial attention) 分别处理不同空间维度
- 比全局注意力更高效
- 支持 2D/3D 数据

**ICT 启示**:
- **多尺度处理**: 分别处理时间、价格、成交量等维度
- **高效注意力**: 减少计算复杂度，适合实时系统
- **时空分离**: 可用于分别建模时间序列和空间关系

```python
# 轴向注意力模式
spatial_permutations = [
    ("b h w he c -> b h he w c", "b h he w c -> b h w (he c)"),  # 高度轴
    ("b h he w c -> b h w he c", "b h w he c -> b h w (he c)"),  # 宽度轴
]
```

## 数据格式与存储

### HDF5 数据结构
```
(n_traj, n_steps, coord1, coord2, (coord3))  # 单精度 fp32
```

**ICT 启示**:
- **分层存储**: 轨迹 → 时间步 → 空间坐标
- **元数据嵌入**: HDF5 属性存储元信息
- **数据类型分离**: 标量场 (t0), 向量场 (t1), 张量场 (t2)

### 数据集规模参考

| 数据集 | 分辨率 | 时间步 | 轨迹数 | 大小 |
|--------|--------|--------|--------|------|
| MHD_256 | 256³ | 100 | 100 | 4.58 TB |
| euler | 512×512 | 100 | 10,000 | 5.17 TB |
| rayleigh_benard | 512×128 | 200 | 1,750 | 358 GB |

## 基准测试方法

### 评估指标
- **VRMSE**: 归一化均方根误差（预测均值 = 1）
- **Rollout Loss**: 多步预测的累积误差
- **窗口评估**: (6:12) 和 (13:30) 时间窗口

**ICT 启示**:
- **分段评估**: 短期、中期、长期预测分开评估
- **归一化**: 相对于基线（均值预测）的性能
- **滚动预测**: 逐步预测并累积误差

### 模型性能对比

| 模型 | 优势场景 | VRMSE 范围 |
|------|----------|------------|
| FNO | 光滑场、周期性系统 | 0.00046 - 0.84 |
| TFNO | 行星流体、中子星 | 0.0195 - 0.86 |
| AFNO | 多尺度湍流 | - |
| AViT | 时空序列 | - |
| U-Net | 不连续性、复杂边界 | 0.035 - 1.49 |
| CNextU-Net | 大多数场景最佳 | 0.015 - 0.81 |

## 可迁移的数学工具

### 1. 频域变换
```python
# 正向 FFT
x_freq = torch.fft.rfftn(x, dim=spatial_dims, norm="ortho")

# 逆 FFT
x_spatial = torch.fft.irfftn(x_freq, s=resolution, dim=spatial_dims, norm="ortho")
```

### 2. 稀疏阈值
```python
# 软阈值收缩
x_sparse = F.softshrink(x, lambd=threshold)
```

### 3. 张量分解 (Tucker)
- 将高维张量分解为核心张量和因子矩阵
- 减少参数量，保持表达能力

### 4. 轴向注意力
- 分别沿每个空间维度计算注意力
- 复杂度从 O(n²) 降到 O(n√n)

## 建议的 ICT 集成点

### 因子引擎
1. **频域因子**: 使用 FFT 提取频域特征作为因子
2. **稀疏因子**: 应用软阈值筛选显著因子
3. **多尺度因子**: 不同频段对应不同时间尺度

### 序列建模
1. **轴向注意力**: 分离时间维度和资产维度的注意力
2. **块对角权重**: 分块处理不同市场/资产类别
3. **复数处理**: 保留相位信息（如周期性相位）

### 数据管道
1. **HDF5 存储**: 高效存储多维时序数据
2. **元数据嵌入**: 将配置、参数嵌入数据文件
3. **分层访问**: 支持轨迹级、时间步级、特征级访问

## 参考文献

- Ohana, R., et al. (2024). "The Well: A Large-Scale Collection of Diverse Physics Simulations for Machine Learning." NeurIPS 2024.
- [arXiv:2412.00568](https://arxiv.org/abs/2412.00568)
- [neuraloperator 库](https://neuraloperator.github.io/)

## 下一步

1. 实验 FNO/TFNO 在金融时间序列上的效果
2. 测试稀疏阈值对因子选择的影响
3. 评估轴向注意力在多资产建模中的性能
4. 设计 HDF5-based 的 ICT 数据存储格式