use crate::capture::traits::{CaptureError, CaptureRegion};

/// Crops a subregion from a 32-bit BGRA image buffer and returns a new owned Vec<u8>.
///
/// Validates source buffer size against `src_width * src_height * 4` and validates
/// that `crop` is non-empty and completely contained within `[0..src_width, 0..src_height]`.
pub fn crop_bgra(
    source: &[u8],
    src_width: u32,
    src_height: u32,
    crop: &CaptureRegion,
) -> Result<Vec<u8>, CaptureError> {
    if crop.width == 0 || crop.height == 0 {
        return Err(CaptureError::ZeroSizedRegion {
            width: crop.width,
            height: crop.height,
        });
    }

    let expected_src_len = (src_width as usize)
        .checked_mul(src_height as usize)
        .and_then(|px| px.checked_mul(4))
        .ok_or(CaptureError::OutOfBounds {
            x: crop.x,
            y: crop.y,
            width: crop.width,
            height: crop.height,
            src_width,
            src_height,
        })?;

    if source.len() != expected_src_len {
        return Err(CaptureError::InvalidBufferSize {
            expected: expected_src_len,
            actual: source.len(),
        });
    }

    let max_x = crop.x.checked_add(crop.width).ok_or(CaptureError::OutOfBounds {
        x: crop.x,
        y: crop.y,
        width: crop.width,
        height: crop.height,
        src_width,
        src_height,
    })?;

    let max_y = crop.y.checked_add(crop.height).ok_or(CaptureError::OutOfBounds {
        x: crop.x,
        y: crop.y,
        width: crop.width,
        height: crop.height,
        src_width,
        src_height,
    })?;

    if max_x > src_width || max_y > src_height {
        return Err(CaptureError::OutOfBounds {
            x: crop.x,
            y: crop.y,
            width: crop.width,
            height: crop.height,
            src_width,
            src_height,
        });
    }

    let dest_len = (crop.width as usize)
        .checked_mul(crop.height as usize)
        .and_then(|px| px.checked_mul(4))
        .ok_or(CaptureError::OutOfBounds {
            x: crop.x,
            y: crop.y,
            width: crop.width,
            height: crop.height,
            src_width,
            src_height,
        })?;

    let mut dest = vec![0u8; dest_len];
    crop_bgra_into(source, src_width, src_height, crop, &mut dest)?;
    Ok(dest)
}

/// Crops a subregion from a 32-bit BGRA image buffer directly into a pre-allocated destination buffer.
///
/// Enables zero-allocation buffer reuse across consecutive capture frames.
pub fn crop_bgra_into(
    source: &[u8],
    src_width: u32,
    src_height: u32,
    crop: &CaptureRegion,
    dest: &mut [u8],
) -> Result<(), CaptureError> {
    if crop.width == 0 || crop.height == 0 {
        return Err(CaptureError::ZeroSizedRegion {
            width: crop.width,
            height: crop.height,
        });
    }

    let expected_src_len = (src_width as usize)
        .checked_mul(src_height as usize)
        .and_then(|px| px.checked_mul(4))
        .ok_or(CaptureError::OutOfBounds {
            x: crop.x,
            y: crop.y,
            width: crop.width,
            height: crop.height,
            src_width,
            src_height,
        })?;

    if source.len() != expected_src_len {
        return Err(CaptureError::InvalidBufferSize {
            expected: expected_src_len,
            actual: source.len(),
        });
    }

    let max_x = crop.x.checked_add(crop.width).ok_or(CaptureError::OutOfBounds {
        x: crop.x,
        y: crop.y,
        width: crop.width,
        height: crop.height,
        src_width,
        src_height,
    })?;

    let max_y = crop.y.checked_add(crop.height).ok_or(CaptureError::OutOfBounds {
        x: crop.x,
        y: crop.y,
        width: crop.width,
        height: crop.height,
        src_width,
        src_height,
    })?;

    if max_x > src_width || max_y > src_height {
        return Err(CaptureError::OutOfBounds {
            x: crop.x,
            y: crop.y,
            width: crop.width,
            height: crop.height,
            src_width,
            src_height,
        });
    }

    let expected_dest_len = (crop.width as usize)
        .checked_mul(crop.height as usize)
        .and_then(|px| px.checked_mul(4))
        .ok_or(CaptureError::OutOfBounds {
            x: crop.x,
            y: crop.y,
            width: crop.width,
            height: crop.height,
            src_width,
            src_height,
        })?;

    if dest.len() != expected_dest_len {
        return Err(CaptureError::DestinationBufferMismatch {
            expected: expected_dest_len,
            actual: dest.len(),
        });
    }

    let src_stride = (src_width as usize) * 4;
    let row_bytes = (crop.width as usize) * 4;

    for row in 0..(crop.height as usize) {
        let src_y = (crop.y as usize) + row;
        let src_start = src_y * src_stride + (crop.x as usize) * 4;
        let src_end = src_start + row_bytes;

        let dest_start = row * row_bytes;
        let dest_end = dest_start + row_bytes;

        dest[dest_start..dest_end].copy_from_slice(&source[src_start..src_end]);
    }

    Ok(())
}
