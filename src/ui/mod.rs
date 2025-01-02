use crate::chess_board::{ChessBoard, Color, PieceType, Square};
use crate::Field;
use crate::ModelRc;
use slint::{Image, VecModel};
use std::path::Path;
// Constants for piece images
pub const BLACK_BISHOP: &str = "ui/icons/Piece_Black_Bishop.svg";
pub const BLACK_KING: &str = "ui/icons/Piece_Black_King.svg";
pub const BLACK_KNIGHT: &str = "ui/icons/Piece_Black_Knight.svg";
pub const BLACK_PAWN: &str = "ui/icons/Piece_Black_Pawn.svg";
pub const BLACK_QUEEN: &str = "ui/icons/Piece_Black_Queen.svg";
pub const BLACK_ROOK: &str = "ui/icons/Piece_Black_Rock.svg";
pub const WHITE_BISHOP: &str = "ui/icons/Piece_White_Bishop.svg";
pub const WHITE_KING: &str = "ui/icons/Piece_White_King.svg";
pub const WHITE_KNIGHT: &str = "ui/icons/Piece_White_Knight.svg";
pub const WHITE_PAWN: &str = "ui/icons/Piece_White_Pawn.svg";
pub const WHITE_QUEEN: &str = "ui/icons/Piece_White_Queen.svg";
pub const WHITE_ROOK: &str = "ui/icons/Piece_White_Rock.svg";

/// Maps a `ChessBoard` to a UI-compatible VecModel representation
pub fn map_chessboard_to_ui(chess_board: &ChessBoard) -> ModelRc<Field> {
    let mut pieces = vec![
        Field {
            image: Image::default()
        };
        64
    ];

    for row in 0..8 {
        for col in 0..8 {
            let square = chess_board.squares[row][col];
            if let Square::Occupied(piece) = square {
                let piece_svg = match (piece.color, piece.kind) {
                    (Color::White, PieceType::Pawn) => WHITE_PAWN,
                    (Color::White, PieceType::Knight) => WHITE_KNIGHT,
                    (Color::White, PieceType::Bishop) => WHITE_BISHOP,
                    (Color::White, PieceType::Rook) => WHITE_ROOK,
                    (Color::White, PieceType::Queen) => WHITE_QUEEN,
                    (Color::White, PieceType::King) => WHITE_KING,
                    (Color::Black, PieceType::Pawn) => BLACK_PAWN,
                    (Color::Black, PieceType::Knight) => BLACK_KNIGHT,
                    (Color::Black, PieceType::Bishop) => BLACK_BISHOP,
                    (Color::Black, PieceType::Rook) => BLACK_ROOK,
                    (Color::Black, PieceType::Queen) => BLACK_QUEEN,
                    (Color::Black, PieceType::King) => BLACK_KING,
                };

                pieces[row * 8 + col] = create_piece(piece_svg);
            }
        }
    }

    ModelRc::new(VecModel::from(pieces))
}

/// Helper function to load a piece as a `Field` object
fn create_piece(piece_svg: &str) -> Field {
    let path_buf = Path::new(env!("CARGO_MANIFEST_DIR")).join(piece_svg);
    Field {
        image: Image::load_from_path(&path_buf).unwrap(),
    }
}
