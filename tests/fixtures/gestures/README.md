Phase 3 scenario fixtures

Run `python3 tests/fixtures/gestures/generate.py` from the repository to regenerate match32, stress8 and the four synthetic drift traces.

The Z sample was captured through the headless recording UI by s_shape_1_record. It is runner-generated data, not an owner trace. The other encoded shapes are synthetic closed loops; their similar geometry deliberately stresses the complete matching set without confusing the open Z target.

The drift traces are a Python port of fingershape_synth's bounded wandering-heading palm drift, using floating-point trigonometry rather than its integer table. They are explicitly synthetic. Owner J/K traces and legacy device-drag captures were not supplied, so s_shape_9_traces does not claim that acceptance coverage.

The small onefinger, three, tap-undo and actions folders exercise textual bindings through the same # copy directives as the Makefile.
