use std::sync::{Mutex, OnceLock};

type OutputFn = Box<dyn Fn(&str) + Send + Sync>;

static OUTPUT_FN: OnceLock<Mutex<Option<OutputFn>>> = OnceLock::new();

fn output_fn() -> &'static Mutex<Option<OutputFn>> {
    OUTPUT_FN.get_or_init(|| Mutex::new(None))
}

pub fn set_output_fn(f: OutputFn) {
    *output_fn().lock().unwrap() = Some(f);
}

pub fn clear_output_fn() {
    *output_fn().lock().unwrap() = None;
}

pub fn print_output(line: &str) {
    let guard = output_fn().lock().unwrap();
    if let Some(f) = guard.as_ref() {
        f(line);
        return;
    }
    drop(guard);
    println!("{line}");
}

pub fn print_error(line: &str) {
    let guard = output_fn().lock().unwrap();
    if let Some(f) = guard.as_ref() {
        f(line);
        return;
    }
    drop(guard);
    eprintln!("{line}");
}
