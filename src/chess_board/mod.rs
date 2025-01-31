use crate::chess_board::zobrist_hash::ZOBRIST;
use circular_buffer::CircularBuffer;
use std::fmt;

pub mod fen;
pub mod zobrist_hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
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
    pub row: usize,
    pub col: usize,
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

impl ChessField {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
    pub fn from_algebraic(algebraic: &str) -> Self {
        let file = algebraic.chars().next().unwrap();
        let rank = algebraic.chars().nth(1).unwrap();
        let col = (file as u8 - b'a') as usize;
        let row = (rank as u8 - b'1') as usize;
        Self { row, col }
    }
}

impl Move {
    // Create a new Move
    pub fn new(from_row: usize, from_col: usize, to_row: usize, to_col: usize) -> Self {
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

fn to_algebraic_square(row: usize, col: usize) -> String {
    let file = (b'a' + col as u8) as char; // Convert 0-7 column index to 'a'-'h'
    let rank = (row + 1).to_string(); // Convert 0-7 row index to '8'-'1'
    format!("{}{}", file, rank) // Combine file and rank into a string
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChessBoard {
    pub squares: [[Square; 8]; 8],
    pub active_color: Color,
    pub castling_rights: [bool; 4],
    pub en_passant: Option<ChessField>,
    pub halfmove_clock: u32,
    pub fullmove_number: u32,
    pub repetition_map: CircularBuffer<32, u64>,
}

const NO_CAPTURE: i32 = 0;
const CAPTURE: i32 = 10000;
const CAPTURE_BASE: i32 = CAPTURE + 10;
const CASTLING_SCORE: i32 = 50;

fn get_piece_value(piece: &PieceType) -> i32 {
    match piece {
        PieceType::Pawn => 1,
        PieceType::Knight => 3,
        PieceType::Bishop => 3,
        PieceType::Rook => 5,
        PieceType::Queen => 9,
        PieceType::King => 15,
    }
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
            repetition_map: CircularBuffer::new(),
        }
    }

    /// Delegates FEN parsing to the `fen` module.
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        fen::from_fen(fen).map(|mut board| {
            let zobrist = &*ZOBRIST;
            board.repetition_map.push_back(zobrist.calculate_hash(&board));
            board
        })
    }

    pub fn generate_pseudo_moves(&self) -> Vec<(Move, i32)> {
        let mut all_moves: Vec<(Move, i32)> = Vec::new();

        for row in 0..8 {
            for col in 0..8 {
                // Only process pieces of the active color
                all_moves.extend(self.generate_pseudo_moves_from_position(row, col));
            }
        }
        all_moves
    }

    pub fn generate_pseudo_moves_from_position(&self, row: usize, col: usize) -> Vec<(Move, i32)> {
        if let Square::Occupied(piece) = self.squares[row][col] {
            if piece.color == self.active_color {
                let r = match piece.kind {
                    PieceType::Pawn => self.generate_pawn_moves(row, col),
                    PieceType::Knight => self.generate_knight_moves(row, col),
                    PieceType::Bishop => self.generate_bishop_moves(row, col),
                    PieceType::Rook => self.generate_rook_moves(row, col),
                    PieceType::Queen => self.generate_queen_moves(row, col),
                    PieceType::King => self.generate_king_moves(row, col),
                };
                return r;
            }
        }
        Vec::new()
    }

    fn generate_pawn_moves(&self, row: usize, col: usize) -> Vec<(Move, i32)> {
        let mut moves = Vec::new();
        let forward = match self.active_color {
            Color::White => 1,
            Color::Black => -1,
        };

        let start_row = match self.active_color {
            Color::White => 1,
            Color::Black => 6,
        };

        let promotion_row = match self.active_color {
            Color::White => 7,
            Color::Black => 0,
        };

        let new_row = (row as isize + forward) as usize;

        // Regular forward move
        if self.squares[new_row][col] == Square::Empty {
            Self::add_pawn_moves_with_and_without_promotion(
                row,
                col,
                new_row,
                col,
                promotion_row,
                NO_CAPTURE,
                &mut moves,
            );

            // Double move from start position
            if row == start_row {
                let two_forward = (row as isize + 2 * forward) as usize;
                if self.squares[two_forward][col] == Square::Empty {
                    moves.push((Move::new(row, col, two_forward, col), NO_CAPTURE));
                }
            }
        }

        // Capture diagonally
        for &dx in [-1, 1].iter() {
            if (col as isize + dx).is_negative() || (col as isize + dx) >= 8 {
                continue;
            }

            let new_col = (col as isize + dx) as usize;
            if let Square::Occupied(opponent_piece) = self.squares[new_row][new_col] {
                if opponent_piece.color != self.active_color {
                    Self::add_pawn_moves_with_and_without_promotion(
                        row,
                        col,
                        new_row,
                        new_col,
                        promotion_row,
                        CAPTURE_BASE + get_piece_value(&opponent_piece.kind) - 1,
                        &mut moves,
                    );
                }
            }
        }

        // En passant
        if let Some(en_passant) = self.en_passant {
            if new_row == en_passant.row && (col as isize - en_passant.col as isize).abs() == 1 {
                moves.push((Move::new(row, col, en_passant.row, en_passant.col), CAPTURE_BASE));
            }
        }

        moves
    }

    fn add_pawn_moves_with_and_without_promotion(
        row: usize,
        col: usize,
        new_row: usize,
        new_col: usize,
        promotion_row: usize,
        score: i32,
        moves: &mut Vec<(Move, i32)>,
    ) {
        if new_row == promotion_row {
            for &promotion_piece in &[PieceType::Queen, PieceType::Rook, PieceType::Bishop, PieceType::Knight] {
                moves.push((
                    Move::new(row, col, new_row, new_col).with_promotion(promotion_piece),
                    score + 1,
                ));
            }
        } else {
            moves.push((Move::new(row, col, new_row, new_col), score));
        }
    }

    /// Generate knight moves.
    fn generate_knight_moves(&self, row: usize, col: usize) -> Vec<(Move, i32)> {
        const KNIGHT_MOVES: [(isize, isize); 8] =
            [(-2, -1), (-1, -2), (1, -2), (2, -1), (2, 1), (1, 2), (-1, 2), (-2, 1)];

        self.generate_moves_from_directions(row, col, &KNIGHT_MOVES)
    }

    /// Generate sliding piece moves (bishop, rook, queen).
    fn generate_sliding_moves(&self, row: usize, col: usize, directions: &[(isize, isize)]) -> Vec<(Move, i32)> {
        let mut moves: Vec<(Move, i32)> = Vec::new();

        let moving_piece = match self.squares[row][col] {
            Square::Occupied(p) => p,
            _ => return moves,
        };

        for &(dx, dy) in directions {
            let mut new_row = row as isize;
            let mut new_col = col as isize;

            loop {
                new_row += dx;
                new_col += dy;

                if !(0..8).contains(&new_col) || !(0..8).contains(&new_row) {
                    break;
                }

                match self.squares[new_row as usize][new_col as usize] {
                    Square::Empty => moves.push((Move::new(row, col, new_row as usize, new_col as usize), NO_CAPTURE)),
                    Square::Occupied(p) => {
                        if p.color != self.active_color {
                            moves.push((
                                Move::new(row, col, new_row as usize, new_col as usize),
                                CAPTURE_BASE + get_piece_value(&p.kind) - get_piece_value(&moving_piece.kind),
                            ));
                        }
                        break; // Block sliding
                    }
                }
            }
        }

        moves
    }

    /// Generate bishop moves.
    fn generate_bishop_moves(&self, row: usize, col: usize) -> Vec<(Move, i32)> {
        const BISHOP_DIRECTIONS: [(isize, isize); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
        self.generate_sliding_moves(row, col, &BISHOP_DIRECTIONS)
    }

    /// Generate rook moves.
    fn generate_rook_moves(&self, row: usize, col: usize) -> Vec<(Move, i32)> {
        const ROOK_DIRECTIONS: [(isize, isize); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
        self.generate_sliding_moves(row, col, &ROOK_DIRECTIONS)
    }

    /// Generate queen moves.
    fn generate_queen_moves(&self, row: usize, col: usize) -> Vec<(Move, i32)> {
        const QUEEN_DIRECTIONS: [(isize, isize); 8] =
            [(-1, -1), (-1, 1), (1, -1), (1, 1), (0, -1), (0, 1), (-1, 0), (1, 0)];
        self.generate_sliding_moves(row, col, &QUEEN_DIRECTIONS)
    }

    /// Generate king moves (including castling).
    fn generate_king_moves(&self, row: usize, col: usize) -> Vec<(Move, i32)> {
        const KING_MOVES: [(isize, isize); 8] = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];

        let mut moves = self.generate_moves_from_directions(row, col, &KING_MOVES);

        // Castling logic
        let castling_rank = match self.active_color {
            Color::White => 0,
            Color::Black => 7,
        };

        if row == castling_rank && col == 4 {
            // Ensure the king is in its starting position (e1/e8)
            // Kingside castling
            if self.castling_rights[if self.active_color == Color::White { 0 } else { 2 }]
                && self.squares[row][5] == Square::Empty
                && self.squares[row][6] == Square::Empty
                && !self.is_square_attacked(row, 4)
                && !self.is_square_attacked(row, 5)
                && !self.is_square_attacked(row, 6)
            {
                moves.push((Move::new(row, 4, row, 6), CASTLING_SCORE)); // Move King: e1->g1 or e8->g8
            }

            // Queenside castling
            if self.castling_rights[if self.active_color == Color::White { 1 } else { 3 }]
                && self.squares[row][3] == Square::Empty
                && self.squares[row][2] == Square::Empty
                && self.squares[row][1] == Square::Empty
                && !self.is_square_attacked(row, 4)
                && !self.is_square_attacked(row, 3)
                && !self.is_square_attacked(row, 2)
            {
                moves.push((Move::new(row, 4, row, 2), CASTLING_SCORE)); // Move King: e1->c1 or e8->c8
            }
        }
        moves
    }

    pub fn make_move(&mut self, mv: Move) {
        let piece = self.squares[mv.from.row][mv.from.col];

        match piece {
            Square::Empty => {
                self.en_passant = None;
            }
            Square::Occupied(p) => {
                self.squares[mv.from.row][mv.from.col] = Square::Empty;
                self.squares[mv.to.row][mv.to.col] = piece;

                if let Some(en_passant) = self.en_passant {
                    if mv.to == en_passant && p.kind == PieceType::Pawn {
                        //Remove piece from en passant
                        self.squares[mv.from.row][mv.to.col] = Square::Empty;
                    }
                }
                self.en_passant = None;

                // Check if the move is a castling move and if castling is allowed
                if p.kind == PieceType::King {
                    if mv.from.col == 4 && mv.to.col == 6 && mv.from.row == mv.to.row {
                        if self.castling_rights[if self.active_color == Color::White { 0 } else { 2 }] {
                            let rook_col = 7;
                            self.squares[mv.from.row][5] = self.squares[mv.from.row][rook_col];
                            self.squares[mv.from.row][rook_col] = Square::Empty;
                        }
                    } else if mv.from.col == 4 && mv.to.col == 2 && mv.from.row == mv.to.row {
                        // Queenside castling
                        if self.castling_rights[if self.active_color == Color::White { 1 } else { 3 }] {
                            let rook_col = 0;
                            self.squares[mv.from.row][3] = self.squares[mv.from.row][rook_col];
                            self.squares[mv.from.row][rook_col] = Square::Empty;
                        }
                    }
                }
                if mv.from.row == 0 && mv.from.col == 0 {
                    self.castling_rights[1] = false;
                } else if mv.from.row == 7 && mv.from.col == 0 {
                    self.castling_rights[3] = false;
                } else if mv.from.row == 0 && mv.from.col == 7 {
                    self.castling_rights[0] = false;
                } else if mv.from.row == 7 && mv.from.col == 7 {
                    self.castling_rights[2] = false;
                } else if mv.from.row == 0 && mv.from.col == 4 {
                    self.castling_rights[0] = false;
                    self.castling_rights[1] = false;
                } else if mv.from.row == 7 && mv.from.col == 4 {
                    self.castling_rights[2] = false;
                    self.castling_rights[3] = false;
                }
                //capture of the rooks
                if mv.to.row == 0 && mv.to.col == 0 {
                    self.castling_rights[1] = false;
                } else if mv.to.row == 7 && mv.to.col == 0 {
                    self.castling_rights[3] = false;
                } else if mv.to.row == 0 && mv.to.col == 7 {
                    self.castling_rights[0] = false;
                } else if mv.to.row == 7 && mv.to.col == 7 {
                    self.castling_rights[2] = false;
                }

                if p.kind == PieceType::Pawn || matches!(self.squares[mv.to.row][mv.to.col], Square::Occupied(_)) {
                    self.halfmove_clock = 0;
                } else {
                    self.halfmove_clock += 1;
                }

                if p.kind == PieceType::Pawn {
                    if p.color == Color::White && mv.from.row == 1 && mv.to.row == 3 {
                        self.en_passant = Some(ChessField::new(2, mv.from.col));
                    } else if p.color == Color::Black && mv.from.row == 6 && mv.to.row == 4 {
                        self.en_passant = Some(ChessField::new(5, mv.from.col));
                    } else if mv.promotion.is_some() {
                        // Handle promotion
                        self.squares[mv.to.row][mv.to.col] = Square::Occupied(Piece {
                            color: p.color,
                            kind: mv.promotion.unwrap(), // Replace the pawn with the promoted piece
                        });
                    }
                }
            }
        }

        // Switch the active color after a move
        self.active_color = match self.active_color {
            Color::White => Color::Black,
            Color::Black => Color::White,
        };

        if self.active_color == Color::White {
            self.fullmove_number += 1;
        }

        let zobrist = &*ZOBRIST;
        self.repetition_map.push_back(zobrist.calculate_hash(self));
    }

    pub fn is_square_attacked(&self, row: usize, col: usize) -> bool {
        let opponent_color = match self.active_color {
            Color::White => Color::Black,
            Color::Black => Color::White,
        };
        self.is_square_attacked_by_color(row, col, opponent_color)
    }

    pub fn is_square_attacked_by_color(&self, row: usize, col: usize, opponent_color: Color) -> bool {
        const KNIGHT_MOVES: [(isize, isize); 8] =
            [(-2, -1), (-1, -2), (1, -2), (2, -1), (2, 1), (1, 2), (-1, 2), (-2, 1)];

        const KING_MOVES: [(isize, isize); 8] = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];

        // Check for attacks by sliding pieces
        const DIRECTIONS: [(isize, isize); 8] = [
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1), // Rook-like directions (orthogonal)
            (-1, -1),
            (-1, 1),
            (1, -1),
            (1, 1), // Bishop-like directions (diagonals)
        ];
        for &(dx, dy) in &DIRECTIONS {
            let mut new_row = row as isize;
            let mut new_col = col as isize;

            let is_diagonal = dx != 0 && dy != 0; // Diagonal movement
            let is_orthogonal = dx == 0 || dy == 0; // Orthogonal movement

            loop {
                new_row += dx;
                new_col += dy;

                if !(0..8).contains(&new_col) || !(0..8).contains(&new_row) {
                    break;
                }

                match self.squares[new_row as usize][new_col as usize] {
                    Square::Empty => continue,
                    Square::Occupied(piece) => {
                        if piece.color == opponent_color {
                            match piece.kind {
                                PieceType::Rook if is_orthogonal => return true,
                                PieceType::Bishop if is_diagonal => return true,
                                PieceType::Queen => return true,
                                _ => break,
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        let pawn_attacks = match opponent_color {
            Color::Black => [(1, -1), (1, 1)],
            Color::White => [(-1, -1), (-1, 1)],
        };

        if self.check_attack(row, col, opponent_color, &pawn_attacks, PieceType::Pawn) {
            return true;
        }

        if self.check_attack(row, col, opponent_color, &KNIGHT_MOVES, PieceType::Knight) {
            return true;
        }

        if self.check_attack(row, col, opponent_color, &KING_MOVES, PieceType::King) {
            return true;
        }

        false
    }

    fn check_attack(
        &self,
        row: usize,
        col: usize,
        opponent_color: Color,
        directions: &[(isize, isize)],
        piece_type: PieceType,
    ) -> bool {
        for &(dx, dy) in directions {
            let new_row = row as isize + dx;
            let new_col = col as isize + dy;

            if (0..8).contains(&new_col) && (0..8).contains(&new_row) {
                if let Square::Occupied(piece) = self.squares[new_row as usize][new_col as usize] {
                    if piece.color == opponent_color && piece.kind == piece_type {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn generate_moves_from_directions(
        &self,
        row: usize,
        col: usize,
        directions: &[(isize, isize)],
    ) -> Vec<(Move, i32)> {
        let mut moves = Vec::new();

        let moving_piece = match self.squares[row][col] {
            Square::Occupied(p) => p,
            _ => return moves,
        };

        for &(dx, dy) in directions {
            let new_row = (row as isize + dx) as usize;
            let new_col = (col as isize + dy) as usize;

            if new_row < 8 && new_col < 8 {
                match self.squares[new_row as usize][new_col as usize] {
                    Square::Empty => moves.push((Move::new(row, col, new_row as usize, new_col as usize), NO_CAPTURE)),
                    Square::Occupied(p) => {
                        if p.color != self.active_color {
                            moves.push((
                                Move::new(row, col, new_row as usize, new_col as usize),
                                CAPTURE_BASE + get_piece_value(&p.kind) - get_piece_value(&moving_piece.kind),
                            ));
                        }
                    }
                }
            }
        }
        moves
    }

    pub fn find_king_position(&self, color: Color) -> Option<ChessField> {
        for row in 0..8 {
            for col in 0..8 {
                if let Square::Occupied(Piece {
                    color: piece_color,
                    kind: PieceType::King,
                }) = self.squares[row][col]
                {
                    if piece_color == color {
                        return Some(ChessField::new(row, col));
                    }
                }
            }
        }
        None // Should never occur in a valid chess position
    }

    pub fn generate_legal_moves(&self) -> Vec<Move> {
        let mut legal_moves = Vec::new();

        // Generate all pseudo-legal moves
        let pseudo_moves = self.generate_pseudo_moves();

        // For each pseudo-legal move, check if it leaves the king in check
        for mv in pseudo_moves {
            let mut board_clone = self.clone();
            board_clone.make_move(mv.0);

            let king_position = board_clone.find_king_position(self.active_color);

            // if mv.0.from.col == 1 && mv.0.from.row == 3 {
            //     println!("{:?}", mv);
            // }
            if let Some(king_pos) = king_position {
                if !board_clone.is_square_attacked_by_color(king_pos.row, king_pos.col, board_clone.active_color) {
                    legal_moves.push(mv); // Add move to legal moves if not leaving the king in check
                }
            }
        }
        legal_moves.sort_by(|a, b| b.1.cmp(&a.1));
        legal_moves.iter().map(|m| m.0).collect()
    }

    pub fn generate_capture_moves(&self) -> Vec<Move> {
        let mut capture_moves = Vec::new();

        for row in 0..8 {
            for col in 0..8 {
                // Only process pieces of the active color
                if let Square::Occupied(piece) = self.squares[row][col] {
                    if piece.color == self.active_color {
                        let piece_moves = self.generate_pseudo_moves_from_position(row, col);

                        // Filter for capture moves
                        for mv in piece_moves {
                            if let Square::Occupied(target_piece) = self.squares[mv.0.to.row][mv.0.to.col] {
                                if target_piece.color != self.active_color {
                                    capture_moves.push(mv);
                                }
                            }
                        }
                    }
                }
            }
        }

        capture_moves.sort_by(|a, b| b.1.cmp(&a.1));
        capture_moves.iter().map(|m| m.0).collect()
    }

    pub fn generate_legal_capture_moves(&self) -> Vec<Move> {
        let mut legal_moves = Vec::new();

        let pseudo_moves = self.generate_capture_moves();

        for mv in pseudo_moves {
            let mut board_clone = self.clone(); // Clone the board to simulate the move
            board_clone.make_move(mv); // Make the move on the cloned board

            // Locate the king of the current player
            let king_position = board_clone.find_king_position(self.active_color);

            // Check if the king is under attack after the move
            if let Some(king_pos) = king_position {
                if !board_clone.is_square_attacked_by_color(king_pos.row, king_pos.col, board_clone.active_color) {
                    legal_moves.push(mv); // Add move to legal moves if not leaving the king in check
                }
            }
        }

        legal_moves
    }

    #[allow(dead_code)]
    pub fn is_stalemate(&self) -> bool {
        if let Some(king_pos) = self.find_king_position(self.active_color) {
            if self.is_square_attacked(king_pos.row, king_pos.col) {
                return false;
            }
        } else {
            return false;
        }
        self.generate_legal_moves().is_empty()
    }

    #[allow(dead_code)]
    pub fn is_checkmate(&self) -> bool {
        // Step 1: Ensure the active player's king is in check
        if let Some(king_pos) = self.find_king_position(self.active_color) {
            if !self.is_square_attacked(king_pos.row, king_pos.col) {
                return false;
            }
        } else {
            return false;
        }

        self.generate_legal_moves().is_empty()
    }

    #[allow(dead_code)]
    pub fn is_draw(&self) -> bool {
        self.is_draw_by_fifty_move_rule() || self.is_threefold_repetition()
    }
    #[allow(dead_code)]
    pub fn is_draw_by_fifty_move_rule(&self) -> bool {
        self.halfmove_clock >= 100
    }

    pub fn is_threefold_repetition(&self) -> bool {
        let mut repetition_count = 0;

        if let Some(&current_hash) = self.repetition_map.back() {
            for &stored_hash in self.repetition_map.iter() {
                if stored_hash == current_hash {
                    repetition_count += 1;
                }
                if repetition_count >= 3 {
                    return true;
                }
            }
        }

        false
    }
}

pub fn perft(board: &ChessBoard, depth: u8) -> u64 {
    let mut node_count = 0u64;

    if depth <= 0 {
        return 1u64;
    }

    let moves = board.generate_legal_moves();
    if moves.len() == 0 {
        return 0u64;
    }
    for mv in moves {
        let mut new_board = board.clone();
        new_board.make_move(mv);
        node_count += perft(&new_board, depth - 1);
    }
    node_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_board::Square::Occupied;

    impl ChessBoard {
        /// Creates an empty chess board
        pub fn generate_pseudo_moves_from_chess_field(&self, pos: ChessField) -> Vec<Move> {
            self.generate_pseudo_moves_from_position(pos.row, pos.col)
                .into_iter()
                .map(|m| m.0)
                .collect()
        }

        pub fn generate_pseudo_moves_from_algebraic(&self, square: &str) -> Vec<Move> {
            self.generate_pseudo_moves_from_chess_field(ChessField::from_algebraic(square))
        }
    }

    impl ChessField {
        pub fn as_algebraic(&self) -> String {
            to_algebraic_square(self.row, self.col)
        }
    }

    fn assert_moves(generated: Vec<Move>, mut expected: Vec<&str>) {
        let mut generated_converted: Vec<_> = generated.iter().map(|m| m.as_algebraic()).collect();
        generated_converted.sort();
        expected.sort();

        assert_eq!(generated_converted, expected);
    }

    #[test]
    fn test_convertion_method() {
        assert_eq!(ChessField::from_algebraic("b2"), ChessField::new(1, 1));
        assert_eq!(ChessField::from_algebraic("b2").as_algebraic(), "b2");

        assert_eq!(Move::from_algebraic("e2e4").as_algebraic(), "e2e4");
    }
    #[test]
    fn test_generate_pawn_moves_pseudo_legal() {
        // Test simple pawn moves. Pawn at e4 can move forward to e5
        let board = ChessBoard::from_fen("8/8/8/8/4P3/8/8/8 w - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e4"), vec!["e4e5"]);

        // Test blocked pawn a3 by a4
        let board = ChessBoard::from_fen("8/8/8/8/P7/P7/8/8 w - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a3"), vec![]);

        // Test en passant
        let board = ChessBoard::from_fen("8/8/3p4/4Pp2/8/8/8/8 w - f6 0 1").unwrap();

        // White pawn at e5 can capture en passant at f6 and capture at d6 and make a move to 46
        let expected_moves = vec!["e5d6", "e5e6", "e5f6"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e5"), expected_moves);

        // Move pawn on b2 and black a3 and c3
        let board = ChessBoard::from_fen("8/8/8/8/8/p1p5/1P6/8 w - - 0 1").unwrap();
        let expected_moves = vec!["b2b3", "b2b4", "b2a3", "b2c3"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("b2"), expected_moves);

        // Move black pawn on a6 to a5
        let board = ChessBoard::from_fen("8/8/p7/8/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a6"), vec!["a6a5"]);

        // Test blocked blacked pawn a6 by a5
        let board = ChessBoard::from_fen("8/8/p7/p7/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a6"), vec![]);

        // Test single and double step of blacked pawn a7
        let board = ChessBoard::from_fen("8/p7/8/8/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a7"), vec!["a7a6", "a7a5"]);

        // Test single move of blacked pawn a7 and double step is blocked
        let board = ChessBoard::from_fen("8/p7/8/p7/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a7"), vec!["a7a6"]);

        // Test pawn move on a7. Capture on b6 is not allowed by same color
        let board = ChessBoard::from_fen("8/p7/1p6/8/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a7"), vec!["a7a6", "a7a5"]);

        // Test pawn move on a7 with capture on b6
        let board = ChessBoard::from_fen("8/p7/1P6/8/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("a7"),
            vec!["a7a6", "a7a5", "a7b6"],
        );

        // Test pawn move on b7 with capture on a6 and c6
        let board = ChessBoard::from_fen("8/1p6/P1P5/8/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("b7"),
            vec!["b7b6", "b7b5", "b7a6", "b7c6"],
        );

        // Test white promotion
        let board = ChessBoard::from_fen("8/6P1/8/8/8/8/8/8 w - - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("g7"),
            vec!["g7g8q", "g7g8r", "g7g8b", "g7g8n"],
        );

        // Test black promotion
        let board = ChessBoard::from_fen("3r4/2P5/8/8/8/8/8/8 w - - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("c7"),
            vec!["c7c8b", "c7c8n", "c7c8r", "c7c8q", "c7d8b", "c7d8n", "c7d8r", "c7d8q"],
        );

        // Test black promotion
        let board = ChessBoard::from_fen("4k1nr/2p3p1/b2pPp1p/8/1nN1P1P1/5N2/Pp3P2/2R2K2 b k - 1 27").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("b2"),
            vec!["b2b1b", "b2b1n", "b2b1q", "b2b1r", "b2c1b", "b2c1n", "b2c1r", "b2c1q"],
        );
    }

    #[test]
    fn test_generate_knight_moves_pseudo_legal() {
        // White knight at d4 can move to 8 possible squares
        let board = ChessBoard::from_fen("8/8/8/8/3N4/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["d4b3", "d4c2", "d4e2", "d4f3", "d4f5", "d4e6", "d4c6", "d4b5"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d4"), expected_moves);

        // Black knight at d4 can move to 8 possible squares incl. one capture
        let board = ChessBoard::from_fen("8/8/8/5N2/3n4/8/8/8 b - - 0 1").unwrap();
        let expected_moves = vec!["d4b3", "d4c2", "d4e2", "d4f3", "d4f5", "d4e6", "d4c6", "d4b5"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d4"), expected_moves);

        // White knight at a3 with blocked fields
        let board = ChessBoard::from_fen("8/8/8/1rn5/2r5/N7/2B5/1Q6 w - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a3"), vec!["a3c4", "a3b5"]);

        // Black knight knight at a3 with blocked fields
        let board = ChessBoard::from_fen("8/8/8/1RN5/2R5/n7/2b5/1q6 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a3"), vec!["a3c4", "a3b5"]);
    }

    #[test]
    fn test_generate_bishop_moves_pseudo_legal() {
        // Test bishop moves with 2 diagonals
        let board = ChessBoard::from_fen("8/8/8/8/3B4/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec![
            "d4a7", "d4b6", "d4c5", "d4e3", "d4f2", "d4g1", //first diagonal
            "d4a1", "d4b2", "d4c3", "d4e5", "d4f6", "d4g7", "d4h8",
        ];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d4"), expected_moves);

        // Test bishop with a capture and a blocked square
        let board = ChessBoard::from_fen("8/6r1/5B2/8/3P4/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["f6d8", "f6e7", "f6g5", "f6h4", "f6e5", "f6g7"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("f6"), expected_moves);

        // Test black bishop moves with 2 diagonals
        let board = ChessBoard::from_fen("8/8/8/8/8/3b4/8/8 b - - 0 1").unwrap();
        let expected_moves = vec![
            "d3a6", "d3b5", "d3c4", "d3e2", "d3f1", //first diagonal
            "d3b1", "d3c2", "d3e4", "d3f5", "d3g6", "d3h7",
        ];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d3"), expected_moves);

        // Test black bishop with a capture and a blocked square
        let board = ChessBoard::from_fen("8/6R1/5b2/8/3p4/8/8/8 b - - 0 1").unwrap();
        let expected_moves = vec!["f6d8", "f6e7", "f6g5", "f6h4", "f6e5", "f6g7"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("f6"), expected_moves);
    }

    #[test]
    fn test_generate_rook_moves_pseudo_legal() {
        // Test rook moves
        let board = ChessBoard::from_fen("8/8/8/8/3R4/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec![
            "d4d1", "d4d2", "d4d3", "d4d5", "d4d6", "d4d7", "d4d8", "d4a4", "d4b4", "d4c4", "d4e4", "d4f4", "d4g4",
            "d4h4",
        ];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d4"), expected_moves);

        // Test black rook with a capture and blocked squares
        let board = ChessBoard::from_fen("8/8/8/8/3bR3/8/4N3/8 w - - 0 1").unwrap();
        let expected_moves = vec!["e4e3", "e4e5", "e4e6", "e4e7", "e4e8", "e4d4", "e4f4", "e4g4", "e4h4"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e4"), expected_moves);

        // Test black rook moves
        let board = ChessBoard::from_fen("8/8/8/8/8/3r4/8/8 b - - 0 1").unwrap();
        let expected_moves = vec![
            "d3d1", "d3d2", "d3d4", "d3d5", "d3d6", "d3d7", "d3d8", "d3a3", "d3b3", "d3c3", "d3e3", "d3f3", "d3g3",
            "d3h3",
        ];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d3"), expected_moves);

        // Test black rook with a capture and blocked squares
        let board = ChessBoard::from_fen("8/8/8/8/3Br3/8/4n3/8 b - - 0 1").unwrap();
        let expected_moves = vec!["e4e3", "e4e5", "e4e6", "e4e7", "e4e8", "e4d4", "e4f4", "e4g4", "e4h4"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e4"), expected_moves);
    }

    #[test]
    fn test_generate_queen_moves_pseudo_legal() {
        // Test queen moves
        let board = ChessBoard::from_fen("8/8/8/8/3Q4/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec![
            "d4d1", "d4d2", "d4d3", "d4d5", "d4d6", "d4d7", "d4d8", "d4a4", "d4b4", "d4c4", "d4e4", "d4f4", "d4g4",
            "d4h4", "d4a7", "d4b6", "d4c5", "d4e3", "d4f2", "d4g1", //first diagonal
            "d4a1", "d4b2", "d4c3", "d4e5", "d4f6", "d4g7", "d4h8",
        ];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d4"), expected_moves);

        // Test queen move from g6 with 3 capture and a blocked square
        let board = ChessBoard::from_fen("4b1b1/6b1/4r1Q1/5P2/6B1/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["g6e8", "g6f7", "g6e6", "g6f6", "g6g7", "g6g5", "g6h5", "g6h6", "g6h7"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("g6"), expected_moves);

        // Test queen move from a5 with 2 capture and a blocked square
        let board = ChessBoard::from_fen("8/b7/1b6/qb6/1P6/P7/8/8 b - - 0 1").unwrap();
        let expected_moves = vec!["a5a6", "a5a4", "a5a3", "a5b4"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("a5"), expected_moves);
    }

    #[test]
    fn test_generate_king_moves_pseudo_legal() {
        // Test king moves
        let board = ChessBoard::from_fen("8/8/8/8/8/3K4/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["d3c2", "d3c3", "d3c4", "d3d2", "d3d4", "d3e2", "d3e3", "d3e4"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d3"), expected_moves);

        // Test black king
        let board = ChessBoard::from_fen("8/8/8/8/8/3k4/8/8 b - - 0 1").unwrap();
        let expected_moves = vec!["d3c2", "d3c3", "d3c4", "d3d2", "d3d4", "d3e2", "d3e3", "d3e4"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d3"), expected_moves);

        // Test white king, blocked by own pieces and 3 capture
        let board = ChessBoard::from_fen("8/8/8/3ppp2/3PKP2/3PPP2/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["e4d5", "e4e5", "e4f5"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e4"), expected_moves);

        // Test white king, blocked by own pieces and 3 capture
        let board = ChessBoard::from_fen("8/8/8/3PPP2/3pkp2/3ppp2/8/8 b - - 0 1").unwrap();
        let expected_moves = vec!["e4d5", "e4e5", "e4f5"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e4"), expected_moves);

        // Test black king on h1
        let board = ChessBoard::from_fen("8/8/8/8/8/8/8/7k b - - 0 1").unwrap();
        let expected_moves = vec!["h1h2", "h1g1", "h1g2"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("h1"), expected_moves);

        // Test white king on a8
        let board = ChessBoard::from_fen("K7/8/8/8/8/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["a8a7", "a8b8", "a8b7"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("a8"), expected_moves);

        // Test white king starting position
        let board = ChessBoard::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e1"), vec![]);

        // Test black king starting position
        let board = ChessBoard::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e8"), vec![]);

        // Test white king queen side and king side castling
        let board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e1"),
            vec!["e1d1", "e1f1", "e1c1", "e1g1"],
        );

        // Test black king queen side and king side castling
        let board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e8"),
            vec!["e8d8", "e8f8", "e8c8", "e8g8"],
        );

        // Test white king king side castling
        let board = ChessBoard::from_fen("1r2k2r/pppppppp/8/8/8/8/PPPPPPPP/1R2K2R w Kk - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e1"),
            vec!["e1d1", "e1f1", "e1g1"],
        );

        // Test black king king side castling
        let board = ChessBoard::from_fen("1r2k2r/pppppppp/8/8/8/8/PPPPPPPP/1R2K2R b Kk - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e8"),
            vec!["e8d8", "e8f8", "e8g8"],
        );

        // Test white king queen side and king side castling
        let board = ChessBoard::from_fen("r3k1r1/pppppppp/8/8/8/8/PPPPPPPP/R3K1R1 w Qq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e1"),
            vec!["e1d1", "e1f1", "e1c1"],
        );

        // Test black king queen side and king side castling
        let board = ChessBoard::from_fen("r3k1r1/pppppppp/8/8/8/8/PPPPPPPP/R3K1R1 b Qq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e8"),
            vec!["e8d8", "e8f8", "e8c8"],
        );

        // Test white king castling blocked on d and f position
        let board = ChessBoard::from_fen("r2bkb1r/pppppppp/8/8/8/8/PPPPPPPP/R2BKB1R w KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e1"), vec![]);

        // Test black king castling blocked on d and f position
        let board = ChessBoard::from_fen("r2bkb1r/pppppppp/8/8/8/8/PPPPPPPP/R2BKB1R b KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e8"), vec![]);

        // Test white king castling blocked on c and g position
        let board = ChessBoard::from_fen("r1b1k1br/pppppppp/8/8/8/8/PPPPPPPP/R1B1K1BR w KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e1"), vec!["e1d1", "e1f1"]);

        // Test black king castling blocked on c and g position
        let board = ChessBoard::from_fen("r1b1k1br/pppppppp/8/8/8/8/PPPPPPPP/R1B1K1BR b KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e8"), vec!["e8d8", "e8f8"]);

        // Test white king castling blocked on b position
        let board = ChessBoard::from_fen("rb2k2r/pppppppp/8/8/8/8/PPPPPPPP/RB2K2R w KQkq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e1"),
            vec!["e1d1", "e1f1", "e1g1"],
        );

        // Test black king castling blocked on b position
        let board = ChessBoard::from_fen("rb2k2r/pppppppp/8/8/8/8/PPPPPPPP/RB2K2R b KQkq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e8"),
            vec!["e8d8", "e8f8", "e8g8"],
        );

        // Test black king castling blocked cause of a check
        let board = ChessBoard::from_fen("1r2k2r/ppppp1pp/8/8/8/8/PPPPP1PP/R4RK1 b k - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e8"),
            vec!["e8d8", "e8f7", "e8f8"],
        ); //f7,f8 are pseudo legal moves
    }

    #[test]
    fn test_make_move_set_en_passant_legal() {
        let mut board = ChessBoard::from_fen("8/4p3/8/3P4/8/8/8/8 b - - 0 1").unwrap();
        board.make_move(Move::from_algebraic("e7e5"));
        let expected_moves = vec!["d5d6", "d5e6"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d5"), expected_moves);

        let mut board = ChessBoard::from_fen("8/8/8/8/6p1/8/5P2/8 w - - 0 1").unwrap();
        board.make_move(Move::from_algebraic("f2f4"));
        let expected_moves = vec!["g4g3", "g4f3"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("g4"), expected_moves);

        let mut board = ChessBoard::from_fen("8/8/1p6/8/8/p7/PPP5/8 w - - 0 1").unwrap();
        board.make_move(Move::from_algebraic("b2b3"));
        let expected_moves = vec!["b6b5"];
        let moves = board.generate_pseudo_moves().into_iter().map(|m| m.0).collect();
        assert_moves(moves, expected_moves);

        let mut board =
            ChessBoard::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1pB1P3/2N2Q1p/PPPB1PPP/R3K2R b KQkq - 1 1").unwrap();
        board.make_move(Move::from_algebraic("c7c5"));
        let expected_moves = vec!["d5c6", "d5d6", "d5e6"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d5"), expected_moves.clone());
        let generated_moves: Vec<_> = board.generate_legal_moves().iter().map(|&m| m.as_algebraic()).collect();

        for mv in expected_moves {
            if !generated_moves.contains(&mv.to_string()) {
                println!("Move {} is not includes", mv);
            }
            assert!(generated_moves.contains(&mv.to_string()));
        }

        //Test en passant on c6
        let mut board =
            ChessBoard::from_fen("r3k2r/p2pqpb1/bn2pnp1/2pPN3/1pB1P3/2N2Q1p/PPPB1PPP/R3K2R w KQkq c6 0 2").unwrap();
        board.make_move(Move::from_algebraic("d5c6"));
        let field = ChessField::from_algebraic("c5");
        assert_eq!(board.squares[field.row][field.col], Square::Empty);
        let expected_moves = vec!["e7c5", "e7d6", "e7d8", "e7f8"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e7"), expected_moves.clone());

        let board = ChessBoard::from_fen("r2q1rk1/pP1p2pp/Q4n2/bb2p3/1pp5/1BN2NBn/pPPP1PPP/R3K2R b KQ - 1 2").unwrap();
        let expected_moves = vec!["b4c3"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("b4"), expected_moves.clone());
    }

    #[test]
    fn test_breaking() {
        let mut board = ChessBoard::from_fen("8/2p5/3p4/KP5r/1R3pPk/8/4P3/8 b - g3 0 1").unwrap();
        board.make_move(Move::from_algebraic("h4g3"));
        let expected_moves = vec!["g4g5", "g4h5"];
        let mut debug_moves: Vec<_> = board.generate_legal_moves().iter().map(|&m| m.as_algebraic()).collect();
        debug_moves.sort();
        println!("{:?}", debug_moves.len());
        println!("{:?}", debug_moves);
        assert_moves(board.generate_pseudo_moves_from_algebraic("g4"), expected_moves);
    }

    #[test]
    fn test_make_move() {
        let mut board = ChessBoard::from_fen("8/2p5/3p4/KP5r/1R3pPk/8/4P3/8 b - g3 0 1").unwrap();
        board.make_move(Move::from_algebraic("h4g3"));
        let expected_moves = vec!["g4g5", "g4h5"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("g4"), expected_moves);
    }

    #[test]
    fn test_make_move_promotion() {
        let mut board = ChessBoard::from_fen("8/2P5/1p6/8/8/p7/PP6/8 w - - 0 1").unwrap();
        board.make_move(Move::from_algebraic("c7c8Q"));
        assert_eq!(
            board.squares[7][2],
            Occupied(Piece {
                color: Color::White,
                kind: PieceType::Queen
            })
        );
    }

    #[test]
    fn test_make_move_capture_rook_invalidates_castling() {
        let mut board = ChessBoard::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/P7/1PP1NnPP/RNBQK2R b KQ - 0 8").unwrap();
        board.make_move(Move::from_algebraic("f2h1"));
        assert_eq!(board.castling_rights[0], false);
    }

    #[test]
    fn test_make_move_castling() {
        let mut board = ChessBoard::from_fen("rnbqk2r/ppp2pbp/3p1np1/4p3/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        board.make_move(Move::from_algebraic("e1g1"));
        assert_eq!(
            board.squares[0][6],
            Square::Occupied(Piece {
                color: Color::White,
                kind: PieceType::King
            })
        );
        assert_eq!(
            board.squares[0][5],
            Square::Occupied(Piece {
                color: Color::White,
                kind: PieceType::Rook
            })
        );
        assert_eq!(board.castling_rights[0], false);
        assert_eq!(board.castling_rights[1], false);
        assert_eq!(board.castling_rights[2], true);
        assert_eq!(board.castling_rights[3], true);
        assert_eq!(board.en_passant, None);

        let mut board = ChessBoard::from_fen("rnbqk2r/ppp2pbp/3p1np1/4p3/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        board.make_move(Move::from_algebraic("e1c1"));
        assert_eq!(
            board.squares[0][2],
            Square::Occupied(Piece {
                color: Color::White,
                kind: PieceType::King
            })
        );
        assert_eq!(
            board.squares[0][3],
            Square::Occupied(Piece {
                color: Color::White,
                kind: PieceType::Rook
            })
        );
        assert_eq!(board.castling_rights[0], false);
        assert_eq!(board.castling_rights[1], false);
        assert_eq!(board.castling_rights[2], true);
        assert_eq!(board.castling_rights[3], true);
        assert_eq!(board.en_passant, None);

        let mut board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();
        board.make_move(Move::from_algebraic("e8g8"));
        assert_eq!(
            board.squares[7][6],
            Square::Occupied(Piece {
                color: Color::Black,
                kind: PieceType::King
            })
        );
        assert_eq!(
            board.squares[7][5],
            Square::Occupied(Piece {
                color: Color::Black,
                kind: PieceType::Rook
            })
        );
        assert_eq!(board.castling_rights[0], true);
        assert_eq!(board.castling_rights[1], true);
        assert_eq!(board.castling_rights[2], false);
        assert_eq!(board.castling_rights[3], false);
        assert_eq!(board.en_passant, None);

        let mut board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();
        board.make_move(Move::from_algebraic("e8c8"));
        assert_eq!(
            board.squares[7][2],
            Square::Occupied(Piece {
                color: Color::Black,
                kind: PieceType::King
            })
        );
        assert_eq!(
            board.squares[7][3],
            Square::Occupied(Piece {
                color: Color::Black,
                kind: PieceType::Rook
            })
        );
        assert_eq!(board.castling_rights[0], true);
        assert_eq!(board.castling_rights[1], true);
        assert_eq!(board.castling_rights[2], false);
        assert_eq!(board.castling_rights[3], false);
        assert_eq!(board.en_passant, None);

        //white a rook moved
        let mut board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();
        board.make_move(Move::from_algebraic("a1b1"));
        assert_eq!(board.castling_rights[0], true);
        assert_eq!(board.castling_rights[1], false);
        assert_eq!(board.castling_rights[2], true);
        assert_eq!(board.castling_rights[3], true);
        assert_eq!(board.en_passant, None);

        //black a rook moved
        let mut board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();
        board.make_move(Move::from_algebraic("a8b8"));
        assert_eq!(board.castling_rights[0], true);
        assert_eq!(board.castling_rights[1], true);
        assert_eq!(board.castling_rights[2], true);
        assert_eq!(board.castling_rights[3], false);
        assert_eq!(board.en_passant, None);

        //white h rook moved
        let mut board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();
        board.make_move(Move::from_algebraic("h1g1"));
        assert_eq!(board.castling_rights[0], false);
        assert_eq!(board.castling_rights[1], true);
        assert_eq!(board.castling_rights[2], true);
        assert_eq!(board.castling_rights[3], true);
        assert_eq!(board.en_passant, None);
        board.make_move(Move::from_algebraic("a7a6"));
        board.make_move(Move::from_algebraic("e1c1"));
        assert_eq!(
            board.squares[0][2],
            Square::Occupied(Piece {
                color: Color::White,
                kind: PieceType::King
            })
        );
        assert_eq!(
            board.squares[0][3],
            Square::Occupied(Piece {
                color: Color::White,
                kind: PieceType::Rook
            })
        );

        //black h rook moved
        let mut board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();
        board.make_move(Move::from_algebraic("h8g8"));
        assert_eq!(board.castling_rights[0], true);
        assert_eq!(board.castling_rights[1], true);
        assert_eq!(board.castling_rights[2], false);
        assert_eq!(board.castling_rights[3], true);
        assert_eq!(board.en_passant, None);

        //white king moved
        let mut board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();
        board.make_move(Move::from_algebraic("e1d1"));
        assert_eq!(board.castling_rights[0], false);
        assert_eq!(board.castling_rights[1], false);
        assert_eq!(board.castling_rights[2], true);
        assert_eq!(board.castling_rights[3], true);
        assert_eq!(board.en_passant, None);

        //black king moved
        let mut board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();
        board.make_move(Move::from_algebraic("e8d8"));
        assert_eq!(board.castling_rights[0], true);
        assert_eq!(board.castling_rights[1], true);
        assert_eq!(board.castling_rights[2], false);
        assert_eq!(board.castling_rights[3], false);
        assert_eq!(board.en_passant, None);
    }

    #[test]
    fn test_if_field_is_attacked() {
        let board = ChessBoard::from_fen("8/2P5/8/8/8/8/3p4/8 w - - 0 1").unwrap();
        assert_eq!(board.is_square_attacked(0, 2), true);
        assert_eq!(board.is_square_attacked(0, 3), false);
        assert_eq!(board.is_square_attacked(0, 4), true);

        //test attack of White Pawn
        assert_eq!(board.is_square_attacked(7, 1), false);
        assert_eq!(board.is_square_attacked(7, 2), false);
        assert_eq!(board.is_square_attacked(7, 3), false);
        assert_eq!(board.is_square_attacked_by_color(7, 1, Color::White), true);
        assert_eq!(board.is_square_attacked_by_color(7, 2, Color::White), false);
        assert_eq!(board.is_square_attacked_by_color(7, 3, Color::White), true);
    }

    #[test]
    fn test_pinned_piece() {
        let board = ChessBoard::from_fen("1k6/8/8/8/3q4/8/1R6/K7 w - - 0 1").unwrap();
        assert_moves(board.generate_legal_moves(), vec!["a1a2", "a1b1"])
    }

    #[test]
    fn test_checkmate() {
        let board = ChessBoard::from_fen("1k6/8/8/8/8/8/PPn5/KN6 w - - 0 1").unwrap();
        assert_eq!(board.is_checkmate(), true);

        //stalemate
        let board = ChessBoard::from_fen("1k6/8/8/8/8/1r6/7r/K7 w - - 0 1").unwrap();
        assert_eq!(board.is_checkmate(), false);
    }

    #[test]
    fn test_stalemate() {
        let board = ChessBoard::from_fen("1k6/8/8/8/8/1r6/7r/K7 w - - 0 1").unwrap();
        assert_eq!(board.is_stalemate(), true);

        //checkmate
        let board = ChessBoard::from_fen("1k6/8/8/8/8/8/PPn5/KN6 w - - 0 1").unwrap();
        assert_eq!(board.is_stalemate(), false);
    }

    #[test]
    fn test_three_fold_repetition() {
        let mut board =
            ChessBoard::from_fen("1rb2rk1/p4ppp/1p1qp1n1/3n2N1/2pP4/2P3P1/PPQ2PBP/R1B1R1K1 w - - 4 17").unwrap();

        board.make_move(Move::from_algebraic("e1e2"));
        board.make_move(Move::from_algebraic("g8h8"));
        board.make_move(Move::from_algebraic("e2e1"));
        board.make_move(Move::from_algebraic("h8g8"));
        assert_eq!(board.is_threefold_repetition(), false);
        board.make_move(Move::from_algebraic("e1e2"));
        board.make_move(Move::from_algebraic("g8h8"));
        board.make_move(Move::from_algebraic("e2e1"));
        board.make_move(Move::from_algebraic("h8g8"));
        assert_eq!(board.is_threefold_repetition(), true);
    }

    #[test]
    fn test_perft() {
        let board = ChessBoard::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(perft(&board, 3), 8902u64);
        assert_eq!(perft(&board, 4), 197281u64);
        assert_eq!(perft(&board, 5), 4865609u64);
        //assert_eq!(perft(&board, 6), 119060324u64);
    }

    #[test]
    fn test_perft2() {
        let board =
            ChessBoard::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
        assert_eq!(perft(&board, 1), 48);
        assert_eq!(perft(&board, 2), 2039);
        assert_eq!(perft(&board, 3), 97862);
        assert_eq!(perft(&board, 4), 4085603);
        //assert_eq!(perft(&board, 5), 193690690);
    }

    #[test]
    fn test_perft3() {
        let board = ChessBoard::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
        assert_eq!(perft(&board, 1,), 14);
        assert_eq!(perft(&board, 2), 191);
        assert_eq!(perft(&board, 3), 2812);
        assert_eq!(perft(&board, 4), 43238);
        assert_eq!(perft(&board, 5), 674624);
        assert_eq!(perft(&board, 6), 11030083);
    }

    #[test]
    fn test_perft4w() {
        let board = ChessBoard::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1").unwrap();
        assert_eq!(perft(&board, 1), 6);
        assert_eq!(perft(&board, 2), 264);
        assert_eq!(perft(&board, 3), 9467);
        assert_eq!(perft(&board, 4), 422333);
        assert_eq!(perft(&board, 5), 15833292);
    }

    #[test]
    fn test_perft4b() {
        let board = ChessBoard::from_fen("r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1").unwrap();
        assert_eq!(perft(&board, 1), 6);
        assert_eq!(perft(&board, 2), 264);
        assert_eq!(perft(&board, 3), 9467);
        assert_eq!(perft(&board, 4), 422333);
        assert_eq!(perft(&board, 5), 15833292);
    }

    #[test]
    fn test_perft_pos5() {
        let board = ChessBoard::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap();
        assert_eq!(perft(&board, 1), 44u64);
        assert_eq!(perft(&board, 2), 1486u64);
        assert_eq!(perft(&board, 3), 62379u64);
        assert_eq!(perft(&board, 4), 2103487u64);
        //assert_eq!(perft(&board, 5), 89941194u64);
    }

    #[test]
    fn test_perft_pos6() {
        let board =
            ChessBoard::from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10").unwrap();
        assert_eq!(perft(&board, 1), 46u64);
        assert_eq!(perft(&board, 2), 2079u64);
        assert_eq!(perft(&board, 3), 89890u64);
        assert_eq!(perft(&board, 4), 3894594u64);
        //assert_eq!(perft(&board, 5), 164075551u64);
    }

    #[test]
    fn test_perft_pos_cf() {
        let board = ChessBoard::from_fen("r3k2r/1pb2N2/2P5/3N3b/P2n4/1qB2pp1/5np1/R1Q1K2R w KQkq - 0 1").unwrap();
        assert_eq!(perft(&board, 1), 40);
        assert_eq!(perft(&board, 2), 2143);
        assert_eq!(perft(&board, 3), 75353);
        assert_eq!(perft(&board, 4), 3958794);
        //assert_eq!(perft(&board, 5), 140774393);
    }

    #[test]
    fn test_perft_pos_web() {
        //https://github.com/elcabesa/vajolet/blob/master/tests/perft.txt
        let board =
            ChessBoard::from_fen("rnbqkbnr/1p4p1/3pp2p/p1p2p2/7P/2PP1P1N/PP1NP1P1/R1BQKB1R b Qkq - 0 1").unwrap();
        assert_eq!(perft(&board, 1), 30);
        assert_eq!(perft(&board, 2), 784);
        assert_eq!(perft(&board, 3), 23151);
        assert_eq!(perft(&board, 4), 638663);
        //assert_eq!(perft(&board, 5), 19171633);
    }

    #[test]
    fn test_perft_pos_web2() {
        //http://www.rocechess.ch/perft.html
        let board = ChessBoard::from_fen("n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1").unwrap();
        assert_eq!(perft(&board, 1), 24);
        assert_eq!(perft(&board, 2), 496);
        assert_eq!(perft(&board, 3), 9483);
        assert_eq!(perft(&board, 4), 182838);
        assert_eq!(perft(&board, 5), 3605103);
        //assert_eq!(perft(&board, 6), 71179139);
    }
}
