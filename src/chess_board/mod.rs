pub mod fen;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct ChessField {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Move {
    pub from: ChessField,
    pub to: ChessField,
}

impl ChessField {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}
impl Move {
    // Create a new Move
    pub fn new(from_row: usize, from_col: usize, to_row: usize, to_col: usize) -> Self {
        Self {
            from: ChessField::new(from_row, from_col),
            to: ChessField::new(to_row, to_col),
        }
    }

    pub fn as_algebraic(&self) -> String {
        format!(
            "{}{}",
            to_algebraic_square(self.from.row, self.from.col),
            to_algebraic_square(self.to.row, self.to.col)
        )
    }
}

fn to_algebraic_square(row: usize, col: usize) -> String {
    let file = (b'a' + col as u8) as char; // Convert 0-7 column index to 'a'-'h'
    let rank = (row + 1).to_string(); // Convert 0-7 row index to '8'-'1'
    format!("{}{}", file, rank) // Combine file and rank into a string
}

pub struct ChessBoard {
    pub squares: [[Square; 8]; 8],
    pub active_color: Color,
    pub castling_rights: [bool; 4],
    pub en_passant: Option<ChessField>,
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

    pub fn generate_pseudo_moves(&self) -> Vec<Move> {
        let mut all_moves = Vec::new();

        for row in 0..8 {
            for col in 0..8 {
                // Only process pieces of the active color
                all_moves.extend(self.generate_pseudo_moves_from_position(row, col));
            }
        }
        all_moves
    }

    pub fn generate_pseudo_moves_from_position(&self, row: usize, col: usize) -> Vec<Move> {
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

    fn generate_pawn_moves(&self, row: usize, col: usize) -> Vec<Move> {
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
            if new_row == promotion_row {
                // Add all possible promotions
                moves.push(Move::new(row, col, new_row, col)); // Promotion
            } else {
                moves.push(Move::new(row, col, new_row, col));
            }

            // Double move from start position
            if row == start_row {
                let two_forward = (row as isize + 2 * forward) as usize;
                if self.squares[two_forward][col] == Square::Empty {
                    moves.push(Move::new(row, col, two_forward, col));
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
                    moves.push(Move::new(row, col, new_row, new_col));
                }
            }
        }

        // En passant
        if let Some(en_passant) = self.en_passant {
            if new_row == en_passant.row && (col as isize - en_passant.col as isize).abs() == 1 {
                moves.push(Move::new(row, col, en_passant.row, en_passant.col));
            }
        }

        moves
    }

    /// Generate knight moves.
    fn generate_knight_moves(&self, row: usize, col: usize) -> Vec<Move> {
        const KNIGHT_MOVES: [(isize, isize); 8] =
            [(-2, -1), (-1, -2), (1, -2), (2, -1), (2, 1), (1, 2), (-1, 2), (-2, 1)];

        self.generate_moves_from_directions(row, col, &KNIGHT_MOVES)
    }

    /// Generate sliding piece moves (bishop, rook, queen).
    fn generate_sliding_moves(&self, row: usize, col: usize, directions: &[(isize, isize)]) -> Vec<Move> {
        let mut moves: Vec<Move> = Vec::new();

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
                    Square::Empty => moves.push(Move::new(row, col, new_row as usize, new_col as usize)),
                    Square::Occupied(p) => {
                        if p.color != self.active_color {
                            moves.push(Move::new(row, col, new_row as usize, new_col as usize));
                        }
                        break; // Block sliding
                    }
                }
            }
        }

        moves
    }

    /// Generate bishop moves.
    fn generate_bishop_moves(&self, row: usize, col: usize) -> Vec<Move> {
        const BISHOP_DIRECTIONS: [(isize, isize); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
        self.generate_sliding_moves(row, col, &BISHOP_DIRECTIONS)
    }

    /// Generate rook moves.
    fn generate_rook_moves(&self, row: usize, col: usize) -> Vec<Move> {
        const ROOK_DIRECTIONS: [(isize, isize); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
        self.generate_sliding_moves(row, col, &ROOK_DIRECTIONS)
    }

    /// Generate queen moves.
    fn generate_queen_moves(&self, row: usize, col: usize) -> Vec<Move> {
        const QUEEN_DIRECTIONS: [(isize, isize); 8] =
            [(-1, -1), (-1, 1), (1, -1), (1, 1), (0, -1), (0, 1), (-1, 0), (1, 0)];
        self.generate_sliding_moves(row, col, &QUEEN_DIRECTIONS)
    }

    /// Generate king moves (including castling).
    fn generate_king_moves(&self, row: usize, col: usize) -> Vec<Move> {
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
                moves.push(Move::new(row, 4, row, 6)); // Move King: e1->g1 or e8->g8
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
                moves.push(Move::new(row, 4, row, 2)); // Move King: e1->c1 or e8->c8
            }
        }
        moves
    }

    pub fn make_move(&mut self, mv: Move) {
        let piece = self.squares[mv.from.row][mv.from.col];
        self.squares[mv.from.row][mv.from.col] = Square::Empty;
        self.squares[mv.to.row][mv.to.col] = piece;

        // Switch the active color after a move
        self.active_color = match self.active_color {
            Color::White => Color::Black,
            Color::Black => Color::White,
        };
    }

    fn is_square_attacked(&self, row: usize, col: usize) -> bool {
        let opponent_color = match self.active_color {
            Color::White => Color::Black,
            Color::Black => Color::White,
        };

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

        let pawn_attacks = match self.active_color {
            Color::White => [(1, -1), (1, 1)],   // Black pawns attack "downward"
            Color::Black => [(-1, -1), (-1, 1)], // White pawns attack "upward"
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

    fn generate_moves_from_directions(&self, row: usize, col: usize, directions: &[(isize, isize)]) -> Vec<Move> {
        let mut moves = Vec::new();

        for &(dx, dy) in directions {
            let new_row = (row as isize + dx) as usize;
            let new_col = (col as isize + dy) as usize;

            if new_row < 8
                && new_col < 8
                && (self.squares[new_row][new_col] == Square::Empty
                    || matches!(self.squares[new_row][new_col], Square::Occupied(p) if p.color != self.active_color))
            {
                moves.push(Move::new(row, col, new_row, new_col));
            }
        }
        moves
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    impl ChessBoard {
        /// Creates an empty chess board
        pub fn generate_pseudo_moves_from_chess_field(&self, pos: ChessField) -> Vec<Move> {
            self.generate_pseudo_moves_from_position(pos.row, pos.col)
        }

        pub fn generate_pseudo_moves_from_algebraic(&self, square: &str) -> Vec<Move> {
            self.generate_pseudo_moves_from_chess_field(ChessField::from_algebraic(square))
        }
    }

    impl ChessField {
        pub fn as_algebraic(&self) -> String {
            to_algebraic_square(self.row, self.col)
        }
        pub fn from_algebraic(algebraic: &str) -> Self {
            let (row, col) = from_algebraic_square(algebraic);
            Self { row, col }
        }
    }
    impl Move {
        pub fn from_algebraic(algebraic: &str) -> Self {
            let from = from_algebraic_square(&algebraic[0..2]);
            let to = from_algebraic_square(&algebraic[2..4]);
            Self {
                from: ChessField::new(from.0, from.1),
                to: ChessField::new(to.0, to.1),
            }
        }
    }

    fn from_algebraic_square(square: &str) -> (usize, usize) {
        let file = square.chars().next().unwrap();
        let rank = square.chars().nth(1).unwrap();
        let col = (file as u8 - b'a') as usize;
        let row = (rank as u8 - b'1') as usize;
        (row, col)
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
}
