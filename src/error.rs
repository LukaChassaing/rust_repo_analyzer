use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum GithubAnalyzerError {
    NetworkError(String),
    ParseError(String),
    RateLimitError(u64),  // Contains reset timestamp
}

impl fmt::Display for GithubAnalyzerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GithubAnalyzerError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            GithubAnalyzerError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            GithubAnalyzerError::RateLimitError(reset_time) => 
                write!(f, "Rate limit exceeded. Resets at timestamp: {}", reset_time),
        }
    }
}

impl Error for GithubAnalyzerError {}