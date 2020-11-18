use std::ops::{Index, IndexMut};
use std::slice;

type Point = (usize, usize);

/// A wrapper struct for a a one-dimensional vector that represents a square of values
pub struct PixelArray<'a, T> {
    pixels: &'a [T],
    width: usize,
}

impl<'a, T> PixelArray<'a, T> {
    pub fn new(pixels: &'a [T], width: usize) -> Self {
        assert_eq!(pixels.len(), width * width, "data for PixelArray must be a square");
        Self { pixels, width }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn len(&self) -> usize {
        self.pixels.len()
    }

    pub fn index(&self, (x, y): Point) -> usize {
        x * self.width + y
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
    fn index(&self, point: Point) -> &Self::Output {
        &self.pixels[self.index(point)]
    }
}

/// A wrapper struct for a a one-dimensional vector that represents a square of mutable values
pub struct PixelArrayMut<'a, T> {
    pixels: &'a mut [T],
    width: usize,
}

impl<'a, T> PixelArrayMut<'a, T> {
    pub fn new(pixels: &'a mut [T], width: usize) -> Self {
        assert_eq!(pixels.len(), width * width, "data for PixelArrayMut must be a square");
        Self { pixels, width }
    }

    /// Instantiates a new `PixelArrayMut` from a pointer to a C array
    pub unsafe fn from_raw_parts(data: *mut T, width: usize) -> Self {
        Self::new(slice::from_raw_parts_mut(data, width * width), width)
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn len(&self) -> usize {
        self.pixels.len()
    }

    pub fn index(&self, (x, y): Point) -> usize {
        x * self.width + y
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
    fn index(&self, point: Point) -> &Self::Output {
        &self.pixels[self.index(point)]
    }
}

impl<'a, T> IndexMut<Point> for PixelArrayMut<'a, T> {
    fn index_mut(&mut self, point: Point) -> &mut Self::Output {
        &mut self.pixels[(self.index(point))]
    }
}
