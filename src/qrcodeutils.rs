use anyhow::{anyhow, Result};
use png;
use resvg::render;
use tiny_skia::Pixmap;
use usvg::{Options, Tree};

/// Converts SVG (as bytes) to PNG (returns Vec<u8> with PNG data).
pub fn convert_svg_to_png(svg_data: &[u8]) -> Result<Vec<u8>> {
    // 1) Parse SVG using usvg
    let opt = Options::default();
    let tree = Tree::from_data(svg_data, &opt).map_err(|e| anyhow!("Error parsing SVG: {}", e))?;

    // 2) Get SVG dimensions from the root node
    let svg_size = tree.size();
    let width = svg_size.width() as u32;
    let height = svg_size.height() as u32;

    // 3) Create a Pixmap of the required size
    let mut pixmap =
        Pixmap::new(width, height).ok_or_else(|| anyhow!("Failed to create Pixmap"))?;

    // 4) Render SVG to Pixmap using FitTo::Original
    render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    // 5) Encode Pixmap (RGBA) to PNG
    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(pixmap.data())?;
    }

    Ok(png_data)
}
