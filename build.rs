use vergen::EmitBuilder;

fn main() {
    // configure vergen to generate the required environment variables
    if let Err(error) = EmitBuilder::builder()
        .rustc_semver()
        .cargo_target_triple()
        .git_describe(true, true, None)
        .build_date()
        .build_timestamp()
        .emit()
    {
        panic!(
            "Could not extract the required version information. The error was: {}",
            error
        );
    }
}
