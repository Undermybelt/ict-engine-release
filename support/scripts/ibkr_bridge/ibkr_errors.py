"""IBKR error code catalog with bilingual cause + suggested fix.

Many IBKR error messages are returned in the user's locale (Chinese for
mainland China accounts), and even the English originals leave the actual
*remediation* implicit. This module translates the most common codes a
quant bridge encounters into:

  * a normalised severity (info / warning / error)
  * an English + Chinese cause description
  * a concrete, parameter-level suggested fix
  * a link to the official IBKR docs

`bridge.py` consults `lookup(code)` whenever IBKR raises an error and
prints the enriched form instead of the raw IBKR string.

References:
  https://ibkrcampus.com/campus/ibkr-api-page/twsapi-doc/#error-codes
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

Severity = Literal["info", "warning", "error"]


@dataclass(frozen=True)
class ErrorInfo:
    code: int
    severity: Severity
    cause_en: str
    cause_zh: str
    fix_en: str | None = None
    fix_zh: str | None = None
    doc_url: str | None = None


_DOC_HISTORICAL_LIMITS = (
    "https://ibkrcampus.com/campus/ibkr-api-page/twsapi-doc/"
    "#historical-data-limitations"
)
_DOC_MD_SUBS = (
    "https://ibkrcampus.com/campus/ibkr-api-page/market-data-subscriptions/"
)
_DOC_DELAYED_MD = (
    "https://ibkrcampus.com/campus/ibkr-api-page/twsapi-doc/"
    "#market-data-delayed"
)


_CATALOG: dict[int, ErrorInfo] = {
    # -----------------------------------------------------------------
    # Connectivity / system info codes (2xxx) — informational only
    # -----------------------------------------------------------------
    2103: ErrorInfo(
        2103, "info",
        "Market-data farm connection broken; reconnecting.",
        "市场数据场连接中断，正在自动重连。",
    ),
    2104: ErrorInfo(
        2104, "info",
        "Market-data farm connection is OK.",
        "市场数据场连接正常。",
    ),
    2105: ErrorInfo(
        2105, "info",
        "Historical data farm connection broken; will reconnect on demand.",
        "历史数据场连接中断，将在需要时自动重连。",
    ),
    2106: ErrorInfo(
        2106, "info",
        "Historical data farm connection is OK.",
        "历史数据场连接正常。",
    ),
    2107: ErrorInfo(
        2107, "info",
        "Historical data farm inactive but available on demand.",
        "历史数据场目前空闲，按需可用。",
    ),
    2108: ErrorInfo(
        2108, "info",
        "Market-data farm inactive but available on demand.",
        "市场数据场目前空闲，按需可用。",
    ),
    2119: ErrorInfo(
        2119, "info",
        "Market-data farm connecting.",
        "市场数据场连接中。",
    ),
    2158: ErrorInfo(
        2158, "info",
        "Sec-def data farm connection is OK.",
        "证券定义数据场连接正常。",
    ),

    # -----------------------------------------------------------------
    # Hard connection / session events
    # -----------------------------------------------------------------
    1100: ErrorInfo(
        1100, "warning",
        "Connectivity between IB and TWS has been lost.",
        "TWS / IB 网关与 Interactive Brokers 服务器的连接已中断。",
        fix_en="Wait for auto-reconnect (1102) or restart IB Gateway / TWS.",
        fix_zh="等待自动重连（1102）或重启 IB Gateway / TWS。",
    ),
    1101: ErrorInfo(
        1101, "warning",
        "Connectivity restored — data subscriptions LOST and must be re-sent.",
        "连接已恢复 —— 数据订阅丢失，需要重新订阅。",
        fix_en="The bridge auto-reconnects and re-subscribes; no manual action.",
        fix_zh="bridge 会自动重连并重新订阅，无需手动操作。",
    ),
    1102: ErrorInfo(
        1102, "info",
        "Connectivity restored — data subscriptions maintained.",
        "连接已恢复，数据订阅自动保留。",
    ),
    1300: ErrorInfo(
        1300, "warning",
        "TWS socket port has been reset and the connection has been dropped.",
        "TWS socket 端口被重置，连接已断开。",
        fix_en=(
            "Almost always caused by changing API → Settings → Socket Port "
            "while the bridge is connected. The bridge auto-reconnects on "
            "the new port if you update `gateway.port` in your config."
        ),
        fix_zh=(
            "通常因为在 bridge 连接期间修改了 API → Settings → Socket Port。"
            "更新配置里的 `gateway.port` 后 bridge 会自动重连新端口。"
        ),
    ),

    # -----------------------------------------------------------------
    # Hard handshake / wire-level failures (often the FIRST error a new
    # user sees — they need actionable guidance, not a raw error number).
    # -----------------------------------------------------------------
    502: ErrorInfo(
        502, "error",
        "Couldn't connect to TWS. Confirm that 'Enable ActiveX and Socket "
        "EClients' is checked under API settings.",
        "无法连接到 TWS / IB Gateway。请确认 API 设置中 "
        "'Enable ActiveX and Socket EClients' 已勾选。",
        fix_en=(
            "1) Make sure TWS or IB Gateway is *running and logged in*; "
            "2) In TWS: Edit → Global Configuration → API → Settings, tick "
            "'Enable ActiveX and Socket Clients'; "
            "3) Verify the socket port matches your `gateway.port` config "
            "(TWS 7497=paper / 7496=live; standalone Gateway 4002=paper / "
            "4001=live); "
            "4) Add 127.0.0.1 to 'Trusted IPs' for headless setups; "
            "5) If running TWS / Gateway on a different host, set "
            "`gateway.host` accordingly and ensure the port is firewalled "
            "open ONLY for that host."
        ),
        fix_zh=(
            "1) 确认 TWS / IB Gateway 已启动且已登录；"
            "2) TWS 内：Edit → Global Configuration → API → Settings，"
            "勾选 'Enable ActiveX and Socket Clients'；"
            "3) 校验 socket 端口与配置 `gateway.port` 匹配（TWS 7497=paper / "
            "7496=live；独立 Gateway 4002=paper / 4001=live）；"
            "4) 无头/远程场景在 'Trusted IPs' 中加 127.0.0.1；"
            "5) 跨主机部署时设置 `gateway.host` 并仅对该主机开放对应端口。"
        ),
    ),
    504: ErrorInfo(
        504, "error",
        "Not connected — the API socket is closed.",
        "未连接 —— API socket 已关闭。",
        fix_en=(
            "Either TWS / Gateway shut down or it logged you out (idle "
            "timeout / 'Read-Only API' toggle change / overnight auto-restart). "
            "The bridge auto-reconnects with exponential backoff once "
            "TWS / Gateway is back up. If you keep seeing 504 in tight loops, "
            "the issue is upstream (TWS not running) — start it first."
        ),
        fix_zh=(
            "TWS / Gateway 已关闭或已登出（空闲超时 / 'Read-Only API' 开关变更 / "
            "夜间自动重启）。等 TWS / Gateway 重启后 bridge 会指数回退自动重连。"
            "若 504 反复刷屏说明上游问题（TWS 未运行），先启动 TWS。"
        ),
    ),
    507: ErrorInfo(
        507, "error",
        "Bad message length — API version mismatch between TWS and ib_async.",
        "Bad message length —— TWS 与 ib_async 的 API 版本不匹配。",
        fix_en=(
            "Update TWS / IB Gateway to a build that matches the ib_async "
            "wire protocol (TWS 10.x with API 10.19+ is required for "
            "ib_async ≥ 1.0). If you upgraded ib_async, also upgrade TWS."
        ),
        fix_zh=(
            "升级 TWS / IB Gateway 到与 ib_async 协议匹配的版本（ib_async ≥ 1.0 "
            "需要 TWS 10.x 配合 API 10.19+）。升级了 ib_async 就要同步升级 TWS。"
        ),
    ),
    326: ErrorInfo(
        326, "error",
        "Unable to connect as the client id is already in use.",
        "无法连接：clientId 已被占用。",
        fix_en=(
            "Another process is using the same clientId. The bridge "
            "default is 20; fetch_external uses 21 (ibkr-historical) and 22 "
            "(ibkr-bulk); the account probe uses 99. If you ran the bridge "
            "and it crashed without disconnecting, IBKR may keep the slot "
            "for ~30 s — wait it out, or set a different `gateway.client_id` "
            "in your config."
        ),
        fix_zh=(
            "其他进程占用了相同的 clientId。bridge 默认 20，fetch_external 用 21 "
            "(ibkr-historical) 和 22 (ibkr-bulk)，account probe 用 99。"
            "若 bridge 异常崩溃未正常断开，IBKR 会保留该 slot 约 30 秒 —— 稍等，"
            "或在配置中改 `gateway.client_id`。"
        ),
    ),

    # -----------------------------------------------------------------
    # Pacing / busy server (322 is what TWS returns when it's overloaded
    # OR when you reuse a reqId before the previous request finished).
    # -----------------------------------------------------------------
    322: ErrorInfo(
        322, "warning",
        "Error processing request — duplicate ticker id or server busy.",
        "请求处理错误 —— ticker id 重复或服务器繁忙。",
        fix_en=(
            "Almost always benign. The bridge's rate_limiter will retry "
            "after a short pause. If it persists, your historical-bar "
            "load is exceeding 60 distinct contracts per 10 minutes; "
            "spread your requests out or reduce the subscription set."
        ),
        fix_zh=(
            "通常无害，rate_limiter 会短暂回退后重试。若反复出现说明历史数据 "
            "请求超过了 60 contracts/10min 限额；分散请求或减少订阅。"
        ),
        doc_url=_DOC_HISTORICAL_LIMITS,
    ),

    # -----------------------------------------------------------------
    # Contract / request errors
    # -----------------------------------------------------------------
    162: ErrorInfo(
        162, "error",
        "Historical data service error (HMDS) — no data returned for query.",
        "历史数据服务错误，本次查询未返回数据。",
        fix_en=(
            "Common causes: (1) symbol or exchange wrong; "
            "(2) requested timeframe outside data availability "
            "(e.g. 30s bars older than 6 months); "
            "(3) market closed for the entire requested window. "
            "Try a longer duration (e.g. '5 D' instead of '1 D') and "
            "use --rth to filter to regular trading hours."
        ),
        fix_zh=(
            "常见原因：(1) 标的或交易所写错；"
            "(2) 时间窗口超出数据可用范围（如 30s bars 早于 6 个月）；"
            "(3) 整个查询窗口期间市场关闭。"
            "建议加大 duration（如 '5 D' 替代 '1 D'）并加 --rth。"
        ),
        doc_url=_DOC_HISTORICAL_LIMITS,
    ),
    200: ErrorInfo(
        200, "error",
        "No security definition found for the request.",
        "未找到对应的合约定义。",
        fix_en=(
            "Check `symbol`, `sec_type`, `exchange`, `currency` and "
            "`primary_exchange`. For US stocks try sec_type=STK exchange=SMART "
            "currency=USD with primary_exchange (NASDAQ for AAPL, ARCA for SPY)."
        ),
        fix_zh=(
            "检查 symbol / sec_type / exchange / currency / primary_exchange。"
            "美股建议 sec_type=STK, exchange=SMART, currency=USD，"
            "primary_exchange 显式指定（AAPL→NASDAQ, SPY→ARCA）。"
        ),
    ),
    354: ErrorInfo(
        354, "error",
        "Requested market data is not subscribed for this account.",
        "本账户未订阅请求的市场数据。",
        fix_en=(
            "Either subscribe to the matching market-data package on the "
            "*live* account, or set `market_data_type: 3` in your bridge "
            "config to use delayed data."
        ),
        fix_zh=(
            "在 *live* 账户上订阅对应的市场数据包，或在 bridge 配置中设 "
            "`market_data_type: 3` 改用延时数据。"
        ),
        doc_url=_DOC_MD_SUBS,
    ),
    366: ErrorInfo(
        366, "warning",
        "No historical data query found for the given ticker id "
        "(stale request or already cancelled).",
        "未找到对应 ticker id 的历史数据查询（请求已过期或已取消）。",
    ),
    420: ErrorInfo(
        420, "error",
        "No market-data permissions for the requested exchange route.",
        "请求的交易所路由没有市场数据权限。",
        fix_en=(
            "The IBKR-PRO 'Non-consolidated Real-Time Quotes' bundle covers "
            "common venues but excludes some specific routes (ISLAND for "
            "NASDAQ direct, AMEX TOP/ALL, etc.). Either subscribe to the "
            "specific exchange MD package (TotalView for NASDAQ, AMEX TOP, …) "
            "OR set `market_data_type: 3` in your config to fall back to "
            "delayed data which works on every account."
        ),
        fix_zh=(
            "IBKR-PRO 非整合实时报价包覆盖常见路由，但不含某些专属路由"
            "（NASDAQ 直连 ISLAND、AMEX TOP/ALL 等）。要么订阅对应交易所 "
            "MD 包（TotalView for NASDAQ, AMEX TOP 等），要么在配置中设 "
            "`market_data_type: 3` 改用延时数据 —— 后者所有账户都可用。"
        ),
        doc_url=_DOC_MD_SUBS,
    ),

    # -----------------------------------------------------------------
    # Market data subscription / session
    # -----------------------------------------------------------------
    10089: ErrorInfo(
        10089, "error",
        "Requested market data requires additional subscription for API; "
        "delayed market data is available.",
        "API 请求的市场数据需要额外订阅；延时市场数据可用。",
        fix_en=(
            "Set `market_data_type: 3` in your bridge config — IBKR will "
            "automatically substitute ~15 min delayed data (free for most "
            "accounts) for the routes you don't subscribe to."
        ),
        fix_zh=(
            "在 bridge 配置中设 `market_data_type: 3` —— IBKR 会自动用 "
            "~15min 延时数据（多数账户免费）替代未订阅的路由。"
        ),
        doc_url=_DOC_DELAYED_MD,
    ),
    10167: ErrorInfo(
        10167, "info",
        "Displaying delayed market data (because live not subscribed).",
        "正在显示延时市场数据（因为未订阅实时）。",
    ),
    10299: ErrorInfo(
        10299, "error",
        "Wrong whatToShow for this security type "
        "(usually crypto needs AGGTRADES instead of TRADES).",
        "whatToShow 类型不匹配（加密货币历史数据需用 AGGTRADES 而非 TRADES）。",
        fix_en=(
            "For CRYPTO contracts on PAXOS, set `what_to_show: AGGTRADES` "
            "(or MIDPOINT/BID/ASK) in your bars_kup subscription. TRADES is "
            "only valid for instruments with explicit trade prints (stocks, "
            "futures, FX). Crypto on IBKR's PAXOS sub-broker uses aggregate "
            "trade ticks instead."
        ),
        fix_zh=(
            "PAXOS 加密货币合约请将 `what_to_show: AGGTRADES`（或 "
            "MIDPOINT/BID/ASK）。TRADES 仅对有明确成交打印的标的（股票、"
            "期货、外汇）有效。IBKR 的 PAXOS 子经纪商用聚合成交 tick。"
        ),
        doc_url=(
            "https://ibkrcampus.com/campus/ibkr-api-page/twsapi-doc/"
            "#historical-bar-whattoshow"
        ),
    ),
    10197: ErrorInfo(
        10197, "error",
        "No market data during associated live-account login.",
        "关联的真实账户登录期间无市场数据。",
        fix_en=(
            "(1) Log out of ALL other live IBKR sessions: mobile app (kill "
            "from app switcher), Client Portal web, any other TWS / Gateway "
            "instance. Wait 60-90 s. (2) Ensure your live account has at "
            "least the free 'US Securities Snapshot and Futures Value Bundle' "
            "or 'IBKR-PRO Non-Consolidated Real-Time Quotes' subscribed. "
            "(3) As a temporary workaround, set `market_data_type: 3`."
        ),
        fix_zh=(
            "(1) 退出所有其他 live IBKR session：手机 app（从多任务划掉，不只后台）、"
            "Client Portal 网页、其他 TWS / Gateway 实例。等 60-90 秒。"
            "(2) 确保你的 live 账户至少订阅了免费的 'US Securities Snapshot' "
            "或 'IBKR-PRO 非整合实时报价' 包。"
            "(3) 临时绕开：在配置中设 `market_data_type: 3`。"
        ),
        doc_url=_DOC_MD_SUBS,
    ),

    # -----------------------------------------------------------------
    # Pacing / rate limit:
    # IBKR pacing violations surface through several codes depending on
    # TWS version (often code 420 with a "Pacing violation" message body).
    # Our rate_limiter.py enforces 60 req / 10 min + 6 req / 2 s same-contract
    # locally so well-behaved bridges never see them. If you do hit one,
    # check that capabilities have been probed via setup.py status.
    # -----------------------------------------------------------------
}


def lookup(code: int) -> ErrorInfo | None:
    """Return enriched info for an IBKR error code or None if unknown."""
    return _CATALOG.get(code)


def is_info(code: int) -> bool:
    """True for purely informational codes that should not pollute logs."""
    info = _CATALOG.get(code)
    return info is not None and info.severity == "info"


def format_for_log(code: int, raw_msg: str, contract_label: str | None) -> str:
    """Render a multi-line, bilingual log entry for an IBKR error.

    Falls back to a single-line raw form for codes not in the catalog.
    """
    info = _CATALOG.get(code)
    if info is None:
        return (
            f"IBKR error code={code} contract={contract_label} "
            f"msg={raw_msg!r}  (uncatalogued — please file an issue)"
        )

    parts = [
        f"IBKR {info.severity.upper()} {code} contract={contract_label}",
        f"  Cause (EN): {info.cause_en}",
        f"  原因 (中文): {info.cause_zh}",
    ]
    if info.fix_en:
        parts.append(f"  Fix  (EN): {info.fix_en}")
    if info.fix_zh:
        parts.append(f"  修复 (中文): {info.fix_zh}")
    if info.doc_url:
        parts.append(f"  Docs: {info.doc_url}")
    if raw_msg:
        # Keep the IBKR-original string as the last line for traceability.
        parts.append(f"  Raw: {raw_msg!r}")
    return "\n".join(parts)
