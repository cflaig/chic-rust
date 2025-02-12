use crate::engines::uci::run_uci_interface;

#[path = "../chess_board/mod.rs"]
mod chess_board;
#[path = "../engines/mod.rs"]
mod engines;

fn main() {
    run_uci_interface();
}
