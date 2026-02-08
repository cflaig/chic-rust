use chic_lib::chess_boards::chess_board::{ChessBoard, Move};
use chic_lib::engines::engine_alpha_beta::AlphaBetaEngine;
use chic_lib::engines::ChessEngine;
use std::sync::mpsc::{channel, Receiver};

#[derive(Debug, Clone)]
pub enum EngineMessage {
    Move(Move),
    Error(String),
}

/// Start engine computation for the given board position.
/// Returns a receiver that will deliver the engine's move when ready.
///
/// Native version: Runs in a separate thread with 7-second timeout.
/// WASM version: Runs synchronously with 2-second timeout to prevent UI freeze.
#[cfg(not(target_arch = "wasm32"))]
pub fn start_engine_move(board: ChessBoard) -> Receiver<EngineMessage> {
    let (tx, rx) = channel();

    std::thread::spawn(move || {
        let mut engine = AlphaBetaEngine::with_board(board);

        match engine.find_best_move_iterative(
            std::time::Duration::from_secs(7),
            |_depth, _seldepth, _eval, _nodes, _elapsed, _pv| {
                // No-op callback
            },
        ) {
            Some((best_moves, score, node_count, depth)) => {
                if !best_moves.is_empty() {
                    println!(
                        "Engine found move: {} (score: {}, nodes: {}, depth: {})",
                        best_moves[0].as_algebraic(),
                        score,
                        node_count,
                        depth
                    );
                    let _ = tx.send(EngineMessage::Move(best_moves[0]));
                } else {
                    let _ = tx.send(EngineMessage::Error("No move in result".to_string()));
                }
            }
            None => {
                let _ = tx.send(EngineMessage::Error("No best move found".to_string()));
            }
        }
    });

    rx
}

/// WASM version: Runs synchronously with reduced timeout
#[cfg(target_arch = "wasm32")]
pub fn start_engine_move(board: ChessBoard) -> Receiver<EngineMessage> {
    let (tx, rx) = channel();

    let mut engine = AlphaBetaEngine::with_board(board);

    match engine.find_best_move_iterative(
        std::time::Duration::from_secs(2),
        |_depth, _seldepth, _eval, _nodes, _elapsed, _pv| {
            // No-op callback
        },
    ) {
        Some((best_moves, score, node_count, depth)) => {
            if !best_moves.is_empty() {
                println!(
                    "Engine found move: {} (score: {}, nodes: {}, depth: {})",
                    best_moves[0].as_algebraic(),
                    score,
                    node_count,
                    depth
                );
                let _ = tx.send(EngineMessage::Move(best_moves[0]));
            } else {
                let _ = tx.send(EngineMessage::Error("No move in result".to_string()));
            }
        }
        None => {
            let _ = tx.send(EngineMessage::Error("No best move found".to_string()));
        }
    }

    rx
}
