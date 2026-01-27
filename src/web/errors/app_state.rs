/// Represents different conflicting app states.
/// 
/// For example if the App is already running, Running will be returned.
#[derive(Debug)]
pub enum AppState {
    // The app is running
    Running,

    //The app is closed
    Closed
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        
        let state = match self {
            Self::Running => "already running",
            Self::Closed => "already closed"
        };
        
        write!(f, "{}", state)
    }
}

impl std::error::Error for AppState{}