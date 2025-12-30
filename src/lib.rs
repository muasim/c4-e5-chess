//! `C4-E5 Chess` is a UCI compatible chess engine based on the move generator in crate Chess.
//!
//! These features are provided:
//! * Parallelised iterative depthening
//! * Late move pruning
//! * Principal variant search
//! * Transposition table

/// UCI connector
pub mod cmd;

/// Chess engine
pub mod engine;

/// Board evaluation
pub mod eval;

/// Helpers
pub mod misc;

use wasm_bindgen::prelude::*;
use crate::engine::game::Game;
use crate::engine::move_gen::MoveGenPrime;
use crate::cmd::time_management::TimeManagement;
use cozy_chess::Board;
use std::str::FromStr;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

/// A wrapper for the chess engine to be used from WebAssembly
#[wasm_bindgen]
pub struct ChessEngine {
    game: Game,
    tm: TimeManagement,
}

#[wasm_bindgen]
impl ChessEngine {
    /// Create a new chess engine instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> ChessEngine {
        ChessEngine {
            game: Game::default(),
            tm: TimeManagement::default(),
        }
    }

    /// Set the board position from a FEN string
    pub fn set_position_fen(&mut self, fen: &str) -> bool {
        match Board::from_str(fen) {
            Ok(board) => {
                self.game.board = board;
                true
            }
            Err(_) => {
                log(&format!("Invalid FEN: {}", fen));
                false
            }
        }
    }

    /// Set the board to the starting position
    pub fn set_position_startpos(&mut self) {
        self.game = Game::default();
        self.tm = TimeManagement::default();
    }

    /// Apply moves to the current position
    /// Takes a comma-separated or space-separated string of moves in UCI format
    pub fn apply_moves(&mut self, moves_str: &str) -> bool {
        let moves: Vec<&str> = moves_str.split(|c| c == ',' || c == ' ').filter(|s| !s.is_empty()).collect();
        
        for move_uci in moves {
            match cozy_chess::util::parse_uci_move(&self.game.board, move_uci) {
                Ok(mv) => {
                    self.game.game_history.inc(&self.game.board);
                    self.game.board.play_unchecked(mv);
                    if self.game.board.side_to_move() == cozy_chess::Color::Black {
                        self.game.move_number += 1;
                    }
                }
                Err(_) => {
                    log(&format!("Illegal move: {}", move_uci));
                    return false;
                }
            }
        }
        true
    }

    /// Get the current board position as FEN
    pub fn get_position_fen(&self) -> String {
        self.game.board.to_string()
    }

    /// Get all legal moves for the current position
    pub fn get_legal_moves(&self) -> String {
        let moves = self.game.board.get_legal_sorted(None);
        let move_strings: Vec<String> = moves
            .iter()
            .map(|m| format!("{}", cozy_chess::util::display_uci_move(&self.game.board, m.mv)))
            .collect();
        move_strings.join(",")
    }

    /// Get legal moves for a specific square (if valid)
    pub fn get_moves_for_square(&self, square_str: &str) -> String {
        use cozy_chess::Square;
        
        match Square::from_str(square_str) {
            Ok(square) => {
                let mut moves = Vec::new();
                let square_bb = square.bitboard();
                self.game.board.generate_moves_for(square_bb, |mv| {
                    for m in mv {
                        moves.push(format!("{}", cozy_chess::util::display_uci_move(&self.game.board, m)));
                    }
                    false
                });
                moves.join(",")
            }
            Err(_) => {
                log(&format!("Invalid square: {}", square_str));
                String::new()
            }
        }
    }

    /// Find the best move with given depth
    pub fn find_best_move(&mut self, depth: i16, time_ms: u64) -> String {
        self.game.max_depth = depth;
        self.game.move_time = time_ms;
        
        match self.game.find_move() {
            Some(mv) => format!("{}", cozy_chess::util::display_uci_move(&self.game.board, mv)),
            None => String::from("(none)"),
        }
    }

    /// Find the best move with time control
    pub fn find_best_move_with_time(
        &mut self,
        depth: i16,
        white_time_ms: u64,
        black_time_ms: u64,
        white_inc_ms: u64,
        black_inc_ms: u64,
    ) -> String {
        self.game.max_depth = depth;
        self.tm.white_time = white_time_ms;
        self.tm.black_time = black_time_ms;
        self.tm.white_inc = white_inc_ms;
        self.tm.black_inc = black_inc_ms;
        
        self.tm.set_game_time(&mut self.game);
        
        match self.game.find_move() {
            Some(mv) => format!("{}", cozy_chess::util::display_uci_move(&self.game.board, mv)),
            None => String::from("(none)"),
        }
    }

    /// Make a move on the board
    pub fn make_move(&mut self, move_uci: &str) -> bool {
        match cozy_chess::util::parse_uci_move(&self.game.board, move_uci) {
            Ok(mv) => {
                self.game.game_history.inc(&self.game.board);
                self.game.board.play_unchecked(mv);
                if self.game.board.side_to_move() == cozy_chess::Color::Black {
                    self.game.move_number += 1;
                }
                true
            }
            Err(_) => {
                log(&format!("Illegal move: {}", move_uci));
                false
            }
        }
    }

    /// Check if the game is over (checkmate, stalemate, etc.)
    pub fn is_game_over(&self) -> bool {
        use cozy_chess::GameStatus;
        self.game.board.status() != cozy_chess::GameStatus::Ongoing
    }

    /// Get the game status
    pub fn get_game_status(&self) -> String {
        match self.game.board.status() {
            cozy_chess::GameStatus::Won => "checkmate".to_string(),
            cozy_chess::GameStatus::Drawn => "stalemate".to_string(),
            cozy_chess::GameStatus::Ongoing => "ongoing".to_string(),
        }
    }

    /// Get whose turn it is
    pub fn get_side_to_move(&self) -> String {
        match self.game.board.side_to_move() {
            cozy_chess::Color::White => "white".to_string(),
            cozy_chess::Color::Black => "black".to_string(),
        }
    }

    /// Get the current move number
    pub fn get_move_number(&self) -> u64 {
        self.game.move_number
    }

    /// Reset to a new game
    pub fn reset(&mut self) {
        self.game = Game::default();
        self.tm = TimeManagement::default();
    }
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

