//! Colouring algorithms.

use bytemuck::{Pod, Zeroable};
use fnv::{FnvHashMap, FnvHashSet};
use log::trace;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::{pixel::Pixel, screen};

/// [`bytemuck`]-compatible colour output for a single pixel.
#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
pub struct ColourRange {
    pub escaped: u32,
    pub value: f32,
}

impl Default for ColourRange {
    fn default() -> Self {
        Self {
            escaped: 0,
            value: 0.0,
        }
    }
}

/// Histogram-based colouring algorithm ([Wikipedia](https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set#Histogram_coloring)).
pub struct HistogramColouring {
    total_samples: usize,
    bucket_labels: Vec<u32>,
    histogram: FnvHashMap<u32, u32>,
    histogram_ranges: FnvHashMap<u32, f32>,
}

impl HistogramColouring {
    pub fn new() -> Self {
        let total_samples = 0;
        let bucket_labels: Vec<u32> = Vec::new();
        let histogram: FnvHashMap<u32, u32> = FnvHashMap::default();
        let histogram_ranges: FnvHashMap<u32, f32> = FnvHashMap::default();
        Self {
            total_samples,
            bucket_labels,
            histogram,
            histogram_ranges,
        }
    }

    pub fn reset(&mut self) {
        self.total_samples = 0;
        self.bucket_labels.clear();
        self.histogram.clear();
        self.histogram_ranges.clear();
    }

    /// Update the colour output (`colour_ranges`) given some newly escaped pixels (`newly_escaped_pixels`).
    pub fn update_colours(
        &mut self,
        screen_size: screen::Size,
        all_pixels: &[Pixel],
        newly_escaped_pixels: &[Pixel],
        colour_ranges: &mut [ColourRange],
    ) {
        trace!("begin compute_colour_ranges");

        if !newly_escaped_pixels.is_empty() {
            debug_assert!(colour_ranges.len() == (screen_size.width * screen_size.height) as usize);

            self.histogram_ranges.clear();

            for pixel in newly_escaped_pixels {
                debug_assert!(pixel.escaped == 1);

                colour_ranges[pixel.y as usize * screen_size.width as usize + pixel.x as usize]
                    .escaped = 1;

                let value = self
                    .histogram
                    .entry(pixel.iteration_count)
                    .or_insert_with(|| {
                        self.bucket_labels.push(pixel.iteration_count);
                        0
                    });
                *value += 1;
                self.total_samples += 1;
            }

            debug_assert_eq!(
                self.total_samples,
                self.histogram.values().map(|value| *value as usize).sum()
            );

            debug_assert!(
                self.bucket_labels.len()
                    == self
                        .bucket_labels
                        .iter()
                        .copied()
                        .collect::<FnvHashSet<u32>>()
                        .len(),
                "bucket_labels contains duplicates: {:?}",
                self.bucket_labels
            );
            self.bucket_labels.sort();

            let mut acc = 0;
            let total_samples = self.total_samples as f32;
            for bucket_label in &self.bucket_labels {
                self.histogram_ranges
                    .insert(*bucket_label, acc as f32 / total_samples);
                acc += self.histogram.get(bucket_label).unwrap();
            }

            colour_ranges
                .par_iter_mut()
                .enumerate()
                .for_each(|(index, colour_range)| {
                    let pixel = all_pixels[index];
                    if pixel.escaped == 1 {
                        colour_range.value = self
                            .histogram_ranges
                            .get(&pixel.iteration_count)
                            .copied()
                            .unwrap_or_else(|| {
                                panic!("{} was not in histogram_ranges", pixel.iteration_count)
                            })
                    }
                });
        }

        trace!("end compute_colour_ranges");
    }
}

impl Default for HistogramColouring {
    fn default() -> Self {
        Self::new()
    }
}
