//! Organization recovery (core `docs/specs/org-recovery.md`): an
//! organization's recovery key is an Ed25519 key whose seed the owner keeps
//! offline; the node stores the public half. The seed signs here, never on a
//! node, byte-for-byte with core's `calimero_identity::recovery` (pinned in
//! `tests/test_org_recovery.py`).

use ed25519_dalek::{Signer, SigningKey};

use super::prelude::*;

/// What a recovery signature covers before the organization, the account and
/// the nonce.
const RECOVERY_DOMAIN: &[u8] = b"calimero.org-recovery.v1\0";

/// The bytes a recovery signs: `"calimero.org-recovery.v1\0" || org || 0 ||
/// account || 0 || nonce`.
fn recovery_message(org: &str, account: &str, nonce: &str) -> Vec<u8> {
    let mut m = RECOVERY_DOMAIN.to_vec();
    m.extend_from_slice(org.as_bytes());
    m.push(0);
    m.extend_from_slice(account.as_bytes());
    m.push(0);
    m.extend_from_slice(nonce.as_bytes());
    m
}

/// A seed from its written form: 64 hex digits; `-`, whitespace and case are
/// ignored.
fn parse_seed(text: &str) -> PyResult<SigningKey> {
    let digits: String = text
        .chars()
        .filter(|c| *c != '-' && !c.is_whitespace())
        .collect();
    let seed: [u8; 32] = hex::decode(digits)
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| PyValueError::new_err("a recovery seed is 64 hex digits"))?;
    Ok(SigningKey::from_bytes(&seed))
}

/// The seed as people write it down: eight groups of eight hex digits.
fn seed_text(seed: &[u8; 32]) -> String {
    seed.chunks(4)
        .map(hex::encode)
        .collect::<Vec<_>>()
        .join("-")
}

#[pymethods]
impl PyCloudClient {
    /// A new recovery seed, as the owner writes it down (64 hex digits in
    /// groups of eight). Keep it offline; give the organization only
    /// `recovery_public_key(seed)`.
    #[staticmethod]
    fn new_recovery_seed() -> String {
        seed_text(&calimero_identity::random::<32>())
    }

    /// The public key (base64url) of a recovery seed: what the organization
    /// stores (`create_org(.., recovery_key=)`, `set_recovery_key`).
    #[staticmethod]
    fn recovery_public_key(seed: &str) -> PyResult<String> {
        Ok(calimero_identity::b64url(
            parse_seed(seed)?.verifying_key().as_bytes(),
        ))
    }

    /// The seed's signature (base64url) handing `org` to `account` under
    /// `nonce`, as `start_recovery` sends it.
    #[staticmethod]
    fn sign_recovery(seed: &str, org: &str, account: &str, nonce: &str) -> PyResult<String> {
        if nonce.is_empty() || nonce.len() > 64 {
            return Err(PyValueError::new_err("a recovery nonce is 1 to 64 bytes"));
        }
        let sig = parse_seed(seed)?.sign(&recovery_message(org, account, nonce));
        Ok(calimero_identity::b64url(&sig.to_bytes()))
    }

    /// Set or rotate (`public_key`) or remove (`None`) the organization's
    /// recovery key: its owner's, approved with `passkey`. A recovery under
    /// way is cancelled, and the old seed is refused from then on.
    #[pyo3(signature = (org, public_key, passkey))]
    fn set_recovery_key(
        &self,
        py: Python<'_>,
        org: &str,
        public_key: Option<String>,
        passkey: &PySoftPasskey,
    ) -> PyResult<()> {
        let body = serde_json::json!({ "publicKey": public_key });
        self.confirmed(py, format!("orgs/{}/recovery", seg(org)), body, passkey)
            .map(drop)
    }

    /// Start handing `org` to the signed-in account with the recovery
    /// `seed`, which signs here and is never sent. Answers `{ startedAt,
    /// readyAt }`; `finish_recovery` once `readyAt` has passed. A wrong seed
    /// raises `CloudError` (403 `WRONG_RECOVERY_KEY`).
    fn start_recovery(&self, py: Python<'_>, org: &str, seed: &str) -> PyResult<PyObject> {
        let key = parse_seed(seed)?;
        let me = self.send(py, Verb::Get, "auth/me".into(), Value::Null)?;
        let account = field(&me, "accountId")?;
        let nonce = calimero_identity::b64url(&calimero_identity::random::<16>());
        let signature = calimero_identity::b64url(
            &key.sign(&recovery_message(org, &account, &nonce))
                .to_bytes(),
        );
        let v = self.send(
            py,
            Verb::Post,
            format!("orgs/{}/recovery/start", seg(org)),
            serde_json::json!({ "nonce": nonce, "signature": signature }),
        )?;
        to_py(py, &v)
    }

    /// Cancel the recovery under way: the owner's, approved with `passkey`.
    fn cancel_recovery(&self, py: Python<'_>, org: &str, passkey: &PySoftPasskey) -> PyResult<()> {
        self.confirmed(
            py,
            format!("orgs/{}/recovery/cancel", seg(org)),
            serde_json::json!({}),
            passkey,
        )
        .map(drop)
    }

    /// Finish a recovery this account started, once its delay passed
    /// (`409 RECOVERY_NOT_READY` before), approved with this account's
    /// `passkey`: it becomes the owner and the previous owner leaves the
    /// organization. Answers `{ org, previousOwners }`.
    fn finish_recovery(
        &self,
        py: Python<'_>,
        org: &str,
        passkey: &PySoftPasskey,
    ) -> PyResult<PyObject> {
        let v = self.confirmed(
            py,
            format!("orgs/{}/recovery/finish", seg(org)),
            serde_json::json!({}),
            passkey,
        )?;
        to_py(py, &v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The vectors core pins in
    /// `calimero_identity::recovery::the_wire_bytes_are_pinned_for_the_clients`.
    #[test]
    fn the_wire_bytes_match_core() {
        let seed = "01".repeat(32);
        assert_eq!(
            PyCloudClient::recovery_public_key(&seed).unwrap(),
            "iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w"
        );
        assert_eq!(
            PyCloudClient::sign_recovery(&seed, "o", "a", "n").unwrap(),
            "bbbHMaH2SC50zQADskIDS5xg8WxZFrtxdasJzhzCOMZh-aBrUxeDKro2_UUIyU4PzH5LmsS1gQRQmVfx3E1eDw"
        );
        let s: [u8; 32] = core::array::from_fn(|i| u8::try_from(i * 7).unwrap_or(0));
        let text = seed_text(&s);
        assert_eq!(text.len(), 71);
        assert_eq!(parse_seed(&text.to_uppercase()).unwrap().to_bytes(), s);
        assert!(parse_seed(&text[..70]).is_err());
    }
}
