use std::slice;

use failure::{err_msg, Error};
use x11::xcursor::{XcursorImageCreate, XcursorImageDestroy, XcursorImageLoadCursor};
use xcb::base as xbase;
use xcb::base::Connection;
use xcb::xproto;

use crate::color::{self, ARGB};

// Left mouse button
const SELECTION_BUTTON: xproto::Button = 1;
const GRAB_MASK: u16 =
    xproto::EVENT_MASK_BUTTON_PRESS as u16 | xproto::EVENT_MASK_POINTER_MOTION as u16;
const PREVIEW_SIZE: u32 = 256 - 1;

fn grab_cursor(conn: &Connection, root: u32, cursor: u32) -> Result<(), Error> {
    let reply = xproto::grab_pointer(
        conn,
        false,
        root,
        GRAB_MASK,
        xproto::GRAB_MODE_ASYNC as u8,
        xproto::GRAB_MODE_ASYNC as u8,
        xbase::NONE,
        cursor,
        xbase::CURRENT_TIME,
    )
    .get_reply()?;

    if reply.status() != xproto::GRAB_STATUS_SUCCESS as u8 {
        return Err(err_msg("Could not grab pointer"));
    }

    Ok(())
}

fn update_cursor(conn: &Connection, cursor: u32) -> Result<(), Error> {
    xproto::change_active_pointer_grab_checked(conn, cursor, xbase::CURRENT_TIME, GRAB_MASK)
        .request_check()?;

    Ok(())
}

#[inline]
fn is_inside_circle(x: isize, y: isize, r: isize) -> bool {
    (x - r).pow(2) + (y - r).pow(2) < r.pow(2)
}

const GRID_COLOR: ARGB = ARGB::new(0xff, 0x55, 0x55, 0x55);

use std::ops::{Index, IndexMut};

struct PixelData<'a, T> {
    pub pixels: &'a mut [T],
    pub length: usize,
}

impl<'a, T> Index<usize> for PixelData<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.pixels[index]
    }
}

impl<'a, T> IndexMut<usize> for PixelData<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.pixels[index]
    }
}

// TODO: dynamic zoom/size (modifier, etc)
// TODO: simplify vec indexing by using a wrapper struct
fn draw_magnifying_glass(
    cursor: &mut PixelData<u32>,
    screenshot: &mut PixelData<ARGB>,
    pixel_size: usize,
) {
    // TODO: change depending on pixel/background, etc
    let border_color: u32 = std::u32::MAX;
    let grid_color: u32 = GRID_COLOR.into();

    let border_width = 1;
    let border_radius = (PREVIEW_SIZE as isize) / 2;
    let content_radius = border_radius - border_width;

    assert!(pixel_size % 2 != 0, "pixel_size must be odd");
    assert!(cursor.length % 2 != 0, "cursor be a square with odd length");
    assert!(
        screenshot.length % 2 != 0,
        "screenshot be a square with odd length"
    );

    let screenshot_center = screenshot.length / 2;
    let cursor_center = screenshot_center * pixel_size;
    let normalised_cursor_center_pixel = cursor.length / 2 - pixel_size / 2;
    let translation_offset = cursor_center.saturating_sub(normalised_cursor_center_pixel);

    for cursor_x in 0..cursor.length {
        for cursor_y in 0..cursor.length {
            let screenshot_x = (cursor_x + translation_offset) / pixel_size;
            let screenshot_y = (cursor_y + translation_offset) / pixel_size;
            let screenshot_idx = screenshot_x + screenshot_y * screenshot.length;
            let cursor_idx = cursor_x + cursor_y * cursor.length;

            let cx = cursor_x as isize;
            let cy = cursor_y as isize;
            cursor[cursor_idx] = if is_inside_circle(cx, cy, content_radius) {
                let is_grid_line = (cursor_x + translation_offset) % pixel_size == 0
                    || (cursor_y + translation_offset) % pixel_size == 0;

                if is_grid_line {
                    let center_x_pixel_box = cursor_x >= cursor_center && cursor_x <= cursor_center + pixel_size;
                    let center_y_pixel_box = cursor_y >= cursor_center && cursor_y <= cursor_center + pixel_size;
                    if center_x_pixel_box && center_y_pixel_box {
                        border_color
                    } else {
                        grid_color
                    }
                } else {
                    screenshot[screenshot_idx].into()
                }
            } else if is_inside_circle(cx + border_width, cy + border_width, border_radius) {
                border_color
            } else {
                0
            };
        }
    }

    // let scale = scale as usize;
    // let screenshot_len = screenshot_len as usize;

    // // Upscale window pixels into the cursor image
    // let cursor_len = PREVIEW_SIZE as usize;
    // for x in 0..screenshot_len {
    //     for y in 0..screenshot_len {
    //         let screenshot_pos = (x * screenshot_len) + y;
    //         let cursor_pos = ((x * cursor_len) * scale) + (y * scale);

    //         for i in 0..scale {
    //             for j in 0..scale {
    //                 let pos = cursor_pos + (i * cursor_len) + j;

    //                 // Check if we're inside our circle
    //                 let cx = (pos / cursor_len) as isize;
    //                 let cy = (pos % cursor_len) as isize;

    //                 cursor_pixels[pos] = if is_inside_circle(cx, cy, content_radius) {
    //                     if i == 0 || j == 0 {
    //                         grid_color
    //                     } else {
    //                         screenshot_pixels[screenshot_pos].into()
    //                     }
    //                 } else if is_inside_circle(cx + border_width, cy + border_width, border_radius) {
    //                     std::u32::MAX
    //                 } else {
    //                     0
    //                 };
    //             }
    //         }
    //     }
    // }
}

pub trait EnsureOdd {
    fn ensure_odd(self) -> Self;
}

impl EnsureOdd for u16 {
    fn ensure_odd(self) -> Self {
        if self % 2 == 0 {
            self + 1
        } else {
            self
        }
    }
}

impl EnsureOdd for usize {
    fn ensure_odd(self) -> Self {
        if self % 2 == 0 {
            self + 1
        } else {
            self
        }
    }
}

fn create_new_cursor(
    conn: &Connection,
    screenshot_pixels: &mut PixelData<ARGB>,
) -> Result<u32, Error> {
    Ok(unsafe {
        let mut cursor_image = XcursorImageCreate(PREVIEW_SIZE as i32, PREVIEW_SIZE as i32);

        // Set the "hot spot" - this is where the pointer actually is inside the image
        (*cursor_image).xhot = PREVIEW_SIZE / 2;
        (*cursor_image).yhot = PREVIEW_SIZE / 2;

        // Draw our custom image
        let pixels = slice::from_raw_parts_mut(
            (*cursor_image).pixels,
            (PREVIEW_SIZE * PREVIEW_SIZE) as usize,
        );

        let mut cursor_pixels = PixelData {
            pixels,
            length: PREVIEW_SIZE as usize,
        };
        let pixel_size = (cursor_pixels.length / screenshot_pixels.length).ensure_odd();
        draw_magnifying_glass(&mut cursor_pixels, screenshot_pixels, pixel_size);

        // Convert our XcursorImage into a cursor
        let cursor_id = XcursorImageLoadCursor(conn.get_raw_dpy(), cursor_image) as u32;

        // Free the XcursorImage
        XcursorImageDestroy(cursor_image);

        cursor_id
    } as u32)
}

fn get_window_rect_around_pointer(
    conn: &Connection,
    root: u32,
    (x, y): (i16, i16),
    scale: u32,
) -> Result<(u16, Vec<ARGB>), Error> {
    let size = ((PREVIEW_SIZE / scale) as u16).ensure_odd();
    let x = x - ((size as i16) / 2);
    let y = y - ((size as i16) / 2);

    Ok((size, color::window_rect(conn, root, (x, y, size, size))?))
}

pub fn wait_for_location(
    conn: &Connection,
    screen: &xproto::Screen,
) -> Result<Option<(i16, i16)>, Error> {
    let root = screen.root();

    // FIXME: allow multiples of 2, not just powers of 2
    let scale = 16;

    let pointer = xproto::query_pointer(conn, root).get_reply()?;
    let (size, mut initial_rect) =
        get_window_rect_around_pointer(conn, root, (pointer.root_x(), pointer.root_y()), scale)?;

    // TODO: remove mutability
    let mut screenshot_pixels = PixelData {
        pixels: &mut initial_rect[..],
        length: size as usize,
    };

    grab_cursor(conn, root, create_new_cursor(conn, &mut screenshot_pixels)?)?;

    let result = loop {
        let event = conn.wait_for_event();
        if let Some(event) = event {
            match event.response_type() {
                // TODO: handle escape?
                xproto::BUTTON_PRESS => {
                    let event: &xproto::ButtonPressEvent = unsafe { xbase::cast_event(&event) };
                    if event.detail() == SELECTION_BUTTON {
                        break Some((event.root_x(), event.root_y()));
                    }
                }
                xproto::MOTION_NOTIFY => {
                    let event: &xproto::MotionNotifyEvent = unsafe { xbase::cast_event(&event) };
                    let (size, mut rect) = get_window_rect_around_pointer(
                        conn,
                        root,
                        (event.root_x(), event.root_y()),
                        scale,
                    )?;

                    // TODO: remove mutability
                    let mut screenshot_pixels = PixelData {
                        pixels: &mut rect[..],
                        length: size as usize,
                    };

                    update_cursor(conn, create_new_cursor(conn, &mut screenshot_pixels)?)?;
                }
                _ => {
                    // ???
                }
            }
        } else {
            break None;
        }
    };
    xproto::ungrab_pointer(conn, xbase::CURRENT_TIME);
    conn.flush();
    Ok(result)
}
