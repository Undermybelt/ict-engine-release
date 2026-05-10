//! Co-occurrence and precedence matrices over a `Vec<PdaEvent>`.
//!
//! Both matrices are `13 × 13` (one row/col per `PdaEventKind`) and
//! indexed in the canonical order `ALL_EVENT_KINDS`. They count
//! ordered pairs of events whose emission bars satisfy a window
//! constraint:
//!
//! - **Co-occurrence** is symmetric: `M[A,B]` counts the number of
//!   ordered (i, j) index pairs in the sorted timeline where
//!   `events[i].kind == A && events[j].kind == B && i != j &&
//!   |events[i].bar - events[j].bar| <= window`.
//!   Note: `M[A,A]` counts self-pairs (different timeline indices,
//!   same kind, within window).
//! - **Precedence** is asymmetric: `P[A,B]` counts pairs where A
//!   *precedes* B (`events[i].bar < events[j].bar`) within the
//!   window.
//!
//! Algorithm: O(n²) over the timeline with early termination once
//! `events[j].bar - events[i].bar > window`. n is typically a few
//! hundred per session so the constant matters more than the bound.

use serde::{Deserialize, Serialize};

use super::event::{PdaEvent, PdaEventKind, ALL_EVENT_KINDS};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMatrix {
    pub kinds: Vec<PdaEventKind>,
    pub counts: Vec<Vec<u32>>,
    pub window_bars: usize,
    pub kind: MatrixKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatrixKind {
    Cooccurrence,
    Precedence,
}

impl EventMatrix {
    pub fn empty(window_bars: usize, kind: MatrixKind) -> Self {
        let n = ALL_EVENT_KINDS.len();
        Self {
            kinds: ALL_EVENT_KINDS.to_vec(),
            counts: vec![vec![0u32; n]; n],
            window_bars,
            kind,
        }
    }

    pub fn count(&self, from: PdaEventKind, to: PdaEventKind) -> u32 {
        let i = kind_index(from);
        let j = kind_index(to);
        self.counts[i][j]
    }

    pub fn total(&self) -> u64 {
        self.counts
            .iter()
            .flat_map(|row| row.iter())
            .map(|c| *c as u64)
            .sum()
    }
}

pub fn compute_cooccurrence_matrix(events: &[PdaEvent], window_bars: usize) -> EventMatrix {
    let mut m = EventMatrix::empty(window_bars, MatrixKind::Cooccurrence);
    if events.len() < 2 {
        return m;
    }
    for i in 0..events.len() {
        for j in (i + 1)..events.len() {
            let dt = events[j].bar_index.saturating_sub(events[i].bar_index);
            if dt > window_bars {
                break; // events are sorted; no later j will be closer
            }
            // Count both (i,j) and (j,i) for symmetry.
            let a = kind_index(events[i].kind);
            let b = kind_index(events[j].kind);
            m.counts[a][b] = m.counts[a][b].saturating_add(1);
            m.counts[b][a] = m.counts[b][a].saturating_add(1);
        }
    }
    m
}

pub fn compute_precedence_matrix(events: &[PdaEvent], window_bars: usize) -> EventMatrix {
    let mut m = EventMatrix::empty(window_bars, MatrixKind::Precedence);
    if events.len() < 2 {
        return m;
    }
    for i in 0..events.len() {
        for j in (i + 1)..events.len() {
            let dt = events[j].bar_index.saturating_sub(events[i].bar_index);
            if dt > window_bars {
                break;
            }
            // Strict precedence: A at bar < B at bar. Skip ties (same bar).
            if events[i].bar_index == events[j].bar_index {
                continue;
            }
            let a = kind_index(events[i].kind);
            let b = kind_index(events[j].kind);
            m.counts[a][b] = m.counts[a][b].saturating_add(1);
        }
    }
    m
}

fn kind_index(kind: PdaEventKind) -> usize {
    ALL_EVENT_KINDS
        .iter()
        .position(|k| *k == kind)
        .expect("ALL_EVENT_KINDS covers every PdaEventKind variant by construction")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Direction;

    fn ev(kind: PdaEventKind, bar: usize) -> PdaEvent {
        PdaEvent::new(kind, bar, Direction::Bull).with_level(100.0)
    }

    #[test]
    fn empty_timeline_yields_empty_matrices() {
        let cm = compute_cooccurrence_matrix(&[], 5);
        let pm = compute_precedence_matrix(&[], 5);
        assert_eq!(cm.total(), 0);
        assert_eq!(pm.total(), 0);
    }

    #[test]
    fn single_event_yields_empty_matrices() {
        let events = vec![ev(PdaEventKind::FairValueGap, 10)];
        assert_eq!(compute_cooccurrence_matrix(&events, 5).total(), 0);
        assert_eq!(compute_precedence_matrix(&events, 5).total(), 0);
    }

    #[test]
    fn cooccurrence_is_symmetric() {
        let events = vec![
            ev(PdaEventKind::FairValueGap, 10),
            ev(PdaEventKind::OrderBlock, 12),
        ];
        let m = compute_cooccurrence_matrix(&events, 5);
        assert_eq!(
            m.count(PdaEventKind::FairValueGap, PdaEventKind::OrderBlock),
            m.count(PdaEventKind::OrderBlock, PdaEventKind::FairValueGap)
        );
        assert_eq!(
            m.count(PdaEventKind::FairValueGap, PdaEventKind::OrderBlock),
            1
        );
    }

    #[test]
    fn precedence_is_asymmetric() {
        let events = vec![
            ev(PdaEventKind::FairValueGap, 10),
            ev(PdaEventKind::OrderBlock, 12),
        ];
        let m = compute_precedence_matrix(&events, 5);
        assert_eq!(
            m.count(PdaEventKind::FairValueGap, PdaEventKind::OrderBlock),
            1,
            "FVG precedes OB"
        );
        assert_eq!(
            m.count(PdaEventKind::OrderBlock, PdaEventKind::FairValueGap),
            0,
            "OB does not precede FVG in this timeline"
        );
    }

    #[test]
    fn window_excludes_far_pairs() {
        let events = vec![
            ev(PdaEventKind::FairValueGap, 10),
            ev(PdaEventKind::OrderBlock, 100),
        ];
        let near = compute_cooccurrence_matrix(&events, 5);
        let far = compute_cooccurrence_matrix(&events, 200);
        assert_eq!(near.total(), 0);
        assert_eq!(far.total(), 2); // (FVG,OB) + (OB,FVG)
    }

    #[test]
    fn same_kind_self_cooccurrence_is_counted() {
        let events = vec![
            ev(PdaEventKind::FairValueGap, 10),
            ev(PdaEventKind::FairValueGap, 13),
        ];
        let m = compute_cooccurrence_matrix(&events, 5);
        assert_eq!(
            m.count(PdaEventKind::FairValueGap, PdaEventKind::FairValueGap),
            2 // both (i,j) and (j,i) directions
        );
    }

    #[test]
    fn precedence_skips_ties() {
        let events = vec![
            ev(PdaEventKind::FairValueGap, 10),
            ev(PdaEventKind::OrderBlock, 10), // same bar
            ev(PdaEventKind::Cisd, 12),
        ];
        let m = compute_precedence_matrix(&events, 5);
        assert_eq!(
            m.count(PdaEventKind::FairValueGap, PdaEventKind::OrderBlock),
            0,
            "ties must not contribute to precedence"
        );
        assert_eq!(m.count(PdaEventKind::FairValueGap, PdaEventKind::Cisd), 1);
        assert_eq!(m.count(PdaEventKind::OrderBlock, PdaEventKind::Cisd), 1);
    }

    #[test]
    fn determinism_across_calls() {
        let events: Vec<PdaEvent> = (0..50)
            .map(|i| {
                let kind = match i % 3 {
                    0 => PdaEventKind::FairValueGap,
                    1 => PdaEventKind::OrderBlock,
                    _ => PdaEventKind::PropulsionBlock,
                };
                ev(kind, i * 2)
            })
            .collect();
        let a = compute_cooccurrence_matrix(&events, 10);
        let b = compute_cooccurrence_matrix(&events, 10);
        assert_eq!(a.counts, b.counts);
        let c = compute_precedence_matrix(&events, 10);
        let d = compute_precedence_matrix(&events, 10);
        assert_eq!(c.counts, d.counts);
    }
}
