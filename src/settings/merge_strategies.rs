use super::*;

pub struct TakeRight;

impl<V> MergeFn<V> for TakeRight {
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
