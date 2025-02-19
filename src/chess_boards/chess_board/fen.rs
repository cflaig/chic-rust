use super::Square::Occupied;
use super::ChessBoard;
use super::{ChessField, Color, Piece, PieceType, Square};

pub const INITIAL_POSITION: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Parses a square like "e3" into (file, rank).
fn parse_square(square: &str) -> Result<ChessField, String> {
    if square.len() != 2 {
        return Err(format!("Invalid square: {}", square));
    }
    let file = square.chars().next().unwrap() as usize;
    let rank = square.chars().nth(1).unwrap() as usize;
    if ('a'..='h').contains(&(file as u8 as char)) && ('1'..='8').contains(&(rank as u8 as char)) {
        Ok(ChessField::new((rank - '1' as usize) as u8, (file - 'a' as usize) as u8))
    } else {
        Err(format!("Invalid square: {}", square))
    }
}

/// Parses a FEN string and sets up a ChessBoard.
pub fn from_fen(fen: &str) -> Result<ChessBoard, String> {
    let mut board = ChessBoard::new();
    let parts: Vec<&str> = fen.split(' ').collect();
    if parts.len() != 6 {
        return Err(String::from("Invalid FEN string: must have 6 parts."));
    }

    // Parse board squares
    let rows: Vec<&str> = parts[0].split('/').collect();
    if rows.len() != 8 {
        return Err(String::from("Invalid FEN string: expected 8 rows"));
    }

    for (row_index, row) in rows.iter().enumerate() {
        let mut col_index = 0;

        for c in row.chars() {
            if col_index > 7 {
                return Err(String::from("Invalid FEN string: too many columns"));
            }
            if c.is_ascii_digit() {
                col_index += c.to_digit(10).unwrap() as usize;
            } else {
                let piece = match c {
                    'p' => Some((Color::Black, PieceType::Pawn)),
                    'r' => Some((Color::Black, PieceType::Rook)),
                    'n' => Some((Color::Black, PieceType::Knight)),
                    'b' => Some((Color::Black, PieceType::Bishop)),
                    'q' => Some((Color::Black, PieceType::Queen)),
                    'k' => Some((Color::Black, PieceType::King)),
                    'P' => Some((Color::White, PieceType::Pawn)),
                    'R' => Some((Color::White, PieceType::Rook)),
                    'N' => Some((Color::White, PieceType::Knight)),
                    'B' => Some((Color::White, PieceType::Bishop)),
                    'Q' => Some((Color::White, PieceType::Queen)),
                    'K' => Some((Color::White, PieceType::King)),
                    _ => None,
                };

                if let Some((color, kind)) = piece {
                    board.squares[7 - row_index][col_index] = Square::Occupied(Piece { color, kind });
                    col_index += 1;
                } else {
                    return Err(format!("Invalid piece character in FEN string: {}", c));
                }
            }
        }
        if col_index > 8 {
            return Err(format!("Too many squares in row {} when parsing FEN", row_index));
        }
    }

    // Parse active color
    board.active_color = match parts[1] {
        "w" => Color::White,
        "b" => Color::Black,
        _ => return Err(String::from("Invalid FEN string: invalid active color.")),
    };

    // Parse castling rights
    board.castling_rights = [
        parts[2].contains('K'), // White king-side castling
        parts[2].contains('Q'), // White queen-side castling
        parts[2].contains('k'), // Black king-side castling
        parts[2].contains('q'), // Black queen-side castling
    ];

    // Parse en passant square
    board.en_passant = if parts[3] == "-" {
        None
    } else {
        Some(parse_square(parts[3])?)
    };

    // Parse halfmove clock
    board.halfmove_clock = parts[4]
        .parse::<u8>()
        .map_err(|_| format!("Invalid FEN string: halfmove clock is not a valid number: {}", parts[4]))?;

    // Parse fullmove number
    board.fullmove_number = parts[5].parse::<u8>().map_err(|_| {
        format!(
            "Invalid FEN string: fullmove number is not a valid number: {}",
            parts[5]
        )
    })?;

    Ok(board)
}

pub fn to_fen(board: &ChessBoard) -> String {
    let mut board_representation = String::new();

    for rank in (0..8).rev() {
        let mut empty_count = 0;

        for file in 0..8 {
            match board.squares[rank][file] {
                Occupied(piece) => {
                    if empty_count > 0 {
                        board_representation.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    board_representation.push(piece.to_char());
                }
                Square::Empty => {
                    empty_count += 1;
                }
            }
        }

        if empty_count > 0 {
            board_representation.push_str(&empty_count.to_string());
        }

        if rank > 0 {
            board_representation.push('/');
        }
    }

    let active_color = if board.active_color == Color::White { "w" } else { "b" };

    let mut castling = String::from("KQkq");
    for (i, right) in board.castling_rights.iter().enumerate().rev() {
        if *right == false {
            castling.remove(i);
        }
    }
    if castling.is_empty() {
        castling = "-".to_string();
    }

    // Add en passant square
    let en_passant_square = match board.en_passant {
        Some(square) => square.as_algebraic(),
        None => "-".to_string(),
    };

    let halfmove_clock = board.halfmove_clock;
    let fullmove_number = board.fullmove_number;

    // Construct the full FEN string
    format!(
        "{} {} {} {} {} {}",
        board_representation,
        active_color,
        castling,
        en_passant_square,
        halfmove_clock,
        fullmove_number
    )
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fen_empty_board() {
        let board = ChessBoard::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").expect("Failed to parse FEN");

        for row in 0..8 {
            for col in 0..8 {
                assert_eq!(board.squares[row][col], Square::Empty);
            }
        }
        assert_eq!(board.active_color, Color::White);
        assert_eq!(board.castling_rights, [false, false, false, false]);
        assert_eq!(board.en_passant, None);
        assert_eq!(board.halfmove_clock, 0);
        assert_eq!(board.fullmove_number, 1);
    }

    #[test]
    fn fen_one_pawn() {
        let board = ChessBoard::from_fen("8/8/8/8/8/8/8/P7 w - - 0 1").expect("Failed to parse FEN");
        assert_eq!(
            board.squares[0][0],
            Square::Occupied(Piece {
                color: Color::White,
                kind: PieceType::Pawn
            })
        );
    }

    #[test]
    fn fen_two_pawns() {
        let board = ChessBoard::from_fen("8/8/8/8/8/8/8/P3P3 w - - 0 1").expect("Failed to parse FEN");

        assert_eq!(
            board.squares[0][0],
            Square::Occupied(Piece {
                color: Color::White,
                kind: PieceType::Pawn
            })
        );
        assert_eq!(
            board.squares[0][4],
            Square::Occupied(Piece {
                color: Color::White,
                kind: PieceType::Pawn
            })
        );
    }

    #[test]
    fn fen_initial_board() {
        let board = ChessBoard::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .expect("Failed to parse FEN");

        for col in 0..8 {
            assert_eq!(
                board.squares[1][col],
                Square::Occupied(Piece {
                    color: Color::White,
                    kind: PieceType::Pawn
                })
            );
            assert_eq!(
                board.squares[1][col],
                Square::Occupied(Piece {
                    color: Color::White,
                    kind: PieceType::Pawn
                })
            );
        }

        // Check some specific squares
        assert_eq!(
            board.squares[7][0],
            Square::Occupied(Piece {
                color: Color::Black,
                kind: PieceType::Rook
            })
        );
        assert_eq!(
            board.squares[0][4],
            Square::Occupied(Piece {
                color: Color::White,
                kind: PieceType::King
            })
        );
        assert_eq!(board.squares[3][4], Square::Empty); // Check an empty square

        assert_eq!(board.active_color, Color::White);
        assert_eq!(board.castling_rights, [true, true, true, true]);
        assert_eq!(board.en_passant, None);
        assert_eq!(board.halfmove_clock, 0);
        assert_eq!(board.fullmove_number, 1);
    }

    #[test]
    fn fen_invalid_square() {
        let result = ChessBoard::from_fen("8/8/8/8/8/8/8/X7 w - - 0 1");
        assert!(result.is_err());
    }

    #[test]
    fn fen_invalid_fen_extra_columns() {
        // Too many pieces in the first row
        let fen = "rnbqkbnrr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let result = ChessBoard::from_fen(fen);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_fen_missing_parts() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w";
        let result = ChessBoard::from_fen(fen);
        assert!(result.is_err());
    }

    #[test]
    fn test_en_passant_parsing() {
        let fen = "8/8/8/8/4pP2/8/8/8 b - f3 0 1";
        let board = ChessBoard::from_fen(fen).expect("Failed to parse FEN");

        assert_eq!(board.active_color, Color::Black);
        assert_eq!(board.en_passant, Some(ChessField::from_algebraic("f3"))); // En passant square: f3
    }

    #[test]
    fn fen_halfmove_and_fullmove() {
        let fen = "8/8/8/8/8/8/PPPPPPPP/RNBQKBNR b - - 12 34";
        let board = ChessBoard::from_fen(fen).expect("Failed to parse FEN");

        assert_eq!(board.halfmove_clock, 12);
        assert_eq!(board.fullmove_number, 34);
    }

    #[test]
    fn fen_castling_rights() {
        let fen = "8/8/8/8/8/8/8/8 w Kq - 0 1";
        let board = ChessBoard::from_fen(fen).expect("Failed to parse FEN");

        assert_eq!(board.castling_rights, [true, false, false, true]); // White King side, Black Queen side
    }

    #[test]
    fn test_to_fen_initial_position() {
        let board = ChessBoard::from_fen(INITIAL_POSITION).unwrap();
        assert_eq!(board.to_fen(), INITIAL_POSITION);
    }

    #[test]
    fn test_to_fen_empty_board() {
        let board = ChessBoard::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").unwrap(); // Assuming there's a method to create an empty board
        assert_eq!(board.to_fen(), "8/8/8/8/8/8/8/8 w - - 0 1");
    }

    #[test]
    fn test_to_fen_custom_position() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w Kq e3 0 2";
        let board = ChessBoard::from_fen(fen).unwrap();
        assert_eq!(board.to_fen(), fen);
    }

}
