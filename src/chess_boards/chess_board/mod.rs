use model::Square::Occupied;

pub mod fen;
pub mod zobrist_hash;
pub use zobrist_hash::ZobristHash;
pub use zobrist_hash::ZOBRIST;
pub mod model;
pub use model::{ChessField, Color, Move, Piece, PieceType, Square};

mod chess_board;
mod move_generation;
pub mod test_utils;
pub use chess_board::ChessBoard;
pub use move_generation::LazySortedMoves;

#[cfg(test)]
mod tests {
    use super::model::to_algebraic_square;
    use super::test_utils::assert_moves;
    use super::*;


    impl ChessBoard {
        /// Creates an empty chess board
        pub fn generate_pseudo_moves_from_chess_field(&self, pos: ChessField) -> Vec<Move> {
            self.generate_pseudo_moves_from_position(pos.row, pos.col)
                .into_iter()
                .map(|m| m.1)
                .collect()
        }

        pub fn generate_pseudo_moves_from_algebraic(&self, square: &str) -> Vec<Move> {
            self.generate_pseudo_moves_from_chess_field(ChessField::from_algebraic(square))
        }
    }

    #[test]
    fn test_three_fold_repetition() {
        let mut board =
            ChessBoard::from_fen("1rb2rk1/p4ppp/1p1qp1n1/3n2N1/2pP4/2P3P1/PPQ2PBP/R1B1R1K1 w - - 4 17").unwrap();

        board.make_move(Move::from_algebraic("e1e2"));
        board.make_move(Move::from_algebraic("g8h8"));
        board.make_move(Move::from_algebraic("e2e1"));
        board.make_move(Move::from_algebraic("h8g8"));
        //assert_eq!(board.is_threefold_repetition(), false);
        board.make_move(Move::from_algebraic("e1e2"));
        board.make_move(Move::from_algebraic("g8h8"));
        board.make_move(Move::from_algebraic("e2e1"));
        board.make_move(Move::from_algebraic("h8g8"));
        //assert_eq!(board.is_threefold_repetition(), true);
    }

    #[test]
    fn test_convertion_method() {
        assert_eq!(ChessField::from_algebraic("b2"), ChessField::new(1, 1));
        assert_eq!(ChessField::from_algebraic("b2").as_algebraic(), "b2");
        assert_eq!(Move::from_algebraic("e2e4").as_algebraic(), "e2e4");
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
}
