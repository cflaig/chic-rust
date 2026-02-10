use crate::chess_boards::chess_board::{ChessBoard, Move};

pub fn parse_pgn(pgn: &str) -> Result<(ChessBoard, Vec<Move>), String> {
    let mut current_board = ChessBoard::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")?;
    let mut moves = Vec::new();

    // Skip tags
    let moves_text = pgn
        .lines()
        .filter(|line| !line.starts_with('[') && !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    // Basic tokenizer for SAN moves and move numbers
    let tokens = moves_text.split_whitespace();

    for token in tokens {
        // Skip move numbers like "1.", "1...", "2."
        if token.contains('.') {
            continue;
        }

        // Skip result like "1-0", "0-1", "1/2-1/2", "*"
        if token == "1-0" || token == "0-1" || token == "1/2-1/2" || token == "*" {
            break;
        }

        // Handle variations (skip them)
        if token.starts_with('(') {
            // This is a very simple parser, it won't handle nested variations well
            // but for "always only the first game and only the main variant" it might be enough
            // if we just stop or skip until matching bracket.
            // For now, let's just ignore everything after a variation starts if we want strictly main variant.
            // Or better, let's just skip the variation token.
            continue;
        }

        if token.contains('{') || token.contains('}') || token.contains('(') || token.contains(')') {
            // Complex PGN features we don't support yet, just skip or break
            continue;
        }

        match Move::from_san(&current_board, token) {
            Ok(mv) => {
                moves.push(mv);
                current_board.make_move(mv);
            }
            Err(e) => {
                // If it's not a move, it might be a result we missed or just garbage
                println!("Skipping token {}: {}", token, e);
            }
        }
    }

    Ok((
        ChessBoard::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")?,
        moves,
    ))
}
