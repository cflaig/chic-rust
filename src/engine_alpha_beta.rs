use crate::chess_board::{ChessBoard, Color, Move, PieceType, Square};
use rand::prelude::SliceRandom;
use std::time::{Duration, Instant};

#[allow(dead_code)]
pub fn find_best_move(board: &ChessBoard, depth: i32, random: bool) -> Option<(Move, i32, u64)> {
    find_best_move_with_timeout(board, depth, random, Duration::from_secs(60 * 60))
}
pub fn find_best_move_with_timeout(
    board: &ChessBoard,
    depth: i32,
    random: bool,
    remaining_time: Duration,
) -> Option<(Move, i32, u64)> {
    let mut best_move = None;
    let mut best_score = i32::MIN;
    let mut node_count = 0;

    let mut moves = board.generate_legal_moves();
    if random {
        moves.shuffle(&mut rand::thread_rng());
    }
    let start_time = Instant::now();

    for mv in moves {
        if start_time.elapsed() > remaining_time {
            return None;
        }
        let mut new_board = board.clone();
        new_board.make_move(mv);

        let score = -negamax(&new_board, depth, MIN_EVALUATION, -MIN_EVALUATION, &mut node_count);

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
        //println!("With depth {} Move: {} Score: {}", depth, mv.as_algebraic(), score);
    }

    best_move.map(|mv| (mv, best_score, node_count))
}

pub fn find_best_move_iterative(board: &ChessBoard, time_limit: Duration) -> Option<(Move, i32, u64, i32)> {
    let mut best_move = None;
    let mut total_node_count = 0;

    let start_time = Instant::now();
    let mut depth = 1;

    while start_time.elapsed() < time_limit {
        let remaining_time = time_limit - start_time.elapsed();

        // Call the existing find_best_move function for the current depth.
        if let Some((current_move, current_score, node_count)) =
            find_best_move_with_timeout(board, depth, true, remaining_time)
        {
            best_move = Some((current_move, current_score, total_node_count + node_count, depth));
            total_node_count += node_count;
        } else {
            break;
        }

        depth += 1; // Increase the depth for the next iteration
    }

    best_move
}

const MIN_EVALUATION: i32 = i32::MIN + 1; // +1 is important because -MIN is not a i32 number
const WIN: i32 = 10_000_000;
const LOSS: i32 = -10_000_000;
const DRAW: i32 = 0;

fn negamax(board: &ChessBoard, depth: i32, alpha: i32, beta: i32, node_count: &mut u64) -> i32 {
    *node_count += 1;
    if board.is_threefold_repetition() {
        return 0;
    }
    if depth <= 0 {
        *node_count -= 1;
        return quiescence_search_prunning(board, node_count, alpha, beta);
    }

    let mut alpha = alpha;
    let mut max_score = MIN_EVALUATION;

    let moves = board.generate_legal_moves();
    if moves.is_empty() {
        // Handle checkmate or stalemate
        if board.is_checkmate() {
            return LOSS - depth;
        } else if board.is_stalemate() {
            return DRAW;
        }
    }

    for mv in moves {
        let mut new_board = board.clone();
        new_board.make_move(mv);
        let score = -negamax(&new_board, depth - 1,  -beta, -alpha, node_count);
        max_score = max_score.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            // Beta cutoff fail soft
            break;
        }
    }

    max_score
}

fn quiescence_search_prunning(board: &ChessBoard, node_count: &mut u64, mut alpha: i32, beta: i32) -> i32 {
    *node_count += 1;

    let stand_pat = evaluate_board(board) * if board.active_color == Color::White { 1 } else { -1 };
    let mut max_score = stand_pat;
    alpha = alpha.max(stand_pat);

    if alpha >= beta {
        return max_score;
    }

    let moves = board.generate_legal_capture_moves();

    //println!("Number of Capture Moves: {}", moves.len() );

    for mv in moves {
        let mut new_board = board.clone();
        new_board.make_move(mv);
        let score = -quiescence_search_prunning(&new_board, node_count, -beta, -alpha);
        max_score = max_score.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            // Beta cutoff
            break;
        }
    }
    max_score
}

#[rustfmt::skip]
const PAWN_SQUARE_TABLE: [[i32; 8]; 8] = [
    [  0,   0,   0,   0,   0,   0,   0,   0],
    [100, 100, 100, 100, 100, 100, 100, 100],
    [ 25,  50,  50,  50,  50,  50,  50,  25],
    [  0,   0,   0,   2,   2,   0,   0,   0],
    [  0,   0,  20,  25,  25,  20,   0,   0],
    [  0,   0,  15,  10,  10,  15,   0,   0],
    [  0,   0,   0,-250,-250,   0,   0,   0],
    [  0,   0,   0,   0,   0,   0,   0,   0],
];

#[rustfmt::skip]
const KNIGHT_SQUARE_TABLE: [[i32; 8]; 8] = [
    [-200,-100,-100,-100,-100,-100,-100,-200],
    [-100,   0,   0,   0,   0,   0,   0,-100],
    [-100,   0,  50,  50,  50,  50,   0,-100],
    [-100,   0,  50, 100, 150,  50,   0,-100],
    [-100,   0,  50, 100, 100,  50,   0,-100],
    [-100,   0,  50,  50,  50,  50,   0,-100],
    [-100,   0,   0,   0,   0,   0,   0,-100],
    [-200,-100,-100,-100,-100,-100,-100,-200],
];

#[rustfmt::skip]
const BISHOP_SQUARE_TABLE: [[i32; 8]; 8] = [
    [-200,-100,-100,-100,-100,-100,-100,-200],
    [-100,   0,   0,   0,   0,   0,   0,-100],
    [-100,   0,  50,  50,  50,  50,   0,-100],
    [-100,   0,  50, 100, 150,  50,   0,-100],
    [-100,   0,  50, 100, 100,  50,   0,-100],
    [-100,   0,  50,  50,  50,  50,   0,-100],
    [-100,  25,   0,   0,   0,  25,   0,-100],
    [-200,-100,-100,-100,-100,-100,-100,-200],
];

#[rustfmt::skip]
const KING_SQUARE_TABLE: [[i32; 8]; 8] = [
    [-100, -100, -100, -100, -100, -100, -100, -100],
    [-100, -100, -100, -100, -100, -100, -100, -100],
    [-100, -100, -100, -100, -100, -100, -100, -100],
    [-100, -100, -100, -100, -100, -100, -100, -100],
    [-100, -100, -100, -100, -100, -100, -100, -100],
    [-100, -100, -100, -100, -100, -100, -100, -100],
    [ -50,  -50,  -50,  -50,  -50, -500,  -50,  -50],
    [ 300,  350,  400,  -50,    0,  -50,  500,  300],
];

/// Evaluates the board state and assigns a score based on material balance.
fn evaluate_board(board: &ChessBoard) -> i32 {
    let mut evaluation = 0;

    for row in 0..8 {
        for col in 0..8 {
            match board.squares[row][col] {
                Square::Occupied(piece) => {
                    let piece_value = match piece.kind {
                        PieceType::Pawn => 1_000,
                        PieceType::Knight => 3_000,
                        PieceType::Bishop => 3_000,
                        PieceType::Rook => 5_000,
                        PieceType::Queen => 9_000,
                        PieceType::King => WIN, // if one king is on the board, it is won
                    };

                    //Check position value
                    let psq_row = match piece.color {
                        Color::White => 7 - row,
                        Color::Black => row,
                    };

                    let possition_value = match piece.kind {
                        PieceType::King => KING_SQUARE_TABLE[psq_row][col],
                        PieceType::Pawn => PAWN_SQUARE_TABLE[psq_row][col],
                        PieceType::Knight => KNIGHT_SQUARE_TABLE[psq_row][col],
                        PieceType::Bishop => BISHOP_SQUARE_TABLE[psq_row][col],
                        _ => 0,
                    };

                    let piece_evaluation = piece_value + possition_value;
                    evaluation += match piece.color {
                        Color::White => piece_evaluation,
                        Color::Black => -piece_evaluation,
                    };
                }

                Square::Empty => {}
            }
        }
    }

    evaluation
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_board::ChessBoard;

    #[test]
    fn test_some_positions() {
        let board = ChessBoard::from_fen("8/4p3/8/3P4/8/8/8/8 b - - 0 1").unwrap();
        if let Some((best_move, score, nodes)) = find_best_move(&board, 2, false) {
            println!(
                "Best move: {} with score: {} evaluated nodes: {}",
                best_move.as_algebraic(),
                score,
                nodes
            );
        } else {
            println!("No best move found!");
        }

        let board = ChessBoard::from_fen("8/7k/5KR1/8/8/8/8/8 w - - 0 1").unwrap();
        if let Some((best_move, score, nodes)) = find_best_move(&board, 6, false) {
            println!(
                "Best move: {} with score: {} evaluated nodes: {}",
                best_move.as_algebraic(),
                score,
                nodes
            );
        } else {
            println!("No best move found!");
        }

        let board = ChessBoard::from_fen("4k1nr/2p3p1/b2pPp1p/8/1nN1P1P1/p1R2N2/PR3P2/5K2 b k - 1 26").unwrap();
        if let Some((best_move, score, nodes)) = find_best_move(&board, 3, false) {
            println!(
                "Best move: {} with score: {} evaluated nodes: {}",
                best_move.as_algebraic(),
                score,
                nodes
            );
        } else {
            println!("No best move found!");
        }
    }

    #[test]
    fn test_from_a_played_position() {
        let board = ChessBoard::from_fen("4k1nr/2p3p1/b2pPp1p/8/1nN1P1P1/p1R2N2/PR3P2/5K2 b k - 1 26").unwrap();
        if let Some((best_move, score, nodes)) = find_best_move(&board, 0, false) {
            println!(
                "Best move: {} with score: {} evaluated nodes: {}",
                best_move.as_algebraic(),
                score,
                nodes
            );
        } else {
            println!("No best move found!");
        }
    }

    #[test]
    fn test_from_before_rochade() {
        let board = ChessBoard::from_fen("rnbqkbnr/p1p2ppp/1p1p4/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4").unwrap();
        if let Some((best_move, score, nodes)) = find_best_move(&board, 0, false) {
            println!(
                "Best move: {} with score: {} evaluated nodes: {}",
                best_move.as_algebraic(),
                score,
                nodes
            );
        } else {
            println!("No best move found!");
        }
        let board = ChessBoard::from_fen("rnbqkbnr/p1p2ppp/1p1p4/4p3/2B1P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 1 4").unwrap();
        println!("Evaluation: {}", evaluate_board(&board));
    }
}
