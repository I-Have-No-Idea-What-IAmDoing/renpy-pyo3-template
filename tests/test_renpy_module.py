#!/usr/bin/env python3
"""
Automated unit and integration test suite for the compiled Ren'Py Rust module.
Verifies module imports, functions, classes, and pickle/rollback compatibility.
"""

import sys
import os
import glob
import pickle
import unittest

# Ensure the module and its dependent DLLs can be found
TEST_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(TEST_DIR)
PKG_DIST_DIR = os.path.join(PROJECT_ROOT, "dist", "python-packages")
LIB_DIST_DIR = os.path.join(PROJECT_ROOT, "dist", "lib", "py3-windows-x86_64")
INCLUDES_DIR = os.path.join(PROJECT_ROOT, "renpy_includes")

for path in [PKG_DIST_DIR, LIB_DIST_DIR, PROJECT_ROOT]:
    if os.path.isdir(path) and path not in sys.path:
        sys.path.insert(0, path)

# Add DLL directories on Windows (Python 3.8+)
if hasattr(os, "add_dll_directory"):
    for dll_dir in [INCLUDES_DIR, LIB_DIST_DIR, PKG_DIST_DIR]:
        if os.path.isdir(dll_dir):
            try:
                os.add_dll_directory(dll_dir)
            except Exception:
                pass

if "PATH" in os.environ:
    os.environ["PATH"] = f"{INCLUDES_DIR};{LIB_DIST_DIR};" + os.environ["PATH"]

try:
    import renpy_rust_module
except ImportError as e:
    print(f"[!] Could not import 'renpy_rust_module': {e}")
    print(f"    Current sys.path: {sys.path}")
    sys.exit(1)


class TestRenpyRustModule(unittest.TestCase):
    def test_greet_function(self):
        msg = renpy_rust_module.greet("Ren'Py Developer")
        self.assertIn("Ren'Py Developer", msg)
        self.assertTrue(msg.startswith("Hello from Rust"))

    def test_heavy_computation(self):
        result = renpy_rust_module.heavy_computation(1000)
        self.assertIsInstance(result, int)
        # Sum of 0..999 is (999 * 1000) / 2 = 499500
        self.assertEqual(result, 499500)

    def test_game_state_tracker(self):
        tracker = renpy_rust_module.GameStateTracker(starting_points=100)
        self.assertEqual(tracker.points, 100)

        # Add points
        new_total = tracker.add_points(50)
        self.assertEqual(new_total, 150)
        self.assertEqual(tracker.points, 150)

        # Verify history
        history = tracker.get_history()
        self.assertGreater(len(history), 0)
        self.assertIn("150", history[-1])

        # Verify error handling
        with self.assertRaises(ValueError):
            tracker.add_points(-10)

        # Verify reset
        tracker.reset()
        self.assertEqual(tracker.points, 0)
        self.assertEqual(len(tracker.get_history()), 0)

    def test_renpy_pickle_and_rollback(self):
        """
        Crucial for Ren'Py: Game variables stored in Ren'Py's state
        must be picklable for Save/Load and Rollback to function without errors.
        """
        original = renpy_rust_module.GameStateTracker(starting_points=250)
        original.add_points(75)

        # Simulate Ren'Py game save / rollback snapshot
        saved_bytes = pickle.dumps(original)
        self.assertIsInstance(saved_bytes, bytes)

        # Simulate Ren'Py game load / rollback restore
        restored = pickle.loads(saved_bytes)
        self.assertEqual(restored.points, 325)
        self.assertEqual(restored.get_history(), original.get_history())


if __name__ == "__main__":
    print("\n=======================================================")
    print("Running Ren'Py Rust Module Test Suite...")
    print("=======================================================\n")
    unittest.main(verbosity=2)
