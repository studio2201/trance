//! Tests for the monitor-bounds API. Lives in its own file to keep the
//! `monitor/mod.rs` production code under the standing 250-line cap.

#[cfg(test)]
mod tests {
    use super::super::*;

    fn bounds(
        start_col: usize,
        end_col: usize,
        start_row: usize,
        end_row: usize,
    ) -> MonitorCellBounds {
        MonitorCellBounds {
            start_col,
            end_col,
            start_row,
            end_row,
            is_primary: true,
        }
    }

    #[test]
    fn bounds_contains_inside() {
        let b = bounds(0, 10, 0, 5);
        assert!(b.contains(5, 3));
    }

    #[test]
    fn bounds_excludes_outside() {
        let b = bounds(0, 10, 0, 5);
        assert!(!b.contains(11, 3));
        assert!(!b.contains(5, 6));
    }

    #[test]
    fn bounds_excludes_end_exclusive() {
        let b = bounds(0, 10, 0, 5);
        assert!(!b.contains(10, 0));
        assert!(!b.contains(0, 5));
    }

    #[test]
    fn bounds_width_height() {
        let b = bounds(0, 10, 0, 5);
        assert_eq!(b.width(), 10);
        assert_eq!(b.height(), 5);
    }

    #[test]
    fn bounds_width_height_saturate_when_inverted() {
        let b = bounds(8, 4, 6, 2);
        assert_eq!(b.width(), 0);
        assert_eq!(b.height(), 0);
    }

    #[test]
    fn bounds_centers() {
        let b = bounds(0, 10, 0, 6);
        assert_eq!(b.center_col(), 5);
        assert_eq!(b.center_row(), 3);
    }

    #[test]
    fn get_primary_monitor_bounds_default_is_full_grid() {
        // No callback set and no env vars in test by default
        unsafe {
            std::env::remove_var("TRANCE_PRIMARY_START_COL");
            std::env::remove_var("TRANCE_PRIMARY_END_COL");
            std::env::remove_var("TRANCE_PRIMARY_START_ROW");
            std::env::remove_var("TRANCE_PRIMARY_END_ROW");
        }
        clear_primary_bounds();
        let b = get_primary_monitor_bounds(10, 5);
        assert_eq!(b.start_col, 0);
        assert_eq!(b.end_col, 10);
        assert_eq!(b.start_row, 0);
        assert_eq!(b.end_row, 5);
        assert!(b.is_primary);
    }
}
