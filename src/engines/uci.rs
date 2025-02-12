use crate::chess_board::fen::INITIAL_POSITION;
use crate::chess_board::Color;
use crate::engines::engine_alpha_beta::AlphaBetaEngine;
use crate::engines::ChessEngine;
use std::io::BufRead;
use std::io::Write;
use std::io::{stdin, stdout};
use std::result;
use std::time::Duration;

pub(crate) fn run_uci_interface() {
    let mut engine = AlphaBetaEngine::new();

    for line in stdin().lock().lines() {
        let line = match line {
            Ok(l) => l.trim().to_string(),
            Err(_) => continue,
        };

        if line.is_empty() {
            continue;
        }

        // Parse UCI commands:
        let tokens: Vec<&str> = line.split_whitespace().collect();
        match tokens[0] {
            "uci" => {
                println!("id name {}", engine.name());
                println!("id author {}", engine.author());
                println!("uciok");
                stdout().flush().unwrap();
            }
            "isready" => {
                println!("readyok");
                stdout().flush().unwrap();
            }
            "ucinewgame" => {
                //current_board_state.clear();
            }
            "position" => match parse_position(tokens) {
                Ok((start_fen, moves)) => {
                    engine.set_position(start_fen.as_str()).unwrap();
                    for mv in moves {
                        engine.make_move(mv.as_str()).unwrap();
                    }
                }
                Err(e) => {
                    println!("Error parsing position command: {}", e);
                }
            },
            "go" => {
                let search_time = parse_go_command(&tokens[1..], engine.get_active_player());
                let (best_move, score, node_count, depth) =
                    engine.find_best_move_iterative(search_time, uci_info_callback).unwrap();

                println!("bestmove {}", best_move.as_algebraic());
                stdout().flush().unwrap();
            }
            "stop" => {
                // If the engine was searching asynchronously, you’d stop and return best move
                // For a simple synchronous example, there’s nothing special to do here
            }
            "quit" => {
                return;
            }
            _ => {
                // Ignore or handle custom commands
            }
        }
    }
}

fn uci_info_callback(depth: i32, score: i32, nodes: u64, elapsed: Duration, pv: String) {
    let time_ms = elapsed.as_millis();
    let nps = if elapsed.as_secs_f64() > 0.0 {
        (nodes as f64 / elapsed.as_secs_f64()) as u64
    } else {
        0
    };

    println!(
        "info depth {} score cp {} time {} nodes {} nps {} pv {}",
        depth,
        score / 10,
        time_ms,
        nodes,
        nps,
        pv
    );
    stdout().flush().unwrap();
}

fn parse_position(tokens: Vec<&str>) -> result::Result<(String, Vec<String>), &'static str> {
    if tokens.len() < 2 {
        return Err("Invalid position command");
    }

    let mut idx = 1;

    let position = match tokens[idx] {
        "startpos" => INITIAL_POSITION.to_string(),
        "fen" => {
            let mut v: Vec<String> = Vec::new();
            for token in tokens.iter().take(idx + 7).skip(idx + 1) {
                v.push((*token).to_string());
            }
            idx += 6;
            v.join(" ")
        }
        _ => return Err("Invalid position command"),
    };

    let mut moves: Vec<String> = Vec::new();
    idx += 1;
    if tokens.len() == idx + 1 {
        return Err("Invalid position command. No Moves specified.");
    } else if tokens.len() > idx + 1 {
        if tokens[idx] != "moves" {
            return Err("Invalid position command. Keyword 'moves' expected.");
        }
        idx += 1;
        for move_str in tokens[idx..].iter() {
            moves.push(move_str.to_string());
        }
    }
    Ok((position, moves))
}

fn parse_go_command(tokens: &[&str], active_color: Color) -> Duration {
    let fallback = Duration::from_secs(5);

    let mut wtime: Option<u64> = None;
    let mut btime: Option<u64> = None;
    let mut movestogo: Option<u64> = None;
    let mut winc: Option<u64> = None;
    let mut binc: Option<u64> = None;

    // Parse the sub-commands following "go"
    // Example: ["wtime", "266667", "btime", "244787", "movestogo", "33"]
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "wtime" => {
                if i + 1 < tokens.len() {
                    wtime = tokens[i + 1].parse().ok();
                    i += 1;
                }
            }
            "btime" => {
                if i + 1 < tokens.len() {
                    btime = tokens[i + 1].parse().ok();
                    i += 1;
                }
            }
            "movestogo" => {
                if i + 1 < tokens.len() {
                    movestogo = tokens[i + 1].parse().ok();
                    i += 1;
                }
            }
            "winc" => {
                if i + 1 < tokens.len() {
                    winc = tokens[i + 1].parse().ok();
                    i += 1;
                }
            }
            "binc" => {
                if i + 1 < tokens.len() {
                    binc = tokens[i + 1].parse().ok();
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let (time_left_millis, increment_milis) = match active_color {
        Color::White => (wtime.unwrap_or(0), winc.unwrap_or(0)),
        Color::Black => (btime.unwrap_or(0), binc.unwrap_or(0)),
    };

    let moves_to_go = movestogo.unwrap_or(30).max(1); // avoid divide by zero
    let time_for_this_move_ms = time_left_millis / (moves_to_go) + increment_milis;

    if time_for_this_move_ms > time_left_millis {
        Duration::from_millis(time_left_millis - 5)
    } else if time_for_this_move_ms > 0 {
        Duration::from_millis(time_for_this_move_ms)
    } else {
        fallback
    }
}
