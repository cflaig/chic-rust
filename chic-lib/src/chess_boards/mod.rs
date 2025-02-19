pub mod chess_board;
pub mod perft;

use chess_board::{ChessField, Piece};
use chess_board::Move;

pub trait Board {
    fn make_move(&mut self, mv: &Move) -> Result<(), &'static str>;
    fn hash(&self) -> u64;
    fn halfmove_clock(&self) -> u32;
    fn active_color(&self) -> u32;
    fn is_checkmate(&self) -> bool;
    fn is_stalemate(&self) -> bool;
    fn pieces_with_coordinates<'a>(&'a self) -> impl Iterator<Item = (ChessField, &'a Piece)>;
    fn render_to_string(&self) -> String;
    fn is_check(&self) -> bool;

    fn is_draw(&self) -> bool;
    fn get_legal_moves(&self) -> Vec<Move>;
    fn get_legal_capture_moves(&self) -> Vec<Move>;
}
