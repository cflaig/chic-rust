use crate::chess_boards::chess_board::{Color, Move};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

pub mod engine_alpha_beta;
pub mod engine_minmax;
pub mod uci;

type InfoCallback = fn(depth: usize, seldepth: usize, best_eval: i32, nodes: u64, elapsed: Duration, pv: String);

pub trait ChessEngine {
    fn name(&self) -> &str;
    fn author(&self) -> &str;
    fn set_position(&mut self, position: &str) -> Result<(), String>;
    fn make_move(&mut self, move_algebraic_notation: &str) -> Result<(), &'static str>;
    fn find_best_move_iterative(
        &mut self,
        time_limit: Duration,
        info_callback: InfoCallback,
    ) -> Option<(Vec<Move>, i32, u64, i32)>;
    fn get_active_player(&self) -> Color;
    fn get_abort_channel(&self) -> Arc<AtomicBool>;
    fn render_board(&self);
}
