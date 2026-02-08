use chic_lib::chess_boards::chess_board::{Color, PieceType};
use egui::{ColorImage, TextureHandle};
use std::collections::HashMap;

// Embed all SVG files at compile time
const SVG_BLACK_BISHOP: &[u8] = include_bytes!("../ui/icons/Piece_Black_Bishop.svg");
const SVG_BLACK_KING: &[u8] = include_bytes!("../ui/icons/Piece_Black_King.svg");
const SVG_BLACK_KNIGHT: &[u8] = include_bytes!("../ui/icons/Piece_Black_Knight.svg");
const SVG_BLACK_PAWN: &[u8] = include_bytes!("../ui/icons/Piece_Black_Pawn.svg");
const SVG_BLACK_QUEEN: &[u8] = include_bytes!("../ui/icons/Piece_Black_Queen.svg");
const SVG_BLACK_ROOK: &[u8] = include_bytes!("../ui/icons/Piece_Black_Rock.svg");
const SVG_WHITE_BISHOP: &[u8] = include_bytes!("../ui/icons/Piece_White_Bishop.svg");
const SVG_WHITE_KING: &[u8] = include_bytes!("../ui/icons/Piece_White_King.svg");
const SVG_WHITE_KNIGHT: &[u8] = include_bytes!("../ui/icons/Piece_White_Knight.svg");
const SVG_WHITE_PAWN: &[u8] = include_bytes!("../ui/icons/Piece_White_Pawn.svg");
const SVG_WHITE_QUEEN: &[u8] = include_bytes!("../ui/icons/Piece_White_Queen.svg");
const SVG_WHITE_ROOK: &[u8] = include_bytes!("../ui/icons/Piece_White_Rock.svg");

/// Convert SVG bytes to PNG image data at specified size
fn svg_to_png(svg_data: &[u8], size: u32) -> Result<ColorImage, String> {
    // Parse SVG
    let tree = resvg::usvg::Tree::from_data(svg_data, &resvg::usvg::Options::default())
        .map_err(|e| format!("Failed to parse SVG: {}", e))?;

    // Create pixmap for rendering
    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| "Failed to create pixmap".to_string())?;

    // Calculate scale to fit SVG in the target size
    let svg_size = tree.size();
    let scale_x = size as f32 / svg_size.width();
    let scale_y = size as f32 / svg_size.height();
    let scale = scale_x.min(scale_y);

    // Render with scaling
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert to ColorImage
    let pixels = pixmap.data();
    let mut rgba_pixels = Vec::with_capacity((size * size * 4) as usize);

    for chunk in pixels.chunks_exact(4) {
        rgba_pixels.push(chunk[0]); // R
        rgba_pixels.push(chunk[1]); // G
        rgba_pixels.push(chunk[2]); // B
        rgba_pixels.push(chunk[3]); // A
    }

    Ok(ColorImage::from_rgba_unmultiplied(
        [size as usize, size as usize],
        &rgba_pixels,
    ))
}

/// Load all piece textures and return as HashMap
pub fn load_piece_textures(
    ctx: &egui::Context,
    piece_size: u32,
) -> HashMap<(Color, PieceType), TextureHandle> {
    let mut textures = HashMap::new();

    let pieces = [
        ((Color::Black, PieceType::Bishop), SVG_BLACK_BISHOP),
        ((Color::Black, PieceType::King), SVG_BLACK_KING),
        ((Color::Black, PieceType::Knight), SVG_BLACK_KNIGHT),
        ((Color::Black, PieceType::Pawn), SVG_BLACK_PAWN),
        ((Color::Black, PieceType::Queen), SVG_BLACK_QUEEN),
        ((Color::Black, PieceType::Rook), SVG_BLACK_ROOK),
        ((Color::White, PieceType::Bishop), SVG_WHITE_BISHOP),
        ((Color::White, PieceType::King), SVG_WHITE_KING),
        ((Color::White, PieceType::Knight), SVG_WHITE_KNIGHT),
        ((Color::White, PieceType::Pawn), SVG_WHITE_PAWN),
        ((Color::White, PieceType::Queen), SVG_WHITE_QUEEN),
        ((Color::White, PieceType::Rook), SVG_WHITE_ROOK),
    ];

    for ((color, piece_type), svg_data) in pieces {
        match svg_to_png(svg_data, piece_size) {
            Ok(image) => {
                let texture_name = format!("{:?}_{:?}", color, piece_type);
                let texture = ctx.load_texture(&texture_name, image, Default::default());
                textures.insert((color, piece_type), texture);
            }
            Err(e) => {
                eprintln!("Failed to load texture for {:?} {:?}: {}", color, piece_type, e);
            }
        }
    }

    textures
}
