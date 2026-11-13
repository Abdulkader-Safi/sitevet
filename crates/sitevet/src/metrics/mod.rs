//! Collectors, ported from the TS package's `src/metrics/*.ts`. Metrics are values and tables
//! with their own status; they never touch the score.

mod directives;
mod headings;
mod links;
mod markup;
mod meta;
mod page;
mod text;

use std::sync::Arc;

use crate::report::{MetricCell, ValueStatus};
use crate::{Collector, MetricInput};

/// Every built-in collector, in the TS package's order.
pub(crate) fn all() -> Vec<Arc<dyn Collector>> {
    [
        page::collectors(),
        markup::collectors(),
        directives::collectors(),
        links::collectors(),
        headings::collectors(),
        meta::collectors(),
        text::collectors(),
    ]
    .concat()
}

/// A value metric without a unit.
pub(crate) fn value(id: &str, label: &str, v: Option<MetricCell>) -> MetricInput {
    MetricInput::value(id, label, v, None)
}

pub(crate) fn num(n: impl Into<f64>) -> Option<MetricCell> {
    Some(MetricCell::Number(n.into()))
}

pub(crate) fn txt(s: impl Into<String>) -> Option<MetricCell> {
    Some(MetricCell::Text(s.into()))
}

pub(crate) fn cell_n(n: impl Into<f64>) -> MetricCell {
    MetricCell::Number(n.into())
}

pub(crate) fn cell(s: impl Into<String>) -> MetricCell {
    MetricCell::Text(s.into())
}

/// ok inside the band, warn outside it.
pub(crate) fn in_range(n: usize, min: usize, max: usize) -> ValueStatus {
    if (min..=max).contains(&n) { ValueStatus::Ok } else { ValueStatus::Warn }
}

/// ok up to `warn`, warn up to `bad`, bad past it.
pub(crate) fn up_to(n: f64, warn: f64, bad: f64) -> ValueStatus {
    if n <= warn {
        ValueStatus::Ok
    } else if n <= bad {
        ValueStatus::Warn
    } else {
        ValueStatus::Bad
    }
}

/// An empty table cell. The TS package uses an em dash; sitevet's text avoids it.
pub(crate) const EMPTY: &str = "-";
