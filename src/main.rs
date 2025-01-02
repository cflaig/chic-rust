use slint::ModelRc;

mod chess_board;
mod ui;

use chess_board::ChessBoard;
use ui::map_chessboard_to_ui;

slint::include_modules!();

fn main() {
    let main_window = MainWindow::new().unwrap();

    //let fen = "8/8/8/4p1K1/2k1P3/8/8/8 b - - 0 1";
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let chess_board = ChessBoard::from_fen(fen).expect("Invalid FEN string");
    let generated_converted: Vec<_> = chess_board
        .generate_pseudo_moves()
        .iter()
        .map(|m| m.as_algebraic())
        .collect();
    println!("{:?}", generated_converted);

    main_window.set_chess_fields(map_chessboard_to_ui(&chess_board));
    main_window.run().unwrap();
}
