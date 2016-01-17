//! Functions for manipulating the contrast of images.

use image::{GrayImage, Luma};
use definitions::VecBuffer;

/// Returns the Otsu threshold level of an 8bpp image.
/// This threshold will optimally binarize an image that
/// contains two classes of pixels which have distributions
/// with equal variances. For details see:
/// Xu, X., et al. Pattern recognition letters 32.7 (2011)
pub fn otsu_level(image: &VecBuffer<Luma<u8>>) -> u8 {
    let hist = histogram(image);
    let levels = hist.len();
    let mut histc = [0i32; 256];
    let mut meanc = [0.0f64; 256];
    let mut pixel_count = hist[0] as f64;

    histc[0] = hist[0];

    for i in 1..levels {
        pixel_count += hist[i] as f64;
        histc[i] = histc[i - 1] + hist[i];
        meanc[i] = meanc[i - 1] + (hist[i] as f64) * (i as f64);
    }

    let mut sigma_max = -1f64;
    let mut otsu = 0f64;
    let mut otsu2 = 0f64;

    for i in 0..levels {
        meanc[i] /= pixel_count;

        let p0 = (histc[i] as f64) / pixel_count;
        let p1 = 1f64 - p0;
        let mu0 = meanc[i] / p0;
        let mu1 = (meanc[levels - 1] / pixel_count - meanc[i]) / p1;

        let sigma = p0 * p1 * (mu0 - mu1).powi(2);
        if sigma >= sigma_max {
            if sigma > sigma_max {
                otsu = i as f64;
                sigma_max = sigma;
            } else {
                otsu2 = i as f64;
            }
        }
    }
    ((otsu + otsu2) / 2.0).ceil() as u8
}

/// Returns a binarized image from an input 8bpp grayscale image
/// obtained by applying the given threshold.
pub fn threshold(image: &VecBuffer<Luma<u8>>, thresh: u8) -> GrayImage {
    let mut out = image.clone();
    threshold_mut(&mut out, thresh);
    out
}

/// Mutates given image to form a binarized version produced by applying
/// the given threshold.
pub fn threshold_mut(image: &mut VecBuffer<Luma<u8>>, thresh: u8) {
    for c in image.iter_mut() {
        *c = if *c as u8 <= thresh { 0 } else { 255 };
    }
}

/// Returns the histogram of grayscale values in an 8bpp
/// grayscale image.
pub fn histogram(image: &VecBuffer<Luma<u8>>) -> [i32; 256] {
    let mut hist = [0i32; 256];

    for pix in image.iter() {
        hist[*pix as usize] += 1;
    }

    hist
}

/// Returns the cumulative histogram of grayscale values in an 8bpp
/// grayscale image.
pub fn cumulative_histogram(image: &VecBuffer<Luma<u8>>) -> [i32; 256] {
    let mut hist = histogram(image);

    for i in 1..hist.len() {
        hist[i] += hist[i - 1];
    }

    hist
}

/// Equalises the histogram of an 8bpp grayscale image in place.
/// https://en.wikipedia.org/wiki/Histogram_equalization
pub fn equalize_histogram_mut(image: &mut VecBuffer<Luma<u8>>) {
    let hist = cumulative_histogram(image);
    let total = hist[255] as f32;

    for c in image.iter_mut() {
        let fraction = hist[*c as usize] as f32 / total;
        *c = f32::min(255f32, 255f32 * fraction) as u8;
    }
}

/// Equalises the histogram of an 8bpp grayscale image.
/// https://en.wikipedia.org/wiki/Histogram_equalization
pub fn equalize_histogram(image: &VecBuffer<Luma<u8>>) -> GrayImage {
    let mut out = image.clone();
    equalize_histogram_mut(&mut out);
    out
}

/// Adjusts contrast of an 8bpp grayscale image in place so that its
/// histogram is as close as possible to that of the target image.
pub fn match_histogram_mut(image: &mut VecBuffer<Luma<u8>>, 
                           target: &VecBuffer<Luma<u8>>) {
    let image_histc = cumulative_histogram(image);
    let target_histc = cumulative_histogram(target);
    let lut = histogram_lut(&image_histc, &target_histc);

    for c in image.iter_mut() {
        *c = lut[*c as usize] as u8;
    }
}

/// Adjusts contrast of an 8bpp grayscale image so that its
/// histogram is as close as possible to that of the target image.
pub fn match_histogram(image: &VecBuffer<Luma<u8>>, 
                       target: &VecBuffer<Luma<u8>>) -> GrayImage {
    let mut out = image.clone();
    match_histogram_mut(&mut out, target);
    out
}

/// l = histogram_lut(s, t) is chosen so that target_histc[l[i]] / sum(target_histc)
/// is as close as possible to source_histc[i] / sum(source_histc).
fn histogram_lut(source_histc: &[i32; 256], target_histc: &[i32; 256]) -> [usize; 256] {
    let source_total = source_histc[255] as f32;
    let target_total = target_histc[255] as f32;

    let mut lut = [0usize; 256];
    let mut y = 0usize;
    let mut prev_target_fraction = 0f32;

    for s in 0..256 {
        let source_fraction = source_histc[s] as f32 / source_total;
        let mut target_fraction = target_histc[y] as f32 / target_total;

        while source_fraction > target_fraction && y < 255 {
            y = y + 1;
            prev_target_fraction = target_fraction;
            target_fraction = target_histc[y] as f32 / target_total;
        }

        if y == 0 {
            lut[s] = y;
        }
        else {
            let prev_dist = f32::abs(prev_target_fraction - source_fraction);
            let dist = f32::abs(target_fraction - source_fraction);
            if prev_dist < dist {
                lut[s] = y - 1;
            }
            else {
                lut[s] = y;
            }
        }
    }

    lut
}

#[cfg(test)]
mod test {

    use super::{
        cumulative_histogram,
        equalize_histogram,
        equalize_histogram_mut,
        histogram,
        histogram_lut,
        otsu_level,
        threshold,
        threshold_mut,
    };
    use utils::gray_bench_image;
    use image::{GrayImage, ImageBuffer};
    use test;

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn test_cumulative_histogram() {
        let image: GrayImage = ImageBuffer::from_raw(5, 1, vec![
            1u8, 2u8, 3u8, 2u8, 1u8]).unwrap();

        let hist = cumulative_histogram(&image);

        assert_eq!(hist[0], 0);
        assert_eq!(hist[1], 2);
        assert_eq!(hist[2], 4);
        assert_eq!(hist[3], 5);
        assert!(hist.iter().skip(4).all(|x| *x == 5));
    }

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn test_histogram() {
        let image: GrayImage = ImageBuffer::from_raw(5, 1, vec![
            1u8, 2u8, 3u8, 2u8, 1u8]).unwrap();

        let hist = histogram(&image);

        assert_eq!(hist[0], 0);
        assert_eq!(hist[1], 2);
        assert_eq!(hist[2], 2);
        assert_eq!(hist[3], 1);
    }

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn test_histogram_lut_source_and_target_equal() {
        let mut histc = [0i32; 256];
        for i in 1..histc.len() {
            histc[i] = 2 * i as i32;
        }

        let lut = histogram_lut(&histc, &histc);
        let expected = (0..256).collect::<Vec<_>>();

        assert_eq!(&lut[0..256], &expected[0..256]);
    }

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn test_histogram_lut_gradient_to_step_contrast() {
        let mut grad_histc = [0i32; 256];
        for i in 0..grad_histc.len() {
            grad_histc[i] = i as i32;
        }

        let mut step_histc = [0i32; 256];
        for i in 30..130 {
            step_histc[i] = 100;
        }
        for i in 130..256 {
            step_histc[i] = 200;
        }

        let lut = histogram_lut(&grad_histc, &step_histc);
        let mut expected = [0usize; 256];

        // No black pixels in either image
        expected[0] = 0;

        for i in 1..64 {
            expected[i] = 29;
        }
        for i in 64..128 {
            expected[i] = 30;
        }
        for i in 128..192 {
            expected[i] = 129;
        }
        for i in 192..256 {
            expected[i] = 130;
        }

        assert_eq!(&lut[0..256], &expected[0..256]);
    }

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn test_otsu_level() {
        let image: GrayImage = ImageBuffer::from_raw(26, 1,
            vec![0u8, 10u8, 20u8, 30u8, 40u8, 50u8, 60u8, 70u8,
                80u8, 90u8, 100u8, 110u8, 120u8, 130u8, 140u8,
                150u8, 160u8, 170u8, 180u8, 190u8, 200u8,  210u8,
                220u8,  230u8,  240u8,  250u8]).unwrap();
        let level = otsu_level(&image);
        assert_eq!(level, 125);
    }

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn test_threshold() {
        let original: GrayImage = ImageBuffer::from_raw(26, 1,
            vec![0u8, 10u8, 20u8, 30u8, 40u8, 50u8, 60u8, 70u8,
                80u8, 90u8, 100u8, 110u8, 120u8, 130u8, 140u8,
                150u8, 160u8, 170u8, 180u8, 190u8, 200u8,  210u8,
                220u8,  230u8,  240u8,  250u8]).unwrap();
        let expected: GrayImage = ImageBuffer::from_raw(26, 1,
            vec![0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                0u8, 0u8, 0u8, 0u8, 0u8, 255u8, 255u8,
                255u8, 255u8, 255u8, 255u8, 255u8, 255u8,  255u8,
                255u8,  255u8,  255u8,  255u8]).unwrap();

        let actual = threshold(&original, 125u8);
        assert_pixels_eq!(expected, actual);
    }

    #[bench]
    fn bench_equalize_histogram(b: &mut test::Bencher) {
        let image = gray_bench_image(500, 500);
        b.iter(|| {
            let equalized = equalize_histogram(&image);
            test::black_box(equalized);
        });
    }

    #[bench]
    fn bench_equalize_histogram_mut(b: &mut test::Bencher) {
        let mut image = gray_bench_image(500, 500);
        b.iter(|| {
            test::black_box(equalize_histogram_mut(&mut image));
        });
    }

    #[bench]
    fn bench_threshold(b: &mut test::Bencher) {
        let image = gray_bench_image(500, 500);
        b.iter(|| {
            let thresholded = threshold(&image, 125);
            test::black_box(thresholded);
        });
    }
    
    #[bench]
    fn bench_threshold_mut(b: &mut test::Bencher) {
        let mut image = gray_bench_image(500, 500);
        b.iter(|| {
            test::black_box(threshold_mut(&mut image, 125));
        });
    }
}
