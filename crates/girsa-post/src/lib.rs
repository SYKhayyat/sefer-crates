//! The post between the two applications: loopback, token-gated, no network.
//!
//! spec.md §10.6, BUILDER.md W16. Girsa is the library and Ksav is the pen, and
//! this is how one hands something to the other while both are running:
//!
//! ```text
//! Girsa ──POST /insert──▶ Ksav      a source, on its way into a document
//! Ksav  ──POST /open  ──▶ Girsa     a citation, on its way to the page it names
//! Ksav  ──GET  /cite  ──▶ Girsa     print this ref in that style
//! either ─GET  /health──▶ other     is my sibling there?
//! ```
//!
//! # Nothing leaves the machine
//!
//! The listener binds `127.0.0.1` on a port the operating system picks. There
//! is no configuration for the address, deliberately: a bind address in a
//! settings file is a bind address that is one typo away from `0.0.0.0`, and
//! spec.md §14 says offline is the product.
//!
//! # Why a token, when it is only localhost
//!
//! **Localhost is not private.** Every process on the machine can reach a
//! loopback port, and so can a web page: a browser tab may `POST` to
//! `127.0.0.1:51234` without asking anybody. So every request carries a token,
//! minted per run, published in a file only the user can read:
//!
//! - **The token is required on every path**, including `/health`. A health
//!   endpoint that answers without one tells a stranger which port is Girsa.
//! - **It travels in a header, not the URL.** URLs end up in logs and in
//!   `Referer`; a token in a query string is a token that leaks sideways.
//! - **A custom header is itself part of the defence.** A browser cannot send
//!   one cross-origin without a preflight, and this server answers no preflight
//!   and sends no CORS headers at all — so a page that guesses the port and
//!   the token still cannot read a reply.
//!
//! # Presence, so nothing is offered that would fail
//!
//! [`presence`] reads the sibling's endpoint file and *asks it*. A file left
//! behind by a crashed app is not presence, and neither is a port that
//! something else has taken since — both come back as [`Presence::Stale`] with
//! the reason, which is what the window shows instead of an affordance that
//! would do nothing.

#![doc(html_root_url = "https://docs.rs/girsa-post/0.3.0")]

use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[cfg(feature = "serve")]
pub mod desk;

mod link;
pub use link::{deep_link, Errand};

/// The header every request carries its token in.
pub const TOKEN_HEADER: &str = "X-Girsa-Token";

/// How long to wait on a sibling that is not answering.
///
/// Short on purpose: this runs while a reader is looking at a menu, and a
/// second of nothing is a window that has frozen for no reason they can see.
const PATIENCE: Duration = Duration::from_millis(400);

/// The two applications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum App {
    Girsa,
    Ksav,
}

impl App {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Girsa => "girsa",
            Self::Ksav => "ksav",
        }
    }

    /// The other one.
    #[must_use]
    pub fn sibling(self) -> Self {
        match self {
            Self::Girsa => Self::Ksav,
            Self::Ksav => Self::Girsa,
        }
    }
}

impl fmt::Display for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a running application can be reached, and what to say to it.
///
/// Written to a file the moment the listener is up and deleted when it stops.
/// The file is the only discovery mechanism there is: no port scanning, no
/// fixed port to collide with whatever else the machine is running.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub app: App,
    pub port: u16,
    /// Minted per run. See the module note for why it exists on localhost.
    pub token: String,
    /// So a stale file can be told from a live one before anything is sent.
    pub pid: u32,
    /// The application's own version, shown in the presence chip.
    pub version: String,
}

/// Why the post could not be delivered.
#[derive(Debug, thiserror::Error)]
pub enum PostError {
    #[error("{0} is not running")]
    NotRunning(App),
    #[error("could not reach {app}: {source}")]
    Unreachable {
        app: App,
        #[source]
        source: std::io::Error,
    },
    #[error("{app} refused it: {status} {body}")]
    Refused { app: App, status: u16, body: String },
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

impl Endpoint {
    /// Where the endpoint file for an application lives.
    ///
    /// Under the user's own data directory, never anywhere shared: the token in
    /// it is what stands between a citation being inserted into your document
    /// and anybody on the machine doing it.
    #[must_use]
    pub fn path(app: App) -> PathBuf {
        home().join(format!("{app}-endpoint.json"))
    }

    /// Write it down, so the sibling can find this application.
    ///
    /// # Errors
    ///
    /// If the directory cannot be made or the file cannot be written.
    pub fn publish(&self) -> std::io::Result<()> {
        let path = Self::path(self.app);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, serde_json::to_vec_pretty(self)?)?;
        restrict_to_owner(&path)?;
        Ok(())
    }

    /// Read the sibling's endpoint file, if it has one.
    #[must_use]
    pub fn read(app: App) -> Option<Self> {
        let body = std::fs::read_to_string(Self::path(app)).ok()?;
        serde_json::from_str(&body).ok()
    }

    /// Take the file away. Called when the application stops.
    pub fn withdraw(app: App) {
        let _ = std::fs::remove_file(Self::path(app));
    }

    #[must_use]
    fn address(&self) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, self.port))
    }
}

/// Whether the sibling is there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Presence {
    /// Answering, and this is what it says it is.
    Live { version: String },
    /// No endpoint file: it has not been started.
    NotRunning,
    /// There is a file and nothing behind it — a crash, or a port something
    /// else has taken since. **Shown rather than hidden**: an affordance that
    /// silently does nothing is worse than one that says why.
    Stale { why: String },
}

impl Presence {
    #[must_use]
    pub fn is_live(&self) -> bool {
        matches!(self, Self::Live { .. })
    }
}

/// Ask the sibling whether it is there.
#[must_use]
pub fn presence(app: App) -> Presence {
    let Some(endpoint) = Endpoint::read(app) else {
        return Presence::NotRunning;
    };
    match ask(&endpoint, "GET", "/health", None) {
        Ok(body) => match serde_json::from_str::<Health>(&body) {
            Ok(health) if health.app == app => Presence::Live {
                version: health.version,
            },
            Ok(health) => Presence::Stale {
                why: format!("something calling itself {} answered", health.app),
            },
            Err(e) => Presence::Stale {
                why: format!("the answer was not {app}'s: {e}"),
            },
        },
        Err(e) => Presence::Stale { why: e.to_string() },
    }
}

/// What `/health` answers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub app: App,
    pub version: String,
}

/// Send something to the sibling and read what comes back.
///
/// # Errors
///
/// If it is not running, will not answer, or refuses the request.
pub fn send(app: App, path: &str, body: Option<&str>) -> Result<String, PostError> {
    let endpoint = Endpoint::read(app).ok_or(PostError::NotRunning(app))?;
    ask(
        &endpoint,
        if body.is_some() { "POST" } else { "GET" },
        path,
        body,
    )
}

/// One request, over loopback, spoken by hand.
///
/// HTTP/1.1 with a fixed shape and a known-length body is a dozen lines, and
/// the alternative is an HTTP client stack compiled into both applications for
/// two request kinds that never leave the machine.
fn ask(
    endpoint: &Endpoint,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<String, PostError> {
    let app = endpoint.app;
    let mut stream = TcpStream::connect_timeout(&endpoint.address(), PATIENCE)
        .map_err(|source| PostError::Unreachable { app, source })?;
    stream.set_read_timeout(Some(PATIENCE))?;
    stream.set_write_timeout(Some(PATIENCE))?;

    let body = body.unwrap_or_default();
    let request = format!(
        "{method} {path} HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         {TOKEN_HEADER}: {token}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\r\n{body}",
        port = endpoint.port,
        token = endpoint.token,
        len = body.len(),
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);

    // Headers, to the blank line. Nothing here reads them: the body length is
    // whatever arrives before the connection closes, which is well defined
    // because the request asked for `Connection: close`.
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
    }
    let mut answer = String::new();
    reader.read_to_string(&mut answer)?;

    if (200..300).contains(&status) {
        Ok(answer)
    } else {
        Err(PostError::Refused {
            app,
            status,
            body: answer.trim().to_string(),
        })
    }
}

/// A token nobody can guess: 32 hex characters from the operating system's own
/// randomness.
///
/// Not from the clock and not from the pid. Both are things another process on
/// the machine knows, and a token another process can work out is not a token.
///
/// # Errors
///
/// If the operating system has no randomness to give. Fallible rather than
/// panicking, and with no fallback: the alternatives to real entropy are all
/// guessable, and a guessable token on a port every process can reach is worse
/// than no pairing at all.
pub fn mint_token() -> Result<String, std::io::Error> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|e| std::io::Error::other(format!("no randomness to mint a token from: {e}")))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Where both applications keep their per-user state.
///
/// `GIRSA_POST_HOME` overrides it. That exists for two reasons and neither is
/// configurability for its own sake: a test suite must not touch the endpoint
/// file of the copy you are actually running, and a portable install on a
/// stick has nowhere else to put one.
fn home() -> PathBuf {
    if let Some(set) = std::env::var_os("GIRSA_POST_HOME") {
        return PathBuf::from(set);
    }
    // `LOCALAPPDATA` on Windows, `XDG_DATA_HOME` or `~/.local/share` elsewhere,
    // `~/Library/Application Support` on macOS. Read directly rather than
    // through a crate: it is four environment variables, and a dependency
    // compiled into both applications for this is a dependency to keep.
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| {
            let home = std::env::var_os("HOME").map(PathBuf::from)?;
            Some(if cfg!(target_os = "macos") {
                home.join("Library/Application Support")
            } else {
                home.join(".local/share")
            })
        })
        .unwrap_or_else(std::env::temp_dir);
    base.join("girsa")
}

/// Keep the token to the user who minted it.
#[cfg(unix)]
fn restrict_to_owner(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

/// On Windows the file inherits the directory's ACL, which is the user's own
/// `%LOCALAPPDATA%`. There is no mode to set.
#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)]
fn restrict_to_owner(_path: &std::path::Path) -> std::io::Result<()> {
    Ok(())
}

/// Keeping the tests out of the endpoint file of the copy you are running.
///
/// There is exactly one endpoint file per application per user, which is the
/// right design for the product and means every test that touches one is
/// touching the same one. So they take a lock and share a scratch directory,
/// and the suite never writes where a running Girsa would look.
#[cfg(test)]
pub(crate) mod testing {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    static LOCK: Mutex<()> = Mutex::new(());

    /// Hold this for as long as the test touches an endpoint file.
    pub(crate) fn alone() -> MutexGuard<'static, ()> {
        static SCRATCH: OnceLock<()> = OnceLock::new();
        SCRATCH.get_or_init(|| {
            let dir = std::env::temp_dir().join("girsa-post-tests");
            let _ = std::fs::remove_dir_all(&dir);
            let _ = std::fs::create_dir_all(&dir);
            std::env::set_var("GIRSA_POST_HOME", &dir);
        });
        // A test that panicked while holding it left the scratch directory in
        // some state; the next test is entitled to carry on rather than fail
        // for a reason that is not its own.
        LOCK.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace denies these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::testing::alone;

    #[test]
    fn a_token_is_not_something_another_process_can_work_out() {
        let (a, b) = (
            mint_token().expect("randomness"),
            mint_token().expect("randomness"),
        );
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn each_application_has_its_own_endpoint_file() {
        assert_ne!(Endpoint::path(App::Girsa), Endpoint::path(App::Ksav));
        assert!(Endpoint::path(App::Girsa)
            .to_string_lossy()
            .contains("girsa-endpoint"));
    }

    #[test]
    fn a_sibling_that_was_never_started_is_not_running_rather_than_stale() {
        // The difference matters in the window: *not started* offers to start
        // it, *stale* says something is wrong.
        let _alone = alone();
        let app = App::Ksav;
        Endpoint::withdraw(app);
        assert_eq!(presence(app), Presence::NotRunning);
    }

    #[test]
    fn an_endpoint_file_left_behind_by_a_crash_is_stale_and_says_so() {
        let _alone = alone();
        let app = App::Ksav;
        Endpoint {
            app,
            // Port 1 is reserved and nothing will be listening on it.
            port: 1,
            token: mint_token().expect("randomness"),
            pid: 999_999,
            version: "0.0.0".into(),
        }
        .publish()
        .expect("publishes");

        match presence(app) {
            Presence::Stale { why } => assert!(!why.is_empty()),
            other => panic!("expected stale, got {other:?}"),
        }
        Endpoint::withdraw(app);
    }
}
