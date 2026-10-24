use sysinfo::{ProcessesToUpdate, System};

fn main() {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    println!("processes: {}", system.processes().len());

    for (pid, process) in system.processes() {
        let name = process.name().to_string_lossy().to_ascii_lowercase();
        if !name.contains("language_server") {
            continue;
        }
        println!(
            "pid={} name={} exe={:?} cmd_len={} start={}",
            pid.as_u32(),
            name,
            process.exe().map(|p| p.to_string_lossy().into_owned()),
            process.cmd().len(),
            process.start_time()
        );
        for arg in process.cmd() {
            println!("   arg: {}", arg.to_string_lossy());
        }
    }
}
