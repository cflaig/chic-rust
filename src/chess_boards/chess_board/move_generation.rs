use super::Square::Occupied;
use super::{ChessBoard, Color, Move, Piece, PieceType, Square};
use std::collections::BinaryHeap;

const NO_CAPTURE: i32 = 0;
const CAPTURE: i32 = 10000;
const CAPTURE_BASE: i32 = CAPTURE + 10;
const CASTLING_SCORE: i32 = 50;
const BEST_MOVE: i32 = 1_000_000;

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
    pub fn generate_pseudo_moves(&self) -> Vec<(i32, Move)> {
        let mut all_moves: Vec<(i32, Move)> = Vec::with_capacity(128);

        for (field, piece) in self.pieces_with_coordinates() {
                all_moves.extend(self.generate_pseudo_moves_from_position(field.row, field.col));
            }

        all_moves
    }

    pub fn generate_pseudo_moves_from_position(&self, row: u8, col: u8) -> Vec<(i32, Move)> {
        if let Square::Occupied(piece) = self.squares[row as usize][col as usize] {
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

    fn generate_pawn_moves(&self, row: u8, col: u8) -> Vec<(i32, Move)> {
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
        if self.squares[new_row][col as usize] == Square::Empty {
            let mv = Move::new(row, col, new_row as u8, col);
            Self::add_pawn_moves_with_and_without_promotion(mv, promotion_row, NO_CAPTURE, &mut moves);

            // Double move from start position
            if row == start_row {
                let two_forward = (row as isize + 2 * forward) as usize;
                if self.squares[two_forward][col as usize] == Square::Empty {
                    moves.push((NO_CAPTURE, Move::new(row, col, two_forward as u8, col)));
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
                    let mv = Move::new(row, col, new_row as u8, new_col as u8);
                    Self::add_pawn_moves_with_and_without_promotion(
                        mv,
                        promotion_row,
                        self.compute_capture_score(&mv),
                        &mut moves,
                    );
                }
            }
        }

        // En passant
        if let Some(en_passant) = self.en_passant {
            if new_row == en_passant.row as usize && (col as isize - en_passant.col as isize).abs() == 1 {
                let pawn = Piece {
                    color: Color::White,
                    kind: PieceType::Pawn,
                };
                let mv = Move::new(row, col, en_passant.row, en_passant.col);
                moves.push((self.compute_capture_score(&mv), mv));
            }
        }

        moves
    }

    fn add_pawn_moves_with_and_without_promotion(
        mv: Move,
        promotion_row: u8,
        score: i32,
        moves: &mut Vec<(i32, Move)>,
    ) {
        if mv.to.row == promotion_row {
            for &promotion_piece in &[PieceType::Queen, PieceType::Rook, PieceType::Bishop, PieceType::Knight] {
                moves.push((score + 1, mv.with_promotion(promotion_piece)));
            }
        } else {
            moves.push((score, mv));
        }
    }

    /// Generate knight moves.
    fn generate_knight_moves(&self, row: u8, col: u8) -> Vec<(i32, Move)> {
        const KNIGHT_MOVES: [(isize, isize); 8] =
            [(-2, -1), (-1, -2), (1, -2), (2, -1), (2, 1), (1, 2), (-1, 2), (-2, 1)];

        self.generate_moves_from_directions(row, col, &KNIGHT_MOVES)
    }

    /// Generate sliding piece moves (bishop, rook, queen).
    fn generate_sliding_moves(&self, row: u8, col: u8, directions: &[(isize, isize)]) -> Vec<(i32, Move)> {
        let mut moves: Vec<(i32, Move)> = Vec::new();

        let moving_piece = match self.squares[row as usize][col as usize] {
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
                    Square::Empty => moves.push((NO_CAPTURE, Move::new(row, col, new_row as u8, new_col as u8))),
                    Square::Occupied(p) => {
                        if p.color != self.active_color {
                            let mv = Move::new(row, col, new_row as u8, new_col as u8);
                            moves.push((self.compute_capture_score(&mv), mv));
                        }
                        break; // Block sliding
                    }
                }
            }
        }

        moves
    }

    /// Generate bishop moves.
    fn generate_bishop_moves(&self, row: u8, col: u8) -> Vec<(i32, Move)> {
        const BISHOP_DIRECTIONS: [(isize, isize); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
        self.generate_sliding_moves(row, col, &BISHOP_DIRECTIONS)
    }

    /// Generate rook moves.
    fn generate_rook_moves(&self, row: u8, col: u8) -> Vec<(i32, Move)> {
        const ROOK_DIRECTIONS: [(isize, isize); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
        self.generate_sliding_moves(row, col, &ROOK_DIRECTIONS)
    }

    /// Generate queen moves.
    fn generate_queen_moves(&self, row: u8, col: u8) -> Vec<(i32, Move)> {
        const QUEEN_DIRECTIONS: [(isize, isize); 8] =
            [(-1, -1), (-1, 1), (1, -1), (1, 1), (0, -1), (0, 1), (-1, 0), (1, 0)];
        self.generate_sliding_moves(row, col, &QUEEN_DIRECTIONS)
    }

    /// Generate king moves (including castling).
    fn generate_king_moves(&self, row: u8, col: u8) -> Vec<(i32, Move)> {
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
                && self.squares[row as usize][5] == Square::Empty
                && self.squares[row as usize][6] == Square::Empty
                && !self.is_square_attacked(row, 4)
                && !self.is_square_attacked(row, 5)
                && !self.is_square_attacked(row, 6)
            {
                moves.push((CASTLING_SCORE, Move::new(row, 4, row, 6))); // Move King: e1->g1 or e8->g8
            }

            // Queenside castling
            if self.castling_rights[if self.active_color == Color::White { 1 } else { 3 }]
                && self.squares[row as usize][3] == Square::Empty
                && self.squares[row as usize][2] == Square::Empty
                && self.squares[row as usize][1] == Square::Empty
                && !self.is_square_attacked(row, 4)
                && !self.is_square_attacked(row, 3)
                && !self.is_square_attacked(row, 2)
            {
                moves.push((CASTLING_SCORE, Move::new(row, 4, row, 2))); // Move King: e1->c1 or e8->c8
            }
        }
        moves
    }

    fn generate_moves_from_directions(
        &self,
        row: u8,
        col: u8,
        directions: &[(isize, isize)],
    ) -> Vec<(i32, Move)> {
        let mut moves = Vec::new();

        let moving_piece = match self.squares[row as usize][col as usize] {
            Square::Occupied(p) => p,
            _ => return moves,
        };

        for &(dx, dy) in directions {
            let new_row = (row as isize + dx) as usize;
            let new_col = (col as isize + dy) as usize;

            if new_row < 8 && new_col < 8 {
                match self.squares[new_row][new_col] {
                    Square::Empty => moves.push((NO_CAPTURE, Move::new(row, col, new_row as u8, new_col as u8))),
                    Square::Occupied(p) => {
                        if p.color != self.active_color {
                            let mv = Move::new(row, col, new_row as u8, new_col as u8);
                            moves.push((self.compute_capture_score(&mv), mv));
                        }
                    }
                }
            }
        }
        moves
    }

    fn compute_capture_score(&self, mv: &Move) -> i32 {
        if let Occupied(moving_piece) = self.squares[mv.from.row as usize][mv.from.col as usize] {
            match self.squares[mv.to.row as usize][mv.to.col as usize] {
                Square::Empty => NO_CAPTURE,
                Square::Occupied(captured_piece) => {
                    if mv.to == self.last_capture {
                        //CAPTURE * 2 + 1000 * (get_piece_value(&captured_piece.kind) - get_piece_value(&moving_piece.kind)) + 10 * get_piece_value(&captured_piece.kind) - get_piece_value(&captured_piece.kind)
                        //CAPTURE + 100 * get_piece_value(&captured_piece.kind) - get_piece_value(&moving_piece.kind)

                        CAPTURE_BASE
                            + 1000 * (get_piece_value(&captured_piece.kind) - get_piece_value(&moving_piece.kind))
                            + 10 * get_piece_value(&captured_piece.kind)
                            - get_piece_value(&captured_piece.kind)
                    } else {
                        //CAPTURE + 100 * get_piece_value(&captured_piece.kind) - get_piece_value(&moving_piece.kind)

                        CAPTURE_BASE
                            + 1000 * (get_piece_value(&captured_piece.kind) - get_piece_value(&moving_piece.kind))
                            + 10 * get_piece_value(&captured_piece.kind)
                            - get_piece_value(&captured_piece.kind)
                    }
                }
            }
        } else {
            NO_CAPTURE
        }
    }

    pub fn generate_legal_moves(&self, guess_of_best_move: Option<Move>) -> Vec<Move> {
        let mut legal_moves = Vec::new();

        // Generate all pseudo-legal moves
        let pseudo_moves = self.generate_pseudo_moves();

        // For each pseudo-legal move, check if it leaves the king in check
        for mv in pseudo_moves {
            let mut board_clone = self.clone();
            board_clone.make_move(mv.1);

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
        Self::compute_move_weights(&mut legal_moves, guess_of_best_move);

        //legal_moves.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        legal_moves.sort_unstable_by(|a, b| b.cmp(a));
        legal_moves.iter().map(|m| m.1).collect()

        //LazySortedMoves::from(legal_moves)
    }

    fn compute_move_weights(moves: &mut Vec<(i32, Move)>, guess_of_best_move: Option<Move>) {
        moves.iter_mut().for_each(|mut mv| {
            if let Some(guess) = guess_of_best_move {
                if mv.1 == guess {
                    mv.0 = BEST_MOVE;
                }
            }
        });
    }

    pub fn generate_capture_moves(&self) -> Vec<Move> {
        let mut capture_moves = Vec::new();

        for (field, piece) in self.pieces_with_coordinates() {

                // Only process pieces of the active color
                if let Square::Occupied(piece) = self.squares[field.row as usize][field.col as usize] {
                    if piece.color == self.active_color {
                        let piece_moves = self.generate_pseudo_moves_from_position(field.row, field.col);

                        // Filter for capture moves
                        for mv in piece_moves {
                            if let Square::Occupied(target_piece) = self.squares[mv.1.to.row as usize][mv.1.to.col as usize] {
                                if target_piece.color != self.active_color {
                                    capture_moves.push(mv);
                                }
                            }
                        }
                    }
                }
        }

        //LazySortedMoves::from(capture_moves)

        capture_moves.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        //capture_moves.sort_unstable();
        capture_moves.iter().map(|m| m.1).collect()
    }

    pub fn generate_legal_capture_moves(&self) -> Vec<Move> {
        let mut legal_moves = Vec::new();

        for mv in self.generate_capture_moves() {
            let mut board_clone = self.clone(); // Clone the board to simulate the move
            board_clone.make_move(mv); // Make the move on the cloned board

            // Locate the king of the current player
            let king_position = board_clone.find_king_position(self.active_color);

            // Check if the king is under attack after the move
            if let Some(king_pos) = king_position {
                if !board_clone.is_square_attacked_by_color(king_pos.row, king_pos.col, board_clone.active_color) {
                    legal_moves.push(mv);
                }
            }
        }
        legal_moves
    }
}

pub struct LazySortedMoves<I> {
    heap: BinaryHeap<(i32, I)>, // Max-heap based on score (default behavior)
}

impl<I: std::cmp::Ord> LazySortedMoves<I> {
    pub fn from(moves: Vec<(i32, I)>) -> Self {
        let heap: BinaryHeap<(i32, I)> = BinaryHeap::from(moves);
        Self { heap }
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }
}

impl<I> Iterator for LazySortedMoves<I>
where
    I: Ord,
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        self.heap.pop().map(|(_, item)| item)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::*;
    use super::super::ChessField;
    use super::*;

    impl ChessBoard {}

    #[test]
    fn test_generate_pawn_moves_pseudo_legal() {
        // Test simple pawn moves. Pawn at e4 can move forward to e5
        let board = ChessBoard::from_fen("8/8/8/8/4P3/8/8/8 w - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e4").into_iter(), vec!["e4e5"]);

        // Test blocked pawn a3 by a4
        let board = ChessBoard::from_fen("8/8/8/8/P7/P7/8/8 w - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a3").into_iter(), vec![]);

        // Test en passant
        let board = ChessBoard::from_fen("8/8/3p4/4Pp2/8/8/8/8 w - f6 0 1").unwrap();

        // White pawn at e5 can capture en passant at f6 and capture at d6 and make a move to 46
        let expected_moves = vec!["e5d6", "e5e6", "e5f6"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e5").into_iter(), expected_moves);

        // Move pawn on b2 and black a3 and c3
        let board = ChessBoard::from_fen("8/8/8/8/8/p1p5/1P6/8 w - - 0 1").unwrap();
        let expected_moves = vec!["b2b3", "b2b4", "b2a3", "b2c3"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("b2").into_iter(), expected_moves);

        // Move black pawn on a6 to a5
        let board = ChessBoard::from_fen("8/8/p7/8/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a6").into_iter(), vec!["a6a5"]);

        // Test blocked blacked pawn a6 by a5
        let board = ChessBoard::from_fen("8/8/p7/p7/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a6").into_iter(), vec![]);

        // Test single and double step of blacked pawn a7
        let board = ChessBoard::from_fen("8/p7/8/8/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a7").into_iter(), vec!["a7a6", "a7a5"]);

        // Test single move of blacked pawn a7 and double step is blocked
        let board = ChessBoard::from_fen("8/p7/8/p7/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a7").into_iter(), vec!["a7a6"]);

        // Test pawn move on a7. Capture on b6 is not allowed by same color
        let board = ChessBoard::from_fen("8/p7/1p6/8/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a7").into_iter(), vec!["a7a6", "a7a5"]);

        // Test pawn move on a7 with capture on b6
        let board = ChessBoard::from_fen("8/p7/1P6/8/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("a7").into_iter(),
            vec!["a7a6", "a7a5", "a7b6"],
        );

        // Test pawn move on b7 with capture on a6 and c6
        let board = ChessBoard::from_fen("8/1p6/P1P5/8/8/8/8/8 b - - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("b7").into_iter(),
            vec!["b7b6", "b7b5", "b7a6", "b7c6"],
        );

        // Test white promotion
        let board = ChessBoard::from_fen("8/6P1/8/8/8/8/8/8 w - - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("g7").into_iter(),
            vec!["g7g8q", "g7g8r", "g7g8b", "g7g8n"],
        );

        // Test black promotion
        let board = ChessBoard::from_fen("3r4/2P5/8/8/8/8/8/8 w - - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("c7").into_iter(),
            vec!["c7c8b", "c7c8n", "c7c8r", "c7c8q", "c7d8b", "c7d8n", "c7d8r", "c7d8q"],
        );

        // Test black promotion
        let board = ChessBoard::from_fen("4k1nr/2p3p1/b2pPp1p/8/1nN1P1P1/5N2/Pp3P2/2R2K2 b k - 1 27").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("b2").into_iter(),
            vec!["b2b1b", "b2b1n", "b2b1q", "b2b1r", "b2c1b", "b2c1n", "b2c1r", "b2c1q"],
        );
    }

    #[test]
    fn test_generate_knight_moves_pseudo_legal() {
        // White knight at d4 can move to 8 possible squares
        let board = ChessBoard::from_fen("8/8/8/8/3N4/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["d4b3", "d4c2", "d4e2", "d4f3", "d4f5", "d4e6", "d4c6", "d4b5"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d4").into_iter(), expected_moves);

        // Black knight at d4 can move to 8 possible squares incl. one capture
        let board = ChessBoard::from_fen("8/8/8/5N2/3n4/8/8/8 b - - 0 1").unwrap();
        let expected_moves = vec!["d4b3", "d4c2", "d4e2", "d4f3", "d4f5", "d4e6", "d4c6", "d4b5"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d4").into_iter(), expected_moves);

        // White knight at a3 with blocked fields
        let board = ChessBoard::from_fen("8/8/8/1rn5/2r5/N7/2B5/1Q6 w - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a3").into_iter(), vec!["a3c4", "a3b5"]);

        // Black knight knight at a3 with blocked fields
        let board = ChessBoard::from_fen("8/8/8/1RN5/2R5/n7/2b5/1q6 b - - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("a3").into_iter(), vec!["a3c4", "a3b5"]);
    }

    #[test]
    fn test_generate_bishop_moves_pseudo_legal() {
        // Test bishop moves with 2 diagonals
        let board = ChessBoard::from_fen("8/8/8/8/3B4/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec![
            "d4a7", "d4b6", "d4c5", "d4e3", "d4f2", "d4g1", //first diagonal
            "d4a1", "d4b2", "d4c3", "d4e5", "d4f6", "d4g7", "d4h8",
        ];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d4").into_iter(), expected_moves);

        // Test bishop with a capture and a blocked square
        let board = ChessBoard::from_fen("8/6r1/5B2/8/3P4/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["f6d8", "f6e7", "f6g5", "f6h4", "f6e5", "f6g7"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("f6").into_iter(), expected_moves);

        // Test black bishop moves with 2 diagonals
        let board = ChessBoard::from_fen("8/8/8/8/8/3b4/8/8 b - - 0 1").unwrap();
        let expected_moves = vec![
            "d3a6", "d3b5", "d3c4", "d3e2", "d3f1", //first diagonal
            "d3b1", "d3c2", "d3e4", "d3f5", "d3g6", "d3h7",
        ];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d3").into_iter(), expected_moves);

        // Test black bishop with a capture and a blocked square
        let board = ChessBoard::from_fen("8/6R1/5b2/8/3p4/8/8/8 b - - 0 1").unwrap();
        let expected_moves = vec!["f6d8", "f6e7", "f6g5", "f6h4", "f6e5", "f6g7"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("f6").into_iter(), expected_moves);
    }

    #[test]
    fn test_generate_rook_moves_pseudo_legal() {
        // Test rook moves
        let board = ChessBoard::from_fen("8/8/8/8/3R4/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec![
            "d4d1", "d4d2", "d4d3", "d4d5", "d4d6", "d4d7", "d4d8", "d4a4", "d4b4", "d4c4", "d4e4", "d4f4", "d4g4",
            "d4h4",
        ];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d4").into_iter(), expected_moves);

        // Test black rook with a capture and blocked squares
        let board = ChessBoard::from_fen("8/8/8/8/3bR3/8/4N3/8 w - - 0 1").unwrap();
        let expected_moves = vec!["e4e3", "e4e5", "e4e6", "e4e7", "e4e8", "e4d4", "e4f4", "e4g4", "e4h4"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e4").into_iter(), expected_moves);

        // Test black rook moves
        let board = ChessBoard::from_fen("8/8/8/8/8/3r4/8/8 b - - 0 1").unwrap();
        let expected_moves = vec![
            "d3d1", "d3d2", "d3d4", "d3d5", "d3d6", "d3d7", "d3d8", "d3a3", "d3b3", "d3c3", "d3e3", "d3f3", "d3g3",
            "d3h3",
        ];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d3").into_iter(), expected_moves);

        // Test black rook with a capture and blocked squares
        let board = ChessBoard::from_fen("8/8/8/8/3Br3/8/4n3/8 b - - 0 1").unwrap();
        let expected_moves = vec!["e4e3", "e4e5", "e4e6", "e4e7", "e4e8", "e4d4", "e4f4", "e4g4", "e4h4"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e4").into_iter(), expected_moves);
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
        assert_moves(board.generate_pseudo_moves_from_algebraic("d4").into_iter(), expected_moves);

        // Test queen move from g6 with 3 capture and a blocked square
        let board = ChessBoard::from_fen("4b1b1/6b1/4r1Q1/5P2/6B1/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["g6e8", "g6f7", "g6e6", "g6f6", "g6g7", "g6g5", "g6h5", "g6h6", "g6h7"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("g6").into_iter(), expected_moves);

        // Test queen move from a5 with 2 capture and a blocked square
        let board = ChessBoard::from_fen("8/b7/1b6/qb6/1P6/P7/8/8 b - - 0 1").unwrap();
        let expected_moves = vec!["a5a6", "a5a4", "a5a3", "a5b4"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("a5").into_iter(), expected_moves);
    }

    #[test]
    fn test_generate_king_moves_pseudo_legal() {
        // Test king moves
        let board = ChessBoard::from_fen("8/8/8/8/8/3K4/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["d3c2", "d3c3", "d3c4", "d3d2", "d3d4", "d3e2", "d3e3", "d3e4"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d3").into_iter(), expected_moves);

        // Test black king
        let board = ChessBoard::from_fen("8/8/8/8/8/3k4/8/8 b - - 0 1").unwrap();
        let expected_moves = vec!["d3c2", "d3c3", "d3c4", "d3d2", "d3d4", "d3e2", "d3e3", "d3e4"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d3").into_iter(), expected_moves);

        // Test white king, blocked by own pieces and 3 capture
        let board = ChessBoard::from_fen("8/8/8/3ppp2/3PKP2/3PPP2/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["e4d5", "e4e5", "e4f5"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e4").into_iter(), expected_moves);

        // Test white king, blocked by own pieces and 3 capture
        let board = ChessBoard::from_fen("8/8/8/3PPP2/3pkp2/3ppp2/8/8 b - - 0 1").unwrap();
        let expected_moves = vec!["e4d5", "e4e5", "e4f5"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e4").into_iter(), expected_moves);

        // Test black king on h1
        let board = ChessBoard::from_fen("8/8/8/8/8/8/8/7k b - - 0 1").unwrap();
        let expected_moves = vec!["h1h2", "h1g1", "h1g2"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("h1").into_iter(), expected_moves);

        // Test white king on a8
        let board = ChessBoard::from_fen("K7/8/8/8/8/8/8/8 w - - 0 1").unwrap();
        let expected_moves = vec!["a8a7", "a8b8", "a8b7"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("a8").into_iter(), expected_moves);

        // Test white king starting position
        let board = ChessBoard::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e1").into_iter(), vec![]);

        // Test black king starting position
        let board = ChessBoard::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e8").into_iter(), vec![]);

        // Test white king queen side and king side castling
        let board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e1").into_iter(),
            vec!["e1d1", "e1f1", "e1c1", "e1g1"],
        );

        // Test black king queen side and king side castling
        let board = ChessBoard::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e8").into_iter(),
            vec!["e8d8", "e8f8", "e8c8", "e8g8"],
        );

        // Test white king king side castling
        let board = ChessBoard::from_fen("1r2k2r/pppppppp/8/8/8/8/PPPPPPPP/1R2K2R w Kk - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e1").into_iter(),
            vec!["e1d1", "e1f1", "e1g1"],
        );

        // Test black king king side castling
        let board = ChessBoard::from_fen("1r2k2r/pppppppp/8/8/8/8/PPPPPPPP/1R2K2R b Kk - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e8").into_iter(),
            vec!["e8d8", "e8f8", "e8g8"],
        );

        // Test white king queen side and king side castling
        let board = ChessBoard::from_fen("r3k1r1/pppppppp/8/8/8/8/PPPPPPPP/R3K1R1 w Qq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e1").into_iter(),
            vec!["e1d1", "e1f1", "e1c1"],
        );

        // Test black king queen side and king side castling
        let board = ChessBoard::from_fen("r3k1r1/pppppppp/8/8/8/8/PPPPPPPP/R3K1R1 b Qq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e8").into_iter(),
            vec!["e8d8", "e8f8", "e8c8"],
        );

        // Test white king castling blocked on d and f position
        let board = ChessBoard::from_fen("r2bkb1r/pppppppp/8/8/8/8/PPPPPPPP/R2BKB1R w KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e1").into_iter(), vec![]);

        // Test black king castling blocked on d and f position
        let board = ChessBoard::from_fen("r2bkb1r/pppppppp/8/8/8/8/PPPPPPPP/R2BKB1R b KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e8").into_iter(), vec![]);

        // Test white king castling blocked on c and g position
        let board = ChessBoard::from_fen("r1b1k1br/pppppppp/8/8/8/8/PPPPPPPP/R1B1K1BR w KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e1").into_iter(), vec!["e1d1", "e1f1"]);

        // Test black king castling blocked on c and g position
        let board = ChessBoard::from_fen("r1b1k1br/pppppppp/8/8/8/8/PPPPPPPP/R1B1K1BR b KQkq - 0 1").unwrap();
        assert_moves(board.generate_pseudo_moves_from_algebraic("e8").into_iter(), vec!["e8d8", "e8f8"]);

        // Test white king castling blocked on b position
        let board = ChessBoard::from_fen("rb2k2r/pppppppp/8/8/8/8/PPPPPPPP/RB2K2R w KQkq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e1").into_iter(),
            vec!["e1d1", "e1f1", "e1g1"],
        );

        // Test black king castling blocked on b position
        let board = ChessBoard::from_fen("rb2k2r/pppppppp/8/8/8/8/PPPPPPPP/RB2K2R b KQkq - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e8").into_iter(),
            vec!["e8d8", "e8f8", "e8g8"],
        );

        // Test black king castling blocked cause of a check
        let board = ChessBoard::from_fen("1r2k2r/ppppp1pp/8/8/8/8/PPPPP1PP/R4RK1 b k - 0 1").unwrap();
        assert_moves(
            board.generate_pseudo_moves_from_algebraic("e8").into_iter(),
            vec!["e8d8", "e8f7", "e8f8"],
        );
        //f7,f8 are pseudo legal moves
    }

    #[test]
    fn test_pinned_piece() {
        let board = ChessBoard::from_fen("1k6/8/8/8/3q4/8/1R6/K7 w - - 0 1").unwrap();
        assert_moves(board.generate_legal_moves(None).into_iter(), vec!["a1a2", "a1b1"])
    }

    #[test]
    fn test_make_move_set_en_passant_legal() {
        let mut board = ChessBoard::from_fen("8/4p3/8/3P4/8/8/8/8 b - - 0 1").unwrap();
        board.make_move(Move::from_algebraic("e7e5"));
        let expected_moves = vec!["d5d6", "d5e6"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d5").into_iter(), expected_moves);

        let mut board = ChessBoard::from_fen("8/8/8/8/6p1/8/5P2/8 w - - 0 1").unwrap();
        board.make_move(Move::from_algebraic("f2f4"));
        let expected_moves = vec!["g4g3", "g4f3"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("g4").into_iter(), expected_moves);

        let mut board = ChessBoard::from_fen("8/8/1p6/8/8/p7/PPP5/8 w - - 0 1").unwrap();
        board.make_move(Move::from_algebraic("b2b3"));
        let expected_moves = vec!["b6b5"];
        let moves: Vec<_> = board.generate_pseudo_moves().into_iter().map(|m| m.1).collect();
        assert_moves(moves.into_iter(), expected_moves);

        let mut board =
            ChessBoard::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1pB1P3/2N2Q1p/PPPB1PPP/R3K2R b KQkq - 1 1").unwrap();
        board.make_move(Move::from_algebraic("c7c5"));
        let expected_moves = vec!["d5c6", "d5d6", "d5e6"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("d5").into_iter(), expected_moves.clone());
        let generated_moves: Vec<_> = board
            .generate_legal_moves(None)
            .into_iter().map(|m| m.as_algebraic())
            .collect();

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
        assert_eq!(board.squares[field.row as usize][field.col as usize], Square::Empty);
        let expected_moves = vec!["e7c5", "e7d6", "e7d8", "e7f8"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("e7").into_iter(), expected_moves.clone());

        let board = ChessBoard::from_fen("r2q1rk1/pP1p2pp/Q4n2/bb2p3/1pp5/1BN2NBn/pPPP1PPP/R3K2R b KQ - 1 2").unwrap();
        let expected_moves = vec!["b4c3"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("b4").into_iter(), expected_moves.clone());
    }

    #[test]
    fn test_breaking() {
        let mut board = ChessBoard::from_fen("8/2p5/3p4/KP5r/1R3pPk/8/4P3/8 b - g3 0 1").unwrap();
        board.make_move(Move::from_algebraic("h4g3"));
        let expected_moves = vec!["g4g5", "g4h5"];
        let mut debug_moves: Vec<_> = board
            .generate_legal_moves(None)
            .into_iter().map(|m| m.as_algebraic())
            .collect();
        debug_moves.sort();
        println!("{:?}", debug_moves.len());
        println!("{:?}", debug_moves);
        assert_moves(board.generate_pseudo_moves_from_algebraic("g4").into_iter(), expected_moves);
    }
}
