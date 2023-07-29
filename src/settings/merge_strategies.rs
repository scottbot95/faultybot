use super::*;

/// Always chooses the rhs of the merge since sources are already sorted by least-to-most specific
pub struct MostSpecific;

impl<V> MergeFn<V> for MostSpecific {
    fn merge(&self, _lhs: &V, _rhs: &V) -> MergeDecision {
        MergeDecision::Right
    }
}

pub struct Smallest;

impl<V: PartialOrd> MergeFn<V> for Smallest {
    fn merge(&self, lhs: &V, rhs: &V) -> MergeDecision {
        if rhs > lhs {
            MergeDecision::Right
        } else {
            MergeDecision::Left
        }
    }
}

pub struct Largest;

impl<V: PartialOrd> MergeFn<V> for Largest {
    fn merge(&self, lhs: &V, rhs: &V) -> MergeDecision {
        if rhs > lhs {
            MergeDecision::Right
        } else {
            MergeDecision::Left
        }
    }
}
