//! Window geometry helpers.
//!
//! The widget is frameless and transparent; its window is slightly larger than
//! the visible card so the CSS shadow has room. All maths is done in physical
//! pixels so mixed-DPI multi-monitor setups behave.

use tauri::{
    LogicalSize, Monitor, PhysicalPosition, PhysicalSize, WebviewWindow,
};

#[allow(dead_code)]
pub const COLLAPSED_WINDOW: (f64, f64) = (540.0, 360.0);
pub const EXPANDED_WINDOW: (f64, f64) = (540.0, 360.0);

const DEFAULT_MARGIN: i32 = 28;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    fn from_monitor(monitor: &Monitor) -> Self {
        let area = monitor.work_area();
        Self {
            x: area.position.x,
            y: area.position.y,
            width: area.size.width as i32,
            height: area.size.height as i32,
        }
    }

    fn right(&self) -> i32 {
        self.x + self.width
    }

    fn bottom(&self) -> i32 {
        self.y + self.height
    }

    pub(crate) fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
}

fn work_areas(window: &WebviewWindow) -> Vec<(Monitor, Rect)> {
    window
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|monitor| {
            let rect = Rect::from_monitor(&monitor);
            (monitor, rect)
        })
        .collect()
}

fn primary_area(window: &WebviewWindow) -> Rect {
    window
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| Rect::from_monitor(&monitor))
        .or_else(|| work_areas(window).first().map(|(_, rect)| *rect))
        .unwrap_or(Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        })
}

/// Picks the work area that contains `(x, y)`, falling back to the nearest one.
fn area_for(window: &WebviewWindow, x: i32, y: i32) -> Rect {
    let areas = work_areas(window);
    if let Some((_, rect)) = areas.iter().find(|(_, rect)| rect.contains(x, y)) {
        return *rect;
    }

    // Nearest work area by distance to its centre.
    areas
        .iter()
        .min_by_key(|(_, rect)| {
            let cx = rect.x + rect.width / 2;
            let cy = rect.y + rect.height / 2;
            (cx - x).abs() + (cy - y).abs()
        })
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| primary_area(window))
}

fn clamp(position: (i32, i32), size: PhysicalSize<u32>, area: Rect) -> (i32, i32) {
    let max_x = area.right() - size.width as i32;
    let max_y = area.bottom() - size.height as i32;
    // If the window is larger than the work area (extreme scaling), pin its
    // top-left corner to the work-area origin instead of pushing it off screen.
    let x = if max_x < area.x {
        area.x
    } else {
        position.0.clamp(area.x, max_x)
    };
    let y = if max_y < area.y {
        area.y
    } else {
        position.1.clamp(area.y, max_y)
    };
    (x, y)
}

fn desired_size(_expanded: bool) -> (f64, f64) {
    EXPANDED_WINDOW
}

/// Resizes the window for the given mode, anchoring the top-left corner and
/// keeping the whole window inside the current monitor's work area.
pub fn apply_size(window: &WebviewWindow, expanded: bool) -> Result<(), String> {
    let (width, height) = desired_size(expanded);
    let anchor = window.outer_position().unwrap_or(PhysicalPosition::new(0, 0));
    let area = area_for(window, anchor.x, anchor.y);
    eprintln!("[quoty] apply_size(expanded={expanded}) anchor={anchor:?} area={area:?}");

    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|error| error.to_string())?;

    let size = window.outer_size().map_err(|error| error.to_string())?;
    let (x, y) = clamp((anchor.x, anchor.y), size, area);
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| error.to_string())?;

    Ok(())
}

/// Restores the saved position (physical pixels) or places the widget in the
/// bottom-right corner of the primary work area. Always clamped on screen.
pub fn restore_position(
    window: &WebviewWindow,
    saved: Option<(i32, i32)>,
    expanded: bool,
) -> Result<(), String> {
    let (width, height) = desired_size(expanded);
    let area = saved
        .map(|(x, y)| area_for(window, x, y))
        .unwrap_or_else(|| primary_area(window));

    let estimated = PhysicalSize::new(
        (width * window.scale_factor().unwrap_or(1.0)).round() as u32,
        (height * window.scale_factor().unwrap_or(1.0)).round() as u32,
    );

    let position = saved.unwrap_or_else(|| {
        (
            area.right() - estimated.width as i32 - DEFAULT_MARGIN,
            area.bottom() - estimated.height as i32 - DEFAULT_MARGIN,
        )
    });

    let (x, y) = clamp(position, estimated, area);
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| error.to_string())?;
    apply_size(window, expanded)?;
    ensure_visible(window);
    Ok(())
}

/// Moves the widget back on screen if displays changed and it became unreachable.
pub fn ensure_visible(window: &WebviewWindow) {
    let Ok(position) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };

    let areas = work_areas(window);
    let visible = areas.iter().any(|(_, rect)| {
        let overlap_x = (position.x + size.width as i32).min(rect.right())
            - position.x.max(rect.x);
        let overlap_y = (position.y + size.height as i32).min(rect.bottom())
            - position.y.max(rect.y);
        overlap_x >= 48 && overlap_y >= 24
    });

    let area = area_for(window, position.x, position.y);
    let target = if visible {
        clamp((position.x, position.y), size, area)
    } else {
        (
            area.right() - size.width as i32 - DEFAULT_MARGIN,
            area.bottom() - size.height as i32 - DEFAULT_MARGIN,
        )
    };

    if target != (position.x, position.y) {
        let _ = window.set_position(PhysicalPosition::new(target.0, target.1));
    }
}

/// Shows the widget without taking keyboard focus from the active app.
pub fn show_without_focus(window: &WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};

        if let Ok(hwnd) = window.hwnd() {
            // SAFETY: the handle belongs to this window; SW_SHOWNOACTIVATE does
            // not steal focus.
            unsafe {
                let _ = ShowWindow(HWND(hwnd.0), SW_SHOWNOACTIVATE);
            }
            return;
        }
    }

    let _ = window.show();
}

pub fn hide(window: &WebviewWindow) {
    let _ = window.hide();
}

/// Applies the "Always on top" preference and keeps the desktop glue in sync
/// with the window's z-order band.
pub fn set_always_on_top(window: &WebviewWindow, enabled: bool) -> Result<(), String> {
    window
        .set_always_on_top(enabled)
        .map_err(|error| error.to_string())?;
    crate::desktop::set_always_on_top(enabled);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_inside_work_area() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let size = PhysicalSize::new(372, 600);

        assert_eq!(clamp((100, 100), size, area), (100, 100));
        assert_eq!(clamp((-50, -50), size, area), (0, 0));
        assert_eq!(clamp((1900, 1000), size, area), (1548, 480));
    }

    #[test]
    fn clamps_windows_on_secondary_monitors() {
        let area = Rect {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let size = PhysicalSize::new(224, 76);
        assert_eq!(clamp((-2000, -20), size, area), (-1920, 0));
        assert_eq!(clamp((-100, 1000), size, area), (-224, 964));
    }

    #[test]
    fn handles_windows_larger_than_the_work_area() {
        let area = Rect {
            x: 100,
            y: 100,
            width: 300,
            height: 300,
        };
        let size = PhysicalSize::new(600, 700);
        let (x, y) = clamp((0, 0), size, area);
        assert_eq!(x, 100);
        assert_eq!(y, 100);
    }

    #[test]
    fn contains_edges_exclusively() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        assert!(area.contains(0, 0));
        assert!(area.contains(99, 99));
        assert!(!area.contains(100, 50));
        assert!(!area.contains(-1, 50));
    }
}
