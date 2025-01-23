use super::{ChessBoard, Color, PieceType, Square};
use lazy_static::lazy_static;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;
use std::sync::Arc;

const BOARD_SIZE: usize = 8;

pub struct ZobristHash {
    piece_keys: [[[u64; BOARD_SIZE * BOARD_SIZE]; 6]; 2],
    side_to_move_key: u64,
    castling_keys: [u64; 4],
    en_passant_keys: [u64; BOARD_SIZE],
}

impl ZobristHash {
    fn new(seed: u64) -> Self {
        let mut rng = Pcg64::seed_from_u64(seed);

        // Random numbers for pieces on squares
        let mut piece_keys = [[[0; BOARD_SIZE * BOARD_SIZE]; 6]; 2];
        for color_keys in &mut piece_keys {
            for piece_type_keys in color_keys {
                // 6 unique piece types (e.g., Pawn, Knight, etc.)
                for square_key in piece_type_keys {
                    *square_key = rng.gen();
                }
            }
        }

        // Random number for side-to-move
        let side_to_move_key = rng.gen();

        // Random numbers for castling rights
        let mut castling_keys = [0; 4];
        for key in &mut castling_keys {
            *key = rng.gen();
        }

        // Random numbers for en passant file
        let mut en_passant_keys = [0; BOARD_SIZE];
        for file in &mut en_passant_keys {
            *file = rng.gen();
        }

        ZobristHash {
            piece_keys,
            side_to_move_key,
            castling_keys,
            en_passant_keys,
        }
    }

    pub fn calculate_hash(&self, board: &ChessBoard) -> u64 {
        let mut hash = 0;

        // Hash pieces on squares
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                if let Square::Occupied(piece) = board.squares[row][col] {
                    let color_index = match piece.color {
                        Color::White => 0,
                        Color::Black => 1,
                    };
                    let piece_index = match piece.kind {
                        PieceType::Pawn => 0,
                        PieceType::Knight => 1,
                        PieceType::Bishop => 2,
                        PieceType::Rook => 3,
                        PieceType::Queen => 4,
                        PieceType::King => 5,
                    };
                    let square_index = row * BOARD_SIZE + col;
                    hash ^= self.piece_keys[color_index][piece_index][square_index];
                }
            }
        }

        // Hash side to move
        if board.active_color == Color::Black {
            hash ^= self.side_to_move_key;
        }

        // Hash castling rights
        for (i, castling) in board.castling_rights.iter().enumerate() {
            if *castling {
                hash ^= self.castling_keys[i];
            }
        }

        // Hash en passant
        if let Some(en_passant) = board.en_passant {
            hash ^= self.en_passant_keys[en_passant.col];
        }

        hash
    }
}

lazy_static! {
    pub static ref ZOBRIST: Arc<ZobristHash> = Arc::new(ZobristHash::new(42));
}
