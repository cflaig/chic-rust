use super::Color::White;
use super::PieceType::Rook;
use super::PieceType::Pawn;
use super::Square::Occupied;
use super::zobrist_hash::ZOBRIST;
use super::{fen, ChessField, Color, Move, Piece, PieceType, Square};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChessBoard {
    pub squares: [[Square; 8]; 8],
    pub active_color: Color,
    pub castling_rights: [bool; 4],
    pub en_passant: Option<ChessField>,
    pub halfmove_clock: u8,
    pub fullmove_number: u8,
    pub hash: u64,
    pub last_capture: ChessField,
        pub black_pieces_positions: [ChessField; 16],
    pub white_pieces_positions: [ChessField; 16],
    pub black_pieces: [u8; 7],
    pub white_pieces: [u8; 7],
}

pub fn get_piece_type_index(piece: &PieceType) -> usize {
    match piece {
        PieceType::Pawn => 5,
        PieceType::Knight => 4,
        PieceType::Bishop => 3,
        PieceType::Rook => 2,
        PieceType::Queen => 1,
        PieceType::King => 0,
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
            hash: 0,
            last_capture: ChessField { row: 99, col: 99 },
            black_pieces: [0; 7],
            white_pieces: [0; 7],
            black_pieces_positions: [ChessField { row: 99, col: 99 }; 16],
            white_pieces_positions: [ChessField { row: 99, col: 99 }; 16],
        }
    }

    /// Delegates FEN parsing to the `fen` module.
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        fen::from_fen(fen).map(|mut board| {
            let zobrist = &*ZOBRIST;
            board.hash = zobrist.calculate_hash(&board);
            let (positions, piece_indexes) = board.get_piece_position_data_structure(Color::White);
            for (i, pos) in positions.iter().enumerate() {
                board.white_pieces_positions[i] = *pos;
            }
            let size = std::mem::size_of::<ChessBoard>();
            for (i, pos) in piece_indexes.iter().enumerate() {
                board.white_pieces[i] = piece_indexes[i];
            }
            let (positions, piece_indexes) = board.get_piece_position_data_structure(Color::Black);
            for (i, pos) in positions.iter().enumerate() {
                board.black_pieces_positions[i] = *pos;
            }
            for (i, pos) in piece_indexes.iter().enumerate() {
                board.black_pieces[i] = piece_indexes[i];
            }
            board
        })
    }

    pub fn to_fen(&self) -> String {
        fen::to_fen(self)
    }
}

impl ChessBoard {
    /// Returns an iterator over all pieces on the chessboard along with their coordinates.
    pub fn pieces_with_coordinates<'a>(&'a self) -> impl Iterator<Item = (ChessField, &'a Piece)> {
        let piece_position = if self.active_color == Color::White {
            self.white_pieces_positions[0..self.white_pieces[6] as usize].iter()
        } else {
            self.black_pieces_positions[0..self.black_pieces[6] as usize].iter()
        };

        piece_position.filter_map(move |field| {
            if let Occupied(piece) = &self.squares[field.row as usize][field.col as usize] {
                Some((*field, piece))
            } else {
                None
            }
        })
    }

    pub fn all_pieces_with_coordinates<'a>(&'a self) -> impl Iterator<Item = (ChessField, &'a Piece)> {
        let piece_position = self.white_pieces_positions[0..self.white_pieces[6] as usize].iter()
            .chain(self.black_pieces_positions[0..self.black_pieces[6] as usize].iter());

        piece_position.filter_map(move |field| {
            if let Occupied(piece) = &self.squares[field.row as usize][field.col as usize] {
                Some((*field, piece))
            } else {
                None
            }
        })
    }

    pub fn make_move(&mut self, mv: Move) {
        let piece = self.squares[mv.from.row as usize][mv.from.col as usize];
        let zobrist = &*ZOBRIST;
        let mut hash = self.hash;
        //undo castling rights in hash
        hash = zobrist.update_castling(hash, self.castling_rights);

        match piece {
            Square::Empty => {
                hash = zobrist.update_enpassing(hash, self.en_passant);
                self.en_passant = None;
            }
            Square::Occupied(p) => {
                if p.kind == PieceType::Pawn || matches!(self.squares[mv.to.row as usize][mv.to.col as usize], Square::Occupied(_)) {
                    self.halfmove_clock = 0;
                } else {
                    self.halfmove_clock += 1;
                }

                hash = zobrist.update_piece(hash, p, mv.from.row, mv.from.col);
                self.squares[mv.from.row as usize][mv.from.col as usize] = Square::Empty;

                if let Square::Occupied(piece) = self.squares[mv.to.row as usize][mv.to.col as usize] {
                    hash = zobrist.update_piece(hash, piece, mv.to.row, mv.to.col);
                    self.last_capture = mv.to;
                    self.remove_piece_from_piece_position(mv.to, piece);
                } else {
                    self.last_capture = ChessField { row: 99, col: 99 };
                }
                hash = zobrist.update_piece(hash, p, mv.to.row, mv.to.col);
                self.squares[mv.to.row as usize][mv.to.col as usize] = piece;

                self.update_piece_position(mv, p);

                if let Some(en_passant) = self.en_passant {
                    if mv.to == en_passant && p.kind == PieceType::Pawn {
                        //Remove piece from en passant
                        hash =
                            zobrist.update_square(hash, self.squares[mv.from.row as usize][mv.to.col as usize], mv.from.row, mv.to.col);
                        self.squares[mv.from.row as usize][mv.to.col as usize] = Square::Empty;
                        self.remove_piece_from_piece_position(ChessField {
                            row: mv.from.row,
                            col: mv.to.col,
                        }, Piece {kind: Pawn, color: p.color.opposite()} );
                    }
                }
                hash = zobrist.update_enpassing(hash, self.en_passant);
                self.en_passant = None;

                // Check if the move is a castling move and if castling is allowed
                if p.kind == PieceType::King {
                    if mv.from.col == 4 && mv.to.col == 6 && mv.from.row == mv.to.row {
                        if self.castling_rights[if self.active_color == Color::White { 0 } else { 2 }] {
                            let rook_col = 7;
                            self.squares[mv.from.row as usize][5] = self.squares[mv.from.row as usize][rook_col];
                            hash = zobrist.update_square(hash, self.squares[mv.from.row as usize][5], mv.from.row, 5);
                            hash =
                                zobrist.update_square(hash, self.squares[mv.from.row as usize][rook_col], mv.from.row, rook_col as u8);
                            self.squares[mv.from.row as usize][rook_col] = Square::Empty;
                            let mv = Move {
                                to: ChessField::new(mv.from.row, 5),
                                from: ChessField::new(mv.from.row, rook_col as u8),
                                promotion: None,
                            };
                            let rook = Piece { kind: Rook, color: p.color };
                            self.update_piece_position(mv,rook)
                        }
                    } else if mv.from.col == 4 && mv.to.col == 2 && mv.from.row == mv.to.row {
                        // Queenside castling
                        if self.castling_rights[if self.active_color == Color::White { 1 } else { 3 }] {
                            let rook_col = 0;
                            self.squares[mv.from.row as usize][3] = self.squares[mv.from.row as usize][rook_col];
                            hash = zobrist.update_square(hash, self.squares[mv.from.row as usize][3], mv.from.row, 3);
                            hash =
                                zobrist.update_square(hash, self.squares[mv.from.row as usize][rook_col], mv.from.row, rook_col as u8);
                            self.squares[mv.from.row as usize][rook_col] = Square::Empty;
                            let mv = Move {
                                to: ChessField::new(mv.from.row, 3),
                                from: ChessField::new(mv.from.row, rook_col as u8),
                                promotion: None,
                            };
                            let rook = Piece { kind: Rook, color: p.color };
                            self.update_piece_position(mv,rook);
                            self.update_piece_position(mv,p)
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

                if p.kind == PieceType::Pawn {
                    if p.color == Color::White && mv.from.row == 1 && mv.to.row == 3 {
                        self.en_passant = Some(ChessField::new(2, mv.from.col));
                    } else if p.color == Color::Black && mv.from.row == 6 && mv.to.row == 4 {
                        self.en_passant = Some(ChessField::new(5, mv.from.col));
                    } else if mv.promotion.is_some() {
                        // Handle promotion
                        let promotion_piece = Piece {
                            color: p.color,
                            kind: mv.promotion.unwrap(), // Replace the pawn with the promoted piece
                        };
                        self.squares[mv.to.row as usize][mv.to.col as usize] = Square::Occupied(promotion_piece);
                        hash = zobrist.update_piece(hash, p, mv.to.row, mv.to.col);
                        hash = zobrist.update_piece(hash, promotion_piece, mv.to.row, mv.to.col);
                        self.remove_piece_from_piece_position(mv.to, p);
                        self.insert_piece_from_piece_position(mv.to, promotion_piece);
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

        hash = zobrist.update_castling(hash, self.castling_rights);
        hash = zobrist.update_active_side(hash);
        hash = zobrist.update_enpassing(hash, self.en_passant);

        self.hash = hash;
    }

    fn update_piece_position(&mut self, mv: Move, piece: Piece) {
        let piece_index = get_piece_type_index(&piece.kind);
        if self.active_color == Color::White {
            self.white_pieces_positions[self.white_pieces[piece_index] as usize..self.white_pieces[piece_index + 1] as usize].iter_mut().filter(|f| **f == mv.from).for_each(|f| *f = mv.to);
        } else {
            self.black_pieces_positions[self.black_pieces[piece_index] as usize..self.black_pieces[piece_index + 1] as usize].iter_mut().filter(|f| **f == mv.from).for_each(|f| *f = mv.to);
        }
    }

    fn insert_piece_from_piece_position(&mut self, field: ChessField, piece: Piece) {
        let piece_index = get_piece_type_index(&piece.kind) + 1;
        if self.active_color == Color::White {
            Self::insert_pieces_array_and_positions(field, piece_index, &mut self.white_pieces, &mut self.white_pieces_positions);
        } else {
            Self::insert_pieces_array_and_positions(field, piece_index, &mut self.black_pieces, &mut self.black_pieces_positions);
        }
    }

    fn insert_pieces_array_and_positions(field: ChessField, piece_index: usize, pieces: &mut [u8; 7], piece_position: &mut [ChessField; 16]) {
        for i in (pieces[piece_index]..pieces[6]).rev() {
                piece_position[i as usize + 1] = piece_position[i as usize] ;
        }
        piece_position[pieces[piece_index] as usize] = field;
        for i in piece_index..pieces.len() {
            pieces[i] += 1;
        }
    }

    fn remove_piece_from_piece_position(&mut self, field: ChessField, piece: Piece) {
        let piece_index = get_piece_type_index(&piece.kind);
        if piece.color == Color::White {
            Self::remove_pieces_array_and_positions(field, piece_index, &mut self.white_pieces, &mut self.white_pieces_positions);
        } else {
            Self::remove_pieces_array_and_positions(field, piece_index, &mut self.black_pieces, &mut self.black_pieces_positions);
        }
    }

    fn remove_pieces_array_and_positions(field: ChessField, piece_index: usize, pieces: &mut [u8; 7], piece_position: &mut [ChessField; 16]) {
        let mut found = false;
        for i in pieces[piece_index]..pieces[6] - 1 {
            if found || piece_position[i as usize] == field {
                found = true;
                piece_position[i as usize] = piece_position[i as usize + 1];
            }
        }
        if piece_position[pieces[6] as usize - 1] == field {
            found = true;
            piece_position[pieces[6] as usize - 1] = ChessField::new(99, 99);
        }
        if found {
            for i in piece_index + 1..pieces.len() {
                pieces[i] -= 1;
            }
        }
    }

    pub fn is_square_attacked(&self, row: u8, col: u8) -> bool {
        let opponent_color = match self.active_color {
            Color::White => Color::Black,
            Color::Black => Color::White,
        };
        self.is_square_attacked_by_color(row, col, opponent_color)
    }

    pub fn is_square_attacked_by_color(&self, row: u8, col: u8, opponent_color: Color) -> bool {
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

        if self.check_attack(row as u8, col as u8, opponent_color, &pawn_attacks, PieceType::Pawn) {
            return true;
        }


        let indexes = if opponent_color == White {
            self.white_pieces
        } else {
            self.black_pieces

        };
        let position = if opponent_color == White {
            self.white_pieces_positions
        } else {
            self.black_pieces_positions
        };

        for i in position[indexes[get_piece_type_index(&PieceType::Knight)] as usize..indexes[get_piece_type_index(&PieceType::Knight)+1] as usize].iter() {
            let diff = (i.row as isize - row as isize, i.col as isize - col as isize);
            if diff.0 * diff.0 + diff.1*diff.1 == 5 {
                return true
            }
        }

        if let Some(king) =self.find_king_position(opponent_color) {
            let diff = (king.row as isize - row as isize, king.col as isize - col as isize);
            if (diff.0 > -2 && diff.0 < 2 ) && (diff.1 > -2 && diff.1 < 2 ) {
                return true
            }
        }

        false
    }

    fn check_attack(
        &self,
        row: u8,
        col: u8,
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

    pub fn find_king_position(&self, color: Color) -> Option<ChessField> {
        let king = match color {
            Color::White => Some(self.white_pieces_positions[0]),
            Color::Black => Some(self.black_pieces_positions[0]),
        };
        if matches!(king, Some(ChessField { row: 99, col: 99 })) {
            None
        } else {
            king
        }
    }

    fn get_piece_position_data_structure(&self, color: Color) -> (Vec<ChessField>, Vec<u8>) {
        let mut positions = Vec::new();
        let mut piece_indexes = Vec::new();
        piece_indexes.push(0u8);
        for p in [PieceType::King, PieceType::Queen, PieceType::Rook, PieceType::Bishop, PieceType::Knight, PieceType::Pawn] {
            self.find_piece_position_position_by_scanning(Piece {
                color,
                kind: p,
            })
            .iter()
            .for_each(|f| {
                positions.push(ChessField::new(f.row, f.col));
            }
            );
            piece_indexes.push(positions.len() as u8);
        }
        (positions, piece_indexes)
    }

    fn find_piece_position_position_by_scanning(&self, piece: Piece) -> Vec<ChessField> {
        let mut positions = Vec::new();
        for row in 0..8 {
            for col in 0..8 {
                if let Square::Occupied(p) = self.squares[row][col]
                {
                    if p == piece {
                        positions.push(ChessField::new(row as u8, col as u8));
                    }
                }
            }
        }
        positions
    }


    #[allow(dead_code)]
    pub fn is_stalemate(&self) -> bool {
        if let Some(king_pos) = self.find_king_position(self.active_color) {
            if self.is_square_attacked(king_pos.row as u8, king_pos.col as u8) {
                return false;
            }
        } else {
            return false;
        }
        self.generate_legal_moves(None).len() == 0
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

        self.generate_legal_moves(None).len() == 0
    }

    #[allow(dead_code)]
    pub fn is_draw(&self) -> bool {
        self.is_draw_by_fifty_move_rule()
    }
    #[allow(dead_code)]
    pub fn is_draw_by_fifty_move_rule(&self) -> bool {
        self.halfmove_clock >= 100
    }

    pub(crate) fn render_to_string(&self) -> String {
        let mut board_representation = String::new();
        board_representation.push_str("    a   b   c   d   e   f   g   h  \n");
        board_representation.push_str("  ┌───┬───┬───┬───┬───┬───┬───┬───┐\n");

        for row in (0..8).rev() {
            // Render rows from top (8) to bottom (1)
            board_representation.push_str(&format!("{} │", row + 1)); // Row number on the left
            for col in 0..8 {
                let square = match &self.squares[row][col] {
                    Square::Empty => ' ',
                    Square::Occupied(piece) => match piece.kind {
                        PieceType::Pawn => {
                            if piece.color == Color::White {
                                'P'
                            } else {
                                'p'
                            }
                        }
                        PieceType::Knight => {
                            if piece.color == Color::White {
                                'N'
                            } else {
                                'n'
                            }
                        }
                        PieceType::Bishop => {
                            if piece.color == Color::White {
                                'B'
                            } else {
                                'b'
                            }
                        }
                        PieceType::Rook => {
                            if piece.color == Color::White {
                                'R'
                            } else {
                                'r'
                            }
                        }
                        PieceType::Queen => {
                            if piece.color == Color::White {
                                'Q'
                            } else {
                                'q'
                            }
                        }
                        PieceType::King => {
                            if piece.color == Color::White {
                                'K'
                            } else {
                                'k'
                            }
                        }
                    },
                };
                board_representation.push_str(&format!(" {} │", square));
            }
            board_representation.push_str(&format!(" {}\n", row + 1));

            // Add horizontal grid border between rows
            if row > 0 {
                board_representation.push_str("  ├───┼───┼───┼───┼───┼───┼───┼───┤\n");
            }
        }

        board_representation.push_str("  └───┴───┴───┴───┴───┴───┴───┴───┘\n");
        board_representation.push_str("    a   b   c   d   e   f   g   h  \n");

        board_representation
    }
}

#[test]
fn test_hashing() {
    let mut board = ChessBoard::from_fen("1k6/q6P/8/2n5/5p2/8/6P1/R3K2R w KQ - 0 1").unwrap();
    let zobrist = board.make_move(Move::from_algebraic("a1a7"));
    assert_eq!(board.hash, ZOBRIST.calculate_hash(&board));
    board.make_move(Move::from_algebraic("c5e6"));
    board.make_move(Move::from_algebraic("e1g1"));
    assert_eq!(board.hash, ZOBRIST.calculate_hash(&board));
    board.make_move(Move::from_algebraic("e6c5"));
    board.make_move(Move::from_algebraic("g2g4"));
    assert_eq!(board.hash, ZOBRIST.calculate_hash(&board));
    board.make_move(Move::from_algebraic("g4g3"));
    assert_eq!(board.hash, ZOBRIST.calculate_hash(&board));
    board.make_move(Move::from_algebraic("g4g3b"));
    assert_eq!(board.hash, ZOBRIST.calculate_hash(&board));
}

#[test]
fn test_hashing2() {
    let mut board = ChessBoard::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1").unwrap();
    let zobrist = board.make_move(Move::from_algebraic("f1f2"));
    assert_eq!(board.hash, ZOBRIST.calculate_hash(&board));
    board.make_move(Move::from_algebraic("b2a1q"));
    assert_eq!(board.hash, ZOBRIST.calculate_hash(&board));
}

#[test]
fn test_hashing_recursive() {
    let mut board = ChessBoard::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1").unwrap();
    let mut mvs = vec![];
    check_hash_recursive(&board, 5, &mut mvs);
}

pub fn check_hash_recursive(board: &ChessBoard, depth: u8, mvs: &mut Vec<Move>) {
    if depth == 0 {
        return;
    }

    let moves = board.generate_legal_moves(None);
    for mv in moves {
        let mut new_board = board.clone();
        new_board.make_move(mv);
        mvs.push(mv);
        let board_hash = ZOBRIST.calculate_hash(&new_board);

        if new_board.hash != board_hash {
            println!("{:?}", mvs.iter().map(|&m| m.as_algebraic()).collect::<Vec<_>>())
        }
        assert_eq!(new_board.hash, board_hash);
        check_hash_recursive(&new_board, depth - 1, mvs);
        mvs.pop();
    }
}

#[cfg(test)]
mod tests {
    use crate::chess_boards::chess_board::fen::INITIAL_POSITION;
    use crate::chess_boards::perft::perft;
    use super::super::test_utils::assert_moves;
    use super::super::Square::Occupied;
    use super::*;
    #[test]
    fn test_make_move() {
        let mut board = ChessBoard::from_fen("8/2p5/3p4/KP5r/1R3pPk/8/4P3/8 b - g3 0 1").unwrap();
        board.make_move(Move::from_algebraic("h4g3"));
        let expected_moves = vec!["g4g5", "g4h5"];
        assert_moves(board.generate_pseudo_moves_from_algebraic("g4").into_iter(), expected_moves);
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
        assert_eq!(board.white_pieces_positions[0], ChessField::new(0,6));
        println!("{:?}", board.white_pieces_positions[0]);
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
        assert_eq!(board.black_pieces_positions[0], ChessField::new(7,6));
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
    fn assert_piece_position(board: &ChessBoard, expected_board: &ChessBoard, msg: String) {
        let indexes = board.white_pieces.into_iter().collect::<Vec<_>>();
        let position = board.white_pieces_positions[0..board.white_pieces[6] as usize].to_vec();
        let (expected_position, expected_indexes) = expected_board.get_piece_position_data_structure(Color::White);
        assert_piece_position_arrays(indexes, position, expected_position, expected_indexes, msg.clone());

        let indexes = board.black_pieces.into_iter().collect::<Vec<_>>();
        let position = board.black_pieces_positions[0..board.black_pieces[6] as usize].to_vec();
        let (expected_position, expected_indexes) = expected_board.get_piece_position_data_structure(Color::Black);
        assert_piece_position_arrays(indexes, position, expected_position, expected_indexes, msg);
    }

    fn assert_piece_position_arrays(indexes: Vec<u8>, position: Vec<ChessField>, expected_position: Vec<ChessField>, expected_indexes: Vec<u8>, msg: String) {
        assert_eq!(indexes, expected_indexes, "Wrong piece indexes for {}", msg);
        for i in 0..indexes.len() - 1 {
            let piece_positions = position[indexes[i] as usize..indexes[i + 1] as usize].to_vec().sort();
            let expected_piece_positions = expected_position[expected_indexes[i] as usize..expected_indexes[i + 1] as usize].to_vec().sort();
            assert_eq!(piece_positions, expected_piece_positions, "failed at index {}, {}", i, msg);
        }
    }

    #[test]
    fn test_piece_position() {
        let mut board = ChessBoard::from_fen("r3k3/1b4P1/n7/7n/1p6/Q7/2P3N1/4K2R w Kq - 0 1").unwrap();
        board.make_move(Move::from_algebraic("g7g8n"));
        let mut newboard = ChessBoard::from_fen("r3k1N1/1b6/n7/7n/1p6/Q7/2P3N1/4K2R b Kq - 0 1").unwrap();
        assert_piece_position(&board, &newboard, "".to_string());


        board.make_move(Move::from_algebraic("e8c8"));
        let mut newboard = ChessBoard::from_fen("2kr2N1/1b6/n7/7n/1p6/Q7/2P3N1/4K2R w K - 1 2").unwrap();
        assert_piece_position(&board, &newboard, "".to_string());


        board.make_move(Move::from_algebraic("c2c4"));
        let mut newboard = ChessBoard::from_fen("2kr2N1/1b6/n7/7n/1pP5/Q7/6N1/4K2R b K c3 0 2").unwrap();
        assert_piece_position(&board, &newboard, "".to_string());


        board.make_move(Move::from_algebraic("b4c3"));
        let mut newboard = ChessBoard::from_fen("2kr2N1/1b6/n7/7n/8/Q1p5/6N1/4K2R w K - 0 3").unwrap();
        assert_piece_position(&board, &newboard, "".to_string());


        board.make_move(Move::from_algebraic("e1g1"));
        let mut newboard = ChessBoard::from_fen("2kr2N1/1b6/n7/7n/8/Q1p5/6N1/5RK1 b - - 1 3").unwrap();
        assert_piece_position(&board, &newboard, "".to_string());


        board.make_move(Move::from_algebraic("c3c2"));
        board.make_move(Move::from_algebraic("g2f4"));
        board.make_move(Move::from_algebraic("c2c1r"));
        let mut newboard = ChessBoard::from_fen("2kr2N1/1b6/n7/7n/5N2/Q7/8/2r2RK1 w - - 0 5").unwrap();
        assert_piece_position(&board, &newboard, "".to_string());

        let mut board = ChessBoard::from_fen("r3k2r/p1ppqpb1/Bn2pnp1/3PN3/4P3/2p2Q1p/PPPB1PPP/R3K2R w KQkq - 0 2").unwrap();
        board.make_move(Move::from_algebraic("e5f7"));
        let mut newboard = ChessBoard::from_fen("r3k2r/p1ppqNb1/Bn2pnp1/3P4/4P3/2p2Q1p/PPPB1PPP/R3K2R b KQkq - 0 2").unwrap();
        assert_piece_position(&board, &newboard, "".to_string());
    }

    pub fn assert_possition_recursive(board: &ChessBoard, depth: u8) {
        let mut node_count = 0u64;

        if depth == 0 {
            return;
        }

        for mv in board.generate_legal_moves(None) {
            let mut new_board = board.clone();
            new_board.make_move(mv);
            let fen = new_board.to_fen();
            let expected_board = ChessBoard::from_fen(&fen).unwrap();
            let msg = format!("fen {} moves {}", board.to_fen(), mv.as_algebraic());
            assert_piece_position(&new_board, &expected_board, msg);
            assert_possition_recursive(&new_board, depth - 1);
        }
    }
    #[test]
    fn test_piece_position_recursive() {
        let board = ChessBoard::from_fen(INITIAL_POSITION).unwrap();
        assert_possition_recursive(&board, 3);

        let board =
            ChessBoard::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
        assert_possition_recursive(&board, 4);

        let board = ChessBoard::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
        assert_possition_recursive(&board, 4);
    }
}
