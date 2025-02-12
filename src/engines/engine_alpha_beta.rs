use crate::chess_board::{ChessBoard, Color, Move, PieceType, Square};
use crate::engines::{ChessEngine, InfoCallback};
use rand::prelude::SliceRandom;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::collections::BTreeMap;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::{Instant, SystemTime};

pub const MAX_PLY: usize = 20;
const MIN_EVALUATION: i32 = i32::MIN + 1; // +1 is important because -MIN is not a i32 number
const WIN: i32 = 10_000_000;
const LOSS: i32 = -10_000_000;
const DRAW: i32 = 0;

pub struct AlphaBetaEngine {
    board: ChessBoard,
    principal_variation: [([Move; MAX_PLY], usize); MAX_PLY],
    max_depth: usize,
    aborted: Arc<AtomicBool>,
    last_pvs: Vec<Move>,
    repetition_map: BTreeMap<u64, u8>,
}

impl AlphaBetaEngine {
    pub fn new() -> Self {
        AlphaBetaEngine {
            board: ChessBoard::new(),
            principal_variation: [([Move::new(99, 99, 99, 99); MAX_PLY], 0); MAX_PLY],
            max_depth: 20,
            aborted: Arc::new(AtomicBool::new(false)),
            last_pvs: Vec::new(),
            repetition_map: BTreeMap::new(),
        }
    }

    pub fn with_board(board: ChessBoard) -> Self {
        let mut engine = AlphaBetaEngine::new();
        engine.board = board;
        engine.insert_hash(engine.board.hash);
        engine
    }
}
impl ChessEngine for AlphaBetaEngine {
    fn name(&self) -> &str {
        "Chic Alpha Beta Engine"
    }
    fn author(&self) -> &str {
        "Cyril Flaig"
    }
    fn set_position(&mut self, position: &str) -> Result<(), String> {
        self.board = ChessBoard::from_fen(position)?;
        self.repetition_map.clear();
        self.insert_hash(self.board.hash);
        Ok(())
    }
    fn make_move(&mut self, move_algebraic_notation: &str) -> Result<(), &'static str> {
        let mv = Move::from_algebraic(move_algebraic_notation);
        self.board.make_move(mv);
        if self.board.halfmove_clock == 0 {
            self.repetition_map.clear();
        }
        self.insert_hash(self.board.hash);
        Ok(())
    }
    fn find_best_move_iterative(
        &mut self,
        time_limit: Duration,
        info_callback: InfoCallback,
    ) -> Option<(Vec<Move>, i32, u64, i32)> {
        let mut best_move = None;
        let mut total_node_count = 0;

        self.aborted.store(false, Relaxed);

        let start_time = Instant::now();
        let mut depth = 1;

        while start_time.elapsed() < time_limit {
            let remaining_time = time_limit - start_time.elapsed();

            // Call the existing find_best_move function for the current depth.
            if let Some((current_move, current_score, node_count)) =
                self.find_best_move_with_timeout(depth, false, remaining_time)
            {
                best_move = Some((
                    self.principal_variation[0].0[0..self.principal_variation[0].1].to_vec(),
                    current_score,
                    total_node_count + node_count,
                    depth,
                ));
                total_node_count += node_count;
                let pv = self.principal_variation[0].0[0..self.principal_variation[0].1]
                    .iter()
                    .map(|mv| mv.as_algebraic())
                    .collect::<Vec<_>>()
                    .join(" ");
                info_callback(depth, current_score, total_node_count, start_time.elapsed(), pv);
                self.last_pvs = self.principal_variation[0].0[0..self.principal_variation[0].1].iter().rev().map(|c|c.clone()).collect();

                depth += 1; // Increase the depth for the next iteration
            } else {
                break;
            }
        }

        best_move
    }
    fn get_active_player(&self) -> Color {
        self.board.active_color
    }

    fn get_abort_channel(&self) -> Arc<AtomicBool> {
        self.aborted.clone()
    }

    fn render_board(&self) {
        println!("{}", self.board.render_to_string());
    }
}

impl AlphaBetaEngine {
    #[allow(dead_code)]
    pub fn find_best_move(&mut self, depth: i32, random: bool) -> Option<(Move, i32, u64)> {
        self.find_best_move_with_timeout(depth, random, Duration::from_secs(60 * 60))
    }
    pub fn find_best_move_with_timeout(
        &mut self,
        depth: i32,
        random: bool,
        remaining_time: Duration,
    ) -> Option<(Move, i32, u64)> {
        let mut best_move = None;
        let mut best_score = i32::MIN;
        let mut node_count = 0;

        let deadline = Instant::now() + remaining_time;


        let mut moves = self.board.generate_legal_moves();
        if random {
            moves.shuffle(&mut rand::thread_rng());
        }

        let mut alpha = MIN_EVALUATION;
        for mv in moves {
            if Instant::now() > deadline || self.aborted.load(Relaxed) {
                return None;
            }
            let mut new_board = self.board.clone();
            new_board.make_move(mv);

            let score = match self.negamax(&new_board, depth, MIN_EVALUATION, -alpha, 1, deadline, &mut node_count) {
                None => return None,
                Some(score) => -score,
            };

            if score > best_score {
                alpha = score;
                best_score = score;
                best_move = Some(mv);
                self.save_principal_variation(mv, depth as usize, 0);
            }
            //println!("With depth {} Move: {} Score: {}", depth, mv.as_algebraic(), score);
        }

        best_move.map(|mv| (mv, best_score, node_count))
    }

    fn negamax(
        &mut self,
        board: &ChessBoard,
        depth: i32,
        alpha: i32,
        beta: i32,
        ply: usize,
        deadline: Instant,
        node_count: &mut u64,
    ) -> Option<i32> {
        if Instant::now() > deadline || self.aborted.load(Relaxed) {
            return None;
        }
        *node_count += 1;
        self.principal_variation[ply].1 = 0;

        let hash = board.hash;
        if let Some(count) = self.repetition_map.get(&hash) {
            if *count == 2 {
                return Some(0);
            }
        }
        self.insert_hash(hash);

        if depth <= 0 || ply > MAX_PLY {
            *node_count -= 1;
            self.remove_hash(&hash);

            return AlphaBetaEngine::quiescence_search_prunning(
                board,
                node_count,
                alpha,
                beta,
                deadline,
                &self.aborted,
            );
        }

        let mut alpha = alpha;
        let mut max_score = MIN_EVALUATION;

        let moves = board.generate_legal_moves();
        if moves.is_empty() {
            // Handle checkmate or stalemate
            if board.is_checkmate() {
                self.remove_hash(&hash);
                return Some(LOSS - depth);
            } else if board.is_stalemate() {
                self.remove_hash(&hash);
                return Some(DRAW);
            }
        }

        for mv in moves {
            let mut new_board = board.clone();
            new_board.make_move(mv);
            let score = match self.negamax(&new_board, depth - 1, -beta, -alpha, ply + 1, deadline, node_count) {
                None => {
                    self.remove_hash(&hash);
                    return None;
                }
                Some(score) => -score,
            };
            if score > max_score {
                max_score = score;
                if score > alpha {
                    alpha = score;
                    self.save_principal_variation(mv, depth as usize, ply);
                    if alpha >= beta {
                        // Beta cutoff fail soft
                        break;
                    }
                }
            }
        }

        self.remove_hash(&hash);
        Some(max_score)
    }

    fn remove_hash(&mut self, hash: &u64) {
        if let Some(count) = self.repetition_map.get_mut(hash) {
            if *count > 1 {
                *count -= 1;
            } else {
                self.repetition_map.remove(hash);
            }
        }
    }

    fn insert_hash(&mut self, hash: u64) {
        match self.repetition_map.get_mut(&hash) {
            Some(count) => *count += 1,
            None => {
                self.repetition_map.insert(hash, 1);
            }
        }
    }

    fn save_principal_variation(&mut self, mv: Move, depth: usize, ply: usize) {
        self.principal_variation[ply].0[0] = mv;
        for i in 0..self.principal_variation[ply + 1].1 {
            self.principal_variation[ply].0[i + 1] = self.principal_variation[ply + 1].0[i];
        }
        self.principal_variation[ply].1 = self.principal_variation[ply + 1].1 + 1;
    }

    fn quiescence_search_prunning(
        board: &ChessBoard,
        node_count: &mut u64,
        mut alpha: i32,
        beta: i32,
        deadline: Instant,
        aborted: &Arc<AtomicBool>,
    ) -> Option<i32> {
        if Instant::now() > deadline || aborted.load(Relaxed) {
            return None;
        }
        *node_count += 1;

        let stand_pat =
            AlphaBetaEngine::evaluate_board(board) * if board.active_color == Color::White { 1 } else { -1 };
        let mut max_score = stand_pat;
        alpha = alpha.max(stand_pat);

        if alpha >= beta {
            return Some(max_score);
        }

        let moves = board.generate_legal_capture_moves();

        //println!("Number of Capture Moves: {}", moves.len() );

        for mv in moves {
            let mut new_board = board.clone();
            new_board.make_move(mv);
            let score = match AlphaBetaEngine::quiescence_search_prunning(
                &new_board, node_count, -beta, -alpha, deadline, aborted,
            ) {
                None => return None,
                Some(score) => -score,
            };
            max_score = max_score.max(score);
            alpha = alpha.max(score);
            if alpha >= beta {
                // Beta cutoff
                break;
            }
        }
        Some(max_score)
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
                            PieceType::King => AlphaBetaEngine::KING_SQUARE_TABLE[psq_row][col],
                            PieceType::Pawn => AlphaBetaEngine::PAWN_SQUARE_TABLE[psq_row][col],
                            PieceType::Knight => AlphaBetaEngine::KNIGHT_SQUARE_TABLE[psq_row][col],
                            PieceType::Bishop => AlphaBetaEngine::BISHOP_SQUARE_TABLE[psq_row][col],
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_board::ChessBoard;

    #[test]
    fn test_some_positions() {
        let mut engine = AlphaBetaEngine::new();
        engine.set_position("8/4p3/8/3P4/8/8/8/8 b - - 0 1");
        if let Some((best_move, score, nodes)) = engine.find_best_move(2, false) {
            assert!(false); //no valid position
        } else {
            println!("No best move found!");
        }

        let mut engine = AlphaBetaEngine::new();
        engine.set_position("8/7k/5KR1/8/8/8/8/8 w - - 0 1");
        let depth = 5usize;
        if let Some((best_move, score, nodes)) = engine.find_best_move(depth as i32, false) {
            println!(
                "Best move: {} with score: {} evaluated nodes: {}",
                best_move.as_algebraic(),
                score,
                nodes
            );
            println!(
                "Principal variation: {}",
                engine.principal_variation[0].0[0..engine.principal_variation[0].1]
                    .iter()
                    .map(|mv| mv.as_algebraic())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        } else {
            println!("No best move found!");
        }

        let depth = 6usize;
        let mut engine = AlphaBetaEngine::new();
        engine.set_position("4k1nr/2p3p1/b2pPp1p/8/1nN1P1P1/p1R2N2/PR3P2/5K2 b k - 1 26");
        if let Some((best_move, score, nodes)) = engine.find_best_move(depth as i32, false) {
            println!(
                "Best move: {} with score: {} evaluated nodes: {}",
                best_move.as_algebraic(),
                score,
                nodes
            );
            println!(
                "Principal variation: {}",
                engine.principal_variation[0].0[0..engine.principal_variation[0].1]
                    .iter()
                    .map(|mv| mv.as_algebraic())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        } else {
            println!("No best move found!");
        }
    }

    #[test]
    fn test_from_a_played_position() {
        let mut engine = AlphaBetaEngine::new();
        engine.set_position("4k1nr/2p3p1/b2pPp1p/8/1nN1P1P1/p1R2N2/PR3P2/5K2 b k - 1 26");
        if let Some((best_move, score, nodes)) = engine.find_best_move(0, false) {
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
        let mut engine = AlphaBetaEngine::new();
        engine.set_position("rnbqkbnr/p1p2ppp/1p1p4/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4");
        if let Some((best_move, score, nodes)) = engine.find_best_move(0, false) {
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
        println!("Evaluation: {}", AlphaBetaEngine::evaluate_board(&board));
    }

    #[test]
    fn test_perpetual_check() {
        let mut engine = AlphaBetaEngine::new();
        engine.set_position("1k1r2rq/6pp/Q7/8/8/8/6PP/7K w - - 0 1");
        engine.make_move("a6b6");
        engine.make_move("b8a8");
        engine.make_move("b6a6");
        engine.make_move("a8b8");

        let depth = 5usize;
        for depth in 0..6 {
            if let Some((best_move, score, nodes)) = engine.find_best_move(depth, false) {
                println!(
                    "Best move: {} with score: {} evaluated nodes: {}",
                    best_move.as_algebraic(),
                    score,
                    nodes
                );
                println!(
                    "Principal variation: {}",
                    engine.principal_variation[0].0[0..engine.principal_variation[0].1]
                        .iter()
                        .map(|mv| mv.as_algebraic())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            } else {
                println!("No best move found!");
            }
        }
    }
}
