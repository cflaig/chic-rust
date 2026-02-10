use chic_lib::chess_boards::chess_board::{ChessBoard, ChessField, Color, Move, PieceType, Square};
use egui::{Color32, Image, Pos2, Rect, TextureHandle, Vec2};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use crate::assets::load_piece_textures;
use crate::engine::{start_engine_analysis, start_engine_move, EngineMessage};

#[derive(Clone)]
struct MoveHistoryEntry {
    mv: Move,
    board_after_move: ChessBoard,
    notation: String,
}

const LIGHT_SQUARE: Color32 = Color32::from_rgb(236, 218, 185); // #ECDAB9
const DARK_SQUARE: Color32 = Color32::from_rgb(174, 138, 104); // #AE8A68

// Lazy initialization for HIGHLIGHT_COLOR since Color32::from_rgba_unmultiplied is not const
fn highlight_color() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 0, 96)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    HumanVsEngine, // Human plays White, Engine plays Black
    EngineVsHuman, // Engine plays White, Human plays Black
    HumanVsHuman,  // Both sides human-controlled
}

impl Default for GameMode {
    fn default() -> Self {
        GameMode::HumanVsEngine
    }
}

pub struct ChicApp {
    chess_board: ChessBoard,
    initial_board: ChessBoard,
    selected_field: Option<ChessField>,
    legal_moves: Vec<Move>,
    promotion_dialog_visible: bool,
    promotion_moves: Vec<Move>,
    piece_textures: HashMap<(Color, PieceType), TextureHandle>,
    engine_rx: Option<Receiver<EngineMessage>>,
    engine_thinking: bool,
    engine_thinking_at_position: Option<usize>, // Position where engine started thinking
    game_mode: GameMode,
    move_history: Vec<MoveHistoryEntry>,
    current_position_index: usize, // 0 = initial position, 1 = after first move, etc.

    // Analysis view state
    show_analysis: bool,
    analysis_rx: Option<Receiver<EngineMessage>>,
    analysis_running: bool,
    analysis_abort: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    last_analysis_board: Option<ChessBoard>, // Track which board we're analyzing

    // Analysis display data
    analysis_depth: usize,
    analysis_eval: i32,
    analysis_pv: String,
    analysis_nodes: u64,
    analysis_elapsed_ms: u64,

    // PGN Load Dialog state
    pgn_dialog_visible: bool,
    pgn_text: String,
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

        let mut app = Self {
            initial_board: chess_board.clone(),
            chess_board,
            selected_field: None,
            legal_moves: Vec::new(),
            promotion_dialog_visible: false,
            promotion_moves: Vec::new(),
            piece_textures,
            engine_rx: None,
            engine_thinking: false,
            engine_thinking_at_position: None,
            game_mode: GameMode::default(),
            move_history: Vec::new(),
            current_position_index: 0,

            show_analysis: false,
            analysis_rx: None,
            analysis_running: false,
            analysis_abort: None,
            last_analysis_board: None,
            analysis_depth: 0,
            analysis_eval: 0,
            analysis_pv: String::new(),
            analysis_nodes: 0,
            analysis_elapsed_ms: 0,

            pgn_dialog_visible: false,
            pgn_text: String::new(),
        };

        // Start engine if needed (e.g., Engine vs Human mode)
        app.check_engine_turn(&cc.egui_ctx);
        app
    }

    fn handle_square_click(&mut self, field: ChessField, ctx: &egui::Context) {
        // Don't allow clicks when engine is thinking or it's engine's turn
        if self.engine_thinking || self.is_engine_turn() {
            return;
        }

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
        self.record_move(mv, None);

        self.selected_field = None;
        self.legal_moves.clear();
        self.promotion_dialog_visible = false;
        self.promotion_moves.clear();

        // Start engine thinking if it's the engine's turn
        self.check_engine_turn(ctx);

        // Restart analysis for the new position if analysis is enabled
        if self.show_analysis {
            self.start_analysis();
        }
    }

    /// Returns true if it's the engine's turn based on current game mode
    fn is_engine_turn(&self) -> bool {
        // Only consider it engine's turn if we're at the latest position
        if self.current_position_index < self.move_history.len() {
            return false;
        }

        match self.game_mode {
            GameMode::HumanVsEngine => self.chess_board.active_color == Color::Black,
            GameMode::EngineVsHuman => self.chess_board.active_color == Color::White,
            GameMode::HumanVsHuman => false,
        }
    }

    /// Check if engine should make a move and trigger it if needed
    fn check_engine_turn(&mut self, ctx: &egui::Context) {
        if !self.engine_thinking && self.is_engine_turn() {
            self.engine_thinking = true;
            self.engine_thinking_at_position = Some(self.current_position_index);
            self.engine_rx = Some(start_engine_move(self.chess_board.clone()));
            ctx.request_repaint();
        }
    }

    /// Convert ChessField to algebraic notation (e.g., e2, d4)
    fn field_to_algebraic(field: ChessField) -> String {
        let file = (b'a' + field.col) as char;
        let rank = (b'1' + field.row) as char;
        format!("{}{}", file, rank)
    }

    /// Convert PieceType to character for promotion notation
    fn piece_to_char(piece_type: PieceType) -> char {
        match piece_type {
            PieceType::Queen => 'Q',
            PieceType::Rook => 'R',
            PieceType::Bishop => 'B',
            PieceType::Knight => 'N',
            PieceType::King => 'K',
            PieceType::Pawn => 'P',
        }
    }

    /// Record a move in history (used by both user and engine moves)
    /// If at_position is Some, the move is added at that position (for engine moves when user has navigated away)
    /// If at_position is None, the move is added at current_position_index (for user moves)
    fn record_move(&mut self, mv: Move, at_position: Option<usize>) {
        let target_position = at_position.unwrap_or(self.current_position_index);

        // Truncate history if we're making a move from a past position
        if target_position < self.move_history.len() {
            self.move_history.truncate(target_position);
        }

        // Get or create the board state at the target position
        let mut board_for_move = if target_position == 0 {
            self.initial_board.clone()
        } else if target_position <= self.move_history.len() {
            self.move_history[target_position - 1].board_after_move.clone()
        } else {
            self.chess_board.clone()
        };

        let notation = mv.to_san(&board_for_move);
        board_for_move.make_move(mv);

        // Record the move in history
        self.move_history.push(MoveHistoryEntry {
            mv,
            board_after_move: board_for_move.clone(),
            notation,
        });

        // Update current board if we're recording at the current position
        if at_position.is_none() {
            self.chess_board = board_for_move;
            self.current_position_index = self.move_history.len();
        }
    }

    fn check_engine_response(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.engine_rx {
            if let Ok(msg) = rx.try_recv() {
                match msg {
                    EngineMessage::Move(mv) => {
                        // Check if user is viewing the latest position
                        let was_at_latest = self.current_position_index == self.move_history.len();

                        // Use the saved position where engine started thinking
                        self.record_move(mv, self.engine_thinking_at_position);

                        // If user was at the latest position, update to show the new move
                        if was_at_latest {
                            self.current_position_index = self.move_history.len();
                            self.chess_board = self
                                .move_history
                                .last()
                                .map(|entry| entry.board_after_move.clone())
                                .unwrap_or_else(|| self.initial_board.clone());
                        }

                        self.engine_thinking = false;
                        self.engine_thinking_at_position = None;
                        self.engine_rx = None;

                        // Restart analysis for the new position if analysis is enabled
                        if self.show_analysis {
                            self.start_analysis();
                        }
                    }
                    EngineMessage::Error(e) => {
                        eprintln!("Engine error: {}", e);
                        self.engine_thinking = false;
                        self.engine_thinking_at_position = None;
                        self.engine_rx = None;
                    }
                    EngineMessage::Analysis { .. } => {
                        // Ignore analysis messages in engine move channel
                    }
                }
            } else if self.engine_thinking {
                // Still thinking, request repaint to check again
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
    }

    /// Process analysis updates from the engine
    fn check_analysis_response(&mut self, ctx: &egui::Context) {
        let mut stop_analysis = false;

        if let Some(rx) = &self.analysis_rx {
            // Process all available messages (don't block)
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    EngineMessage::Analysis {
                        depth,
                        eval,
                        pv,
                        nodes,
                        elapsed_ms,
                    } => {
                        self.analysis_depth = depth;
                        // Normalize eval to White's perspective for the UI
                        self.analysis_eval = if self.chess_board.active_color == Color::Black {
                            -eval
                        } else {
                            eval
                        };
                        self.analysis_pv = pv;
                        self.analysis_nodes = nodes;
                        self.analysis_elapsed_ms = elapsed_ms;
                    }
                    EngineMessage::Error(e) => {
                        eprintln!("Analysis error: {}", e);
                        stop_analysis = true;
                    }
                    _ => {} // Ignore Move messages in analysis
                }
            }

            if self.analysis_running {
                // Request repaint to show updates
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }

        if stop_analysis {
            self.analysis_running = false;
            self.analysis_rx = None;
        }
    }

    /// Format evaluation as human-readable string
    fn format_evaluation(&self, eval: i32) -> String {
        const MATE_THRESHOLD: i32 = 9_000_000;

        if eval > MATE_THRESHOLD {
            let mate_in = (10_000_000 - eval) / 2 + 1;
            format!("Mate in {}", mate_in)
        } else if eval < -MATE_THRESHOLD {
            let mate_in = (10_000_000 + eval) / 2 + 1;
            format!("Mate in -{}", mate_in)
        } else {
            // Convert centipawns to pawns (divide by 1000)
            let pawns = eval as f64 / 1000.0;
            format!("{:+.2}", pawns)
        }
    }

    /// Format large numbers with K/M suffixes
    fn format_nodes(&self, nodes: u64) -> String {
        if nodes >= 1_000_000 {
            format!("{:.1}M", nodes as f64 / 1_000_000.0)
        } else if nodes >= 1_000 {
            format!("{:.1}K", nodes as f64 / 1_000.0)
        } else {
            format!("{}", nodes)
        }
    }

    fn render_menu_bar(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // File menu
                ui.menu_button("File", |ui| {
                    if ui.button("Load PGN...").clicked() {
                        self.pgn_dialog_visible = true;
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                // Play menu with game modes
                ui.menu_button("Play", |ui| {
                    ui.label("Game Mode:");
                    ui.separator();

                    if ui
                        .radio(
                            self.game_mode == GameMode::HumanVsEngine,
                            "Human (White) vs Engine (Black)",
                        )
                        .clicked()
                    {
                        self.change_game_mode(GameMode::HumanVsEngine, ctx);
                        ui.close_menu();
                    }
                    if ui
                        .radio(
                            self.game_mode == GameMode::EngineVsHuman,
                            "Engine (White) vs Human (Black)",
                        )
                        .clicked()
                    {
                        self.change_game_mode(GameMode::EngineVsHuman, ctx);
                        ui.close_menu();
                    }
                    if ui
                        .radio(self.game_mode == GameMode::HumanVsHuman, "Human vs Human")
                        .clicked()
                    {
                        self.change_game_mode(GameMode::HumanVsHuman, ctx);
                        ui.close_menu();
                    }
                });

                // View menu
                ui.menu_button("View", |ui| {
                    if ui.checkbox(&mut self.show_analysis, "Show Analysis").clicked() {
                        if self.show_analysis {
                            self.start_analysis();
                        } else {
                            self.stop_analysis();
                        }
                        ui.close_menu();
                    }
                });
            });
        });
    }

    fn change_game_mode(&mut self, new_mode: GameMode, ctx: &egui::Context) {
        if self.engine_thinking {
            return; // Don't change mode while engine is thinking
        }

        self.game_mode = new_mode;

        // Clear selection state
        self.selected_field = None;
        self.legal_moves.clear();

        // If changing to a mode where engine should move now, trigger it
        self.check_engine_turn(ctx);
    }

    /// Navigate to a specific position in the move history
    fn navigate_to_position(&mut self, index: usize) {
        if index > self.move_history.len() {
            return;
        }

        // If we're already at this position, do nothing
        if index == self.current_position_index {
            return;
        }

        self.current_position_index = index;

        // Update board to the position
        if index == 0 {
            self.chess_board = self.initial_board.clone();
        } else {
            self.chess_board = self.move_history[index - 1].board_after_move.clone();
        }

        // Clear selection
        self.selected_field = None;
        self.legal_moves.clear();

        // Restart analysis for the new position if analysis is enabled
        if self.show_analysis {
            self.start_analysis();
        }
    }

    /// Start or restart analysis for the current board position
    fn start_analysis(&mut self) {
        if !self.show_analysis {
            return;
        }

        // Stop existing analysis if running
        if self.analysis_running {
            self.stop_analysis();
        }

        // Start new analysis for current board position
        let (rx, abort) = start_engine_analysis(self.chess_board.clone());
        self.analysis_rx = Some(rx);
        self.analysis_abort = Some(abort);
        self.analysis_running = true;
        self.last_analysis_board = Some(self.chess_board.clone());

        // Reset display
        self.analysis_depth = 0;
        self.analysis_eval = 0;
        self.analysis_pv = String::from("Analyzing...");
        self.analysis_nodes = 0;
        self.analysis_elapsed_ms = 0;
    }

    /// Stop current analysis
    fn stop_analysis(&mut self) {
        // Signal the engine to abort (like UCI "stop" command)
        if let Some(abort) = &self.analysis_abort {
            abort.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.analysis_rx = None;
        self.analysis_abort = None;
        self.analysis_running = false;
    }

    /// Toggle analysis view on/off
    pub fn toggle_analysis(&mut self) {
        self.show_analysis = !self.show_analysis;

        if self.show_analysis {
            self.start_analysis();
        } else {
            self.stop_analysis();
        }
    }

    fn render_move_history(&mut self, ui: &mut egui::Ui) {
        let mut navigate_to: Option<usize> = None;

        ui.vertical(|ui| {
            ui.heading("Move History");
            ui.separator();

            // Navigation buttons
            ui.horizontal(|ui| {
                if ui.button("|◄").clicked() {
                    navigate_to = Some(0);
                }
                if ui.button("◄").clicked() {
                    if self.current_position_index > 0 {
                        navigate_to = Some(self.current_position_index - 1);
                    }
                }
                if ui.button("►").clicked() {
                    if self.current_position_index < self.move_history.len() {
                        navigate_to = Some(self.current_position_index + 1);
                    }
                }
                if ui.button("►|").clicked() {
                    navigate_to = Some(self.move_history.len());
                }
            });

            ui.separator();

            // Display move list
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                if self.move_history.is_empty() {
                    ui.label("No moves yet");
                    return;
                }

                // Group moves by pairs (White and Black moves)
                for move_idx in 0..self.move_history.len() {
                    let entry = &self.move_history[move_idx];
                    let move_number = move_idx / 2 + 1;
                    let is_white_move = move_idx % 2 == 0;
                    let position_idx = move_idx + 1;

                    ui.horizontal(|ui| {
                        // Show move number for white moves
                        if is_white_move {
                            ui.label(format!("{}.", move_number));
                        } else {
                            ui.label("   "); // Indent for black moves
                        }

                        // Highlight current position
                        let is_current = position_idx == self.current_position_index;
                        let move_text = egui::RichText::new(&entry.notation).strong().color(if is_current {
                            Color32::from_rgb(0, 150, 255)
                        } else {
                            Color32::WHITE
                        });

                        if ui.selectable_label(is_current, move_text).clicked() {
                            navigate_to = Some(position_idx);
                        }
                    });
                }
            });
        });

        // Perform navigation after rendering to avoid borrow conflicts
        if let Some(position) = navigate_to {
            self.navigate_to_position(position);
        }
    }

    fn render_analysis_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("Analysis");
            ui.separator();

            if !self.show_analysis {
                ui.label("Analysis disabled");
                return;
            }

            if !self.analysis_running {
                ui.label("Analysis not running");
                return;
            }

            // Display depth
            ui.horizontal(|ui| {
                ui.label("Depth:");
                ui.label(format!("{}", self.analysis_depth));
            });

            // Display evaluation
            ui.horizontal(|ui| {
                ui.label("Score:");
                ui.label(self.format_evaluation(self.analysis_eval));
            });

            // Display nodes and speed
            if self.analysis_elapsed_ms > 0 {
                let nps = (self.analysis_nodes as f64 / self.analysis_elapsed_ms as f64) * 1000.0;
                ui.horizontal(|ui| {
                    ui.label("Nodes:");
                    ui.label(format!(
                        "{} ({}/s)",
                        self.format_nodes(self.analysis_nodes),
                        self.format_nodes(nps as u64)
                    ));
                });
            }

            ui.separator();

            // Display PV
            ui.label("Principal Variation:");
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                ui.label(&self.analysis_pv);
            });
        });
    }

    fn render_board(&mut self, ui: &mut egui::Ui) {
        let available_size = ui.available_size();
        let board_size = available_size.x.min(available_size.y);
        let square_size = board_size / 8.0;

        let (response, painter) = ui.allocate_painter(Vec2::splat(board_size), egui::Sense::click());

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
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                ui.painter()
                    .rect_filled(screen_rect, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 128));
            });

        // Promotion dialog window
        egui::Window::new("Promote Pawn")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let piece_types = [PieceType::Queen, PieceType::Rook, PieceType::Bishop, PieceType::Knight];

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
                                    self.promotion_dialog_visible = false;
                                }
                            }
                        }
                    }
                });
            });
    }

    fn render_pgn_dialog(&mut self, ctx: &egui::Context) {
        if !self.pgn_dialog_visible {
            return;
        }

        egui::Window::new("Load PGN")
            .collapsible(false)
            .resizable(true)
            .default_size([400.0, 300.0])
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Paste PGN below:");
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut self.pgn_text).hint_text("Paste PGN here..."));
                });

                ui.horizontal(|ui| {
                    if ui.button("Load").clicked() {
                        match chic_lib::chess_boards::chess_board::pgn::parse_pgn(&self.pgn_text) {
                            Ok((initial_board, moves)) => {
                                self.initial_board = initial_board.clone();
                                self.chess_board = initial_board;
                                self.move_history.clear();
                                self.current_position_index = 0;

                                for mv in moves {
                                    self.record_move(mv, None);
                                }
                                self.pgn_dialog_visible = false;
                                self.pgn_text.clear();
                                self.check_engine_turn(ctx);
                            }
                            Err(e) => {
                                eprintln!("Error parsing PGN: {}", e);
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.pgn_dialog_visible = false;
                        self.pgn_text.clear();
                    }
                });
            });
    }
}

impl eframe::App for ChicApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Check for engine and analysis responses
        self.check_engine_response(ctx);
        self.check_analysis_response(ctx);

        // Render menu bar
        self.render_menu_bar(ctx, frame);

        // Right side panel for move history
        egui::SidePanel::right("move_history_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                self.render_move_history(ui);
            });

        // Optional bottom panel for analysis (if enabled)
        if self.show_analysis {
            egui::TopBottomPanel::bottom("analysis_panel")
                .default_height(200.0)
                .min_height(100.0)
                .resizable(true)
                .show(ctx, |ui| {
                    self.render_analysis_panel(ui);
                });
        }

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
        self.render_pgn_dialog(ctx);
    }
}
