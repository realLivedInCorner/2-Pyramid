// scale_factor.rs
//
// Shared `determine_scale_factor` helper. Mirrors pack.py
// `determine_scale_factor` (lines 2119-2133):
//
//   * Picks the scale factor from [1, 2, 4, 8] whose multiple of 256 is
//     closest to the (larger) image dimension.
//   * Returns the chosen scale factor plus an `is_exact` flag, true when
//     the image is a standard 256x256 multiple.
//
// py equivalent:
//   closest_scale_factor = min(standard_scale_factors,
//                              key=lambda x: abs(x * 256 - image_size))
//   is_exact = (closest_scale_factor * 256 == image_size)
//
// Several Surgeon-tier converters used to ship a private `determine_scale_factor`
// that returned `Result<(f64, bool), String>` and fell back to `width / 256.0`
// for non-standard sizes. That behavior is wrong: e.g. a 300x300 image would
// be assigned scale_factor = 1.17, when py would pick 1.0. This module is the
// single source of truth.
pub fn determine_scale_factor(width: u32, height: u32) -> (u32, bool) {
    const STANDARD_SCALE_FACTORS: [u32; 4] = [1, 2, 4, 8];

    let image_size = width.max(height);

    let best = STANDARD_SCALE_FACTORS
        .iter()
        .copied()
        .min_by_key(|&s| (s * 256).abs_diff(image_size))
        .unwrap_or(1);

    let is_exact = best * 256 == image_size;
    (best, is_exact)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_sizes() {
        assert_eq!(determine_scale_factor(256, 256), (1, true));
        assert_eq!(determine_scale_factor(512, 512), (2, true));
        assert_eq!(determine_scale_factor(1024, 1024), (4, true));
        assert_eq!(determine_scale_factor(2048, 2048), (8, true));
    }

    #[test]
    fn non_standard_picks_nearest() {
        // 300 px: closest is 256 (delta 44) vs 512 (delta 212) -> 1
        assert_eq!(determine_scale_factor(300, 300), (1, false));
        // 700 px: closer to 512 (delta 44) than 1024 (delta 324) -> 2
        assert_eq!(determine_scale_factor(700, 700), (2, false));
    }

    #[test]
    fn uses_larger_dimension() {
        // 256x512: image_size=512 -> exact=2
        assert_eq!(determine_scale_factor(256, 512), (2, true));
    }
}
