use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opposite(&self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Square {
    Occupied(Piece),
    Empty,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct ChessField {
    pub row: u8,
    pub col: u8,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Move {
    pub from: ChessField,
    pub to: ChessField,
    pub promotion: Option<PieceType>,
}

impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PieceType::Pawn => write!(f, "P"),
            PieceType::Knight => write!(f, "N"),
            PieceType::Bishop => write!(f, "B"),
            PieceType::Rook => write!(f, "R"),
            PieceType::Queen => write!(f, "Q"),
            PieceType::King => write!(f, "K"),
        }
    }
}

impl Piece {
    pub fn to_char(&self) -> char {
        match self.kind {
            PieceType::Pawn => {
                if self.color == Color::White {
                    'P'
                } else {
                    'p'
                }
            }
            PieceType::Knight => {
                if self.color == Color::White {
                    'N'
                } else {
                    'n'
                }
            }
            PieceType::Bishop => {
                if self.color == Color::White {
                    'B'
                } else {
                    'b'
                }
            }
            PieceType::Rook => {
                if self.color == Color::White {
                    'R'
                } else {
                    'r'
                }
            }
            PieceType::Queen => {
                if self.color == Color::White {
                    'Q'
                } else {
                    'q'
                }
            }
            PieceType::King => {
                if self.color == Color::White {
                    'K'
                } else {
                    'k'
                }
            }
        }
    }
}

impl ChessField {
    pub fn new(row: u8, col: u8) -> Self {
        Self { row, col }
    }
    pub fn from_algebraic(algebraic: &str) -> Self {
        let file = algebraic.chars().next().unwrap();
        let rank = algebraic.chars().nth(1).unwrap();
        let col = (file as u8 - b'a') as u8;
        let row = (rank as u8 - b'1') as u8;
        Self { row, col }
    }
    pub fn as_algebraic(&self) -> String {
        to_algebraic_square(self.row, self.col)
    }

}

impl Move {
    // Create a new Move
    pub fn new(from_row: u8, from_col: u8, to_row: u8, to_col: u8) -> Self {
        Self {
            from: ChessField::new(from_row, from_col),
            to: ChessField::new(to_row, to_col),
            promotion: None,
        }
    }

    pub fn with_promotion(mut self, promotion: PieceType) -> Self {
        self.promotion = Some(promotion);
        self
    }

    pub fn as_algebraic(&self) -> String {
        let base_move = format!(
            "{}{}",
            to_algebraic_square(self.from.row, self.from.col),
            to_algebraic_square(self.to.row, self.to.col)
        );
        if let Some(promo) = self.promotion {
            base_move + &promo.to_string().to_lowercase()
        } else {
            base_move
        }
    }
    pub fn from_algebraic(algebraic: &str) -> Self {
        let from = ChessField::from_algebraic(&algebraic[0..2]);
        let to = ChessField::from_algebraic(&algebraic[2..4]);

        let promotion = if algebraic.len() > 4 {
            match algebraic.chars().nth(4) {
                Some('Q') => Some(PieceType::Queen),
                Some('R') => Some(PieceType::Rook),
                Some('B') => Some(PieceType::Bishop),
                Some('N') => Some(PieceType::Knight),
                Some('q') => Some(PieceType::Queen),
                Some('r') => Some(PieceType::Rook),
                Some('b') => Some(PieceType::Bishop),
                Some('n') => Some(PieceType::Knight),
                _ => None,
            }
        } else {
            None // No promotion if the move string is only 4 characters
        };
        Self { from, to, promotion }
    }
}

pub fn to_algebraic_square(row: u8, col: u8) -> String {
    let file = (b'a' + col) as char; // Convert 0-7 column index to 'a'-'h'
    let rank = (row + 1).to_string(); // Convert 0-7 row index to '8'-'1'
    format!("{}{}", file, rank) // Combine file and rank into a string
}
