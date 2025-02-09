use crate::engines::uci::run_uci_interface;

#[path = "../engines/mod.rs"]
mod engines;
#[path = "../chess_board/mod.rs"]
mod chess_board;


fn main() {
            run_uci_interface();
}