use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorCellBounds {
    pub start_col: usize,
    pub end_col: usize,
    pub start_row: usize,
    pub end_row: usize,
    pub is_primary: bool,
}

impl MonitorCellBounds {
    pub fn width(&self) -> usize {
        self.end_col.saturating_sub(self.start_col)
    }

    pub fn height(&self) -> usize {
        self.end_row.saturating_sub(self.start_row)
    }

    pub fn center_col(&self) -> usize {
        self.start_col + self.width() / 2
    }

    pub fn center_row(&self) -> usize {
        self.start_row + self.height() / 2
    }

    pub fn contains(&self, col: usize, row: usize) -> bool {
        col >= self.start_col && col < self.end_col && row >= self.start_row && row < self.end_row
    }
}

pub static MONITOR_BOUNDS_CALLBACK: OnceLock<fn(usize, usize) -> MonitorCellBounds> =
    OnceLock::new();
pub static IS_SECONDARY_MONITOR_CALLBACK: OnceLock<fn() -> bool> = OnceLock::new();

pub fn get_primary_monitor_bounds(cols: usize, rows: usize) -> MonitorCellBounds {
    if let Some(callback) = MONITOR_BOUNDS_CALLBACK.get() {
        return callback(cols, rows);
    }
    if let Some(bounds) = cached_primary_bounds_from_env() {
        return bounds;
    }
    MonitorCellBounds {
        start_col: 0,
        end_col: cols,
        start_row: 0,
        end_row: rows,
        is_primary: true,
    }
}

static ENV_PRIMARY_BOUNDS: OnceLock<Mutex<Option<MonitorCellBounds>>> = OnceLock::new();

fn env_bounds_cache() -> &'static Mutex<Option<MonitorCellBounds>> {
    ENV_PRIMARY_BOUNDS.get_or_init(|| Mutex::new(None))
}

fn cached_primary_bounds_from_env() -> Option<MonitorCellBounds> {
    let mut cache = env_bounds_cache().lock().unwrap();
    if cache.is_none() {
        *cache = read_primary_bounds_from_env();
    }
    *cache
}

fn read_primary_bounds_from_env() -> Option<MonitorCellBounds> {
    let start_col = std::env::var("TRANCE_PRIMARY_START_COL")
        .ok()?
        .parse()
        .ok()?;
    let end_col = std::env::var("TRANCE_PRIMARY_END_COL").ok()?.parse().ok()?;
    let start_row = std::env::var("TRANCE_PRIMARY_START_ROW")
        .ok()?
        .parse()
        .ok()?;
    let end_row = std::env::var("TRANCE_PRIMARY_END_ROW").ok()?.parse().ok()?;
    if end_col <= start_col || end_row <= start_row {
        return None;
    }
    const MAX_GRID: usize = 16_384;
    if end_col > MAX_GRID || end_row > MAX_GRID {
        return None;
    }
    Some(MonitorCellBounds {
        start_col,
        end_col,
        start_row,
        end_row,
        is_primary: true,
    })
}

pub fn publish_primary_bounds(bounds: MonitorCellBounds) {
    unsafe {
        std::env::set_var("TRANCE_PRIMARY_START_COL", bounds.start_col.to_string());
        std::env::set_var("TRANCE_PRIMARY_END_COL", bounds.end_col.to_string());
        std::env::set_var("TRANCE_PRIMARY_START_ROW", bounds.start_row.to_string());
        std::env::set_var("TRANCE_PRIMARY_END_ROW", bounds.end_row.to_string());
    }
    *env_bounds_cache().lock().unwrap() = Some(bounds);
}

pub fn clear_primary_bounds() {
    unsafe {
        std::env::remove_var("TRANCE_PRIMARY_START_COL");
        std::env::remove_var("TRANCE_PRIMARY_END_COL");
        std::env::remove_var("TRANCE_PRIMARY_START_ROW");
        std::env::remove_var("TRANCE_PRIMARY_END_ROW");
    }
    *env_bounds_cache().lock().unwrap() = None;
}

pub fn is_secondary_monitor() -> bool {
    if let Some(callback) = IS_SECONDARY_MONITOR_CALLBACK.get() {
        callback()
    } else {
        std::env::var("TRANCE_SECONDARY_MONITOR").is_ok()
    }
}
