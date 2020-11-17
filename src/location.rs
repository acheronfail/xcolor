use failure::{err_msg, Error};
use x11::xcursor::{XcursorImageCreate, XcursorImageDestroy, XcursorImageLoadCursor};
use xcb::base as xbase;
use xcb::base::Connection;
use xcb::xproto;

use crate::color::{self, ARGB};
use crate::draw::{draw_magnifying_glass, PREVIEW_SIZE};
use crate::pixel::{PixelArray, PixelArrayMut};
use crate::util::EnsureOdd;

// Left mouse button
const SELECTION_BUTTON: xproto::Button = 1;
const GRAB_MASK: u16 =
    xproto::EVENT_MASK_BUTTON_PRESS as u16 | xproto::EVENT_MASK_POINTER_MOTION as u16;

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

fn create_new_cursor(
    conn: &Connection,
    screenshot_pixels: &PixelArray<ARGB>,
) -> Result<u32, Error> {
    Ok(unsafe {
        let mut cursor_image = XcursorImageCreate(PREVIEW_SIZE as i32, PREVIEW_SIZE as i32);

        // Set the "hot spot" - this is where the pointer actually is inside the image
        (*cursor_image).xhot = PREVIEW_SIZE / 2;
        (*cursor_image).yhot = PREVIEW_SIZE / 2;

        // Get pixel data as a Rust slice
        let mut cursor_pixels =
            PixelArrayMut::from_raw_parts((*cursor_image).pixels, PREVIEW_SIZE as usize);

        // Draw our custom image
        let pixel_size = (cursor_pixels.width / screenshot_pixels.width).ensure_odd();
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
    let pointer_pos = (pointer.root_x(), pointer.root_y());
    let (width, initial_rect) = get_window_rect_around_pointer(conn, root, pointer_pos, scale)?;

    let screenshot_pixels = PixelArray::new(&initial_rect[..], width.into());
    grab_cursor(conn, root, create_new_cursor(conn, &screenshot_pixels)?)?;

    let result = loop {
        let event = conn.wait_for_event();
        if let Some(event) = event {
            match event.response_type() {
                // TODO: handle escape key?
                xproto::BUTTON_PRESS => {
                    let event: &xproto::ButtonPressEvent = unsafe { xbase::cast_event(&event) };
                    if event.detail() == SELECTION_BUTTON {
                        break Some((event.root_x(), event.root_y()));
                    }
                }
                xproto::MOTION_NOTIFY => {
                    let event: &xproto::MotionNotifyEvent = unsafe { xbase::cast_event(&event) };
                    let pointer_pos = (event.root_x(), event.root_y());
                    let (width, rect) =
                        get_window_rect_around_pointer(conn, root, pointer_pos, scale)?;

                    let screenshot_pixels = PixelArray::new(&rect[..], width.into());
                    update_cursor(conn, create_new_cursor(conn, &screenshot_pixels)?)?;
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
