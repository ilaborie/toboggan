use pyo3::prelude::*;
use toboggan_core::{ClientInfo as TClientInfo, ClientRole};

/// The wire spelling of a role, shared with [`crate::Toboggan::role`].
///
/// `ClientRole` derives `Debug` and nothing else printable, and `Debug` is not
/// an API: this is the one place that decides what Python sees.
pub(crate) const fn role_name(role: ClientRole) -> &'static str {
    match role {
        ClientRole::Presenter => "presenter",
        ClientRole::Audience => "audience",
    }
}

/// Information about a connected client.
#[pyclass]
pub struct ClientInfo(pub(crate) TClientInfo);

#[pymethods]
impl ClientInfo {
    /// The name the client registered under.
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }

    /// Where the connection came from, which is also what decided its role.
    #[getter]
    fn ip_addr(&self) -> String {
        self.0.ip_addr.to_string()
    }

    /// What the server granted this client: `"presenter"` or `"audience"`.
    #[getter]
    fn role(&self) -> &'static str {
        role_name(self.0.role)
    }

    /// Whether this client may drive the deck and open terminals.
    #[getter]
    fn is_presenter(&self) -> bool {
        self.0.role.is_presenter()
    }

    /// When the client registered.
    #[getter]
    fn connected_at(&self) -> String {
        self.0.connected_at.to_string()
    }

    fn __repr__(&self) -> String {
        let name = &self.0.name;
        let role = self.role();
        let ip_addr = &self.0.ip_addr;
        let connected_at = &self.0.connected_at;
        format!("ClientInfo(\"{name}\", {role}, {ip_addr}, {connected_at})")
    }
}
