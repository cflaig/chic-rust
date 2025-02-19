use crate::engines::uci::run_uci_interface;

#[path = "../chess_boards/chess_board/mod.rs"]
mod chess_board;
#[path = "../chess_boards/mod.rs"]
mod chess_boards;
#[path = "../engines/mod.rs"]
mod engines;

fn main() {
    run_uci_interface();
}
