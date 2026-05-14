"""Compatibility alias for the generic killzone breakout strategy source path.

The maintained implementation lives in ``TomacNQ_KillzoneBreakout.py``. Some
candidate metadata refers to the generic parent strategy name, so this wrapper
keeps that source path loadable without duplicating strategy logic.
"""

from __future__ import annotations

from TomacNQ_KillzoneBreakout import TomacNQ_KillzoneBreakout


class TomacKillzoneBreakout(TomacNQ_KillzoneBreakout):
    pass
