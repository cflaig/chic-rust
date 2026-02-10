use crate::chess_boards::chess_board::ChessBoard;
use crate::chess_boards::chess_board::{Color, Move, PieceType};
use crate::engines::{ChessEngine, InfoCallback};
use std::cmp::min;
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::{Instant, SystemTime};

pub const MAX_PLY: usize = 20;
const MIN_EVALUATION: i32 = i32::MIN + 1; // +1 is important because -MIN is not a i32 number
pub const WIN: i32 = 10_000_000;
const LOSS: i32 = -10_000_000;
const DRAW: i32 = 0;

pub struct AlphaBetaEngine {
    board: ChessBoard,
    principal_variation: [([Move; MAX_PLY], usize); MAX_PLY],
    max_depth: usize,
    current_max_depth: usize,
    aborted: Arc<AtomicBool>,
    last_pvs: Vec<Move>,
    repetition_map: BTreeMap<u64, u8>,
    killer_moves: [[Option<Move>; 2]; MAX_PLY], // 2 killer moves per ply
}

impl AlphaBetaEngine {
    pub fn new() -> Self {
        AlphaBetaEngine {
            board: ChessBoard::new(),
            principal_variation: [([Move::new(99, 99, 99, 99); MAX_PLY], 0); MAX_PLY],
            max_depth: 20,
            current_max_depth: 0,
            aborted: Arc::new(AtomicBool::new(false)),
            last_pvs: Vec::new(),
            repetition_map: BTreeMap::new(),
            killer_moves: [[None; 2]; MAX_PLY],
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
        self.clear_killer_moves();

        let start_time = Instant::now();
        let mut depth = 1;

        while start_time.elapsed() < time_limit {
            let remaining_time = time_limit - start_time.elapsed();
            self.current_max_depth = depth;

            // Call the existing find_best_move function for the current depth.
            if let Some((_move, current_score, node_count)) =
                self.find_best_move_with_timeout(depth as i32, false, remaining_time)
            {
                best_move = Some((
                    self.principal_variation[0].0[0..self.principal_variation[0].1].to_vec(),
                    current_score,
                    total_node_count + node_count,
                    depth as i32,
                ));
                total_node_count += node_count;
                let pv = self.principal_variation[0].0[0..self.principal_variation[0].1]
                    .iter()
                    .map(|mv| mv.as_algebraic())
                    .collect::<Vec<_>>()
                    .join(" ");

                info_callback(
                    depth,
                    self.current_max_depth,
                    current_score,
                    total_node_count,
                    start_time.elapsed(),
                    pv,
                );
                self.last_pvs = self.principal_variation[0].0[0..self.principal_variation[0].1]
                    .iter()
                    .copied()
                    .collect();

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
        _random: bool,
        remaining_time: Duration,
    ) -> Option<(Move, i32, u64)> {
        let mut best_move = None;
        let mut best_score = i32::MIN;
        let mut node_count = 0;
        let mut is_principal_variation = true;

        let deadline = Instant::now() + remaining_time;

        let last_pv_move = if self.last_pvs.is_empty() {
            None
        } else {
            Some(self.last_pvs[0])
        };

        let moves = self.board.generate_legal_moves(last_pv_move);

        let mut alpha = MIN_EVALUATION;
        for mv in moves {
            if Instant::now() > deadline || self.aborted.load(Relaxed) {
                return None;
            }
            let mut new_board = self.board.clone();
            new_board.make_move(mv);
            let hash = new_board.hash;
            self.insert_hash(hash);

            let score = match self.negamax(
                &new_board,
                depth - 1,
                MIN_EVALUATION,
                -alpha,
                1,
                is_principal_variation,
                deadline,
                &mut node_count,
            ) {
                None => {
                    self.remove_hash(&hash);
                    return None;
                }
                Some(score) => -score,
            };
            self.remove_hash(&hash);
            is_principal_variation = false;

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
        mut is_principal_variation: bool,
        deadline: Instant,
        node_count: &mut u64,
    ) -> Option<i32> {
        if Instant::now() > deadline || self.aborted.load(Relaxed) {
            return None;
        }
        *node_count += 1;
        self.principal_variation[ply].1 = 0;

        if let Some(count) = self.repetition_map.get(&board.hash) {
            if *count == 2 {
                return Some(0);
            }
        }

        if depth <= 0 || ply > MAX_PLY {
            *node_count -= 1;
            return AlphaBetaEngine::quiescence_search_prunning(
                board,
                alpha,
                beta,
                node_count,
                ply,
                &mut self.current_max_depth,
                deadline,
                &self.aborted,
            );
        }

        let mut alpha = alpha;
        let mut max_score = MIN_EVALUATION;

        let last_pv_move = if (0..self.last_pvs.len()).contains(&ply) && is_principal_variation {
            Some(self.last_pvs[ply])
        } else {
            None
        };

        let mut moves: Vec<(i32, Move)> = board.generate_legal_moves(last_pv_move).into_vec();

        // Apply killer move bonuses
        for (score, mv) in moves.iter_mut() {
            // Check if this move is a killer move
            if let Some(killer1) = self.killer_moves[ply][0] {
                if *mv == killer1 {
                    *score = 9500; // Just below good captures (10000+), above bad captures
                    continue;
                }
            }
            if let Some(killer2) = self.killer_moves[ply][1] {
                if *mv == killer2 {
                    *score = 9400; // Slightly lower than killer1
                }
            }
        }

        // Re-sort after applying killer bonuses
        moves.sort_by(|a, b| b.0.cmp(&a.0));
        if moves.is_empty() {
            // Handle checkmate or stalemate
            if board.is_checkmate() {
                return Some(LOSS - depth);
            } else if board.is_stalemate() {
                return Some(DRAW);
            }
        }

        let mut move_count = 0;
        for (_score, mv) in moves {
            let mut new_board = board.clone();
            new_board.make_move(mv);
            let hash = new_board.hash;
            self.insert_hash(hash);

            // Late Move Reductions (LMR)
            // Reduce search depth for moves that are likely not best
            let mut reduction = 0;
            let is_capture = board.squares[mv.to.row as usize][mv.to.col as usize] != crate::chess_boards::chess_board::Square::Empty;
            let gives_check = new_board.is_in_check();

            // Apply LMR if:
            // - Not in PV line
            // - Not first few moves (likely to be good)
            // - Sufficient depth remaining
            // - Move is not tactical (not capture, not check, not promotion)
            if !is_principal_variation
                && move_count >= 3
                && depth >= 3
                && !is_capture
                && !gives_check
                && mv.promotion.is_none() {
                // Calculate reduction based on depth and move number
                reduction = if move_count >= 6 && depth >= 5 {
                    2
                } else {
                    1
                };
            }

            // Search with reduced depth first
            let mut score = match self.negamax(
                &new_board,
                depth - 1 - reduction,
                -beta,
                -alpha,
                ply + 1,
                is_principal_variation,
                deadline,
                node_count,
            ) {
                None => {
                    self.remove_hash(&hash);
                    return None;
                }
                Some(score) => -score,
            };

            // Re-search at full depth if reduced search suggests the move is good
            if reduction > 0 && score > alpha {
                score = match self.negamax(
                    &new_board,
                    depth - 1,
                    -beta,
                    -alpha,
                    ply + 1,
                    false, // Not PV anymore after reduction
                    deadline,
                    node_count,
                ) {
                    None => {
                        self.remove_hash(&hash);
                        return None;
                    }
                    Some(score) => -score,
                };
            }

            move_count += 1;
            is_principal_variation = false;
            self.remove_hash(&hash);
            if score > max_score {
                max_score = score;
                if score > alpha {
                    alpha = score;
                    self.save_principal_variation(mv, depth as usize, ply);
                    if alpha >= beta {
                        // Beta cutoff - update killer moves for quiet moves
                        if !is_capture && mv.promotion.is_none() {
                            self.update_killer_moves(mv, ply);
                        }
                        break;
                    }
                }
            }
        }

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

    fn save_principal_variation(&mut self, mv: Move, _depth: usize, ply: usize) {
        self.principal_variation[ply].0[0] = mv;
        for i in 0..self.principal_variation[ply + 1].1 {
            self.principal_variation[ply].0[i + 1] = self.principal_variation[ply + 1].0[i];
        }
        self.principal_variation[ply].1 = self.principal_variation[ply + 1].1 + 1;
    }

    fn update_killer_moves(&mut self, mv: Move, ply: usize) {
        // Don't store if it's already the first killer
        if let Some(killer1) = self.killer_moves[ply][0] {
            if killer1 == mv {
                return;
            }
        }

        // Shift: killer1 -> killer2, new move -> killer1
        self.killer_moves[ply][1] = self.killer_moves[ply][0];
        self.killer_moves[ply][0] = Some(mv);
    }

    fn clear_killer_moves(&mut self) {
        self.killer_moves = [[None; 2]; MAX_PLY];
    }

    fn quiescence_search_prunning(
        board: &ChessBoard,
        mut alpha: i32,
        beta: i32,
        node_count: &mut u64,
        ply: usize,
        current_max_depth: &mut usize,
        deadline: Instant,
        aborted: &Arc<AtomicBool>,
    ) -> Option<i32> {
        if Instant::now() > deadline || aborted.load(Relaxed) {
            return None;
        }
        *node_count += 1;
        if ply > *current_max_depth {
            *current_max_depth = ply;
        }

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
                &new_board,
                -beta,
                -alpha,
                node_count,
                ply + 1,
                current_max_depth,
                deadline,
                aborted,
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
    [150, 150, 150, 150, 150, 150, 150, 150],
    [ 75,  50,  50,  50,  50,  50,  50,  75],
    [  0,   0,   0,  20,  20,   0,   0,   0],
    [  0,   0,  20,  25,  25,  20,   0,   0],
    [  0,   0,  15, -50, -50,  15,   0,   0],
    [  0,   0,   0,-250,-250,   0,   5,   5],
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
    [ -50,  -50,  -50,  -50,  -50, -100,  -50,  -50],
    [ 200,  250,  200,  -50,    0,  -50,  250,  200],
];

    #[rustfmt::skip]
    const KING_SQUARE_TABLE_ENDGAME: [[i32; 8]; 8] = [
    [-200,-100,-100,-100,-100,-100,-100,-200],
    [-100,   0,   0,   0,   0,   0,   0,-100],
    [-100,   0, 150, 150, 150, 150,   0,-100],
    [-100,   0, 150, 200, 200, 150,   0,-100],
    [-100,   0, 150, 200, 200, 150,   0,-100],
    [-100,   0, 150, 150, 150, 150,   0,-100],
    [-100,   0,   0,   0,   0,   0,   0,-100],
    [-200,-100,-100,-100,-100,-100,-100,-200],
    ];

    pub fn get_piece_type_index(piece: &PieceType) -> usize {
        match piece {
            PieceType::Pawn => 5,
            PieceType::Knight => 4,
            PieceType::Bishop => 3,
            PieceType::Rook => 2,
            PieceType::Queen => 1,
            PieceType::King => 0,
        }
    }

    const DOUBLED_PAWN_PENALTY: i32 = -150;

    const ISOLATED_PAWN_PENALTY: i32 = -250;
    const BACKWARDS_PAWN_PENALTY: i32 = -75;
    const PASSED_PAWN_BONUS: i32 = 250;
    const BISHOP_PAIR_BONUS: i32 = 300;

    /// Evaluates the board state and assigns a score based on material balance.
    fn evaluate_board(board: &ChessBoard) -> i32 {
        let mut evaluation = 0;
        let mut black_material = 0;
        let mut white_material = 0;

        const PIECE_TYPES: [(PieceType, i32); 4] = [
            (PieceType::Queen, 9_000),
            (PieceType::Rook, 5_000),
            (PieceType::Bishop, 3_000),
            (PieceType::Knight, 3_000),
        ];

        for (piece, value) in PIECE_TYPES {
            white_material += (board.white_pieces[AlphaBetaEngine::get_piece_type_index(&piece) + 1]
                - board.white_pieces[AlphaBetaEngine::get_piece_type_index(&piece)])
                as i32
                * value;
        }
        for (piece, value) in PIECE_TYPES {
            black_material += (board.black_pieces[AlphaBetaEngine::get_piece_type_index(&piece) + 1]
                - board.black_pieces[AlphaBetaEngine::get_piece_type_index(&piece)])
                as i32
                * value;
        }

        let black_material_pawns = (board.black_pieces[AlphaBetaEngine::get_piece_type_index(&PieceType::Pawn) + 1]
            - board.black_pieces[AlphaBetaEngine::get_piece_type_index(&PieceType::Pawn)])
            as i32
            * 1_000;
        let white_material_pawns = (board.white_pieces[AlphaBetaEngine::get_piece_type_index(&PieceType::Pawn) + 1]
            - board.white_pieces[AlphaBetaEngine::get_piece_type_index(&PieceType::Pawn)])
            as i32
            * 1_000;

        if board.white_pieces[AlphaBetaEngine::get_piece_type_index(&PieceType::Bishop) + 1]
            - board.white_pieces[AlphaBetaEngine::get_piece_type_index(&PieceType::Bishop)]
            > 1
        {
            white_material += Self::BISHOP_PAIR_BONUS;
        }

        if board.black_pieces[AlphaBetaEngine::get_piece_type_index(&PieceType::Bishop) + 1]
            - board.black_pieces[AlphaBetaEngine::get_piece_type_index(&PieceType::Bishop)]
            > 1
        {
            black_material += Self::BISHOP_PAIR_BONUS;
        }

        let mut pawn_evaluation = 0i32;
        let mut pawns_rank = [[7u8; 10]; 2]; //The board have on each side an empty raw
        pawn_evaluation += Self::generate_pawns_rank_and_check_on_isolated_pawn(board, Color::White, &mut pawns_rank);
        pawn_evaluation += Self::generate_pawns_rank_and_check_on_isolated_pawn(board, Color::Black, &mut pawns_rank);

        pawn_evaluation += Self::evaluate_pawns(board, &pawns_rank, Color::White);
        pawn_evaluation += Self::evaluate_pawns(board, &pawns_rank, Color::Black);

        for (field, piece) in board.all_pieces_with_coordinates() {
            //Check position value
            let psq_row = match piece.color {
                Color::White => 7 - field.row,
                Color::Black => field.row,
            };

            let use_endgame = match piece.color {
                Color::White => black_material <= 13_000,
                Color::Black => white_material <= 13_000,
            };

            let position_value = match piece.kind {
                PieceType::King => {
                    if use_endgame {
                        AlphaBetaEngine::KING_SQUARE_TABLE_ENDGAME[psq_row as usize][field.col as usize]
                    } else {
                        AlphaBetaEngine::KING_SQUARE_TABLE[psq_row as usize][field.col as usize]
                    }
                }
                PieceType::Pawn => {
                    if use_endgame {
                        (7 - psq_row) as i32 * 100
                    } else {
                        AlphaBetaEngine::PAWN_SQUARE_TABLE[psq_row as usize][field.col as usize]
                    }
                }
                PieceType::Knight => AlphaBetaEngine::KNIGHT_SQUARE_TABLE[psq_row as usize][field.col as usize],
                PieceType::Bishop => AlphaBetaEngine::BISHOP_SQUARE_TABLE[psq_row as usize][field.col as usize],
                _ => 0,
            };

            evaluation += match piece.color {
                Color::White => position_value,
                Color::Black => -position_value,
            };
        }

        evaluation + white_material + white_material_pawns - black_material - black_material_pawns + pawn_evaluation
    }

    fn evaluate_pawns(board: &ChessBoard, pawns_rank: &[[u8; 10]; 2], color: Color) -> i32 {
        let mut pawn_evaluation = 0i32;
        let pawn_rank_index = if color == Color::White { 0 } else { 1 };

        for (field, _) in board.iter_pieces(color, PieceType::Pawn) {
            let projected_row = if color == Color::White {
                field.row
            } else {
                7 - field.row
            };
            if pawns_rank[pawn_rank_index][field.col as usize] == 7
                && pawns_rank[pawn_rank_index][field.col as usize + 2] == 7
            {
                pawn_evaluation += Self::ISOLATED_PAWN_PENALTY;
            } else if pawns_rank[pawn_rank_index][field.col as usize] > projected_row
                && pawns_rank[pawn_rank_index][field.col as usize + 2] > projected_row
            {
                pawn_evaluation += Self::BACKWARDS_PAWN_PENALTY;
            }
            if pawns_rank[1 - pawn_rank_index][field.col as usize] >= 7 - projected_row
                && pawns_rank[1 - pawn_rank_index][field.col as usize + 1] >= 7 - projected_row
                && pawns_rank[1 - pawn_rank_index][field.col as usize + 2] >= 7 - projected_row
            {
                pawn_evaluation += projected_row as i32 * Self::PASSED_PAWN_BONUS;
            }
        }
        pawn_evaluation * if color == Color::White { 1 } else { -1 }
    }

    fn generate_pawns_rank_and_check_on_isolated_pawn(
        board: &ChessBoard,
        color: Color,
        pawns_rank: &mut [[u8; 10]; 2],
    ) -> i32 {
        let mut pawn_evaluation = 0i32;
        let pawn_rank_index = if color == Color::White { 0 } else { 1 };

        for (field, _) in board.iter_pieces(color, PieceType::Pawn) {
            let projected_row = if color == Color::White {
                field.row
            } else {
                7 - field.row
            };
            if pawns_rank[pawn_rank_index][field.col as usize + 1] != 7 {
                pawn_evaluation += Self::DOUBLED_PAWN_PENALTY;
                pawns_rank[pawn_rank_index][field.col as usize + 1] =
                    min(pawns_rank[pawn_rank_index][field.col as usize + 1], projected_row);
            } else {
                pawns_rank[pawn_rank_index][field.col as usize + 1] = projected_row;
            }
        }
        pawn_evaluation * if color == Color::White { 1 } else { -1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_boards::chess_board::ChessBoard;

    #[test]
    fn test_some_positions() {
        let mut engine = AlphaBetaEngine::new();
        let _ = engine.set_position("8/7k/5KR1/8/8/8/8/8 w - - 0 1");
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
        let _ = engine.set_position("4k1nr/2p3p1/b2pPp1p/8/1nN1P1P1/p1R2N2/PR3P2/5K2 b k - 1 26");
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
        let _ = engine.set_position("4k1nr/2p3p1/b2pPp1p/8/1nN1P1P1/p1R2N2/PR3P2/5K2 b k - 1 26");
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
        let _ = engine.set_position("rnbqkbnr/p1p2ppp/1p1p4/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4");
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
        let _ = engine.set_position("1k1r2rq/6pp/Q7/8/8/8/6PP/7K w - - 0 1");
        let _ = engine.make_move("a6b6");
        let _ = engine.make_move("b8a8");
        let _ = engine.make_move("b6a6");
        let _ = engine.make_move("a8b8");

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

    #[test]
    fn test_eval_played_position() {
        let mut engine = AlphaBetaEngine::new();
        let _ = engine.set_position("rnbq1rk1/ppp2ppp/3bpn2/3p4/8/3BPN1P/PPPP1P1P/RNBQ1RK1 w Qq - 0 1");
        println!("Evaluation: {}", AlphaBetaEngine::evaluate_board(&engine.board));
    }

    #[test]
    fn test_eval_played_position1() {
        let mut engine = AlphaBetaEngine::new();
        let _ = engine.set_position("1r2r1k1/2b2p2/p1p2p1p/P1pp3P/R6N/2P1PP2/1PK3P1/3R4 w - - 0 1");
        println!("Evaluation: {}", AlphaBetaEngine::evaluate_board(&engine.board));
    }

    #[test]
    fn test_eval_played_position2() {
        let mut engine = AlphaBetaEngine::new();
        let _ = engine.set_position("1k6/5p2/4P3/4Pp2/8/8/1K6/8 w - - 0 1");
        println!("Evaluation: {}", AlphaBetaEngine::evaluate_board(&engine.board));
    }
}
