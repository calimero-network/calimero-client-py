//! Attested transports to a TEE node, for Python.
//!
//! A node's quote proves what runs inside its TD. It says nothing about who
//! reads the traffic on the way there unless the client also uses a key the
//! quote commits to, and trusts that key only because it does. This module is
//! the Python surface over `calimero_client::tee`, which does exactly that:
//!
//! * **attested TLS** pins the TLS key the TD serves, for a node whose TLS
//!   terminates inside the TD;
//! * **sealing** encrypts every request to the node's attested transport key,
//!   for a node whose TLS ends at a proxy outside it.
//!
//! [`PyTeePolicy`] decides which TDs are trusted. The quote is verified here,
//! against Intel's collateral, not by asking the node or anybody else.

use std::str::FromStr;

use calimero_client::tee::{Attestor, PolicyVerifier, VerifierPolicy};
use calimero_primitives::application::ApplicationId;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Which TEE nodes a connection trusts.
///
/// `allowed_mrtd` is required: the MRTD names the image a TD booted, and a
/// quote that proves only "some genuine TD" proves nothing about which code
/// reads the traffic. The RTMR allowlists are optional and unchecked when
/// left out. `allowed_tcb_statuses` defaults to accepting only up-to-date
/// platforms. Mock quotes are never accepted.
///
/// `application_id` and `application_hash` go together: the node binds the
/// bytecode hash of the application it has installed under that id, and the
/// quote verifies only if it is this one.
#[pyclass(name = "TeePolicy", frozen)]
#[derive(Clone, Debug)]
pub struct PyTeePolicy {
    policy: VerifierPolicy,
    application: Option<(ApplicationId, [u8; 32])>,
}

#[pymethods]
impl PyTeePolicy {
    #[new]
    #[pyo3(signature = (
        allowed_mrtd,
        *,
        allowed_rtmr0=None,
        allowed_rtmr1=None,
        allowed_rtmr2=None,
        allowed_rtmr3=None,
        allowed_tcb_statuses=None,
        application_id=None,
        application_hash=None,
    ))]
    #[expect(
        clippy::too_many_arguments,
        reason = "one keyword argument per policy field, as Python callers pass them"
    )]
    pub fn new(
        allowed_mrtd: Vec<String>,
        allowed_rtmr0: Option<Vec<String>>,
        allowed_rtmr1: Option<Vec<String>>,
        allowed_rtmr2: Option<Vec<String>>,
        allowed_rtmr3: Option<Vec<String>>,
        allowed_tcb_statuses: Option<Vec<String>>,
        application_id: Option<&str>,
        application_hash: Option<&str>,
    ) -> PyResult<Self> {
        // `VerifierPolicy` refuses every quote on an empty MRTD list, which is
        // the safe reading, but a policy that can never admit anything is a
        // mistake better reported here than as a refusal from a node later.
        if allowed_mrtd.is_empty() {
            return Err(PyValueError::new_err(
                "allowed_mrtd is empty, so no quote would ever be accepted",
            ));
        }
        for (name, values) in [
            ("allowed_mrtd", Some(&allowed_mrtd)),
            ("allowed_rtmr0", allowed_rtmr0.as_ref()),
            ("allowed_rtmr1", allowed_rtmr1.as_ref()),
            ("allowed_rtmr2", allowed_rtmr2.as_ref()),
            ("allowed_rtmr3", allowed_rtmr3.as_ref()),
        ] {
            for value in values.into_iter().flatten() {
                if hex::decode(value).map_or(true, |bytes| bytes.len() != 48) {
                    return Err(PyValueError::new_err(format!(
                        "{name} holds {value:?}, which is not a 48-byte hex measurement"
                    )));
                }
            }
        }

        let application = match (application_id, application_hash) {
            (None, None) => None,
            (Some(id), Some(hash)) => Some((
                ApplicationId::from_str(id).map_err(|err| {
                    PyValueError::new_err(format!("application_id is not an application id: {err}"))
                })?,
                decode_32(hash, "application_hash")?,
            )),
            _ => {
                return Err(PyValueError::new_err(
                    "application_id and application_hash are given together or not at all",
                ))
            }
        };

        let mut policy = VerifierPolicy::new(allowed_mrtd);
        policy.allowed_rtmr0 = allowed_rtmr0.unwrap_or_default();
        policy.allowed_rtmr1 = allowed_rtmr1.unwrap_or_default();
        policy.allowed_rtmr2 = allowed_rtmr2.unwrap_or_default();
        policy.allowed_rtmr3 = allowed_rtmr3.unwrap_or_default();
        policy.allowed_tcb_statuses = allowed_tcb_statuses.unwrap_or_default();
        Ok(Self {
            policy,
            application,
        })
    }

    #[getter]
    pub fn allowed_mrtd(&self) -> Vec<String> {
        self.policy.allowed_mrtd.clone()
    }

    pub fn __repr__(&self) -> String {
        format!(
            "TeePolicy(allowed_mrtd={:?}, application_id={:?})",
            self.policy.allowed_mrtd,
            self.application.as_ref().map(|(id, _)| id.to_string()),
        )
    }
}

impl PyTeePolicy {
    /// An attestor enforcing this policy.
    pub(crate) fn attestor(&self) -> Attestor {
        let attestor = Attestor::new(PolicyVerifier::new(self.policy.clone()));
        match self.application {
            Some((id, hash)) => attestor.with_application(id, hash),
            None => attestor,
        }
    }
}

/// A 32-byte value given as hex, named in the error when it is not one.
pub(crate) fn decode_32(value: &str, what: &str) -> PyResult<[u8; 32]> {
    let bytes = hex::decode(value.trim())
        .map_err(|err| PyValueError::new_err(format!("{what} is not valid hex: {err}")))?;
    <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| {
        PyValueError::new_err(format!("{what} must be 32 bytes, i.e. 64 hex characters"))
    })
}

#[cfg(test)]
mod tests {
    use super::{decode_32, PyTeePolicy};

    /// A 48-byte measurement.
    fn mrtd() -> Vec<String> {
        vec!["aa".repeat(48)]
    }

    #[test]
    fn a_policy_pins_at_least_one_image() {
        assert!(PyTeePolicy::new(vec![], None, None, None, None, None, None, None).is_err());
        assert!(PyTeePolicy::new(mrtd(), None, None, None, None, None, None, None).is_ok());
    }

    #[test]
    fn a_measurement_that_is_not_one_is_refused() {
        assert!(PyTeePolicy::new(
            vec!["aabb".to_owned()],
            None,
            None,
            None,
            None,
            None,
            None,
            None
        )
        .is_err());
        assert!(PyTeePolicy::new(
            mrtd(),
            Some(vec!["zz".to_owned()]),
            None,
            None,
            None,
            None,
            None,
            None
        )
        .is_err());
    }

    #[test]
    fn an_application_is_named_with_its_hash() {
        let id = "11".repeat(32);
        let hash = "22".repeat(32);
        assert!(
            PyTeePolicy::new(mrtd(), None, None, None, None, None, Some(&id), None).is_err(),
            "an id without the hash it must run"
        );
        assert!(PyTeePolicy::new(mrtd(), None, None, None, None, None, None, Some(&hash)).is_err());
        assert!(
            PyTeePolicy::new(mrtd(), None, None, None, None, None, Some(&id), Some(&hash)).is_ok()
        );
    }

    #[test]
    fn a_32_byte_value_is_checked_for_length() {
        assert_eq!(decode_32(&"ab".repeat(32), "x").unwrap(), [0xab; 32]);
        assert!(decode_32("abcd", "x").is_err());
        assert!(decode_32("not hex", "x").is_err());
    }
}
