use failure::{err_msg, Error};
use x11::xcursor::{XcursorImageCreate, XcursorImageDestroy, XcursorImageLoadCursor};
use xcb::base as xbase;
use xcb::base::Connection;
use xcb::xproto;

use crate::color::{self, ARGB};
use crate::draw::draw_magnifying_glass;
use crate::pixel::{PixelArray, PixelArrayMut};
use crate::util::{EnsureOdd, Clamped};

// Left mouse button
const SELECTION_BUTTON: xproto::Button = 1;
const GRAB_MASK: u16 = (xproto::EVENT_MASK_BUTTON_PRESS | xproto::EVENT_MASK_POINTER_MOTION) as u16;

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
    preview_width: u32,
) -> Result<u32, Error> {
    Ok(unsafe {
        let mut cursor_image = XcursorImageCreate(preview_width as i32, preview_width as i32);

        // set the "hot spot" - this is where the pointer actually is inside the image
        (*cursor_image).xhot = preview_width / 2;
        (*cursor_image).yhot = preview_width / 2;

        // get pixel data as a mutable Rust slice
        let mut cursor_pixels =
            PixelArrayMut::from_raw_parts((*cursor_image).pixels, preview_width as usize);

        // draw our custom image
        let pixel_size = (cursor_pixels.width() / screenshot_pixels.width()).ensure_odd();
        draw_magnifying_glass(&mut cursor_pixels, screenshot_pixels, pixel_size);

        // convert our XcursorImage into a cursor
        let cursor_id = XcursorImageLoadCursor(conn.get_raw_dpy(), cursor_image) as u32;

        // free the XcursorImage
        XcursorImageDestroy(cursor_image);

        cursor_id
    } as u32)
}

pub struct Rect {
    x: i16,
    y: i16,
    width: u16,
    height: u16
}

impl Rect {
    pub fn new(x: i16, y: i16, width: u16, height: u16) -> Rect {
        Rect { x, y, width, height }
    }

    pub fn is_inside(&self, x: i16, y: i16) -> bool {
        let x_is_inside = x >= self.x && x <= self.x + self.width;
        let y_is_inside = y >= self.y && y <= self.y + self.height;
        x_is_inside && y_is_inside
    }

    // TODO: should this be greater than u16?
    pub fn area(&self) -> u16 {
        self.width * self.height
    }
}

impl From<Rect> for (i16, i16, u16, u16) {
    fn from(rect: Rect) -> (i16, i16, u16, u16) {
        (rect.x, rect.y, rect.width, rect.height)
    }
}

// TODO: test multi-monitor
fn get_window_rect_around_pointer(
    conn: &Connection,
    screen: &xproto::Screen,
    (x, y): (i16, i16),
    preview_width: u32,
    scale: u32,
) -> Result<(u16, Vec<ARGB>), Error> {
    let root = screen.root();
    let root_width = screen.width_in_pixels() as i16;
    let root_height = screen.height_in_pixels() as i16;

    // FIXME: fails if we ask for a region outside the screen, so fill those pixels with empty data
    let size = ((preview_width / scale) as u16).ensure_odd();
    let x = x - ((size as i16) / 2);
    let y = y - ((size as i16) / 2);
    let desired_rect = Rect::new(x, y, size, size);
    let actual_rect = Rect::new(
        x.clamped(0, x),
        y.clamped(0, y),
        size.clamped(1, root_width - x),
        size.clamped(1, root_height - y)
    );

    let screenshot = color::window_rect(conn, root, actual_rect)?;
    if screenshot.len() < desired_rect.area() {
        let mut pixels = Vec::with_capacity(desired_area);
        for x in 0..actual.width {
            for y in 0..actual.height {
                if desired_rect.is_inside(x, y) {
                    // FIXME: get coordinates from screenshot vec?
                    // there's probably an easier way to do this...
                    pixels[x * size + y] = screenshot
            }
        }
    } else {
        Ok((size, screenshot))
    }
}

pub fn wait_for_location(
    conn: &Connection,
    screen: &xproto::Screen,
    preview_width: u32,
    scale: u32,
) -> Result<Option<ARGB>, Error> {
    let root = screen.root();
    let preview_width = preview_width.ensure_odd();

    let pointer = xproto::query_pointer(conn, root).get_reply()?;
    let pointer_pos = (pointer.root_x(), pointer.root_y());
    let (width, initial_rect) =
        get_window_rect_around_pointer(conn, screen, pointer_pos, preview_width, scale)?;

    let screenshot_pixels = PixelArray::new(&initial_rect[..], width.into());
    grab_cursor(
        conn,
        root,
        create_new_cursor(conn, &screenshot_pixels, preview_width)?,
    )?;

    let result = loop {
        let event = conn.wait_for_event();
        if let Some(event) = event {
            match event.response_type() {
                xproto::BUTTON_PRESS => {
                    let event: &xproto::ButtonPressEvent = unsafe { xbase::cast_event(&event) };
                    if event.detail() == SELECTION_BUTTON {
                        let pixels =
                            color::window_rect(conn, root, (event.root_x(), event.root_y(), 1, 1))?;
                        break Some(pixels[0]);
                    }
                }
                xproto::MOTION_NOTIFY => {
                    let event: &xproto::MotionNotifyEvent = unsafe { xbase::cast_event(&event) };
                    let pointer_pos = (event.root_x(), event.root_y());
                    let (width, pixels) = get_window_rect_around_pointer(
                        conn,
                        screen,
                        pointer_pos,
                        preview_width,
                        scale,
                    )?;

                    let screenshot_pixels = PixelArray::new(&pixels[..], width.into());
                    update_cursor(
                        conn,
                        create_new_cursor(conn, &screenshot_pixels, preview_width)?,
                    )?;
                }
                _ => {}
            }
        } else {
            break None;
        }
    };

    xproto::ungrab_pointer(conn, xbase::CURRENT_TIME);
    conn.flush();

    Ok(result)
}
