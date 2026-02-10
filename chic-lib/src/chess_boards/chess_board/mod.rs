pub mod fen;
pub mod model;
pub mod pgn;
pub mod zobrist_hash;
pub use model::{ChessField, Color, Move, Piece, PieceType, Square};

mod chess_board;
mod move_generation;
pub mod test_utils;
pub use chess_board::ChessBoard;

#[cfg(test)]
mod tests {
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
    fn test_san_generation() {
        let board = ChessBoard::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        // Pawn move
        assert_eq!(Move::from_algebraic("e2e4").to_san(&board), "e4");
        // Knight move
        assert_eq!(Move::from_algebraic("g1f3").to_san(&board), "Nf3");

        // Disambiguation
        let board2 = ChessBoard::from_fen("r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3").unwrap();
        // Bb5
        assert_eq!(Move::from_algebraic("f1b5").to_san(&board2), "Bb5");

        let board_disambig = ChessBoard::from_fen("N6N/4k3/8/8/8/8/4K3/8 w - - 0 1").unwrap();
        assert_eq!(Move::from_algebraic("a8c7").to_san(&board_disambig), "Nc7");

        let board_disambig2 = ChessBoard::from_fen("8/8/4k3/8/3N1N2/8/4K3/8 w - - 0 1").unwrap();
        assert_eq!(Move::from_algebraic("d4e6").to_san(&board_disambig2), "Ndxe6");
        assert_eq!(Move::from_algebraic("f4e6").to_san(&board_disambig2), "Nfxe6");

        // Capture
        let board_capture =
            ChessBoard::from_fen("r1bqkbnr/ppp1pppp/2n5/3p4/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 1").unwrap();
        assert_eq!(Move::from_algebraic("e4d5").to_san(&board_capture), "exd5");

        // Promotion
        let board_promo = ChessBoard::from_fen("8/4P3/8/8/8/8/8/k6K w - - 0 1").unwrap();
        assert_eq!(
            Move::with_promotion(Move::new(6, 4, 7, 4), PieceType::Queen).to_san(&board_promo),
            "e8=Q"
        );

        // Castling
        let board_castle =
            ChessBoard::from_fen("rnbqk2r/pppp1ppp/5n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 1").unwrap();
        assert_eq!(Move::from_algebraic("e1g1").to_san(&board_castle), "O-O");
    }

    #[test]
    fn test_san_parsing() {
        let board = ChessBoard::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(Move::from_san(&board, "e4").unwrap(), Move::from_algebraic("e2e4"));
        assert_eq!(Move::from_san(&board, "Nf3").unwrap(), Move::from_algebraic("g1f3"));

        let board_disambig2 = ChessBoard::from_fen("8/8/4k3/8/3N1N2/8/4K3/8 w - - 0 1").unwrap();
        assert_eq!(
            Move::from_san(&board_disambig2, "Ndxe6").unwrap(),
            Move::from_algebraic("d4e6")
        );
        assert_eq!(
            Move::from_san(&board_disambig2, "Nfxe6").unwrap(),
            Move::from_algebraic("f4e6")
        );
    }

    #[test]
    fn test_if_field_is_attacked() {
        let board = ChessBoard::from_fen("8/2P5/8/8/7k/8/3p3K/8 w - - 0 1").unwrap();
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
