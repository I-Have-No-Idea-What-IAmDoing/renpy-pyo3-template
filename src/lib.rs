use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

/// A simple standalone function exposed to Python.
///
/// Ren'Py usage:
///     renpy_rust_module.greet("World")
#[pyfunction]
fn greet(name: &str) -> String {
    format!("Hello from Rust to {}!", name)
}

/// A fast mathematical or heavy processing function.
///
/// Notice the use of Python::allow_threads:
/// This releases Python's GIL (Global Interpreter Lock), allowing Ren'Py's
/// UI and rendering loop (60+ FPS) to continue smoothly while Rust computes in the background!
#[pyfunction]
fn heavy_computation(py: Python<'_>, count: u64) -> u64 {
    py.detach(move || {
        let mut sum = 0u64;
        for i in 0..count {
            sum = sum.wrapping_add(i);
        }
        sum
    })
}

/// An example PyO3 class exposed to Ren'Py.
///
/// Ren'Py usage:
///     tracker = renpy_rust_module.GameStateTracker(starting_points=100)
///     tracker.add_points(50)
///     print(tracker.points)
#[pyclass]
struct GameStateTracker {
    #[pyo3(get, set)]
    points: i64,
    history: Vec<String>,
}

#[pymethods]
impl GameStateTracker {
    #[new]
    #[pyo3(signature = (starting_points = 0))]
    fn new(starting_points: i64) -> Self {
        GameStateTracker {
            points: starting_points,
            history: Vec::new(),
        }
    }

    /// Modify state from Python.
    fn add_points(&mut self, amount: i64) -> PyResult<i64> {
        if amount < 0 {
            return Err(PyValueError::new_err("Cannot add negative points! Use deduct_points instead."));
        }
        self.points += amount;
        self.history.push(format!("Added {} points. Total: {}", amount, self.points));
        Ok(self.points)
    }

    /// Retrieve historical events as a Python list of strings.
    fn get_history(&self) -> Vec<String> {
        self.history.clone()
    }

    /// Reset internal state.
    fn reset(&mut self) {
        self.points = 0;
        self.history.clear();
    }

    // =========================================================================
    // Ren'Py Save/Load & Rollback Compatibility (Python Pickle Protocol)
    // =========================================================================

    /// Serializes state when Ren'Py saves the game or takes a rollback snapshot.
    fn __getstate__(&self) -> (i64, Vec<String>) {
        (self.points, self.history.clone())
    }

    /// Restores state when Ren'Py loads a save file or rolls back dialogue.
    fn __setstate__(&mut self, state: (i64, Vec<String>)) {
        self.points = state.0;
        self.history = state.1;
    }
}

/// The main Python module entry point.
///
/// Note: The name of this function/module must match the name specified
/// in `Cargo.toml` under `[lib] name = "..."`.
#[pymodule]
fn renpy_rust_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(greet, m)?)?;
    m.add_function(wrap_pyfunction!(heavy_computation, m)?)?;
    m.add_class::<GameStateTracker>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("Ren'Py"), "Hello from Rust to Ren'Py!");
    }

    #[test]
    fn test_heavy_computation_logic() {
        let count = 1000u64;
        let mut sum = 0u64;
        for i in 0..count {
            sum = sum.wrapping_add(i);
        }
        assert_eq!(sum, 499500);
    }

    #[test]
    fn test_game_tracker_and_pickle_state() {
        let mut tracker = GameStateTracker::new(100);
        assert_eq!(tracker.points, 100);
        let res = tracker.add_points(50).unwrap();
        assert_eq!(res, 150);
        assert_eq!(tracker.points, 150);
        assert!(tracker.add_points(-5).is_err());

        // Test rollback / save state serialization
        let state = tracker.__getstate__();
        assert_eq!(state.0, 150);
        assert_eq!(state.1.len(), 1);

        let mut restored = GameStateTracker::new(0);
        restored.__setstate__(state);
        assert_eq!(restored.points, 150);
        assert_eq!(restored.get_history().len(), 1);
    }
}
