use crate::chess_board::{Color, Move};
use std::time::Duration;

pub mod engine_alpha_beta;
pub mod engine_minmax;
pub mod uci;

type InfoCallback = fn(depth: i32, best_eval: i32, nodes: u64, elapsed: Duration);

pub trait ChessEngine {
    fn name(&self) -> &str;
    fn author(&self) -> &str;
    fn set_position(&mut self, position: &str) -> Result<(), String>;
    fn make_move(&mut self, move_algebraic_notation: &str) -> Result<(), &'static str>;
    fn find_best_move_iterative(
        &self,
        time_limit: Duration,
        info_callback: InfoCallback,
    ) -> Option<(Move, i32, u64, i32)>;
    fn get_active_player(&self) -> Color;
}
