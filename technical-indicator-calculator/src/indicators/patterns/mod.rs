// Module exports
mod recognizer;
mod single_candle;
mod double_candle;
mod triple_candle;
mod utils;

// Public exports
pub use recognizer::PatternRecognizer;
pub use utils::PATTERN_STRENGTH_THRESHOLD;
