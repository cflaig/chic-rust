use crate::chess_boards::chess_board::ChessBoard;

pub fn perft(board: &ChessBoard, depth: u8) -> u64 {
    let mut node_count = 0u64;

    if depth == 0 {
        return 1u64;
    }

    for mv in board.generate_legal_moves(None) {
        let mut new_board = board.clone();
        new_board.make_move(mv);
        node_count += perft(&new_board, depth - 1);
    }
    node_count
}

#[cfg(test)]
mod tests {
    use super::*;
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
