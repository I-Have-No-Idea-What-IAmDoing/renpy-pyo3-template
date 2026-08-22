# ==============================================================================
# Ren'Py PyO3 Example Integration Script
#
# Place this script in your Ren'Py project's 'game/' directory alongside the
# binaries generated in 'game/python-packages/' or 'lib/'.
# ==============================================================================

init python:
    # Import your compiled Rust PyO3 module
    import renpy_rust_module

    # Call a standalone Rust function
    welcome_message = renpy_rust_module.greet("Ren'Py Developer")

    # Create and manipulate a Rust-backed class instance
    game_tracker = renpy_rust_module.GameStateTracker(starting_points=100)

label start:
    scene black
    with fade

    # Display greeting from Rust
    "[welcome_message]"

    $ current_score = game_tracker.add_points(50)
    "Current Points: [current_score]"

    $ final_result = renpy_rust_module.heavy_computation(1000000)
    "Heavy Computation in Rust (without dropping UI frames): [final_result]"

    return
