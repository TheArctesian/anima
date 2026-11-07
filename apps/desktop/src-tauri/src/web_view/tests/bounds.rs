//! WEB-004: the client's CSS-pixel bounds become the web view's logical rectangle.

use tauri::{LogicalPosition, LogicalSize, Position, Size};

use crate::web_view::{WebViewError, bounds::Bounds};

fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
    Bounds {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn web_004_bounds_are_logical_pixels_in_the_main_window() {
    let rect = bounds(12.5, 40.0, 800.0, 600.25).rect().unwrap();
    let Position::Logical(LogicalPosition { x, y }) = rect.position else {
        panic!("logical position");
    };
    let Size::Logical(LogicalSize { width, height }) = rect.size else {
        panic!("logical size");
    };
    assert_eq!((x, y, width, height), (12.5, 40.0, 800.0, 600.25));
}

#[test]
fn web_004_a_zero_sized_view_is_allowed() {
    assert!(bounds(0.0, 0.0, 0.0, 0.0).rect().is_ok());
    assert!(
        bounds(100_000.0, 100_000.0, 100_000.0, 100_000.0)
            .rect()
            .is_ok()
    );
}

#[test]
fn web_004_negative_unbounded_or_not_finite_bounds_are_refused() {
    for bad in [
        bounds(-1.0, 0.0, 10.0, 10.0),
        bounds(0.0, -0.5, 10.0, 10.0),
        bounds(0.0, 0.0, -10.0, 10.0),
        bounds(0.0, 0.0, 10.0, 100_000.5),
        bounds(f64::NAN, 0.0, 10.0, 10.0),
        bounds(0.0, f64::INFINITY, 10.0, 10.0),
        bounds(0.0, 0.0, f64::NEG_INFINITY, 10.0),
    ] {
        assert_eq!(bad.rect().map(|_| ()), Err(WebViewError::Bounds), "{bad:?}");
    }
}

#[test]
fn web_004_bounds_read_exactly_x_y_width_and_height() {
    let read: Bounds =
        serde_json::from_value(serde_json::json!({ "x": 1, "y": 2, "width": 3.5, "height": 4 }))
            .unwrap();
    assert_eq!(read, bounds(1.0, 2.0, 3.5, 4.0));
    let extra = serde_json::json!({ "x": 1, "y": 2, "width": 3, "height": 4, "z": 0 });
    assert!(serde_json::from_value::<Bounds>(extra).is_err());
    let missing = serde_json::json!({ "x": 1, "y": 2, "width": 3 });
    assert!(serde_json::from_value::<Bounds>(missing).is_err());
}
