pub mod fen;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    White,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Square {
    Occupied(Piece),
    Empty,
}

pub struct ChessBoard {
    pub squares: [[Square; 8]; 8],
    pub active_color: Color,
    pub castling_rights: [bool; 4],
    pub en_passant: Option<(usize, usize)>,
    pub halfmove_clock: u32,
    pub fullmove_number: u32,
}

impl ChessBoard {
    /// Creates an empty chess board
    pub fn new() -> Self {
        Self {
            squares: [[Square::Empty; 8]; 8],
            active_color: Color::White,  // Default active color to White
            castling_rights: [false; 4], // No castling rights by default
            en_passant: None,            // No en passant square by default
            halfmove_clock: 0,           // Halfmove clock starts at 0
            fullmove_number: 1,
        }
    }

    /// Delegates FEN parsing to the `fen` module.
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        fen::from_fen(fen)
    }
}
