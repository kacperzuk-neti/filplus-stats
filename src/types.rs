use crate::error::Error;
use axum::Json;
use serde::Serialize;

pub type JsonResult<T> = Result<Json<T>, Error>;

#[derive(Serialize)]
pub struct HistogramEntry {
    pub value_from_exclusive: f64,
    pub value_to_inclusive: f64,
    pub count: i64,
}

#[derive(Serialize)]
pub struct Histogram {
    pub total_count: i64,
    pub buckets: Vec<HistogramEntry>,
}
