use crate::chess_boards::chess_board::LazySortedMoves;
use super::Move;

#[cfg(test)]
pub fn assert_moves<I: Iterator<Item = Move>>(generated: I, mut expected: Vec<&str>) {
    let mut generated_converted: Vec<_> = generated.map(|m| m.as_algebraic()).collect();
    generated_converted.sort();
    expected.sort();

    assert_eq!(generated_converted, expected);
}