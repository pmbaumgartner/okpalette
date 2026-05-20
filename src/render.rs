use crate::color::Rgb8;
use crate::error::{GlasbeyError, Result};

const MAX_IMAGE_BYTES: usize = 100 * 1024 * 1024;

pub fn render_palette_svg(colors: &[Rgb8], width: u32, height: u32) -> Result<String> {
    validate_render_request(colors, width, height)?;
    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-label="Categorical palette" shape-rendering="crispEdges">"#
    ));

    for (index, color) in colors.iter().enumerate() {
        let (x, rect_width) = swatch_bounds(index, colors.len(), width);
        svg.push_str(&format!(
            r#"<rect x="{x}" y="0" width="{rect_width}" height="{height}" fill="{}"/>"#,
            color.to_hex()
        ));
    }

    svg.push_str("</svg>");
    Ok(svg)
}

pub fn render_palette_png(colors: &[Rgb8], width: u32, height: u32) -> Result<Vec<u8>> {
    validate_render_request(colors, width, height)?;
    let row_bytes = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(3))
        .ok_or(GlasbeyError::InvalidRenderRequest {
            message: "palette image dimensions are too large",
        })?;
    let image_bytes = row_bytes
        .checked_mul(
            usize::try_from(height).map_err(|_| GlasbeyError::InvalidRenderRequest {
                message: "palette image dimensions are too large",
            })?,
        )
        .ok_or(GlasbeyError::InvalidRenderRequest {
            message: "palette image dimensions are too large",
        })?;
    if image_bytes > MAX_IMAGE_BYTES {
        return Err(GlasbeyError::InvalidRenderRequest {
            message: "palette image dimensions are too large",
        });
    }

    let mut row = Vec::with_capacity(row_bytes);
    for (index, color) in colors.iter().enumerate() {
        let (_x, rect_width) = swatch_bounds(index, colors.len(), width);
        for _ in 0..rect_width {
            row.extend_from_slice(&[color.r, color.g, color.b]);
        }
    }

    let mut pixels = Vec::with_capacity(image_bytes);
    for _ in 0..height {
        pixels.extend_from_slice(&row);
    }

    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| GlasbeyError::PngEncoding {
                message: error.to_string(),
            })?;
        writer
            .write_image_data(&pixels)
            .map_err(|error| GlasbeyError::PngEncoding {
                message: error.to_string(),
            })?;
    }
    Ok(png_bytes)
}

fn validate_render_request(colors: &[Rgb8], width: u32, height: u32) -> Result<()> {
    if colors.is_empty() {
        return Err(GlasbeyError::InvalidRenderRequest {
            message: "palette must contain at least one color",
        });
    }
    if width == 0 || height == 0 {
        return Err(GlasbeyError::InvalidRenderRequest {
            message: "image dimensions must be positive",
        });
    }

    let color_count =
        u32::try_from(colors.len()).map_err(|_| GlasbeyError::InvalidRenderRequest {
            message: "palette image dimensions are too large",
        })?;
    if width < color_count {
        return Err(GlasbeyError::InvalidRenderRequest {
            message: "width must be at least the number of colors",
        });
    }
    Ok(())
}

fn swatch_bounds(index: usize, color_count: usize, width: u32) -> (u32, u32) {
    let start = (index as u64 * u64::from(width) / color_count as u64) as u32;
    let end = ((index + 1) as u64 * u64::from(width) / color_count as u64) as u32;
    (start, end - start)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{assert_png_dimensions, rgb};

    fn test_palette() -> Vec<Rgb8> {
        vec![rgb(255, 0, 0), rgb(0, 255, 0), rgb(0, 0, 255)]
    }

    #[test]
    fn renders_svg_swatches() {
        let svg = render_palette_svg(&test_palette(), 36, 8).unwrap();

        assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg" width="36" height="8""#));
        assert!(svg.contains(r##"<rect x="0" y="0" width="12" height="8" fill="#ff0000"/>"##));
        assert!(svg.contains(r##"<rect x="12" y="0" width="12" height="8" fill="#00ff00"/>"##));
        assert!(svg.contains(r##"<rect x="24" y="0" width="12" height="8" fill="#0000ff"/>"##));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn renders_png_with_expected_signature_and_dimensions() {
        let png = render_palette_png(&test_palette(), 36, 8).unwrap();

        assert_png_dimensions(&png, 36, 8);
    }

    #[test]
    fn rejects_empty_or_zero_sized_render_requests() {
        assert!(render_palette_svg(&[], 12, 8).is_err());
        assert!(render_palette_png(&test_palette(), 0, 8).is_err());
        assert!(render_palette_png(&test_palette(), 12, 0).is_err());
        assert!(render_palette_png(&test_palette(), 2, 8).is_err());
    }
}
