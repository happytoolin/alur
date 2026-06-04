use std::process::ExitCode;

fn main() -> ExitCode {
    match alur::app::dispatch::run_from_env() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{}", alur::app::error_report::render_error(&err));
            ExitCode::from(1)
        }
    }
}
