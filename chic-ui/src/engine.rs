use chic_lib::chess_boards::chess_board::{ChessBoard, Move};
use chic_lib::engines::engine_alpha_beta::AlphaBetaEngine;
use chic_lib::engines::ChessEngine;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone)]
pub enum EngineMessage {
    Move(Move),
    Analysis {
        depth: usize,
        eval: i32,           // Centipawns (positive = White advantage)
        pv: String,          // Space-separated algebraic moves
        nodes: u64,
        elapsed_ms: u64,
    },
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

// Global channel for analysis callbacks
thread_local! {
    static ANALYSIS_TX: std::cell::RefCell<Option<std::sync::mpsc::Sender<EngineMessage>>> = std::cell::RefCell::new(None);
}

fn analysis_callback(depth: usize, _seldepth: usize, eval: i32, nodes: u64, elapsed: std::time::Duration, pv: String) {
    ANALYSIS_TX.with(|tx| {
        if let Some(sender) = tx.borrow().as_ref() {
            let _ = sender.send(EngineMessage::Analysis {
                depth,
                eval,
                pv,
                nodes,
                elapsed_ms: elapsed.as_millis() as u64,
            });
        }
    });
}

/// Start continuous engine analysis of the given position
/// Sends Analysis messages with updates at each depth
/// Uses unlimited timeout like UCI "go infinite" command
/// Returns (receiver, abort_channel) - use abort to stop analysis
#[cfg(not(target_arch = "wasm32"))]
pub fn start_engine_analysis(board: ChessBoard) -> (Receiver<EngineMessage>, Arc<AtomicBool>) {
    let (tx, rx) = channel();
    let mut engine = AlphaBetaEngine::with_board(board);
    let abort_channel = engine.get_abort_channel();
    let abort_clone = abort_channel.clone();

    std::thread::spawn(move || {
        // Set the thread-local sender
        ANALYSIS_TX.with(|analysis_tx| {
            *analysis_tx.borrow_mut() = Some(tx.clone());
        });

        // Use unlimited timeout for analysis (like UCI "go infinite")
        // 10 days should be sufficient for any practical analysis session
        let timeout = std::time::Duration::from_secs(60 * 60 * 24 * 10);

        match engine.find_best_move_iterative(timeout, analysis_callback) {
            Some(_) => {
                // Analysis completed (extremely unlikely with 10-day timeout)
            }
            None => {
                let _ = tx.send(EngineMessage::Error("Analysis failed".to_string()));
            }
        }

        // Clean up thread-local
        ANALYSIS_TX.with(|analysis_tx| {
            *analysis_tx.borrow_mut() = None;
        });
    });

    (rx, abort_clone)
}

/// WASM version: Use shorter timeout, run synchronously
#[cfg(target_arch = "wasm32")]
pub fn start_engine_analysis(board: ChessBoard) -> (Receiver<EngineMessage>, Arc<AtomicBool>) {
    let (tx, rx) = channel();

    // Set the thread-local sender
    ANALYSIS_TX.with(|analysis_tx| {
        *analysis_tx.borrow_mut() = Some(tx.clone());
    });

    let mut engine = AlphaBetaEngine::with_board(board);
    let abort_channel = engine.get_abort_channel();
    let timeout = std::time::Duration::from_secs(5);

    match engine.find_best_move_iterative(timeout, analysis_callback) {
        Some(_) => {}
        None => {
            let _ = tx.send(EngineMessage::Error("Analysis failed".to_string()));
        }
    }

    // Clean up thread-local
    ANALYSIS_TX.with(|analysis_tx| {
        *analysis_tx.borrow_mut() = None;
    });

    (rx, abort_channel)
}
