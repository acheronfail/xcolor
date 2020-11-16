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
const PREVIEW_SIZE: u32 = 256;
const PREVIEW_WIDTH: u32 = 256;
const PREVIEW_HEIGHT: u32 = 256;

// FIXME: surely there's a better way to grab pointer events while changing cursor?
fn change_cursor(conn: &Connection, root: u32, cursor: u32) -> Result<(), Error> {
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

#[inline]
fn is_inside_circle(x: isize, y: isize, r: isize) -> bool {
    (x - r).pow(2) + (y - r).pow(2) < r.pow(2)
}

// TODO: grid
// TODO: dynamic zoom/size (modifier, etc)
fn draw_magnifying_glass(
    cursor_pixels: &mut [u32],
    window_len: u16,
    window_pixels: Vec<ARGB>,
    scale: u32,
) {
    let magnification = scale as usize;
    let window_len = window_len as usize;

    let border_width = 1;
    let border_radius = (PREVIEW_SIZE as isize) / 2;
    let content_radius = border_radius - border_width;

    // Upscale window pixels into the cursor image
    let cursor_len = window_len * magnification;
    for x in 0..window_len {
        for y in 0..window_len {
            let win_pos = (x * window_len) + y;

            let cur_pos = ((x * magnification.pow(2)) * window_len) + (y * magnification);

            for i in 0..magnification {
                for j in 0..magnification {
                    let pos = cur_pos + (i * cursor_len) + j;

                    // Check if we're inside our circle
                    let x = (pos / cursor_len) as isize;
                    let y = (pos % cursor_len) as isize;

                    cursor_pixels[pos] =
                        if is_inside_circle(x, y, content_radius) {
                            window_pixels[win_pos].into()
                        } else if is_inside_circle(x + border_width, y + border_width, border_radius) {
                            std::u32::MAX
                        } else {
                            0
                        };
                }
            }
        }
    }
}

fn create_new_cursor(
    conn: &Connection,
    window_size: u16,
    window_pixels: Vec<ARGB>,
    scale: u32,
) -> Result<u32, Error> {
    Ok(unsafe {
        let mut cursor_image = XcursorImageCreate(PREVIEW_WIDTH as i32, PREVIEW_HEIGHT as i32);

        // Set the "hot spot" - this is where the pointer actually is inside the image
        (*cursor_image).xhot = PREVIEW_WIDTH / 2;
        (*cursor_image).yhot = PREVIEW_HEIGHT / 2;

        // Draw our custom image
        let mut pixels = slice::from_raw_parts_mut(
            (*cursor_image).pixels,
            (PREVIEW_WIDTH * PREVIEW_HEIGHT) as usize,
        );
        draw_magnifying_glass(&mut pixels, window_size, window_pixels, scale);

        // Convert our XcursorImage into a cursor
        let cursor_id = XcursorImageLoadCursor(conn.get_raw_dpy(), cursor_image) as u32;

        // Free the XcursorImage
        XcursorImageDestroy(cursor_image);

        cursor_id
    } as u32)
}

// TODO: ensure odd numbers for center pixel
fn get_window_rect_around_pointer(
    conn: &Connection,
    root: u32,
    (x, y): (i16, i16),
    scale: u32,
) -> Result<(u16, Vec<ARGB>), Error> {
    let zoom_reciprocal = 1.0 / scale as f32;
    let curr_size = PREVIEW_SIZE as f32 + 1.0;
    let size = (curr_size * zoom_reciprocal).floor() as u16;

    let x = x - ((size as i16) / 2);
    let y = y - ((size as i16) / 2);
    Ok((size, color::window_rect(conn, root, (x, y, size, size))?))
}

pub fn wait_for_location(
    conn: &Connection,
    screen: &xproto::Screen,
) -> Result<Option<(i16, i16)>, Error> {
    let root = screen.root();

    // NOTE: must be a multiple of 2
    let scale = 8;

    let pointer = xproto::query_pointer(conn, root).get_reply()?;
    let (size, initial_rect) =
        get_window_rect_around_pointer(conn, root, (pointer.root_x(), pointer.root_y()), scale)?;
    change_cursor(
        conn,
        root,
        create_new_cursor(conn, size, initial_rect, scale)?,
    )?;

    let result = loop {
        let event = conn.wait_for_event();
        if let Some(event) = event {
            match event.response_type() {
                xproto::BUTTON_PRESS => {
                    let event: &xproto::ButtonPressEvent = unsafe { xbase::cast_event(&event) };
                    if event.detail() == SELECTION_BUTTON {
                        break Some((event.root_x(), event.root_y()));
                    }
                }
                xproto::MOTION_NOTIFY => {
                    let event: &xproto::MotionNotifyEvent = unsafe { xbase::cast_event(&event) };
                    let (size, rect) = get_window_rect_around_pointer(
                        conn,
                        root,
                        (event.root_x(), event.root_y()),
                        scale,
                    )?;

                    change_cursor(conn, root, create_new_cursor(conn, size, rect, scale)?)?;
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
