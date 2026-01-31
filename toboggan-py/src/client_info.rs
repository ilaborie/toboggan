use pyo3::prelude::*;
use toboggan_core::ClientInfo as TClientInfo;

/// Information about a connected client.
#[pyclass]
pub struct ClientInfo(pub(crate) TClientInfo);

#[pymethods]
impl ClientInfo {
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }

    #[getter]
    fn connected_at(&self) -> String {
        self.0.connected_at.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ClientInfo(\"{}\", {})", self.0.name, self.0.connected_at)
    }
}
