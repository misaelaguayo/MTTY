fn main() {
    // a long running process
    // useful for testing that the shell can run processes in the background

    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("Long running process finished!");
}
