"""
FactorEngine — Experience Knowledge Base with Learn-from-Failures

Paper: https://arxiv.org/abs/2603.16365v2
Implements: Chain-of-Experience (CoE) and failure-aware factor mutation

Key contributions:
  - Experience knowledge base stores full evolution trajectories (not just best)
  - Chains of Experience (CoE) expose failure paths to LLM for learning
  - "learning from failures" — agents internalize feedback from dips
  - UCT-based node selection balances exploration/exploitation

For ict-engine:
  - Store rejected mutation reasons as "experience"
  - Before new mutation, query knowledge base for similar failure patterns
  - Avoid repeating known failure modes
"""

import json
import hashlib
import time
from dataclasses import dataclass, field, asdict
from typing import Optional
from pathlib import Path


@dataclass
class MutationExperience:
    """§4.2 — Single mutation experience entry.
    
    "FE explicitly captures the full dynamic process, including transient
    fluctuations and local setbacks, rather than just the final success."
    """
    mutation_id: str
    timestamp: float
    factor_name: str
    spec_hash: str               # hash of mutation spec
    
    # Performance
    score_before: float
    score_after: float
    score_delta: float
    accepted: bool
    
    # Failure analysis (§4.2 — "learning from failures")
    failure_tags: list[str]      # e.g., ["composite_regressed", "gate_regressed"]
    failure_reasons: list[str]   # human-readable explanations
    
    # Context
    regime: str                  # market regime when mutation ran
    objective: str               # research objective
    parent_id: Optional[str] = None  # parent mutation in evolution tree
    
    # Metadata
    params_before: dict = field(default_factory=dict)
    params_after: dict = field(default_factory=dict)
    
    def to_dict(self) -> dict:
        return asdict(self)
    
    @staticmethod
    def spec_hash_fn(spec: dict) -> str:
        """Deterministic hash of mutation spec."""
        raw = json.dumps(spec, sort_keys=True)
        return hashlib.sha256(raw.encode()).hexdigest()[:16]


@dataclass
class ChainOfExperience:
    """§4.2 — Chain of Experience (CoE): historical trajectory.
    
    "Each path serves as a compact representation of prior evolution experience."
    """
    chain_id: str
    experiences: list[MutationExperience]
    
    @property
    def success_rate(self) -> float:
        if not self.experiences:
            return 0.0
        return sum(1 for e in self.experiences if e.accepted) / len(self.experiences)
    
    @property
    def cumulative_delta(self) -> float:
        return sum(e.score_delta for e in self.experiences)
    
    @property
    def common_failures(self) -> dict[str, int]:
        """Most common failure tags across the chain."""
        counts: dict[str, int] = {}
        for e in self.experiences:
            for tag in e.failure_tags:
                counts[tag] = counts.get(tag, 0) + 1
        return dict(sorted(counts.items(), key=lambda x: -x[1]))


class ExperienceKnowledgeBase:
    """§4.2 — Experience knowledge base for factor mutation.
    
    "Unlike prior works which predominantly focus on static high-performing
    nodes, exposing the agent to these winding historical trajectories
    stimulates human-like reasoning. This enables LLMs to internalize
    feedback from failures, learn to recover from performance dips, and
    steer exploration toward more robust and promising directions."
    
    For ict-engine: stores all mutation attempts (accepted + rejected),
    enabling "don't repeat this failure pattern" queries.
    """
    
    def __init__(self, storage_path: Optional[str] = None):
        self.experiences: list[MutationExperience] = []
        self.chains: list[ChainOfExperience] = []
        self.storage_path = Path(storage_path) if storage_path else None
        
        if self.storage_path and self.storage_path.exists():
            self._load()
    
    def record(self, exp: MutationExperience):
        """Record a new mutation experience."""
        self.experiences.append(exp)
        if self.storage_path:
            self._save()
    
    def query_similar_failures(
        self,
        factor_name: str,
        spec: dict,
        regime: str = "",
        k: int = 5,
    ) -> list[MutationExperience]:
        """§4.2 — Query for similar past failures.
        
        Before a new mutation, check if similar specs have failed before.
        Returns the k most similar rejected experiences.
        """
        spec_hash = MutationExperience.spec_hash_fn(spec)
        
        # Find rejected experiences with same factor or similar spec
        candidates = [
            e for e in self.experiences
            if not e.accepted
            and (e.factor_name == factor_name or e.spec_hash == spec_hash)
        ]
        
        # Sort by recency
        candidates.sort(key=lambda e: e.timestamp, reverse=True)
        
        return candidates[:k]
    
    def failure_summary(self, factor_name: str = "") -> dict:
        """§4.2 — Summary of failure patterns.
        
        Returns most common failure tags and reasons, useful for
        "what NOT to do" guidance.
        """
        if factor_name:
            exps = [e for e in self.experiences if e.factor_name == factor_name]
        else:
            exps = self.experiences
        
        rejected = [e for e in exps if not e.accepted]
        
        tag_counts: dict[str, int] = {}
        for e in rejected:
            for tag in e.failure_tags:
                tag_counts[tag] = tag_counts.get(tag, 0) + 1
        
        # Avg delta for each failure tag
        tag_deltas: dict[str, list[float]] = {}
        for e in rejected:
            for tag in e.failure_tags:
                tag_deltas.setdefault(tag, []).append(e.score_delta)
        
        return {
            "total_rejected": len(rejected),
            "total_accepted": len([e for e in exps if e.accepted]),
            "top_failure_tags": dict(sorted(tag_counts.items(), key=lambda x: -x[1])[:10]),
            "avg_delta_by_tag": {
                tag: sum(deltas)/len(deltas)
                for tag, deltas in tag_deltas.items()
            },
        }
    
    def build_coe(self, max_chain_length: int = 10) -> list[ChainOfExperience]:
        """§4.2 — Build Chains of Experience from recorded history.
        
        Group related mutations into chains (by parent_id or factor_name).
        """
        # Group by factor_name + regime
        groups: dict[str, list[MutationExperience]] = {}
        for e in self.experiences:
            key = f"{e.factor_name}:{e.regime}"
            groups.setdefault(key, []).append(e)
        
        chains = []
        for key, exps in groups.items():
            # Sort by timestamp
            exps.sort(key=lambda e: e.timestamp)
            # Split into chains of max_chain_length
            for i in range(0, len(exps), max_chain_length):
                chunk = exps[i:i+max_chain_length]
                chain = ChainOfExperience(
                    chain_id=f"{key}:{i//max_chain_length}",
                    experiences=chunk,
                )
                chains.append(chain)
        
        self.chains = chains
        return chains
    
    def get_avoidance_hints(self, factor_name: str, n: int = 3) -> list[str]:
        """§4.2 — Get actionable "avoid these" hints.
        
        "enables LLMs to internalize feedback from failures"
        For ict-engine: return concrete hints about what NOT to try.
        """
        summary = self.failure_summary(factor_name)
        hints = []
        
        for tag, count in list(summary["top_failure_tags"].items())[:n]:
            avg_delta = summary["avg_delta_by_tag"].get(tag, 0)
            hints.append(
                f"Failure pattern '{tag}' occurred {count} times "
                f"(avg delta={avg_delta:.4f}). "
                f"Avoid mutations that trigger this pattern."
            )
        
        if not hints:
            hints.append("No failure history available. Proceed with caution.")
        
        return hints
    
    def _save(self):
        """Persist to disk."""
        if not self.storage_path:
            return
        self.storage_path.parent.mkdir(parents=True, exist_ok=True)
        data = [e.to_dict() for e in self.experiences]
        with open(self.storage_path, 'w') as f:
            json.dump(data, f, indent=2)
    
    def _load(self):
        """Load from disk."""
        if not self.storage_path or not self.storage_path.exists():
            return
        with open(self.storage_path) as f:
            data = json.load(f)
        self.experiences = [MutationExperience(**d) for d in data]


# ── Tests ──────────────────────────────────────────────────────────────

def _test_record_and_query():
    kb = ExperienceKnowledgeBase()
    
    # Record some failures
    for i in range(5):
        kb.record(MutationExperience(
            mutation_id=f"m{i}",
            timestamp=time.time() + i,
            factor_name="structure_ict",
            spec_hash="abc123",
            score_before=0.5,
            score_after=0.48,
            score_delta=-0.02,
            accepted=False,
            failure_tags=["composite_regressed", "gate_regressed"],
            failure_reasons=["Score regressed", "Gate blocked"],
            regime="ranging",
            objective="expansion_manipulation",
        ))
    
    # Query
    similar = kb.query_similar_failures("structure_ict", {"lookback": 10})
    assert len(similar) == 5
    assert all(not e.accepted for e in similar)
    print(f"  ✓ Record & query: {len(similar)} similar failures found")


def _test_failure_summary():
    kb = ExperienceKnowledgeBase()
    kb.record(MutationExperience("m1", time.time(), "f1", "h1", 0.5, 0.48, -0.02, False,
                                  ["composite_regressed"], ["reason"], "trend", "exp"))
    kb.record(MutationExperience("m2", time.time(), "f1", "h2", 0.5, 0.45, -0.05, False,
                                  ["composite_regressed", "bridge_gap_too_small"], ["r1","r2"], "trend", "exp"))
    
    summary = kb.failure_summary("f1")
    assert summary["total_rejected"] == 2
    assert summary["top_failure_tags"]["composite_regressed"] == 2
    print(f"  ✓ Failure summary: {summary['total_rejected']} rejected, top tag={list(summary['top_failure_tags'].keys())[0]}")


def _test_coe_building():
    kb = ExperienceKnowledgeBase()
    for i in range(15):
        kb.record(MutationExperience(f"m{i}", time.time()+i, "f1", f"h{i}", 0.5, 0.5+i*0.001, i*0.001, i%3!=0,
                                      [], [], "trend", "exp"))
    
    chains = kb.build_coe(max_chain_length=5)
    assert len(chains) > 0
    assert all(len(c.experiences) <= 5 for c in chains)
    print(f"  ✓ CoE building: {len(chains)} chains, avg length={sum(len(c.experiences) for c in chains)/len(chains):.1f}")


def _test_avoidance_hints():
    kb = ExperienceKnowledgeBase()
    for tag in ["composite_regressed", "gate_regressed", "bridge_gap_too_small"]:
        kb.record(MutationExperience(f"m_{tag}", time.time(), "f1", "h", 0.5, 0.45, -0.05, False,
                                      [tag], [f"Failed: {tag}"], "trend", "exp"))
    
    hints = kb.get_avoidance_hints("f1")
    assert len(hints) >= 2
    assert "composite_regressed" in hints[0] or "gate_regressed" in hints[0]
    print(f"  ✓ Avoidance hints: {len(hints)} hints generated")


def run_tests():
    print("Running FactorEngine tests...")
    _test_record_and_query()
    _test_failure_summary()
    _test_coe_building()
    _test_avoidance_hints()
    print("All FactorEngine tests passed.")


if __name__ == "__main__":
    run_tests()
