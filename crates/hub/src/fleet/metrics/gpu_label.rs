//! The label a chart shows for a series (R-099, K1).

/// `GPU <index> <name>` for a GPU series, `GPU <index>` when its GPU is no longer stored, and the
/// stored label for every other series.
pub(super) fn chart_label(stored: &str, gpu_index: Option<i32>, gpu_name: Option<&str>) -> String {
    match (gpu_index, gpu_name) {
        (Some(index), Some(name)) => format!("GPU {index} {name}"),
        (Some(index), None) => format!("GPU {index}"),
        (None, _) => stored.to_owned(),
    }
}
