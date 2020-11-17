use crate::pixel::{PixelArray, PixelArrayMut};
use crate::color::ARGB;

// TODO: scale for HiDPI ?
pub const PREVIEW_SIZE: u32 = 256 - 1;

const GRID_COLOR: ARGB = ARGB::new(0xff, 0x55, 0x55, 0x55);

#[inline]
fn is_inside_circle(x: isize, y: isize, r: isize) -> bool {
    (x - r).pow(2) + (y - r).pow(2) < r.pow(2)
}

// TODO: dynamic zoom/size (modifier, etc)
// TODO: simplify vec indexing by using a wrapper struct
pub fn draw_magnifying_glass(
    cursor: &mut PixelArrayMut<u32>,
    screenshot: &PixelArray<ARGB>,
    pixel_size: usize,
) {
    // TODO: change depending on pixel/background, etc
    let border_color: u32 = std::u32::MAX;
    let grid_color: u32 = GRID_COLOR.into();
    let transparent: u32 = ARGB::TRANSPARENT.into();

    let border_width = 1;
    let border_radius = (PREVIEW_SIZE as isize) / 2;
    let content_radius = border_radius - border_width;

    assert!(pixel_size % 2 != 0, "pixel_size must be odd");
    assert!(cursor.width % 2 != 0, "cursor.length must be odd");
    assert!(screenshot.width % 2 != 0, "screenshot.length must be odd");

    let screenshot_center = screenshot.width / 2;
    let cursor_center = screenshot_center * pixel_size;
    let normalised_cursor_center_pixel = cursor.width / 2 - pixel_size / 2;
    let translation_offset = cursor_center.saturating_sub(normalised_cursor_center_pixel);

    for cursor_x in 0..cursor.width {
        for cursor_y in 0..cursor.width {
            let screenshot_x = (cursor_x + translation_offset) / pixel_size;
            let screenshot_y = (cursor_y + translation_offset) / pixel_size;

            let cx = cursor_x as isize;
            let cy = cursor_y as isize;
            cursor[(cursor_x, cursor_y)] = if is_inside_circle(cx, cy, content_radius) {
                let is_grid_line = (cursor_x + translation_offset) % pixel_size == 0
                    || (cursor_y + translation_offset) % pixel_size == 0;

                if is_grid_line {
                    let center_x_pixel_box =
                        cursor_x >= cursor_center && cursor_x <= cursor_center + pixel_size;
                    let center_y_pixel_box =
                        cursor_y >= cursor_center && cursor_y <= cursor_center + pixel_size;
                    if center_x_pixel_box && center_y_pixel_box {
                        border_color
                    } else {
                        grid_color
                    }
                } else {
                    screenshot[(screenshot_x, screenshot_y)].into()
                }
            } else if is_inside_circle(cx + border_width, cy + border_width, border_radius) {
                border_color
            } else {
                transparent
            };
        }
    }
}
