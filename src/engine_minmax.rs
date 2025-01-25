use crate::chess_board::Square::Empty;
use crate::chess_board::{ChessBoard, Color, Move, PieceType, Square};
use rand::prelude::SliceRandom;
use std::time::{Duration, Instant};

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
        let last_capture_move = if new_board.squares[mv.to.row][mv.to.col] == Empty {
            None
        } else {
            Some(mv)
        };
        new_board.make_move(mv);

        // Negamax for the opponent's position (invert the returned evaluation)
        let score = -negamax(&new_board, depth, &mut node_count, last_capture_move);

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

fn negamax(board: &ChessBoard, depth: i32, node_count: &mut u64, last_capture_move: Option<Move>) -> i32 {
    *node_count += 1;
    if board.is_threefold_repetition() {
        return 0;
    }
    if depth <= 0 {
        return match last_capture_move {
            None => evaluate_board(board) * if board.active_color == Color::White { 1 } else { -1 },
            Some(mv) => {
                *node_count -= 1;
                quiescence_search(board, node_count, &mv)
            }
        };
    }

    let mut max_score = MIN_EVALUATION;

    for mv in board.generate_pseudo_moves() {
        let mut new_board = board.clone();
        let last_capture_move = if new_board.squares[mv.to.row][mv.to.col] == Empty {
            None
        } else {
            Some(mv)
        };
        new_board.make_move(mv);
        let king_position = new_board.find_king_position(board.active_color);
        if let Some(king_pos) = king_position {
            if !new_board.is_square_attacked_by_color(king_pos.row, king_pos.col, new_board.active_color) {
                // No legal move
                // Negate the evaluation of the next level (opponent's perspective)
                let score = -negamax(&new_board, depth - 1, node_count, last_capture_move);
                max_score = max_score.max(score);
            }
        }
    }
    if max_score == MIN_EVALUATION {
        //No legal moves
        if let Some(king_pos) = board.find_king_position(board.active_color) {
            if board.is_square_attacked(king_pos.row, king_pos.col) {
                LOSS - depth // Closer loss is punished harder
            } else {
                DRAW // Stalemate
            }
        } else {
            DRAW //Illegal state when no king is found
        }
    } else {
        max_score
    }
}

fn quiescence_search(board: &ChessBoard, node_count: &mut u64, &last_move: &Move) -> i32 {
    *node_count += 1;

    let mut max_score = MIN_EVALUATION;

    let moves = board.generate_legal_capture_moves();

    //println!("Number of Capture Moves: {}", moves.len() );

    for mv in moves
        .iter()
        .filter(|mv| mv.to.row == last_move.to.row && mv.to.col == last_move.to.col)
    {
        let mut new_board = board.clone();
        new_board.make_move(*mv);
        let score = -quiescence_search(&new_board, node_count, &last_move);
        max_score = max_score.max(score);
    }
    if max_score == MIN_EVALUATION {
        evaluate_board(board) * if board.active_color == Color::White { 1 } else { -1 }
    } else {
        max_score
    }
}

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

                    evaluation += match piece.color {
                        Color::White => piece_value,
                        Color::Black => -piece_value,
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
}
