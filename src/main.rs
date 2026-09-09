fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if saena::cli::main(&argv).is_err() {
        std::process::exit(1);
    }
}
