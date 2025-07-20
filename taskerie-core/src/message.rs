pub enum ExecutionMessage {
    MissingRequiredTaskParameter {
        parameter_name: String,
    },
    WorkingDirectoryNotFound {
        path: String,
    },
    AboutToRunCommand {
        command: String,
        working_directory: String,
    },
    CommandOutput {
        output: String,
    },
    CommandFailed {
        command: String,
        working_directory: String,
    },
    CommandSucceeded {
        command: String,
        working_directory: String,
    },
}
