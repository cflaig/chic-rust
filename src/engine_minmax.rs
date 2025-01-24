use rand::prelude::SliceRandom;
use crate::chess_board::{ChessBoard, Color, Move, PieceType, Square};

pub fn find_best_move(board: &ChessBoard, depth: i32) -> Option<(Move, i32, u64)> {
    let mut best_move = None;
    let mut best_score = i32::MIN;
    let mut node_count = 0;

    for mv in board.generate_legal_moves() {
        let mut new_board = board.clone();
        new_board.make_move(mv);

        // Negamax for the opponent's position (invert the returned evaluation)
        let score = -negamax(&new_board, depth, &mut node_count);

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
    }

    best_move.map(|mv| (mv, best_score, node_count))
}

pub fn find_best_move_random(board: &ChessBoard, depth: i32) -> Option<(Move, i32, u64)> {
    let mut best_moves = Vec::new();
    let mut best_score = i32::MIN;
    let mut node_count = 0;

    for mv in board.generate_legal_moves() {
        let mut new_board = board.clone();
        new_board.make_move(mv);

        // Negamax for the opponent's position (invert the returned evaluation)
        let score = -negamax(&new_board, depth, &mut node_count);

        if score > best_score {
            best_score = score;
            best_moves.clear();
            best_moves.push(mv);
        } else if score == best_score {
            best_moves.push(mv);
        }
    }
    if !best_moves.is_empty() {
        // Randomly pick one of the best moves if there's a tie
        let mut rng = rand::thread_rng();
        let selected_move = best_moves.choose(&mut rng)?.clone();
        return Some((selected_move, best_score, node_count));
    }

    None
}

const MIN_EVALUATION: i32 = i32::MIN + 1; // +1 is important because -MIN is not a i32 number
const WIN: i32 = 10_000_000;
const LOSS: i32 = -10_000_000;
const DRAW: i32 = 0;

fn negamax(board: &ChessBoard, depth: i32, node_count: &mut u64) -> i32 {
    *node_count += 1;
    if board.is_threefold_repetition() {
        return 0;
    }
    if depth == 0 {
        return evaluate_board(board) * if board.active_color == Color::White { 1 } else { -1 };
    }

    let mut max_score = MIN_EVALUATION;

    for mv in board.generate_pseudo_moves() {
        let mut new_board = board.clone();
        new_board.make_move(mv);
        let king_position = new_board.find_king_position(board.active_color);
        if let Some(king_pos) = king_position {
            if !new_board.is_square_attacked_by_color(king_pos.row, king_pos.col, new_board.active_color) {
                // No legal move
                // Negate the evaluation of the next level (opponent's perspective)
                let score = -negamax(&new_board, depth - 1, node_count);
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
}
