//! Signing one request, entirely offline.
//!
//! A request signature is how a caller says who it is **on the request itself**,
//! rather than exchanging a proof for a session the node has to remember. It
//! needs a signing secret — the very thing that must never reach the node — so
//! like `sign_warrant` this opens no connection, reads no config, and contacts
//! nothing.
//!
//! # Why this is a binding rather than an implementation
//!
//! `calimero-account` already implements `RequestSig::sign`, and this crate is
//! Rust, so binding it costs one call. Reimplementing it would mean ed25519
//! signing and borsh encoding kept byte-for-byte identical with the node
//! forever, and a signature that disagrees by one byte is indistinguishable
//! from a forgery — it arrives as a 401, nowhere near the change that caused it.
//!
//! # Which key signs
//!
//! Either the ephemeral **session key** a login statement authorized, or the
//! **device key** itself. The node's verifier handles both chains, so the choice
//! here is about where your key lives rather than about what the node accepts: a
//! script holding its device secret locally has no reason to mint a session
//! key, while a long-lived process has every reason to.
//!
//! # What it binds, and the one thing it does not
//!
//! Method, path and a hash of the body. Not the query string — a proxy may
//! legitimately rewrite it, a token parameter most of all, and signing over
//! bytes something else may change means failing for reasons the caller cannot
//! see. Pass the path alone; anything that must be bound belongs in the body.

use calimero_account::RequestSig;
use calimero_primitives::identity::PrivateKey;
use pyo3::prelude::*;

fn value_error(message: String) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(message)
}

/// Decode 32 bytes of hex, naming the argument it rejected.
fn parse_secret(raw: &str) -> Result<PrivateKey, String> {
    let bytes = hex::decode(raw.trim()).map_err(|e| format!("signer_secret is not hex: {e}"))?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_ignored| "signer_secret is not 32 bytes (64 hex chars)".to_owned())?;
    Ok(PrivateKey::from(bytes))
}

/// Sign the request `method path` with `body`, returning the hex encoding.
///
/// `valid_for` is seconds, and short is right: the window is what bounds replay
/// and nothing else does. A captured signature performs the identical request
/// until it expires — the same read, or the same write with the same bytes —
/// so a long window buys convenience at the cost of the only limit there is.
///
/// `body` must be exactly the bytes that will be sent. The signature commits to
/// their hash, so a body re-serialized between signing and sending is a
/// signature for a different request.
#[pyfunction]
#[pyo3(signature = (method, path, signer_secret, body = "", valid_for = 300))]
pub fn sign_request(
    method: &str,
    path: &str,
    signer_secret: &str,
    body: &str,
    valid_for: u64,
) -> PyResult<String> {
    let issued_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    build_request_sig(
        method,
        path,
        signer_secret,
        body.as_bytes(),
        issued_at,
        issued_at.saturating_add(valid_for),
    )
    .map_err(value_error)
}

/// The whole of [`sign_request`], minus Python and minus the clock.
///
/// Split out for the reason `build_warrant` is: the byte-level properties that
/// matter are testable without a GIL or an interpreter. Taking the timestamps
/// explicitly is the other half of that — the one time-dependent input becomes
/// an argument, so the encoding can be asserted against core's recorded vectors
/// rather than against whatever the clock said.
fn build_request_sig(
    method: &str,
    path: &str,
    signer_secret: &str,
    body: &[u8],
    issued_at: u64,
    expires_at: u64,
) -> Result<String, String> {
    let signer = parse_secret(signer_secret)?;

    let sig = RequestSig::sign(&signer, method, path, body, issued_at, expires_at)
        .map_err(|e| format!("could not sign the request: {e}"))?;

    Ok(hex::encode(borsh::to_vec(&sig).map_err(|e| {
        format!("could not encode the signature: {e}")
    })?))
}

#[cfg(test)]
mod tests {
    use super::build_request_sig;

    /// core's `key(3)` — a `PrivateKey` of 32 bytes of 0x03.
    const SIGNER: &str = "0303030303030303030303030303030303030303030303030303030303030303";
    const METHOD: &str = "GET";
    const PATH: &str = "/admin-api/namespaces";
    const BODY: &[u8] = br#"{"k":"v"}"#;
    const ISSUED_AT: u64 = 1_700_000_000;
    const EXPIRES_AT: u64 = 1_700_000_300;

    /// The encoding core records, byte for byte.
    ///
    /// Pinned here as well as there because this crate is what a Python caller
    /// actually runs. If the binding ever stops producing these bytes — a
    /// dependency pointed at the wrong revision, a field reordered upstream —
    /// the failure without this test is a 401 from a node, which reads as a bad
    /// key rather than as a version skew.
    #[test]
    fn it_reproduces_cores_recorded_encoding() {
        assert_eq!(
            build_request_sig(METHOD, PATH, SIGNER, BODY, ISSUED_AT, EXPIRES_AT).expect("sign"),
            concat!(
                "03000000",
                "474554",
                "15000000",
                "2f61646d696e2d6170692f6e616d657370616365",
                "73",
                "3b26137eb7b296bdf7d84b9193dd52d1947b2056731f38020f4a3b9b88d95121",
                "00f1536500000000",
                "2cf2536500000000",
                "295cdac7a269f7f773735dcc35511d4174d042aa4c20454b6c14d5c5661a4f2e",
                "6401e8d1dc457040a388359982f2b915fdd8ca56a84d26f2b40867f092013706",
            ),
        );
    }

    /// Every signed field reaches the signature.
    ///
    /// Without this a field could be added to the encoding and left out of the
    /// preimage — a signature that covers less than it appears to.
    #[test]
    fn every_signed_field_changes_the_result() {
        let base =
            build_request_sig(METHOD, PATH, SIGNER, BODY, ISSUED_AT, EXPIRES_AT).expect("sign");

        for (label, other) in [
            (
                "method",
                build_request_sig("POST", PATH, SIGNER, BODY, ISSUED_AT, EXPIRES_AT),
            ),
            (
                "path",
                build_request_sig(
                    METHOD,
                    "/admin-api/contexts",
                    SIGNER,
                    BODY,
                    ISSUED_AT,
                    EXPIRES_AT,
                ),
            ),
            (
                "body",
                build_request_sig(METHOD, PATH, SIGNER, br#"{"k":"w"}"#, ISSUED_AT, EXPIRES_AT),
            ),
            (
                "issued_at",
                build_request_sig(METHOD, PATH, SIGNER, BODY, ISSUED_AT + 1, EXPIRES_AT),
            ),
            (
                "expires_at",
                build_request_sig(METHOD, PATH, SIGNER, BODY, ISSUED_AT, EXPIRES_AT + 1),
            ),
        ] {
            assert_ne!(
                base,
                other.expect("sign"),
                "{label} does not reach the signature"
            );
        }
    }

    /// `HEAD` and `GET` are the same permission and not the same request. The
    /// node's permission layer folds them deliberately; this layer must not.
    #[test]
    fn head_is_not_get() {
        assert_ne!(
            build_request_sig("HEAD", PATH, SIGNER, BODY, ISSUED_AT, EXPIRES_AT).expect("sign"),
            build_request_sig("GET", PATH, SIGNER, BODY, ISSUED_AT, EXPIRES_AT).expect("sign"),
        );
    }

    /// A secret that is not 32 bytes of hex is named rather than panicking
    /// somewhere inside the signer.
    #[test]
    fn a_malformed_secret_is_reported() {
        assert!(
            build_request_sig(METHOD, PATH, "nothex", BODY, ISSUED_AT, EXPIRES_AT)
                .unwrap_err()
                .contains("not hex")
        );
        assert!(
            build_request_sig(METHOD, PATH, "0303", BODY, ISSUED_AT, EXPIRES_AT)
                .unwrap_err()
                .contains("32 bytes")
        );
    }
}
