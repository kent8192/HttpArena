//! Management commands for the benchmark app.

use reinhardt::commands::{BaseCommand, CommandContext, CommandError, CommandResult};

pub struct RunGrpcCommand;

#[reinhardt_di::async_trait::async_trait]
impl BaseCommand for RunGrpcCommand {
    fn name(&self) -> &str {
        "rungrpc"
    }

    fn description(&self) -> &str {
        "Runs the HttpArena gRPC benchmark server"
    }

    fn requires_system_checks(&self) -> bool {
        false
    }

    async fn execute(&self, _context: &CommandContext) -> CommandResult<()> {
        crate::config::urls::serve_grpc()
            .await
            .map_err(|error| CommandError::ExecutionError(error.to_string()))
    }
}
