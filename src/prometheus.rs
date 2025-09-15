use crate::state::State;
use std::collections::HashSet;
use std::time::Instant;

/// Generates metrics in Prometheus text format.
pub fn generate_metrics(state: &State, sse_counter: u64, start_time: &Instant) -> String {
    let mut lines = Vec::new();

    // --- Uptime ---
    lines.push("# HELP uptime_seconds Server process uptime in seconds.".to_string());
    lines.push("# TYPE uptime_seconds gauge".to_string());
    lines.push(format!(
        "uptime_seconds {}",
        start_time.elapsed().as_secs_f64()
    ));

    // --- Sessions ---
    lines.push("# HELP ducktracker_active_sessions Number of active sharing sessions.".to_string());
    lines.push("# TYPE ducktracker_active_sessions gauge".to_string());
    lines.push(format!(
        "ducktracker_active_sessions {}",
        state.num_sessions()
    ));

    // --- SSE Streams ---
    lines
        .push("# HELP ducktracker_open_sse_streams Number of open SSE client streams.".to_string());
    lines.push("# TYPE ducktracker_open_sse_streams gauge".to_string());
    lines.push(format!("ducktracker_open_sse_streams {}", sse_counter));

    // --- Points ---
    let ducktracker_total_points: usize = state.iter_sessions().map(|s| s.locations.len()).sum();
    lines.push(
        "# HELP ducktracker_total_points Total number of location points in memory.".to_string(),
    );
    lines.push("# TYPE ducktracker_total_points gauge".to_string());
    lines.push(format!(
        "ducktracker_total_points {}",
        ducktracker_total_points
    ));

    // --- Tags ---
    lines.push("# HELP ducktracker_public_tags Number of known public tags.".to_string());
    lines.push("# TYPE ducktracker_public_tags gauge".to_string());
    lines.push(format!(
        "ducktracker_public_tags {}",
        state.public_tags().len()
    ));

    let ducktracker_private_tags: HashSet<_> = state
        .iter_sessions()
        .flat_map(|s| s.tags.0.iter())
        .filter(|t| !t.is_public())
        .map(|t| &t.name)
        .collect();
    lines.push(
        "# HELP ducktracker_private_tags Number of unique private tags across all sessions."
            .to_string(),
    );
    lines.push("# TYPE ducktracker_private_tags gauge".to_string());
    lines.push(format!(
        "ducktracker_private_tags {}",
        ducktracker_private_tags.len()
    ));

    lines.push(
        "# HELP ducktracker_info Build information about the DuckTracker server.".to_string(),
    );
    lines.push("# TYPE ducktracker_info gauge".to_string());
    lines.push(format!(
        "ducktracker_info{{version=\"{}\"}} 1",
        crate::version::VERSION
    ));

    // NOTE: The more complex average-based metrics require tracking historical
    // data (e.g., all post timestamps, session start/end times) which is
    // not currently stored in the application state.

    lines.join("\n") + "\n"
}
