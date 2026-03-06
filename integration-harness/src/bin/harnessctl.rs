use std::env;
use std::io;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const PROJECT_NAME: &str = "rustms-integration";
const LOGIN_ADDR: &str = "127.0.0.1:18484";
const WORLD_ADDR: &str = "127.0.0.1:18485";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("harnessctl error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args();
    let _program = args.next();
    let command = args.next().unwrap_or_else(|| "test".to_string());

    if matches!(command.as_str(), "up" | "down" | "test") {
        ensure_docker_available()?;
    }

    match command.as_str() {
        "up" => up_stack(),
        "down" => down_stack(),
        "test" => run_tests(),
        other => Err(format!(
            "unsupported command `{other}`; expected one of: up, down, test"
        )),
    }
}

fn ensure_docker_available() -> Result<(), String> {
    let docker_status = Command::new("docker")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match docker_status {
        Ok(status) if status.success() => {}
        Ok(status) => {
            return Err(format!(
                "docker is installed but not usable (status: {status}); ensure Docker daemon is running"
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(
                "docker is not installed or not in PATH; install Docker Engine and Docker Compose"
                    .to_string(),
            );
        }
        Err(error) => return Err(format!("failed to execute `docker --version`: {error}")),
    }

    let compose_status = Command::new("docker")
        .arg("compose")
        .arg("version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match compose_status {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err(
            "docker is installed but `docker compose` is unavailable; install Docker Compose v2 plugin"
                .to_string(),
        ),
        Err(error) => Err(format!("failed to execute `docker compose version`: {error}")),
    }?;

    let daemon_status = Command::new("docker")
        .arg("info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match daemon_status {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err(
            "docker daemon is not reachable; start the Docker service and ensure your user can access /var/run/docker.sock"
                .to_string(),
        ),
        Err(error) => Err(format!("failed to execute `docker info`: {error}")),
    }
}

fn run_tests() -> Result<(), String> {
    if let Err(error) = up_stack() {
        let _ = down_stack();
        return Err(error);
    }

    let workspace_root = workspace_root();
    let status = Command::new("cargo")
        .arg("test")
        .arg("-p")
        .arg("integration-harness")
        .env("HARNESS_LOGIN_ADDR", LOGIN_ADDR)
        .env("HARNESS_WORLD_ADDR", WORLD_ADDR)
        .current_dir(&workspace_root)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("failed to execute cargo test: {error}"))?;

    let down_result = down_stack();
    if !status.success() {
        return Err(format!("cargo test failed with status {status}"));
    }
    down_result
}

fn up_stack() -> Result<(), String> {
    let _ = down_stack();
    compose_cmd(["build", "-q"])?;
    compose_cmd(["up", "-d", "--no-build", "--remove-orphans"])?;
    wait_for_endpoint(LOGIN_ADDR, Duration::from_secs(60))?;
    wait_for_endpoint(WORLD_ADDR, Duration::from_secs(60))?;
    Ok(())
}

fn down_stack() -> Result<(), String> {
    compose_cmd(["down", "-v", "--remove-orphans"])
}

fn compose_cmd<const N: usize>(args: [&str; N]) -> Result<(), String> {
    let status = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg(compose_file_path())
        .arg("-p")
        .arg(PROJECT_NAME)
        .args(args)
        .current_dir(workspace_root())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("failed to execute docker compose: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "docker compose command failed with status {status}"
        ))
    }
}

fn wait_for_endpoint(addr: &str, timeout: Duration) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        match TcpStream::connect(addr) {
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => {
                thread::sleep(Duration::from_millis(250));
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(250));
            }
        }
    }

    Err(format!(
        "timed out waiting for endpoint {addr} after {}s",
        timeout.as_secs()
    ))
}

fn compose_file_path() -> PathBuf {
    workspace_root()
        .join("integration-harness")
        .join("docker-compose.test.yml")
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("integration-harness should be in workspace root")
}
