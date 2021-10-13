pub enum DebuggerCommand {
    Quit,
    Cont,
    Backtrace,
    Break(Vec<String>),
    Run(Vec<String>),
}

impl DebuggerCommand {
    pub fn from_tokens(tokens: &Vec<&str>) -> Option<DebuggerCommand> {
        match tokens[0] {
            "q" | "quit" => Some(DebuggerCommand::Quit),
            "r" | "run" => {
                let args = tokens[1..].to_vec();
                Some(DebuggerCommand::Run(
                    args.iter().map(|s| s.to_string()).collect(),
                ))
            },
            "c" | "cont" | "continue" => Some(DebuggerCommand::Cont),
            "bt" | "back" | "backtrace" => Some(DebuggerCommand::Backtrace), 
            "b" | "break" => {
                let args = tokens[1..].to_vec();
                Some(DebuggerCommand::Break(
                    args.iter().map(|s| s.to_string()).collect(),
                ))
            },
            // Default case:
            _ => None,
        }
    }
}
