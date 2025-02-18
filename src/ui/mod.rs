use crate::chess_board::PieceType;
use crate::chess_board::Square;
use crate::chess_board::Square::Occupied;
use crate::chess_board::{Color, Piece};
use crate::engines::engine_alpha_beta::AlphaBetaEngine;
use crate::engines::ChessEngine;
use crate::ChessBoard;
use crate::ChessField;
use crate::MainWindow;
use crate::Move;
use crate::UiField;
use lazy_static::lazy_static;
use slint::Image;
use slint::Model;
use slint::ModelRc;
use slint::VecModel;
use slint::{ComponentHandle, SharedString};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

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

pub struct State {
    chess_board: RefCell<ChessBoard>,
    main_ui: MainWindow,
    selected_field: RefCell<Option<ChessField>>,
    active_move: RefCell<Option<Move>>,
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

pub fn highlight_move(state: &Rc<State>, position: ChessField) {
    let pieces = state.main_ui.get_chess_fields();
    for index in 0..64 {
        if let Some(mut p) = pieces.row_data(index) {
            p.highlighted_for_move = false;
            pieces.set_row_data(index, p);
        }
    }

    if position.row >= 8 || position.col >= 8 {
        return;
    }

    let moves = state.chess_board.borrow().generate_legal_moves();
    for m in moves {
        if m.from.row == position.row && m.from.col == position.col {
            let index = m.to.row * 8 + m.to.col;
            if let Some(mut p) = pieces.row_data(index) {
                p.highlighted_for_move = true;
                pieces.set_row_data(index, p);
            }
        }
    }
}

pub fn setup_ui(fen: &str) {
    let state = Rc::new(State {
        chess_board: RefCell::new(ChessBoard::from_fen(fen).expect("Invalid FEN string")),
        main_ui: MainWindow::new().unwrap(),
        selected_field: RefCell::new(None),
        active_move: RefCell::new(None),
    });
    let state_weak = Rc::downgrade(&state);

    state.main_ui.on_clicked(move |index| {
        if let Some(state) = state_weak.upgrade() {
            let (row, col) = index_to_row_col(index.try_into().unwrap());
            let clicked_field = ChessField::new(row, col);
            let mut selected_field = state.selected_field.borrow_mut();

            match *selected_field {
                None => {
                    *selected_field = Some(clicked_field);
                    highlight_move(&state, clicked_field);
                }
                Some(source) => {
                    let chess_board = state.chess_board.borrow().clone();
                    let moves = chess_board
                        .generate_legal_moves()
                        .into_iter()
                        .filter(|&m| m.from.row == source.row && m.from.col == source.col)
                        .collect::<Vec<_>>();
                    if moves.iter().any(|m| m.to == clicked_field) {
                        let mv = Move::new(source.row, source.col, clicked_field.row, clicked_field.col);

                        if let Occupied(piece) = chess_board.squares[source.row][source.col] {
                            if is_promotion(clicked_field, piece) {
                                set_piece_color_of_the_promotion_dialog(&state.main_ui, piece.color);
                                state.main_ui.set_promotion_dialog_visible(true);
                                state.active_move.borrow_mut().replace(mv);
                                return;
                            }
                        }

                        state.chess_board.borrow_mut().make_move(mv);
                        state
                            .main_ui
                            .set_chess_fields(map_chessboard_to_ui(&state.chess_board.borrow()));
                        make_engine_move(&state);
                    } else {
                        *selected_field = Some(clicked_field);
                        highlight_move(&state, clicked_field);
                    }
                }
            }
        }
    });

    let state_weak = Rc::downgrade(&state);
    state.main_ui.on_promotion_selected(move |choice_index| {
        if let Some(state) = state_weak.upgrade() {
            let promoted_piece = match choice_index {
                0 => PieceType::Queen,
                1 => PieceType::Rook,
                2 => PieceType::Bishop,
                _ => PieceType::Knight,
            };
            state.main_ui.set_promotion_dialog_visible(false);
            if let Some(mv) = *state.active_move.borrow_mut() {
                let mv = mv.with_promotion(promoted_piece);
                state.chess_board.borrow_mut().make_move(mv);
                state
                    .main_ui
                    .set_chess_fields(map_chessboard_to_ui(&state.chess_board.borrow()));
                make_engine_move(&state);
            }
        }
    });

    let state_weak = Rc::downgrade(&state);
    state.main_ui.on_make_move(move |mv_algebraic: SharedString| {
        if let Some(state) = state_weak.upgrade() {
            state
                .chess_board
                .borrow_mut()
                .make_move(Move::from_algebraic(mv_algebraic.as_str()));
            state
                .main_ui
                .set_chess_fields(map_chessboard_to_ui(&state.chess_board.borrow()));
        }
    });

    let fields = map_chessboard_to_ui(&state.chess_board.borrow());
    state.main_ui.set_chess_fields(fields);
    state.main_ui.run().unwrap();
}

fn set_piece_color_of_the_promotion_dialog(main_window: &MainWindow, color: Color) {
    let promotion_choices = vec![
        create_piece(PIECE_IMAGES.get(&(color, PieceType::Queen)).unwrap()),
        create_piece(PIECE_IMAGES.get(&(color, PieceType::Rook)).unwrap()),
        create_piece(PIECE_IMAGES.get(&(color, PieceType::Bishop)).unwrap()),
        create_piece(PIECE_IMAGES.get(&(color, PieceType::Knight)).unwrap()),
    ];
    main_window.set_promotion_choices(ModelRc::new(VecModel::from(promotion_choices)));
}

fn is_promotion(clicked_field: ChessField, piece: Piece) -> bool {
    piece.kind == PieceType::Pawn && (clicked_field.row == 0 || clicked_field.row == 7)
}

#[cfg(not(target_arch = "wasm32"))]
fn make_engine_move(state: &Rc<State>) {
    let state_weak = Rc::downgrade(state);
    let chess_board = state.chess_board.borrow().clone();
    let ui_weak = state_weak.upgrade().unwrap().main_ui.as_weak();

    std::thread::spawn(move || {
        let mut engine = AlphaBetaEngine::with_board(chess_board);
        if let Some((best_move, score, node_count, depth)) = engine.find_best_move_iterative(
            std::time::Duration::from_secs(7),
            |_depth, _seldepth, _eval, _nodes, _elapsed, _pv| {
                // No-op
            },
        ) {
            println!(
                "Best move: {} with score: {} nodes: {} depth: {}",
                best_move[0].as_algebraic(),
                score,
                node_count,
                depth,
            );
            let handle = ui_weak.clone();
            let mv = best_move[0].as_algebraic();
            // now forward the data to the main thread using invoke_from_event_loop
            let _ = slint::invoke_from_event_loop(move || handle.unwrap().invoke_make_move(SharedString::from(mv)));
        } else {
            println!("No best move found!");
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn make_engine_move(state: &Rc<State>) {
    let state_weak = Rc::downgrade(state);
    let chess_board = state.chess_board.borrow().clone();
    let ui_weak = state_weak.upgrade().unwrap().main_ui.as_weak();
    if let Some((best_move, score, node_count, depth)) =
        find_best_move_iterative(&chess_board, std::time::Duration::from_secs(3))
    {
        println!(
            "Best move: {} with score: {} nodes: {} depth: {}",
            best_move.as_algebraic(),
            score,
            node_count,
            depth,
        );
        let handle = ui_weak.clone();
        let mv = best_move.as_algebraic();
        // now forward the data to the main thread using invoke_from_event_loop
        let _ = slint::invoke_from_event_loop(move || handle.unwrap().invoke_make_move(SharedString::from(mv)));
    } else {
        println!("No best move found!");
    }
}
