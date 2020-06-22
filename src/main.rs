use podcasts;

fn main() {
    let mut app = podcasts::Application::new();
    if let Err(error) = app.parse() {
        eprintln!("{}", error);
        std::process::exit(1);
    }
}
