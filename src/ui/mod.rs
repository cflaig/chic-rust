use crate::chess_board::Color;
use crate::chess_board::PieceType;
use crate::chess_board::Square;
use crate::chess_board::Square::Occupied;
use crate::engine_alpha_beta::{find_best_move, find_best_move_iterative, find_best_move_random};
use crate::ChessBoard;
use crate::ChessField;
use crate::MainWindow;
use crate::Move;
use crate::UiField;
use lazy_static::lazy_static;
use slint::ComponentHandle;
use slint::Image;
use slint::Model;
use slint::ModelRc;
use slint::VecModel;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

// Use a single map for image paths instead of multiple constants
lazy_static! {
    static ref PIECE_IMAGES: HashMap<(Color, PieceType), &'static str> = {
        let mut map = HashMap::new();
        map.insert((Color::White, PieceType::Pawn), "ui/icons/Piece_White_Pawn.svg");
        map.insert((Color::White, PieceType::Knight), "ui/icons/Piece_White_Knight.svg");
        map.insert((Color::White, PieceType::Bishop), "ui/icons/Piece_White_Bishop.svg");
        map.insert((Color::White, PieceType::Rook), "ui/icons/Piece_White_Rock.svg");
        map.insert((Color::White, PieceType::Queen), "ui/icons/Piece_White_Queen.svg");
        map.insert((Color::White, PieceType::King), "ui/icons/Piece_White_King.svg");
        map.insert((Color::Black, PieceType::Pawn), "ui/icons/Piece_Black_Pawn.svg");
        map.insert((Color::Black, PieceType::Knight), "ui/icons/Piece_Black_Knight.svg");
        map.insert((Color::Black, PieceType::Bishop), "ui/icons/Piece_Black_Bishop.svg");
        map.insert((Color::Black, PieceType::Rook), "ui/icons/Piece_Black_Rock.svg");
        map.insert((Color::Black, PieceType::Queen), "ui/icons/Piece_Black_Queen.svg");
        map.insert((Color::Black, PieceType::King), "ui/icons/Piece_Black_King.svg");
        map
    };
}

// Simplify the mapping process by extracting common logic
fn square_to_ui_field(square: &Square) -> UiField {
    if let Square::Occupied(piece) = square {
        if let Some(&piece_svg) = PIECE_IMAGES.get(&(piece.color, piece.kind)) {
            return create_piece(piece_svg);
        }
    }
    UiField {
        image: Image::default(),
        highlighted_for_move: false,
    }
}

fn create_piece(piece_svg: &str) -> UiField {
    //use relative path instead absolute path
    //let path_buf = Path::new(env!("CARGO_MANIFEST_DIR")).join(piece_svg);
    let path_buf = Path::new(piece_svg);

    UiField {
        image: Image::load_from_path(path_buf).unwrap(),
        highlighted_for_move: false,
    }
}

fn index_to_row_col(index: usize) -> (usize, usize) {
    (index / 8, index % 8)
}

pub fn map_chessboard_to_ui(chess_board: &ChessBoard) -> ModelRc<UiField> {
    let pieces: Vec<UiField> = chess_board
        .squares
        .iter()
        .flat_map(|row| row.iter().map(square_to_ui_field))
        .collect();
    ModelRc::new(VecModel::from(pieces))
}

pub fn highlight_move(pieces: &ModelRc<UiField>, board: &ChessBoard, position: ChessField) {
    for index in 0..64 {
        if let Some(mut p) = pieces.row_data(index) {
            p.highlighted_for_move = false;
            pieces.set_row_data(index, p);
        }
    }

    if position.row >= 8 || position.col >= 8 {
        return;
    }

    let moves = board.generate_pseudo_moves_from_position(position.row, position.col);
    for m in moves {
        let index = m.to.row * 8 + m.to.col;
        if let Some(mut p) = pieces.row_data(index) {
            p.highlighted_for_move = true;
            pieces.set_row_data(index, p);
        }
    }
}

pub fn setup_ui(main_window: &MainWindow, chess_board: ChessBoard) {
    let main_window_weak = main_window.as_weak();
    let chess_board = Rc::new(RefCell::new(chess_board));
    let fields = map_chessboard_to_ui(&chess_board.borrow());
    main_window.set_chess_fields(fields);

    let mut selected_field: Option<ChessField> = None;

    {
        let chess_board = Rc::clone(&chess_board);
        main_window.on_clicked(move |index| {
            if let Some(main_window) = main_window_weak.upgrade() {
                let (row, col) = index_to_row_col(index.try_into().unwrap());
                let clicked_field = ChessField::new(row, col);

                match selected_field {
                    None => {
                        selected_field = Some(clicked_field);
                        highlight_move(&main_window.get_chess_fields(), &chess_board.borrow(), clicked_field);
                    }
                    Some(source) => {
                        let moves = chess_board
                            .borrow()
                            .generate_pseudo_moves_from_position(source.row, source.col);
                        if moves.iter().any(|m| m.to == clicked_field) {
                            let mv = Move::new(source.row, source.col, clicked_field.row, clicked_field.col);

                            if let Occupied(piece) = chess_board.borrow().squares[source.row][source.col] {
                                if piece.kind == PieceType::Pawn && (clicked_field.row == 0 || clicked_field.row == 7) {
                                    // Pawn promotion logic
                                    let promotion_choices = vec![
                                        create_piece(PIECE_IMAGES.get(&(piece.color, PieceType::Queen)).unwrap()),
                                        create_piece(PIECE_IMAGES.get(&(piece.color, PieceType::Rook)).unwrap()),
                                        create_piece(PIECE_IMAGES.get(&(piece.color, PieceType::Bishop)).unwrap()),
                                        create_piece(PIECE_IMAGES.get(&(piece.color, PieceType::Knight)).unwrap()),
                                    ];

                                    // Show promotion dialog
                                    selected_field = None;
                                    main_window.set_promotion_choices(ModelRc::new(VecModel::from(promotion_choices)));
                                    main_window.set_promotion_dialog_visible(true);

                                    // Handle promotion selection asynchronously
                                    let value = main_window_weak.clone();
                                    let chess_board_clone = chess_board.clone();
                                    main_window.on_promotion_selected(move |choice_index| {
                                        if let Some(main_window) = value.upgrade() {
                                            let promoted_piece = match choice_index {
                                                0 => PieceType::Queen,
                                                1 => PieceType::Rook,
                                                2 => PieceType::Bishop,
                                                _ => PieceType::Knight,
                                            };
                                            main_window.set_promotion_dialog_visible(false);
                                            let mv = mv.with_promotion(promoted_piece);
                                            chess_board_clone.borrow_mut().make_move(mv);
                                            main_window
                                                .set_chess_fields(map_chessboard_to_ui(&chess_board_clone.borrow()));
                                            make_engine_move(&main_window, &mut chess_board_clone.borrow_mut());
                                        }
                                    });
                                    return;
                                }
                            }

                            let mut chess_board = chess_board.borrow_mut();
                            chess_board.make_move(mv);
                            println!("Best move: {}", mv.as_algebraic());

                            main_window.set_chess_fields(map_chessboard_to_ui(&chess_board.clone()));
                            selected_field = None;

                            make_engine_move(&main_window, &mut chess_board);
                        } else {
                            highlight_move(
                                &main_window.get_chess_fields(),
                                &chess_board.borrow(),
                                ChessField::new(8, 8),
                            );
                            selected_field = None;
                        }
                    }
                }
            }
        });
    }
}


// #[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
// extern "C" {
//     // Use `js_namespace` here to bind `console.log(..)` instead of just
//     // `log(..)`
//     #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = console)]
//     fn log(s: &str);
// }
fn make_engine_move(main_window: &MainWindow, chess_board: &mut ChessBoard) {
    if let Some((best_move, score, node_count, depth)) = find_best_move_iterative(&chess_board.clone(),Duration::from_secs_f32(2.5)) {
        chess_board.make_move(best_move);
        println!(
            "Best move: {} with score: {} nodes: {} depth: {}",
            best_move.as_algebraic(),
            score,
            node_count,
            depth
        );
        #[cfg(target_arch = "wasm32")]
        {
            log(&format!("Best move: {} with score: {} evaluated nodes: {} depth: {}", best_move.as_algebraic(), score, node_count, depth));
        }
        main_window.set_chess_fields(map_chessboard_to_ui(&chess_board.clone()));
    } else {
        println!("No best move found!");
    }
}
