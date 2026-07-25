pub mod detector;
pub mod error;
pub mod normalizer;
pub mod report;

pub use detector::{AnalysisResult, NPlusOneGroup, SlowQuery};
pub use error::DbError;
pub use normalizer::normalize;

use runlens_core::event_v2::EventV2;

pub fn analyze_session(
    repo: &runlens_storage::Repository,
    session_id: &str,
    n_plus_one_threshold: usize,
    slow_query_ns: i64,
) -> Result<AnalysisResult, DbError> {
    let events: Vec<EventV2> = repo
        .list_events(session_id)
        .map_err(|e| DbError::Storage(e.into()))?
        .into_iter()
        .map(EventV2::from_v1)
        .collect();

    let n_plus_one = detector::detect_n_plus_one(&events, n_plus_one_threshold);
    let slow_queries = detector::detect_slow_queries(&events, slow_query_ns);

    Ok(AnalysisResult {
        session_id: session_id.to_string(),
        n_plus_one,
        slow_queries,
    })
}
