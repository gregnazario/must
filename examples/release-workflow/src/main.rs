fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--version" {
        println!("my-app 1.0.0");
        return;
    }
    println!("my-app running");
}
