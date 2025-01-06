use crate::chess_board::Move;
mod chess_board;
mod ui;

use chess_board::ChessBoard;
use chess_board::ChessField;

use ui::setup_ui;

slint::include_modules!();

fn main() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let chess_board = ChessBoard::from_fen(fen).expect("Invalid FEN string");
    let generated_converted: Vec<_> = chess_board
        .generate_pseudo_moves()
        .iter()
        .map(|m| m.as_algebraic())
        .collect();
    println!("{:?}", generated_converted);

    let main_window = MainWindow::new().unwrap();
    setup_ui(&main_window, chess_board);
    main_window.run().unwrap();
}
