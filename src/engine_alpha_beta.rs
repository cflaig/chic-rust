use std::time::{Duration};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::{Instant, SystemTime};

use rand::prelude::SliceRandom;
use crate::chess_board::{Color, PieceType, Square};
use crate::ChessBoard;
use crate::Move; // Use the Move struct from your existing chess module

/// A simple heuristic evaluation function.
/// Calculates the score of the current board state from the perspective of the active player.
fn evaluate_board(board: &ChessBoard) -> i32 {
    const PAWN_VALUE: i32 = 100;
    const KNIGHT_VALUE: i32 = 300;
    const BISHOP_VALUE: i32 = 300;
    const ROOK_VALUE: i32 = 500;
    const QUEEN_VALUE: i32 = 900;
    const KING_VALUE: i32 = 10000;

    let mut score = 0;

    for row in 0..8 {
        for col in 0..8 {
            if let Square::Occupied(piece) = board.squares[row][col] {
                let piece_value = match piece.kind {
                    PieceType::Pawn => PAWN_VALUE,
                    PieceType::Knight => KNIGHT_VALUE,
                    PieceType::Bishop => BISHOP_VALUE,
                    PieceType::Rook => ROOK_VALUE,
                    PieceType::Queen => QUEEN_VALUE,
                    PieceType::King => KING_VALUE,
                };

                match piece.color {
                    Color::White => score += piece_value,
                    Color::Black => score -= piece_value,
                }
            }
        }
    }

    score
}

/// Negamax implementation with alpha-beta pruning.
fn negamax(board: &mut ChessBoard, depth: i32, alpha: i32, beta: i32, node_count: &mut u64) -> i32 {
    *node_count += 1;
    if board.is_threefold_repetition() {
        return 0;
    }
    if depth == 0 {
        // Base case: return heuristic score of the position
        return evaluate_board(board) * if board.active_color == Color::White { 1 } else { -1 };
    }

    let mut alpha = alpha;
    let mut best_value = alpha;

    let moves = board.generate_legal_moves();
    if moves.is_empty() {
        // Handle checkmate or stalemate
        if board.is_checkmate() {
            return -100000 - depth; // Large negative score for a loss
        } else if board.is_stalemate() {
            return 0; // Stalemate is a draw
        }
    }

    for mv in moves {
        let mut board_clone = board.clone();
        board_clone.make_move(mv);

        let value = -negamax(&mut board_clone, depth - 1, -beta, -alpha, node_count);
        best_value = best_value.max(value);
        alpha = alpha.max(best_value);

        if alpha >= beta {
            // Beta cutoff
            break;
        }
    }

    alpha
}

/// Find the best move using negamax and alpha-beta pruning.
pub fn find_best_move(board: &ChessBoard, depth: i32) -> Option<(Move, i32, u64)> {
    let mut best_move = None;
    let mut best_value = i32::MIN;
    let mut alpha = i32::MIN + 1;
    let beta = i32::MAX;

    let mut node_count = 0;

    let mut moves = board.generate_legal_moves();

    for mv in moves {
        let mut board_clone = board.clone();
        board_clone.make_move(mv);

        let value = -negamax(&mut board_clone, depth, -beta, -alpha, &mut node_count);
        //println!("Move: {} Score: {}", mv.as_algebraic(), value);
        if value > best_value {
            best_value = value;
            best_move = Some(mv);
        }

        alpha = alpha.max(best_value);
    }

    best_move.map(|mv| (mv, best_value, node_count))
}

pub fn find_best_move_random(board: &ChessBoard, depth: i32) -> Option<(Move, i32, u64)> {
    find_best_move_random_with_timeout(board, depth, Duration::from_secs(60*60))
}
pub fn find_best_move_random_with_timeout(board: &ChessBoard, depth: i32, remaining_time: Duration) -> Option<(Move, i32, u64)> {
    let mut best_move = None;
    let mut best_value = i32::MIN;
    let mut alpha = i32::MIN + 1;
    let beta = i32::MAX;

    let mut node_count = 0;

    let mut moves = board.generate_legal_moves();
    moves.shuffle(&mut rand::thread_rng());
    let start_time = Instant::now();

    for mv in moves {
        if start_time.elapsed() >= remaining_time {
            // Return None to indicate timeout
            return None;
        }
        let mut board_clone = board.clone();
        board_clone.make_move(mv);

        let value = -negamax(&mut board_clone, depth, -beta, -alpha, &mut node_count);
        //println!("Move: {} Score: {}", mv.as_algebraic(), value);
        if value > best_value {
            best_value = value;
            best_move = Some(mv);
        }

        alpha = alpha.max(best_value);
    }

    best_move.map(|mv| (mv, best_value, node_count))
}

pub fn find_best_move_iterative(board: &ChessBoard, time_limit: Duration) -> Option<(Move, i32, u64, i32)> {
    let mut best_move = None;
    let mut best_score = i32::MIN;
    let mut total_node_count = 0;

    let start_time = Instant::now();
    let mut depth = 1;

    while start_time.elapsed() < time_limit {
        let remaining_time = time_limit - start_time.elapsed();

        // Call the existing find_best_move function for the current depth.
        if let Some((current_move, current_score, node_count)) = find_best_move_random_with_timeout(board, depth, remaining_time) {
            best_move = Some((current_move, current_score, total_node_count + node_count, depth));
            best_score = current_score;
            total_node_count += node_count;
        } else {
            break;
        }

        depth += 1; // Increase the depth for the next iteration
    }

    best_move
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_board::ChessBoard;

    #[test]
    fn test_some_positions() {
        let mut board = ChessBoard::from_fen("8/4p3/8/3P4/8/8/8/8 b - - 0 1").unwrap();
        if let Some((best_move, score, nodes)) = find_best_move(&board, 3) {
            println!(
                "Best move: {} with score: {} evaluated nodes: {}",
                best_move.as_algebraic(),
                score,
                nodes
            );
        } else {
            println!("No best move found!");
        }

        let mut board = ChessBoard::from_fen("8/7k/5KR1/8/8/8/8/8 w - - 0 1").unwrap();
        if let Some((best_move, score, nodes)) = find_best_move(&board, 5) {
            println!(
                "Best move: {} with score: {} evaluated nodes: {}",
                best_move.as_algebraic(),
                score,
                nodes
            );
        } else {
            println!("No best move found!");
        }

        let mut board = ChessBoard::from_fen("4k1nr/2p3p1/b2pPp1p/8/1nN1P1P1/p1R2N2/PR3P2/5K2 b k - 1 26").unwrap();
        if let Some((best_move, score, nodes)) = find_best_move(&board, 5) {
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
        let mut board = ChessBoard::from_fen("4k1nr/2p3p1/b2pPp1p/8/1nN1P1P1/p1R2N2/PR3P2/5K2 b k - 1 26").unwrap();
        if let Some((best_move, score, nodes)) = find_best_move(&board, 5) {
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
    fn test_perpetual_check() {
        let mut board = ChessBoard::from_fen("1k1r2rq/6pp/Q7/8/8/8/6PP/7K w - - 0 1").unwrap();
        board.make_move(Move::from_algebraic("a6b6"));
        if let Some((best_move, score, nodes)) = find_best_move(&board, 7) {
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
}