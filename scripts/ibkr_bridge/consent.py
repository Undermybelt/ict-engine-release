"""User consent gate for IBKR features.

IBKR is opt-in. Every code path that touches IBKR APIs must call
``require_ibkr_enabled()`` first. The first invocation prompts the user
with a bilingual privacy disclaimer; subsequent invocations silently pass
through once consent is recorded in ``~/.ict-engine/ibkr_consent.json``.

This file is gitignored / never enters the repo.
"""

from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

CONSENT_PATH = Path.home() / ".ict-engine" / "ibkr_consent.json"
CONSENT_VERSION = 1


DISCLAIMER_TEXT = """\
IBKR live data — privacy & connectivity notice
──────────────────────────────────────────────
This feature reads real-time data from your *locally running* IB Gateway
or TWS application. Everything stays on this machine.

What this code DOES:
  • Connect to localhost:7497 (paper) or :7496 (live) — your local IB Gateway
  • Subscribe to instruments listed in ibkr_bridge/<your>_config.yaml
  • Write market data to your local Redis (localhost:6379)
  • Honor IBKR's pacing rules (6 s/contract historical, 100 streaming lines)
  • Learn your account capabilities passively from observed errors;
    no proactive probing of your data quota at startup

What this code DOES NOT:
  • Send your IBKR credentials anywhere — they stay in IB Gateway / TWS
  • Contact ict-engine.com, OpenAlice, or any third-party server
  • Place orders, close positions, or modify your IBKR account state
  • Collect telemetry, analytics, or crash reports

Auditable source:  scripts/ibkr_bridge/{bridge,consumer,rate_limiter}.py
Capabilities file: ~/.ict-engine/ibkr_capabilities.json (gitignored, local)
Revoke any time:   python scripts/ibkr_bridge/setup.py --revoke

═════════════════════════════════════════════════════════════════════
中文版
═════════════════════════════════════════════════════════════════════

IBKR 实时数据 — 隐私与连接说明
──────────────────────────────────
本功能从你**本机运行**的 IB Gateway 或 TWS 读取实时数据。所有内容均不离开本机。

本代码会做：
  • 连接 localhost:7497 (paper) 或 :7496 (live) — 你本地的 IB Gateway
  • 订阅 ibkr_bridge/<你的>_config.yaml 列出的合约
  • 写入你本地的 Redis (localhost:6379)
  • 遵守 IBKR 流控规则 (6 秒/合约 历史限制，100 条流式数据线)
  • 通过观察实际错误被动学习账户能力；启动时不做主动配额探测

本代码绝不会：
  • 把你的 IBKR 凭据传到任何地方 — 凭据一直在 IB Gateway / TWS 里
  • 联系 ict-engine.com、OpenAlice 或任何第三方服务
  • 下单、平仓或修改你的 IBKR 账户状态
  • 收集遥测、分析或崩溃报告

源代码可审：    scripts/ibkr_bridge/{bridge,consumer,rate_limiter}.py
能力文件：      ~/.ict-engine/ibkr_capabilities.json (gitignored, 本地)
随时撤回同意：  python scripts/ibkr_bridge/setup.py --revoke
"""


def is_opted_in(path: Path = CONSENT_PATH) -> bool:
    if not path.exists():
        return False
    try:
        data = json.loads(path.read_text())
    except (json.JSONDecodeError, OSError):
        return False
    return bool(data.get("opted_in"))


def set_opt_in(path: Path = CONSENT_PATH) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({
        "opted_in": True,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "version": CONSENT_VERSION,
    }, indent=2))


def revoke(path: Path = CONSENT_PATH) -> bool:
    if path.exists():
        path.unlink()
        return True
    return False


def show_disclaimer_and_prompt(path: Path = CONSENT_PATH,
                                input_fn=input) -> bool:
    """Print disclaimer, ask y/N, persist on yes. Return True if opted in."""
    print(DISCLAIMER_TEXT)
    answer = input_fn(
        "Do you understand and want to enable IBKR live data? "
        "是否理解并启用 IBKR 实时数据？ [y/N]: "
    ).strip().lower()
    if answer in ("y", "yes"):
        set_opt_in(path)
        print("\n✓ IBKR enabled. Consent recorded at "
              f"{path}.\n")
        return True
    print("\n✗ IBKR not enabled. ict-engine continues to work without it.\n")
    return False


def require_ibkr_enabled(path: Path = CONSENT_PATH,
                          interactive: bool | None = None) -> None:
    """Gate function called before any IBKR API touch.

    If consent is already on file, return silently. Otherwise:
        * if running in a TTY (interactive shell), show disclaimer + prompt;
        * if non-interactive (CI, cron, container), raise SystemExit with a
          clear instruction on how to enable.

    Pass ``interactive=False`` to force the non-interactive branch (useful
    for tests or for scripts that should never prompt).
    """
    if is_opted_in(path):
        return

    if interactive is None:
        interactive = sys.stdin.isatty() and sys.stdout.isatty()

    if not interactive:
        sys.exit(
            "IBKR is not enabled.\n"
            "To enable interactively, run:\n"
            "    python scripts/ibkr_bridge/setup.py --enable\n"
            "Or read the disclaimer at:\n"
            "    scripts/ibkr_bridge/README.md\n"
        )

    if not show_disclaimer_and_prompt(path):
        sys.exit(1)
