//! Charts - Chapter 13

use super::*;

tagged_index! {
    pub struct ChartIndex {
        const TAG_BITS: usize = 6;
        tag: ChartSort,
        index: u32,
    }
}

#[c_enum(storage = "u32")]
pub enum ChartSort {
    NONE = 0,
    UNILEVEL = 1,
    MULTILEVEL = 2,
}

/// `chart.unilevel`
#[repr(C)]
#[derive(Clone, AsBytes, FromBytes)]
pub struct ChartUnilevel {
    pub start: Index,
    pub cardinality: Cardinality,
    pub constraint: ExprIndex,
}

/// `chart.multilevel`
#[repr(C)]
#[derive(Clone, AsBytes, FromBytes)]
pub struct ChartMultilevel {
    pub start: Index,
    pub cardinality: Cardinality,
}

