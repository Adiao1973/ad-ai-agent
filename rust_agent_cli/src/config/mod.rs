mod args;

pub use args::Args;

pub fn get_config() -> Args {
    Args::new()
}
