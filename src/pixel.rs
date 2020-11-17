use std::ops::{Index, IndexMut};
use std::slice;

type Point = (usize, usize);

/// TODO: doc
pub struct PixelArray<'a, T> {
    pub pixels: &'a [T],
    pub width: usize,
}

impl<'a, T> PixelArray<'a, T> {
    /// TODO: doc
    pub fn new(pixels: &'a [T], width: usize) -> Self {
        Self { pixels, width }
    }
}

impl<'a, T> Index<usize> for PixelArray<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.pixels[index]
    }
}

impl<'a, T> Index<Point> for PixelArray<'a, T> {
    type Output = T;
    fn index(&self, (x, y): Point) -> &Self::Output {
        &self.pixels[x * self.width + y]
    }
}

/// TODO: doc
pub struct PixelArrayMut<'a, T> {
    pub pixels: &'a mut [T],
    pub width: usize,
}

impl<'a, T> PixelArrayMut<'a, T> {
    /// TODO: doc
    pub fn new(pixels: &'a mut [T], width: usize) -> Self {
        Self { pixels, width }
    }

    /// TODO: doc
    pub unsafe  fn from_raw_parts(data: *mut T, width: usize) -> Self {
        Self::new(slice::from_raw_parts_mut(data, width * width), width)
    }
}

impl<'a, T> Index<usize> for PixelArrayMut<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.pixels[index]
    }
}

impl<'a, T> IndexMut<usize> for PixelArrayMut<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.pixels[index]
    }
}

impl<'a, T> Index<Point> for PixelArrayMut<'a, T> {
    type Output = T;
    fn index(&self, (x, y): Point) -> &Self::Output {
        &self.pixels[x * self.width + y]
    }
}

impl<'a, T> IndexMut<Point> for PixelArrayMut<'a, T> {
    fn index_mut(&mut self, (x, y): Point) -> &mut Self::Output {
        &mut self.pixels[(x * self.width + y)]
    }
}
