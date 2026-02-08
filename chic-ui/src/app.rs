use chic_lib::chess_boards::chess_board::{ChessBoard, ChessField, Color, Move, PieceType, Square};
use egui::{Color32, Image, Pos2, Rect, TextureHandle, Vec2};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use crate::assets::load_piece_textures;
use crate::engine::{start_engine_move, EngineMessage};

const LIGHT_SQUARE: Color32 = Color32::from_rgb(236, 218, 185); // #ECDAB9
const DARK_SQUARE: Color32 = Color32::from_rgb(174, 138, 104); // #AE8A68

// Lazy initialization for HIGHLIGHT_COLOR since Color32::from_rgba_unmultiplied is not const
fn highlight_color() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 0, 96)
}

pub struct ChicApp {
    chess_board: ChessBoard,
    selected_field: Option<ChessField>,
    legal_moves: Vec<Move>,
    promotion_dialog_visible: bool,
    promotion_moves: Vec<Move>,
    piece_textures: HashMap<(Color, PieceType), TextureHandle>,
    engine_rx: Option<Receiver<EngineMessage>>,
    engine_thinking: bool,
}

impl ChicApp {
    pub fn new(cc: &eframe::CreationContext<'_>, fen: Option<String>) -> Self {
        // Load piece textures
        let piece_textures = load_piece_textures(&cc.egui_ctx, 128);

        // Initialize board from FEN or starting position
        let chess_board = if let Some(fen_str) = fen {
            ChessBoard::from_fen(&fen_str).expect("Invalid FEN string")
        } else {
            ChessBoard::new()
        };

        Self {
            chess_board,
            selected_field: None,
            legal_moves: Vec::new(),
            promotion_dialog_visible: false,
            promotion_moves: Vec::new(),
            piece_textures,
            engine_rx: None,
            engine_thinking: false,
        }
    }

    fn handle_square_click(&mut self, field: ChessField, ctx: &egui::Context) {
        // Check if this is a legal move destination
        if let Some(selected) = self.selected_field {
            // Check if clicked square is a valid move destination
            let matching_moves: Vec<Move> = self
                .legal_moves
                .iter()
                .filter(|m| m.from == selected && m.to == field)
                .copied()
                .collect();

            if !matching_moves.is_empty() {
                // Check if this is a pawn promotion
                if matching_moves.len() > 1 {
                    // Multiple moves with same from/to means promotion
                    self.promotion_moves = matching_moves;
                    self.promotion_dialog_visible = true;
                } else {
                    // Execute the move
                    self.execute_move(matching_moves[0], ctx);
                }
                return;
            }
        }

        // Otherwise, try to select this square
        let square = self.chess_board.squares[field.row as usize][field.col as usize];
        if let Square::Occupied(piece) = square {
            if piece.color == self.chess_board.active_color {
                // Select this piece and generate legal moves
                self.selected_field = Some(field);
                self.legal_moves = self
                    .chess_board
                    .generate_legal_moves(None)
                    .into_iter()
                    .filter(|m| m.from == field)
                    .collect();
            } else {
                // Can't select opponent's piece
                self.selected_field = None;
                self.legal_moves.clear();
            }
        } else {
            // Clicked on empty square, deselect
            self.selected_field = None;
            self.legal_moves.clear();
        }
    }

    fn execute_move(&mut self, mv: Move, ctx: &egui::Context) {
        self.chess_board.make_move(mv);
        self.selected_field = None;
        self.legal_moves.clear();
        self.promotion_dialog_visible = false;
        self.promotion_moves.clear();

        // Start engine thinking if it's the engine's turn (Black)
        if self.chess_board.active_color == Color::Black && !self.engine_thinking {
            self.engine_thinking = true;
            self.engine_rx = Some(start_engine_move(self.chess_board.clone()));
            ctx.request_repaint();
        }
    }

    fn check_engine_response(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.engine_rx {
            if let Ok(msg) = rx.try_recv() {
                match msg {
                    EngineMessage::Move(mv) => {
                        self.chess_board.make_move(mv);
                        self.engine_thinking = false;
                        self.engine_rx = None;
                    }
                    EngineMessage::Error(e) => {
                        eprintln!("Engine error: {}", e);
                        self.engine_thinking = false;
                        self.engine_rx = None;
                    }
                }
            } else if self.engine_thinking {
                // Still thinking, request repaint to check again
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
    }

    fn render_board(&mut self, ui: &mut egui::Ui) {
        let available_size = ui.available_size();
        let board_size = available_size.x.min(available_size.y);
        let square_size = board_size / 8.0;

        let (response, painter) = ui.allocate_painter(
            Vec2::splat(board_size),
            egui::Sense::click(),
        );

        let origin = response.rect.min;

        // Handle click
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let rel_pos = pos - origin;
                let file = (rel_pos.x / square_size) as usize;
                let rank = 7 - (rel_pos.y / square_size) as usize; // Flip rank

                if file < 8 && rank < 8 {
                    let field = ChessField::new(rank as u8, file as u8);
                    self.handle_square_click(field, ui.ctx());
                }
            }
        }

        // Draw squares
        for rank in 0..8 {
            for file in 0..8 {
                let x = origin.x + file as f32 * square_size;
                let y = origin.y + (7 - rank) as f32 * square_size; // Flip rank
                let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(square_size));

                // Determine square color
                let is_light = (rank + file) % 2 == 0;
                let color = if is_light { LIGHT_SQUARE } else { DARK_SQUARE };
                painter.rect_filled(rect, 0.0, color);

                // Draw highlight for legal moves
                let field = ChessField::new(rank as u8, file as u8);
                if self.legal_moves.iter().any(|m| m.to == field) {
                    painter.rect_filled(rect, 0.0, highlight_color());
                }

                // Draw piece
                let square = self.chess_board.squares[rank as usize][file as usize];
                if let Square::Occupied(piece) = square {
                    if let Some(texture) = self.piece_textures.get(&(piece.color, piece.kind)) {
                        let piece_rect = rect.shrink(square_size * 0.05); // Small padding
                        Image::new(texture)
                            .fit_to_exact_size(piece_rect.size())
                            .paint_at(ui, piece_rect);
                    }
                }
            }
        }
    }

    fn render_promotion_dialog(&mut self, ctx: &egui::Context) {
        if !self.promotion_dialog_visible {
            return;
        }

        // Semi-transparent background
        egui::Area::new("promotion_overlay".into())
            .fixed_pos(Pos2::ZERO)
            .show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(0, 0, 0, 128),
                );
            });

        // Promotion dialog window
        egui::Window::new("Promote Pawn")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let piece_types = [
                        PieceType::Queen,
                        PieceType::Rook,
                        PieceType::Bishop,
                        PieceType::Knight,
                    ];

                    for piece_type in piece_types {
                        // Find the move with this promotion piece
                        let mv = self
                            .promotion_moves
                            .iter()
                            .find(|m| m.promotion == Some(piece_type))
                            .copied();

                        if let Some(mv) = mv {
                            let color = self.chess_board.active_color;
                            if let Some(texture) = self.piece_textures.get(&(color, piece_type)) {
                                let img = Image::new(texture).max_size(Vec2::splat(80.0));
                                if ui.add(egui::ImageButton::new(img)).clicked() {
                                    self.execute_move(mv, ctx);
                                }
                            }
                        }
                    }
                });
            });
    }
}

impl eframe::App for ChicApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for engine response
        self.check_engine_response(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Chic Chess");

            // Display game status
            let status = if self.engine_thinking {
                "Engine thinking..."
            } else {
                match self.chess_board.active_color {
                    Color::White => "White to move",
                    Color::Black => "Black to move",
                }
            };
            ui.label(status);

            ui.separator();

            // Render the chess board
            self.render_board(ui);
        });

        // Render promotion dialog on top
        self.render_promotion_dialog(ctx);
    }
}
