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

    pub fn to_san(&self, board: &crate::chess_boards::chess_board::ChessBoard) -> String {
        let piece = match board.squares[self.from.row as usize][self.from.col as usize] {
            Square::Occupied(p) => p,
            Square::Empty => return self.as_algebraic(), // Should not happen for legal move
        };

        if piece.kind == PieceType::King {
            // Castling
            if self.from.col == 4 {
                if self.to.col == 6 {
                    return "O-O".to_string();
                } else if self.to.col == 2 {
                    return "O-O-O".to_string();
                }
            }
        }

        let mut san = String::new();

        if piece.kind != PieceType::Pawn {
            san.push_str(&piece.kind.to_string());

            // Disambiguation
            let mut others = Vec::new();
            let all_moves: Vec<_> = board.generate_legal_moves(None).collect();
            for mv in all_moves {
                if mv.to == self.to && mv.from != self.from {
                    let from_square = board.squares[mv.from.row as usize][mv.from.col as usize];
                    if let Square::Occupied(p) = from_square {
                        if p.kind == piece.kind && p.color == piece.color {
                            others.push(mv.from);
                        }
                    }
                }
            }

            if !others.is_empty() {
                let mut same_file = false;
                let mut same_rank = false;
                for other in &others {
                    if other.col == self.from.col {
                        same_file = true;
                    }
                    if other.row == self.from.row {
                        same_rank = true;
                    }
                }

                if !same_file {
                    san.push((b'a' + self.from.col) as char);
                } else if !same_rank {
                    san.push((b'1' + self.from.row) as char);
                } else {
                    san.push((b'a' + self.from.col) as char);
                    san.push((b'1' + self.from.row) as char);
                }
            }
        }

        // Capture
        let is_capture = matches!(
            board.squares[self.to.row as usize][self.to.col as usize],
            Square::Occupied(_)
        ) || (piece.kind == PieceType::Pawn && Some(self.to) == board.en_passant);

        if is_capture {
            if piece.kind == PieceType::Pawn {
                if san.is_empty() {
                    san.push((b'a' + self.from.col) as char);
                }
            }
            san.push('x');
        }

        // Destination
        san.push_str(&to_algebraic_square(self.to.row, self.to.col));

        // Promotion
        if let Some(promo) = self.promotion {
            san.push('=');
            san.push_str(&promo.to_string());
        }

        // Check/Mate
        let mut next_board = board.clone();
        next_board.make_move(*self);
        if next_board.is_checkmate() {
            san.push('#');
        } else if let Some(king_pos) = next_board.find_king_position(next_board.active_color) {
            if next_board.is_square_attacked_by_color(king_pos.row, king_pos.col, board.active_color) {
                san.push('+');
            }
        }

        san
    }

    pub fn from_san(board: &crate::chess_boards::chess_board::ChessBoard, san: &str) -> Result<Self, String> {
        let clean_san = san.trim_matches(|c| c == '+' || c == '#' || c == '!' || c == '?');

        if clean_san == "O-O" {
            let row = if board.active_color == Color::White { 0 } else { 7 };
            return Ok(Move::new(row, 4, row, 6));
        }
        if clean_san == "O-O-O" {
            let row = if board.active_color == Color::White { 0 } else { 7 };
            return Ok(Move::new(row, 4, row, 2));
        }

        let mut piece_kind = PieceType::Pawn;
        let mut move_part = clean_san;

        if let Some(first_char) = clean_san.chars().next() {
            if first_char.is_ascii_uppercase() {
                piece_kind = match first_char {
                    'K' => PieceType::King,
                    'Q' => PieceType::Queen,
                    'R' => PieceType::Rook,
                    'B' => PieceType::Bishop,
                    'N' => PieceType::Knight,
                    _ => PieceType::Pawn,
                };
                if piece_kind != PieceType::Pawn {
                    move_part = &clean_san[1..];
                }
            }
        }

        let promotion = if let Some(idx) = move_part.find('=') {
            let p_char = move_part.chars().nth(idx + 1).ok_or("Invalid promotion")?;
            let p = match p_char {
                'Q' => PieceType::Queen,
                'R' => PieceType::Rook,
                'B' => PieceType::Bishop,
                'N' => PieceType::Knight,
                _ => return Err("Invalid promotion piece".to_string()),
            };
            move_part = &move_part[..idx];
            Some(p)
        } else {
            None
        };

        let move_part = move_part.replace('x', "");
        if move_part.len() < 2 {
            return Err(format!("Invalid SAN move: {}", san));
        }

        let to_str = &move_part[move_part.len() - 2..];
        let to = ChessField::from_algebraic(to_str);

        let mut from_file = None;
        let mut from_rank = None;
        if move_part.len() > 2 {
            let disambig = &move_part[..move_part.len() - 2];
            for c in disambig.chars() {
                if c >= 'a' && c <= 'h' {
                    from_file = Some((c as u8 - b'a') as u8);
                } else if c >= '1' && c <= '8' {
                    from_rank = Some((c as u8 - b'1') as u8);
                }
            }
        }

        let mut candidate_moves = Vec::new();
        for mv in board.generate_legal_moves(None) {
            if mv.to == to {
                if let Square::Occupied(p) = board.squares[mv.from.row as usize][mv.from.col as usize] {
                    if p.kind == piece_kind && p.color == board.active_color {
                        let mut matches = true;
                        if let Some(f) = from_file {
                            if mv.from.col != f {
                                matches = false;
                            }
                        }
                        if let Some(r) = from_rank {
                            if mv.from.row != r {
                                matches = false;
                            }
                        }
                        if matches {
                            candidate_moves.push(mv);
                        }
                    }
                }
            }
        }

        if candidate_moves.len() == 1 {
            let mut mv = candidate_moves[0];
            mv.promotion = promotion;
            Ok(mv)
        } else if candidate_moves.is_empty() {
            Err(format!("No legal move found for SAN: {}", san))
        } else {
            Err(format!("Ambiguous SAN move: {}", san))
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
