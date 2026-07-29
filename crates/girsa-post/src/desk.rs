//! The listening half: a desk the sibling can leave something on.
//!
//! Behind the `serve` feature, because a build that only *sends* — a command
//! line tool, a test — should not compile a server it never opens.
//!
//! # What it refuses, and why each refusal is here
//!
//! | | |
//! |---|---|
//! | no token, or the wrong one | `401`, before the path is even looked at |
//! | a path nothing claims | `404` |
//! | a body over [`MAX_BODY`] | `413` — a quote is a passage, not a file |
//! | `OPTIONS` | `405`, with no CORS headers, so a browser preflight fails |
//!
//! The last is the one that is easy to leave out. A web page cannot send
//! `X-Girsa-Token` cross-origin without a preflight; answering no preflight and
//! sending no `Access-Control-Allow-*` header at all means a page that somehow
//! knew the port *and* the token still cannot read a word of the reply.

use std::io::Read;
use std::sync::Arc;

use tiny_http::{Header, Method, Request, Response, Server};

use crate::{mint_token, App, Endpoint, Health, TOKEN_HEADER};

/// The largest request body accepted.
///
/// A Source Packet is one passage of a sefer; the longest segment in the corpus
/// is a few kilobytes. A megabyte is far more than anything real and small
/// enough that a stuck sender cannot fill memory.
pub const MAX_BODY: usize = 1024 * 1024;

/// What a handler decided.
pub struct Reply {
    pub status: u16,
    pub body: String,
}

impl Reply {
    #[must_use]
    pub fn ok(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            body: body.into(),
        }
    }

    /// A refusal that says why, in words the other side can show a reader.
    #[must_use]
    pub fn refused(status: u16, why: impl Into<String>) -> Self {
        Self {
            status,
            body: serde_json::json!({ "error": why.into() }).to_string(),
        }
    }
}

/// An open desk: a listener on loopback, and the endpoint file that points at
/// it.
pub struct Desk {
    server: Arc<Server>,
    token: String,
    app: App,
    /// The *application's* version, not this crate's. What presence shows is
    /// "Ksav 0.1.0 is there", and a shared crate's own version would be the
    /// same string on both sides and tell a reader nothing.
    version: String,
}

impl Desk {
    /// Bind a port the operating system picks, mint a token, and publish where
    /// to find it.
    ///
    /// # Errors
    ///
    /// If loopback cannot be bound, or the endpoint file cannot be written —
    /// which is not a reason to refuse to start, so callers are expected to
    /// carry on without a desk and say the pairing is unavailable.
    pub fn open(app: App, version: &str) -> Result<Self, std::io::Error> {
        // Port 0: the operating system picks. A fixed port is a port to
        // collide with, and a configurable one is a bind address one typo away
        // from being on the network (spec.md §14).
        let server = Server::http("127.0.0.1:0")
            .map_err(|e| std::io::Error::other(format!("cannot bind loopback: {e}")))?;
        let port = server
            .server_addr()
            .to_ip()
            .map(|addr| addr.port())
            .ok_or_else(|| std::io::Error::other("the listener has no port"))?;

        let token = mint_token()?;
        Endpoint {
            app,
            port,
            token: token.clone(),
            pid: std::process::id(),
            version: version.to_string(),
        }
        .publish()?;

        Ok(Self {
            server: Arc::new(server),
            token,
            app,
            version: version.to_string(),
        })
    }

    /// The port it is listening on.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.server
            .server_addr()
            .to_ip()
            .map_or(0, |addr| addr.port())
    }

    /// Answer requests, forever, on a thread of its own.
    ///
    /// `handle` is given the path and the body and returns what to say. It is
    /// called on the serving thread, so a handler that blocks blocks the next
    /// request — which for two errands a minute is exactly the right trade
    /// against a thread pool nobody can reason about.
    pub fn serve<F>(&self, handle: F)
    where
        F: Fn(&str, &str) -> Reply + Send + Sync + 'static,
    {
        let server = Arc::clone(&self.server);
        let token = self.token.clone();
        let app = self.app;
        let version = self.version.clone();
        std::thread::Builder::new()
            .name(format!("{app}-post"))
            .spawn(move || {
                for request in server.incoming_requests() {
                    answer(request, &token, app, &version, &handle);
                }
            })
            // A desk that cannot get a thread is a desk that is not open; the
            // caller already carries on without one.
            .map(|_| ())
            .unwrap_or_else(|e| eprintln!("the post desk did not open: {e}"));
    }
}

impl Drop for Desk {
    fn drop(&mut self) {
        // The file outliving the listener is exactly what `Presence::Stale`
        // exists for, and it is better not to need it.
        Endpoint::withdraw(self.app);
    }
}

fn answer<F>(mut request: Request, token: &str, app: App, version: &str, handle: &F)
where
    F: Fn(&str, &str) -> Reply,
{
    // Checked before anything else, including the path: an endpoint that
    // answers `/health` without a token tells a stranger which port is Girsa.
    let offered = request
        .headers()
        .iter()
        .find(|h| h.field.equiv(TOKEN_HEADER))
        .map(|h| h.value.as_str().to_string())
        .unwrap_or_default();
    if !constant_time_eq(&offered, token) {
        let _ = request.respond(response(Reply::refused(401, "wrong token")));
        return;
    }

    if request.method() == &Method::Options {
        // No CORS headers, on purpose. See the module note.
        let _ = request.respond(response(Reply::refused(405, "no")));
        return;
    }

    let length = request.body_length().unwrap_or(0);
    if length > MAX_BODY {
        let _ = request.respond(response(Reply::refused(413, "that is not a quote")));
        return;
    }

    let mut body = String::with_capacity(length);
    if request
        .as_reader()
        .take(MAX_BODY as u64)
        .read_to_string(&mut body)
        .is_err()
    {
        let _ = request.respond(response(Reply::refused(400, "the body is not text")));
        return;
    }

    let path = request.url().split('?').next().unwrap_or("/").to_string();
    let reply = match path.as_str() {
        // Every desk answers this one, so presence is one implementation
        // rather than one per application.
        "/health" => Reply::ok(
            serde_json::to_string(&Health {
                app,
                version: version.to_string(),
            })
            .unwrap_or_default(),
        ),
        path => handle(path, &body),
    };
    let _ = request.respond(response(reply));
}

fn response(reply: Reply) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut out = Response::from_string(reply.body).with_status_code(reply.status);
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        out = out.with_header(header);
    }
    out
}

/// Compare two tokens without giving away how much of one is right.
///
/// The attack is not realistic over loopback with a 128-bit token, and the
/// cost of not having to think about whether it is realistic is four lines.
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::testing::alone;
    use crate::{presence, send, Presence};

    /// A desk that answers one errand, so the tests are about the transport.
    fn desk(app: App) -> Desk {
        let desk = Desk::open(app, "test").expect("binds loopback");
        desk.serve(|path, body| match path {
            "/echo" => Reply::ok(body.to_string()),
            _ => Reply::refused(404, "no such errand"),
        });
        desk
    }

    #[test]
    fn a_desk_answers_its_sibling_and_says_who_it_is() {
        let _alone = alone();
        let _desk = desk(App::Ksav);
        match presence(App::Ksav) {
            Presence::Live { .. } => {}
            other => panic!("expected live, got {other:?}"),
        }
        assert_eq!(
            send(App::Ksav, "/echo", Some("שלום")).expect("answers"),
            "שלום"
        );
        Endpoint::withdraw(App::Ksav);
    }

    #[test]
    fn the_wrong_token_is_refused_before_the_path_is_looked_at() {
        let _alone = alone();
        // Localhost is not private: every process on the machine can reach the
        // port, and so can a web page.
        let desk = Desk::open(App::Girsa, "test").expect("binds");
        desk.serve(|_, _| Reply::ok("should never be reached"));

        let stranger = Endpoint {
            app: App::Girsa,
            port: desk.port(),
            token: "0".repeat(32),
            pid: 0,
            version: "test".into(),
        };
        stranger.publish().expect("publishes");
        match send(App::Girsa, "/health", None) {
            Err(crate::PostError::Refused { status, .. }) => assert_eq!(status, 401),
            other => panic!("expected a refusal, got {other:?}"),
        }
        Endpoint::withdraw(App::Girsa);
    }

    #[test]
    fn an_errand_nothing_claims_is_a_refusal_and_not_a_silence() {
        let _alone = alone();
        let _desk = desk(App::Ksav);
        match send(App::Ksav, "/nonsense", Some("{}")) {
            Err(crate::PostError::Refused { status, body, .. }) => {
                assert_eq!(status, 404);
                assert!(body.contains("no such errand"), "{body}");
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
        Endpoint::withdraw(App::Ksav);
    }

    #[test]
    fn the_desk_takes_its_file_away_when_it_closes() {
        let _alone = alone();
        {
            let _desk = desk(App::Ksav);
            assert!(Endpoint::read(App::Ksav).is_some());
        }
        assert_eq!(presence(App::Ksav), Presence::NotRunning);
    }
}
