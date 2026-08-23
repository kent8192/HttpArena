//! Project management command entry point.

use httparena_reinhardt_grpc as _;
use httparena_reinhardt_grpc::get_settings;
use reinhardt::commands::execute_from_command_line_with_settings;

#[tokio::main]
async fn main() {
    unsafe {
        std::env::set_var(
            "REINHARDT_SETTINGS_MODULE",
            "httparena_reinhardt_grpc.config.settings",
        );
    }

    if let Err(error) = execute_from_command_line_with_settings(get_settings()).await {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
