// SPDX-License-Identifier: GPL-2.0

//! Functions for incremental mean and variance.
//!
//! C header: [`include/linux/mean_and_variance.h`](../../../include/linux/mean_and_variance.h)

use crate::bindings;

/// Trait for objects which can record samples and provide mean and variance statistics.
pub trait MeanAndVarianceStats: Copy + Clone {
    /// Record a new sample.
    fn update(&mut self, v1: i64);
    /// Get the mean.
    fn mean(&self) -> i64;
    /// Get the variance.
    fn variance(&self) -> u64;
    /// Get the standard deviation.
    fn stddev(&self) -> u32;
}

#[repr(transparent)]
#[derive(Default, Copy, Clone)]
/// Mean and variance statistics.
pub struct MeanAndVariance(bindings::mean_and_variance);

impl MeanAndVariance {
    /// Create a new instance.
    pub const fn new() -> Self {
        Self(bindings::mean_and_variance {
            n: 0,
            sum: 0,
            sum_squares: 0,
        })
    }
}

impl MeanAndVarianceStats for MeanAndVariance {
    fn update(&mut self, v1: i64) {
        // SAFETY: FFI call.
        self.0 = unsafe { bindings::mean_and_variance_update(self.0, v1) };
    }

    fn mean(&self) -> i64 {
        // SAFETY: FFI call.
        unsafe { bindings::mean_and_variance_get_mean(self.0) }
    }

    fn variance(&self) -> u64 {
        // SAFETY: FFI call.
        unsafe { bindings::mean_and_variance_get_variance(self.0) }
    }

    fn stddev(&self) -> u32 {
        // SAFETY: FFI call.
        unsafe { bindings::mean_and_variance_get_stddev(self.0) }
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
/// Exponentially-weighted mean and variance statistics.
pub struct MeanAndVarianceWeighted(bindings::mean_and_variance_weighted);

impl MeanAndVarianceWeighted {
    /// Create a new instance and set its weight parameter.
    pub const fn new(w: u8) -> Self {
        Self(bindings::mean_and_variance_weighted {
            init: false,
            w,
            mean: 0,
            variance: 0,
        })
    }
}

impl MeanAndVarianceStats for MeanAndVarianceWeighted {
    fn update(&mut self, v1: i64) {
        // SAFETY: FFI call.
        self.0 = unsafe { bindings::mean_and_variance_weighted_update(self.0, v1) };
    }

    fn mean(&self) -> i64 {
        // SAFETY: FFI call.
        unsafe { bindings::mean_and_variance_weighted_get_mean(self.0) }
    }

    fn variance(&self) -> u64 {
        // SAFETY: FFI call.
        unsafe { bindings::mean_and_variance_weighted_get_variance(self.0) }
    }

    fn stddev(&self) -> u32 {
        // SAFETY: FFI call.
        unsafe { bindings::mean_and_variance_weighted_get_stddev(self.0) }
    }
}

/*
 * Test values computed using a spreadsheet from the psuedocode at the bottom:
 * https://fanf2.user.srcf.net/hermes/doc/antiforgery/stats.pdf
 *
 * mean_and_variance_basic_test:
    ```
    let mut s = MeanAndVariance::new();

    s.update(2);
    s.update(2);

    assert_eq!(s.mean(), 2);
    assert_eq!(s.variance(), 0);
    assert_eq!(s.0.n, 2);

    s.update(4);
    s.update(4);

    assert_eq!(s.mean(), 3);
    assert_eq!(s.variance(), 1);
    assert_eq!(s.0.n, 4);
    ```
 *
 * mean_and_variance_weighted_test:
    ```
    let mut s = MeanAndVarianceWeighted::new(2);

    s.update(10);
    assert_eq!(s.mean(), 10);
    assert_eq!(s.variance(), 0);

    s.update(20);

    assert_eq!(s.mean(), 12);
    assert_eq!(s.variance(), 18);

    s.update(30);
    assert_eq!(s.mean(), 16);
    assert_eq!(s.variance(), 72);

    let mut s = MeanAndVarianceWeighted::new(2);

    s.update(-10);
    assert_eq!(s.mean(), -10);
    assert_eq!(s.variance(), 0);

    s.update(-20);
    assert_eq!(s.mean(), -12);
    assert_eq!(s.variance(), 18);

    s.update(-30);
    assert_eq!(s.mean(), -16);
    assert_eq!(s.variance(), 72);
    ```
 *
 * mean_and_variance_weighted_advanced_test:
    ```
    let mut s = MeanAndVarianceWeighted::new(8);

    for i in (10..=100).step_by(10) {
        s.update(i);
    }

    assert_eq!(s.mean(), 11);
    assert_eq!(s.variance(), 107);

    let mut s = MeanAndVarianceWeighted::new(8);

    for i in (-100i16..=-10).step_by(10).rev() {
        s.update(i64::from(i));
    }

    assert_eq!(s.mean(), -11);
    assert_eq!(s.variance(), 107);
    ```
*/
