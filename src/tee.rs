//! Sealed requests to a TEE node, for Python.
//!
//! A node's quote proves what runs inside its TD. It says nothing about who
//! reads the traffic on the way there unless the client also uses a key the
//! quote commits to, and trusts that key only because it does. With
//! `sealed=True`, `calimero_client::tee` asks the node to bind its transport
//! key into a fresh quote, checks the quote against [`PyTeePolicy`], and seals
//! every request to that key, so a proxy in front of the node reads nothing.
//!
//! The quote is verified here, against Intel's collateral, not by asking the
//! node or anybody else.

use std::str::FromStr;

use calimero_client::tee::{Attestor, PolicyVerifier, VerifierPolicy};
use calimero_primitives::application::ApplicationId;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Which TEE nodes a connection trusts.
///
/// The image is named by `allowed_mrtd` and `allowed_rtmr1`–`3`, all
/// required. The MRTD alone names no image: it measures the platform's TD
/// firmware, which on GCP every mero-tee image, profile and release shares.
/// The image is in RTMR1–3. `allowed_rtmr0`, the firmware's configuration, is
/// optional. Build the policy from a release with [`PyTeePolicy::from_releases`]
/// rather than by hand. `allowed_tcb_statuses` defaults to accepting only
/// up-to-date platforms. Mock quotes are never accepted.
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
            ("allowed_rtmr1", &allowed_rtmr1),
            ("allowed_rtmr2", &allowed_rtmr2),
            ("allowed_rtmr3", &allowed_rtmr3),
        ] {
            if !values.as_ref().is_some_and(|values| !values.is_empty()) {
                return Err(PyValueError::new_err(format!(
                    "{name} is required: the MRTD measures the TD firmware, which images share, \
                     and the image is in RTMR1-3. Build the policy with TeePolicy.from_releases"
                )));
            }
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

    /// A policy trusting exactly one profile of the given node releases.
    ///
    /// Each release is the text of its `published-mrtds.json`. Pass every
    /// release the nodes you talk to may run: during a rollout, the old one and
    /// the new one. The TCB statuses accepted are those every release accepts.
    /// Registers are allowed one by one, so trust only releases you would
    /// accept any mix of.
    #[staticmethod]
    #[pyo3(signature = (releases, profile, *, application_id=None, application_hash=None))]
    pub fn from_releases(
        releases: Vec<String>,
        profile: &str,
        application_id: Option<&str>,
        application_hash: Option<&str>,
    ) -> PyResult<Self> {
        let images = trusted_images(&releases, profile)?;
        let collect = |pick: fn(&PublishedImage) -> &String| -> Vec<String> {
            let mut values: Vec<String> = Vec::new();
            for image in &images.images {
                let value = pick(image).to_ascii_lowercase();
                if !values.contains(&value) {
                    values.push(value);
                }
            }
            values
        };
        Self::new(
            collect(|image| &image.mrtd),
            Some(collect(|image| &image.rtmr0)),
            Some(collect(|image| &image.rtmr1)),
            Some(collect(|image| &image.rtmr2)),
            Some(collect(|image| &image.rtmr3)),
            Some(images.tcb_statuses),
            application_id,
            application_hash,
        )
    }

    #[getter]
    pub fn allowed_rtmr1(&self) -> Vec<String> {
        self.policy.allowed_rtmr1.clone()
    }

    #[getter]
    pub fn allowed_rtmr3(&self) -> Vec<String> {
        self.policy.allowed_rtmr3.clone()
    }

    #[getter]
    pub fn allowed_tcb_statuses(&self) -> Vec<String> {
        self.policy.allowed_tcb_statuses.clone()
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

/// One profile's measurements in a release's `published-mrtds.json`.
#[derive(serde::Deserialize)]
struct PublishedImage {
    mrtd: String,
    rtmr0: String,
    rtmr1: String,
    rtmr2: String,
    rtmr3: String,
    #[serde(default)]
    allowed_tcb_statuses: Option<Vec<String>>,
}

/// The fields of a release's `published-mrtds.json` read here.
#[derive(serde::Deserialize)]
struct PublishedMrtds {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    profiles: std::collections::BTreeMap<String, PublishedImage>,
}

/// TCB statuses as DCAP reports them; releases list them in lower case.
const TCB_STATUSES: [&str; 6] = [
    "UpToDate",
    "SWHardeningNeeded",
    "ConfigurationNeeded",
    "ConfigurationAndSWHardeningNeeded",
    "OutOfDate",
    "OutOfDateConfigurationNeeded",
];

struct TrustedImages {
    images: Vec<PublishedImage>,
    tcb_statuses: Vec<String>,
}

/// Each release's image of `profile`, and the TCB statuses all of them accept.
fn trusted_images(releases: &[String], profile: &str) -> PyResult<TrustedImages> {
    if releases.is_empty() {
        return Err(PyValueError::new_err(
            "no release given, so no image would be trusted",
        ));
    }
    let mut images = Vec::new();
    let mut statuses: Option<Vec<String>> = None;
    for (index, text) in releases.iter().enumerate() {
        let mut release: PublishedMrtds = serde_json::from_str(text).map_err(|err| {
            PyValueError::new_err(format!(
                "releases[{index}] is not a published-mrtds.json: {err}"
            ))
        })?;
        let name = release
            .tag
            .clone()
            .unwrap_or_else(|| format!("releases[{index}]"));
        if let Some(role) = release.role.as_deref().filter(|role| *role != "node") {
            return Err(PyValueError::new_err(format!(
                "{name} lists the measurements of a {role}, not of a node image"
            )));
        }
        let known = release
            .profiles
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let image = release.profiles.remove(profile).ok_or_else(|| {
            PyValueError::new_err(format!(
                "{name} has no profile {profile:?} (it has: {known})"
            ))
        })?;
        let accepted = image
            .allowed_tcb_statuses
            .clone()
            .unwrap_or_else(|| vec!["uptodate".to_owned()])
            .iter()
            .map(|status| {
                TCB_STATUSES
                    .iter()
                    .find(|known| known.eq_ignore_ascii_case(status))
                    .map(|known| (*known).to_owned())
                    .ok_or_else(|| {
                        PyValueError::new_err(format!(
                            "{name} accepts TCB status {status:?}, which this policy does not know"
                        ))
                    })
            })
            .collect::<PyResult<Vec<_>>>()?;
        statuses = Some(match statuses {
            None => accepted,
            Some(so_far) => so_far
                .into_iter()
                .filter(|status| accepted.contains(status))
                .collect(),
        });
        images.push(image);
    }
    let tcb_statuses = statuses.unwrap_or_default();
    if tcb_statuses.is_empty() {
        return Err(PyValueError::new_err(
            "the releases given accept no TCB status in common",
        ));
    }
    Ok(TrustedImages {
        images,
        tcb_statuses,
    })
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

    /// An image register's allowlist.
    fn rtmr() -> Option<Vec<String>> {
        Some(vec!["bb".repeat(48)])
    }

    /// A policy for one image, with an application or not.
    fn image(
        application_id: Option<&str>,
        application_hash: Option<&str>,
    ) -> pyo3::PyResult<PyTeePolicy> {
        PyTeePolicy::new(
            mrtd(),
            None,
            rtmr(),
            rtmr(),
            rtmr(),
            None,
            application_id,
            application_hash,
        )
    }

    #[test]
    fn a_policy_pins_at_least_one_image() {
        assert!(PyTeePolicy::new(vec![], None, rtmr(), rtmr(), rtmr(), None, None, None).is_err());
        assert!(image(None, None).is_ok());
    }

    #[test]
    fn an_mrtd_alone_pins_no_image() {
        assert!(PyTeePolicy::new(mrtd(), None, None, None, None, None, None, None).is_err());
        assert!(
            PyTeePolicy::new(mrtd(), None, rtmr(), rtmr(), Some(vec![]), None, None, None).is_err()
        );
    }

    fn release(tag: &str, rtmr3: &str, statuses: &str) -> String {
        let m = "aa".repeat(48);
        format!(
            r#"{{"role":"node","tag":"{tag}","profiles":{{"locked-read-only":{{"mrtd":"{m}","rtmr0":"{m}","rtmr1":"{m}","rtmr2":"{m}","rtmr3":"{rtmr3}","allowed_tcb_statuses":{statuses}}}}}}}"#
        )
    }

    #[test]
    fn a_policy_is_built_from_releases() {
        let old = "cc".repeat(48);
        let new = "dd".repeat(48);
        let policy = PyTeePolicy::from_releases(
            vec![
                release("2.3.76", &old, r#"["uptodate"]"#),
                release("2.3.78", &new, r#"["uptodate","outofdate"]"#),
            ],
            "locked-read-only",
            None,
            None,
        )
        .expect("two releases of one profile");
        assert_eq!(policy.allowed_mrtd(), mrtd());
        assert_eq!(policy.allowed_rtmr3(), vec![old, new]);
        assert_eq!(policy.allowed_tcb_statuses(), vec!["UpToDate".to_owned()]);
    }

    #[test]
    fn what_is_not_a_node_release_is_refused() {
        let good = release("2.3.78", &"dd".repeat(48), r#"["uptodate"]"#);
        assert!(PyTeePolicy::from_releases(vec![], "locked-read-only", None, None).is_err());
        assert!(PyTeePolicy::from_releases(vec![good.clone()], "prod", None, None).is_err());
        assert!(
            PyTeePolicy::from_releases(vec!["{}".to_owned()], "locked-read-only", None, None)
                .is_err()
        );
        assert!(PyTeePolicy::from_releases(
            vec![good.replace(r#""role":"node""#, r#""role":"kms""#)],
            "locked-read-only",
            None,
            None
        )
        .is_err());
        assert!(PyTeePolicy::from_releases(
            vec![good.replace(r#"["uptodate"]"#, r#"["sometimes"]"#)],
            "locked-read-only",
            None,
            None
        )
        .is_err());
    }

    #[test]
    fn a_measurement_that_is_not_one_is_refused() {
        assert!(PyTeePolicy::new(
            vec!["aabb".to_owned()],
            None,
            rtmr(),
            rtmr(),
            rtmr(),
            None,
            None,
            None
        )
        .is_err());
        assert!(PyTeePolicy::new(
            mrtd(),
            Some(vec!["zz".to_owned()]),
            rtmr(),
            rtmr(),
            rtmr(),
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
            image(Some(&id), None).is_err(),
            "an id without the hash it must run"
        );
        assert!(image(None, Some(&hash)).is_err());
        assert!(image(Some(&id), Some(&hash)).is_ok());
    }

    #[test]
    fn a_32_byte_value_is_checked_for_length() {
        assert_eq!(decode_32(&"ab".repeat(32), "x").unwrap(), [0xab; 32]);
        assert!(decode_32("abcd", "x").is_err());
        assert!(decode_32("not hex", "x").is_err());
    }
}
