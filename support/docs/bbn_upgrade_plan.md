# BBN Upgrade Prompt

你现在负责把 `ict-engine` 从“多个概率模块并列存在”升级成“围绕贝叶斯信念网络组织决策”的项目。先不要大改接口，按以下顺序推进：

1. 先盘点现有概率资产：HMM regime、7 层 cascade、BBN trading network、Kelly、Beta learner。
2. 识别哪些模块还只是“算了但没用上”，优先接到 `analyze -> decision -> trade_plan` 这条主路径。
3. 所有新增概率接口都要回答三个问题：
   - 这个概率表示什么事件？
   - 它来自哪个模型？
   - 它如何影响开仓、过滤、仓位或回测？
4. 优先做可验证的小闭环：
   - 用 HMM 输出 regime 概率
   - 用 cascade 输出方向性信号强度
   - 用 BBN 输出 `trade_outcome` 分布
   - 把它们融合成一个最终 `decision_score`
5. 任何概率如果只打印不参与决策，就继续往下接，直到影响某个具体动作。
6. 保持实现可测试：
   - 为新概率融合函数补单元测试
   - 为 planner 决策分支补 bull / no-trade 测试
7. 避免空泛重构。优先做“单一入口可运行、可解释、可验证”的增量改造。
8. 明确约束：
   - ICT 结构、FVG、OB、sweep、CISD 都只是市场现象和证据节点
   - 它们只能作为贝叶斯判断的似然修正或软证据
   - 它们不能被写成直接决定开多/开空的硬触发器

本轮的最低交付标准：

- `analyze` 命令能同时输出 regime 概率、BBN 交易结果分布和最终 trade plan
- trade plan 的仓位或是否交易必须受概率模型影响
- 至少一个新测试覆盖概率融合逻辑
