use std::env;
use std::io::{self, Read};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_PROJECT_NAME: &str = "rustms-integration";
const DEFAULT_LOGIN_PORT: u16 = 18484;
const DEFAULT_WORLD_PORT: u16 = 18485;
const DEFAULT_DB_PORT: u16 = 15432;

/// Host-side identity of one harness stack. Overriding these lets several
/// checkouts (for example git worktrees) run isolated stacks side by side.
struct StackConfig {
    project_name: String,
    login_port: u16,
    world_port: u16,
    db_port: u16,
}

impl StackConfig {
    fn from_env() -> Result<Self, String> {
        Ok(Self {
            project_name: env::var("HARNESS_PROJECT_NAME")
                .unwrap_or_else(|_| DEFAULT_PROJECT_NAME.to_string()),
            login_port: port_from_env("HARNESS_LOGIN_PORT", DEFAULT_LOGIN_PORT)?,
            world_port: port_from_env("HARNESS_WORLD_PORT", DEFAULT_WORLD_PORT)?,
            db_port: port_from_env("HARNESS_DB_PORT", DEFAULT_DB_PORT)?,
        })
    }

    fn login_addr(&self) -> String {
        format!("127.0.0.1:{}", self.login_port)
    }

    fn world_addr(&self) -> String {
        format!("127.0.0.1:{}", self.world_port)
    }

    fn database_url(&self) -> String {
        format!(
            "postgres://maplestory:pass@127.0.0.1:{}/maplestory_test",
            self.db_port
        )
    }
}

fn port_from_env(name: &str, default: u16) -> Result<u16, String> {
    match env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|_| format!("{name} must be a valid port, got `{value}`")),
        Err(_) => Ok(default),
    }
}

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

    let stack = StackConfig::from_env()?;
    match command.as_str() {
        "up" => up_stack(&stack),
        "down" => down_stack(&stack),
        "test" => run_tests(&stack),
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

fn run_tests(stack: &StackConfig) -> Result<(), String> {
    let total_start = Instant::now();
    if let Err(error) = up_stack(stack) {
        dump_stack_logs(stack);
        let _ = down_stack(stack);
        return Err(error);
    }

    let workspace_root = workspace_root();
    let tests_start = Instant::now();
    let status = Command::new("cargo")
        .arg("test")
        .arg("-p")
        .arg("integration-harness")
        .env("HARNESS_LOGIN_ADDR", stack.login_addr())
        .env("HARNESS_WORLD_ADDR", stack.world_addr())
        .env("HARNESS_DATABASE_URL", stack.database_url())
        .current_dir(&workspace_root)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("failed to execute cargo test: {error}"));

    eprintln!(
        "harness cargo test (including compilation): {:.3}s",
        tests_start.elapsed().as_secs_f64()
    );
    if !matches!(&status, Ok(status) if status.success()) {
        dump_stack_logs(stack);
    }

    let down_result = down_stack(stack);
    eprintln!(
        "harness total lifecycle: {:.3}s",
        total_start.elapsed().as_secs_f64()
    );
    let status = status?;
    if !status.success() {
        return Err(format!("cargo test failed with status {status}"));
    }
    down_result
}

fn up_stack(stack: &StackConfig) -> Result<(), String> {
    let _ = down_stack(stack);
    let build_start = Instant::now();
    compose_cmd(stack, ["build", "-q"])?;
    eprintln!(
        "harness image build: {:.3}s",
        build_start.elapsed().as_secs_f64()
    );
    let startup_start = Instant::now();
    compose_cmd(stack, ["up", "-d", "--no-build", "--remove-orphans"])?;
    wait_for_endpoint(&stack.login_addr(), Duration::from_secs(60))?;
    wait_for_endpoint(&stack.world_addr(), Duration::from_secs(60))?;
    eprintln!(
        "harness startup/readiness: {:.3}s",
        startup_start.elapsed().as_secs_f64()
    );
    Ok(())
}

fn dump_stack_logs(stack: &StackConfig) {
    eprintln!("harness failed; collecting container logs before teardown:");
    if let Err(error) = compose_cmd(stack, ["logs", "--no-color", "--tail", "200"]) {
        eprintln!("failed to collect harness logs: {error}");
    }
}

fn down_stack(stack: &StackConfig) -> Result<(), String> {
    compose_cmd(stack, ["down", "-v", "--remove-orphans"])
}

fn compose_cmd<const N: usize>(stack: &StackConfig, args: [&str; N]) -> Result<(), String> {
    let status = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg(compose_file_path())
        .arg("-p")
        .arg(&stack.project_name)
        .args(args)
        .env("HARNESS_PROJECT_NAME", &stack.project_name)
        .env("HARNESS_LOGIN_PORT", stack.login_port.to_string())
        // Channels 1-3 listen on consecutive ports starting at the world port.
        .env("HARNESS_WORLD_PORT", stack.world_port.to_string())
        .env("HARNESS_WORLD_PORT_2", (stack.world_port + 1).to_string())
        .env("HARNESS_WORLD_PORT_3", (stack.world_port + 2).to_string())
        // The vote hook follows them.
        .env("HARNESS_VOTE_PORT", (stack.world_port + 3).to_string())
        .env("HARNESS_DB_PORT", stack.db_port.to_string())
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

/// Wait until the server at `addr` sends the start of its handshake. A bare
/// TCP connect is not enough: Docker's port proxy accepts connections before
/// the server inside the container listens, then closes them.
fn wait_for_endpoint(addr: &str, timeout: Duration) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Ok(mut stream) = TcpStream::connect(addr) {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
            let mut header = [0u8; 2];
            if stream.read_exact(&mut header).is_ok() {
                return Ok(());
            }
        }
        thread::sleep(Duration::from_millis(250));
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
