use crate::chess_board::Move;
use std::time::Instant;
mod chess_board;
mod engine_alpha_beta;
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

const INITIAL_POSITION: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn main() {
    let matches = command!()
        .version("v0.0.1")
        .propagate_version(true)
        .arg(arg!(
            -d --debug "Turn debugging information on"
        ))
        .subcommand(Command::new("benchmark").about("Runs a benchmark"))
        .subcommand(Command::new("play").about("Play a game"))
        .subcommand(
            Command::new("perft")
                .about("Run Perft test")
                .arg(
                    arg!(
                    -f --fen <FEN> "Board position"
                            )
                    .default_value(INITIAL_POSITION),
                )
                .arg(
                    arg!(
                    -x --depth <d> "depth"
                            )
                    .default_value("3")
                    .value_parser(clap::value_parser!(usize)),
                )
                .arg(
                    arg!(
                    -m --moves <moves> "List of moves"
                            )
                    .num_args(1..)
                    .value_parser(clap::value_parser!(String)),
                ),
        )
        .get_matches();

    let _debug = matches.get_flag("debug");

    match matches.subcommand() {
        Some(("benchmark", _)) => {
            benchmark();
        }
        Some(("play", _)) => {
            play_with_ui();
        }
        Some(("perft", arg_matches)) => {
            let fen = arg_matches.get_one::<String>("fen").unwrap();
            let depth = arg_matches.get_one::<usize>("depth").unwrap();
            let moves = arg_matches
                .get_many::<String>("moves")
                .unwrap_or_default()
                .filter(|&v| !v.is_empty())
                .collect::<Vec<_>>();
            perft(fen.clone(), moves, (*depth) as u8);
        }
        None => {
            play_with_ui();
        }
        _ => unreachable!("Exhausted list of subcommands"),
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
fn play_with_ui() {
    let fen = INITIAL_POSITION;
    //let fen = "r2k2nr/3n3p/3b1pp1/4p3/p3P2P/P2RBN2/1PP2PP1/2K4R w - - 0 20";
    setup_ui(fen);
}

#[derive(Tabled)]
struct BenchmarkRow {
    ply: i32,
    score: i32,
    node_count: u64,
    elapsed_time: f32,
    move_per_sec: f32,
    best_move: String,
}
fn benchmark() {
    let fen = "1rb2rk1/p4ppp/1p1qp1n1/3n2N1/2pP4/2P3P1/PPQ2PBP/R1B1R1K1 w - - 4 17";
    //let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let chess_board = ChessBoard::from_fen(fen).expect("Invalid FEN string");
    let mut table_rows = Vec::new();
    for d in 0..6 {
        let start_time = Instant::now();
        if let Some((m, score, node_count)) = find_best_move(&chess_board.clone(), d, false) {
            let elapsed = start_time.elapsed();
            table_rows.push(BenchmarkRow {
                ply: d,
                score,
                node_count,
                elapsed_time: elapsed.as_secs_f32(),
                move_per_sec: node_count as f32 / elapsed.as_secs_f32() / 1000f32,
                best_move: m.as_algebraic(),
            });
            if elapsed.as_secs() > 10 {
                break;
            }
        } else {
            println!("No best move found!");
        }
    }
    println!("{}", Table::new(table_rows).with(Style::modern()));
}

fn perft(fen: String, moves: Vec<&String>, depth: u8) {
    println!("Perft test for {} moves {:?} with depth {}", fen, moves, depth);
    let mut chess_board = ChessBoard::from_fen(&fen).unwrap();
    for m in moves {
        let legal_move = chess_board.generate_legal_moves();
        if legal_move.contains(&Move::from_algebraic(&m)) {
            chess_board.make_move(Move::from_algebraic(&m));
        } else {
            panic!("Invalid move: {}", m);
        }
    }

    let mut result_moves = Vec::<(String, u64)>::new();
    for mv in chess_board.generate_legal_moves() {
        let mut new_board = chess_board.clone();
        new_board.make_move(mv);
        result_moves.push((mv.as_algebraic(), chess_board::perft(&new_board, depth - 1)));
    }
    result_moves.sort();

    let mut num_nodes = 0;
    for (m, c) in result_moves {
        println!("{}: {}", m, c);
        num_nodes += c;
    }
    println!("\nNodes searched: {}", num_nodes);
}
