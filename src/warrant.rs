//! Minting a warrant, entirely offline.
//!
//! A warrant is a member's signed statement that one specific request may be run
//! on their behalf by one specific operator. Signing it needs a device secret
//! key, which is exactly the thing that must never reach the node that runs the
//! request — so this function opens no connection, reads no config, and contacts
//! nothing. It is pure computation over its arguments.
//!
//! # Why this lives here rather than in a JS client
//!
//! `calimero-account` already implements `Warrant::sign`, and this crate is
//! Rust, so binding it costs one call. The same helper in a JavaScript client
//! means reimplementing ed25519 signing and borsh encoding and then keeping both
//! byte-for-byte identical with the node forever — a signature that disagrees by
//! one byte is indistinguishable from a forgery.
//!
//! # The one subtlety: argument normalization
//!
//! The warrant commits to `H(method ‖ args)`, and the node recomputes that hash
//! from the `argsJson` it receives. So both sides have to agree on the bytes of
//! `args`. They do, because both parse the JSON and re-serialize it
//! (`serde_json::to_vec` of a parsed `Value`, which orders object keys), rather
//! than hashing whatever text the caller happened to type. Hashing the raw
//! string here would mint warrants that verify nowhere: a re-indented but
//! semantically identical body would produce a different hash.

use calimero_account::{AccountId, AccountProof, DeviceCert, Warrant};
use calimero_primitives::context::ContextId;
use calimero_primitives::identity::PrivateKey;
use pyo3::prelude::*;

use crate::utils::json_to_python;

fn value_error(message: String) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(message)
}

/// Decode 32 bytes of hex, naming the argument it rejected.
fn parse_secret(raw: &str) -> Result<PrivateKey, String> {
    let bytes = hex::decode(raw.trim()).map_err(|e| format!("device_secret is not hex: {e}"))?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_ignored| "device_secret is not 32 bytes (64 hex chars)".to_owned())?;
    Ok(PrivateKey::from(bytes))
}

/// Sign a warrant authorising `executor` to run `method(args)` in `context_id`.
///
/// Returns a dict carrying the warrant plus the facts a caller needs alongside
/// it — the author's account (which has to be a member before the write is
/// authorised) and the device key the change is attributed to. Those are read
/// out of the credential rather than taken as arguments, because a caller
/// passing them separately is a caller that can pass them inconsistently.
///
/// Both `context_id` and `executor` are 64 hex characters. There used to be an
/// asymmetry here — base58 for the context, hex for the account — and it was
/// core's rather than this binding's; core removed it, so this follows. Nothing
/// in the shape of either argument distinguishes them any more.
#[pyfunction]
#[pyo3(signature = (
    context_id,
    executor,
    method,
    args,
    nonce,
    device_secret,
    credential,
    valid_for = 300,
))]
#[expect(
    clippy::too_many_arguments,
    reason = "every one is a distinct thing the signature commits to; bundling \
              them into a dict would move the type errors to runtime"
)]
pub fn sign_warrant(
    py: Python<'_>,
    context_id: &str,
    executor: &str,
    method: &str,
    args: &str,
    nonce: u64,
    device_secret: &str,
    credential: &str,
    valid_for: u64,
) -> PyResult<PyObject> {
    let payload = build_warrant(
        context_id,
        executor,
        method,
        args,
        nonce,
        device_secret,
        credential,
        valid_for,
    )
    .map_err(value_error)?;
    Ok(json_to_python(py, &payload))
}

/// The whole of `sign_warrant`, minus Python.
///
/// Split out so the byte-level properties that matter — that re-indenting the
/// arguments cannot change the signature, that a credential must certify the
/// signing key — are testable without a GIL or an interpreter. The wrapper above
/// exists only to convert the error and the return value.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors the pyfunction it was split from"
)]
fn build_warrant(
    context_id: &str,
    executor: &str,
    method: &str,
    args: &str,
    nonce: u64,
    device_secret: &str,
    credential: &str,
    valid_for: u64,
) -> Result<serde_json::Value, String> {
    let context: ContextId = context_id
        .trim()
        .parse()
        .map_err(|e| format!("context_id '{context_id}' is not valid: {e}"))?;

    let executor_account: AccountId = executor
        .trim()
        .parse()
        .map_err(|e| format!("executor '{executor}' is not a valid account id: {e}"))?;

    let device_sk = parse_secret(device_secret)?;

    let credential_bytes =
        hex::decode(credential.trim()).map_err(|e| format!("credential is not hex: {e}"))?;
    let proof: AccountProof<DeviceCert> = borsh::from_slice(&credential_bytes)
        .map_err(|e| format!("credential is not a device credential: {e}"))?;

    // Refused here rather than by a peer. A warrant signed by a key the
    // credential does not certify is well-formed, travels fine, and is rejected
    // on arrival — so catching it at mint time turns a silently dropped write
    // into an error at the line that caused it.
    if proof.statement.sign_pk != device_sk.public_key() {
        return Err(
            "this credential certifies a different key than device_secret holds; \
                    a peer would refuse the warrant it signs"
                .to_owned(),
        );
    }

    let args_value: serde_json::Value =
        serde_json::from_str(args).map_err(|e| format!("args is not valid JSON: {e}"))?;
    let args_bytes = serde_json::to_vec(&args_value)
        .map_err(|e| format!("args could not be re-encoded: {e}"))?;

    let not_after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .saturating_add(valid_for);

    let intent_hash = Warrant::intent_hash(method, &args_bytes);
    let warrant = Warrant::sign(
        &device_sk,
        context,
        proof.statement.account,
        executor_account,
        intent_hash,
        nonce,
        not_after,
    )
    .map_err(|e| format!("could not sign the warrant: {e}"))?;

    let encoded =
        borsh::to_vec(&warrant).map_err(|e| format!("could not encode the warrant: {e}"))?;

    let payload = serde_json::json!({
        "warrant": hex::encode(encoded),
        "authorAccount": proof.statement.account.to_string(),
        "authorDeviceKey": warrant.author_device_key.to_string(),
        "intentHash": hex::encode(intent_hash),
        "nonce": nonce,
        "notAfter": not_after,
    });

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::build_warrant;

    /// A device certified by a fixed test account, captured from
    /// `merod account sign-cert --generate` run against the well-known
    /// "legal winner thank year..." phrase. It owns nothing anywhere.
    ///
    /// Frozen rather than generated: the credential has to certify exactly the
    /// key `SECRET` holds, and a generated pair would have to reimplement the
    /// certification to stay consistent — at which point the test would be
    /// exercising the fixture instead of the code.
    const CREDENTIAL: &str = "02b2a942ff4c98718bed76e255987f6d59b1a72d3b2cd2510003e6170ac63a9ffb000000000e2cd2d3dc84e1db5088e32510ca45bc491e4033bbb0f6bbb733bc0c7b7f5e304d0774b93e8028899a745dbe03d7727fa31fc2f060945b5789cb36c23cba380366245580f7aa816a35d1ff324a714355995ef44a72bcd2341e21d9587d16efce973135e50bc7280f06bb32a53a566983cf0f0c8428be4b461df54264f07319540000000000000000e0c3743677508f5cfbe245f043f2d7bc3ba6c88c001464cae581e2e9ec8cb63780f1f5c2a393521a0038b357fffe63092403fa6e0e2ec12da5e96d50692d400f";
    const SECRET: &str = "4987ccd0fb7ef36bf7f61e8f99fd150d33e6adac47649f23bfd7109c2e36a3ba";
    const ACCOUNT: &str = "0e2cd2d3dc84e1db5088e32510ca45bc491e4033bbb0f6bbb733bc0c7b7f5e30";
    /// Hex, as every id is now. The 32 bytes are `00 01 02 .. 1f`, unchanged —
    /// this was base58 for the same bytes, so every signature below is identical.
    const CONTEXT: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    fn mint(args: &str, nonce: u64) -> serde_json::Value {
        build_warrant(
            CONTEXT, ACCOUNT, "set", args, nonce, SECRET, CREDENTIAL, 300,
        )
        .expect("a well-formed warrant must sign")
    }

    /// The property the whole binding rests on.
    ///
    /// The node recomputes `H(method ‖ args)` from the `argsJson` it receives, so
    /// if this helper hashed the caller's literal text instead of re-serializing
    /// it, then a re-indented — or differently key-ordered — body would mint a
    /// warrant that verifies nowhere. The failure would be invisible here and
    /// look like a forged signature at the node.
    ///
    /// `intentHash` rather than the whole warrant, because `not_after` is taken
    /// from the clock and two calls can straddle a second.
    #[test]
    fn reformatting_the_arguments_cannot_change_the_commitment() {
        let compact = mint(r#"{"key":"k","value":"v"}"#, 1);
        let spaced = mint("{ \"key\" : \"k\" ,\n  \"value\" : \"v\" }", 1);
        let reordered = mint(r#"{"value":"v","key":"k"}"#, 1);

        assert_eq!(compact["intentHash"], spaced["intentHash"]);
        assert_eq!(
            compact["intentHash"], reordered["intentHash"],
            "serde_json orders object keys, so key order must not matter either"
        );
    }

    /// Different arguments must not share a commitment, or the hash would be
    /// authorising anything.
    #[test]
    fn different_arguments_commit_differently() {
        let one = mint(r#"{"key":"k","value":"v"}"#, 1);
        let two = mint(r#"{"key":"k","value":"w"}"#, 1);
        assert_ne!(one["intentHash"], two["intentHash"]);
    }

    /// Caught at mint time rather than by a peer. Such a warrant is well-formed
    /// and travels fine; it is refused on arrival, and a silently dropped write
    /// is the hardest kind of bug to attribute.
    #[test]
    fn a_credential_certifying_another_key_is_refused() {
        let other = "11".repeat(32);
        let err = build_warrant(CONTEXT, ACCOUNT, "set", "{}", 1, &other, CREDENTIAL, 300)
            .expect_err("a mismatched key must be refused");
        assert!(err.contains("certifies a different key"), "{err}");
    }

    /// Both ids are hex, and base58 is refused for either.
    ///
    /// This inverts `a_context_id_is_base58_and_an_executor_is_hex`, which pinned
    /// the opposite in both directions and cited "the confusion that cost a CI
    /// cycle in core when a CLI flag parsed one as the other". That confusion is
    /// gone because the distinction is: a context and an account are now spelled
    /// the same way, so passing one where the other belongs is no longer a parse
    /// error here and nothing at this layer can catch it.
    ///
    /// What is still worth pinning is that base58 does not sneak through, which
    /// is what the two halves below check.
    #[test]
    fn both_ids_are_hex_and_base58_is_refused() {
        // The old base58 spelling of CONTEXT's own bytes.
        const CONTEXT_B58: &str = "1thX6LZfHDZZKUs92febYZhYRcXddmzfzF2NvTkPNE";

        let err = build_warrant(
            CONTEXT_B58,
            ACCOUNT,
            "set",
            "{}",
            1,
            SECRET,
            CREDENTIAL,
            300,
        )
        .expect_err("base58 must not pass as a context id");
        assert!(err.contains("context_id"), "{err}");

        let err = build_warrant(
            CONTEXT,
            CONTEXT_B58,
            "set",
            "{}",
            1,
            SECRET,
            CREDENTIAL,
            300,
        )
        .expect_err("base58 must not pass as an account id");
        assert!(err.contains("executor"), "{err}");
    }

    /// The account is read out of the credential, never taken as an argument —
    /// a caller that passes it separately is one that can pass it inconsistently.
    #[test]
    fn the_author_account_is_reported_from_the_credential() {
        let minted = mint("{}", 7);
        assert_eq!(minted["authorAccount"], ACCOUNT);
        assert_eq!(minted["nonce"], 7);
        assert!(
            minted["warrant"].as_str().is_some_and(|w| !w.is_empty()),
            "a warrant must be returned"
        );
    }

    /// The nonce is inside the signed bytes, so it cannot be edited after the
    /// fact to get a second use out of one authorization.
    #[test]
    fn the_nonce_is_covered_by_the_signature() {
        let first = mint("{}", 1);
        let second = mint("{}", 2);
        assert_ne!(first["warrant"], second["warrant"]);
    }

    #[test]
    fn malformed_input_is_named_rather_than_panicking() {
        for (bad, needle) in [
            ("not json", "args is not valid JSON"),
            ("[1,2", "args is not valid JSON"),
        ] {
            let err = build_warrant(CONTEXT, ACCOUNT, "set", bad, 1, SECRET, CREDENTIAL, 300)
                .expect_err("malformed args must be refused");
            assert!(err.contains(needle), "{err}");
        }

        let err = build_warrant(CONTEXT, ACCOUNT, "set", "{}", 1, "zz", CREDENTIAL, 300)
            .expect_err("a non-hex secret must be refused");
        assert!(err.contains("device_secret is not hex"), "{err}");

        let err = build_warrant(CONTEXT, ACCOUNT, "set", "{}", 1, SECRET, "beef", 300)
            .expect_err("a truncated credential must be refused");
        assert!(
            err.contains("credential is not a device credential"),
            "{err}"
        );
    }
}
