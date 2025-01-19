use crate::chess_board::Move;
use std::time::Instant;
mod chess_board;
mod engine_minmax;
mod ui;

use chess_board::ChessBoard;
use chess_board::ChessField;

use crate::engine_minmax::find_best_move;
use ui::setup_ui;

use clap::arg;
use clap::command;
use clap::Command;

use tabled::settings::Style;
use tabled::Table;
use tabled::Tabled;

slint::include_modules!();

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
fn java_script_ui() {
    play_with_ui();
}

fn main() {
    let matches = command!()
        .version("v0.0.1")
        .propagate_version(true)
        .arg(arg!(
            -d --debug "Turn debugging information on"
        ))
        .subcommand(Command::new("benchmark").about("Runs a benchmark"))
        .subcommand(Command::new("play").about("Play a game"))
        .get_matches();

    let _debug = matches.get_flag("debug");

    match matches.subcommand() {
        Some(("benchmark", _)) => {
            benchmark();
        }
        Some(("play", _)) => {
            play_with_ui();
        }
        None => {
            play_with_ui();
        }
        _ => unreachable!("Exhausted list of subcommands"),
    }
}

fn play_with_ui() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    //let fen = "r2k2nr/3n3p/3b1pp1/4p3/p3P2P/P2RBN2/1PP2PP1/2K4R w - - 0 20";
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

#[derive(Tabled)]
struct BenchmarkRow {
    ply: i32,
    score: i32,
    node_count: u64,
    elapsed_time: f32,
    move_per_sec: f32,
}
fn benchmark() {
    let fen = "1rb2rk1/p4ppp/1p1qp1n1/3n2N1/2pP4/2P3P1/PPQ2PBP/R1B1R1K1 w - - 4 17";
    let chess_board = ChessBoard::from_fen(fen).expect("Invalid FEN string");
    let mut table_rows = Vec::new();
    for d in 0..5 {
        let start_time = Instant::now();
        if let Some((_, score, node_count)) = find_best_move(&chess_board.clone(), d) {
            let elapsed = start_time.elapsed();
            table_rows.push(BenchmarkRow {
                ply: d,
                score,
                node_count,
                elapsed_time: elapsed.as_secs_f32(),
                move_per_sec: node_count as f32 / elapsed.as_secs_f32() / 1000f32,
            });
        } else {
            println!("No best move found!");
        }
    }
    println!("{}", Table::new(table_rows).with(Style::modern()));
}
