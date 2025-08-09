# Python avec PyO3

```rust
use pyo3::prelude::*;

#[pyfunction]
fn compute_heavy_task(data: Vec<f64>) -> PyResult<f64> {
    // Calculs intensifs en Rust
    Ok(data.iter().sum())
}

#[pymodule]
fn mymodule(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compute_heavy_task, m)?)?;
    Ok(())
}
```

- Accélération des parties critiques
- Distribution via pip
- Exemples : Pydantic v2, Polars