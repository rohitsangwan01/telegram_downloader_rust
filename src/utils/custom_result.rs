pub type ResultGram<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type ResultUpdate = std::result::Result<(), Box<dyn std::error::Error>>;
