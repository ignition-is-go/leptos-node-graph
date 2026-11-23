# Node menu browser regression

Build the actual demo with `trunk build --locked` from `examples/demo`, then serve
`examples/demo/dist` on port 8181. Install `requirements.txt` in a Python virtual
environment and run `python -m playwright install chromium`, then
`python tests/browser/test_node_menu.py` from the repository root.

`POPUP_TEST_URL` can override the fixture URL; `CHROMIUM_PATH` can select an
installed Chromium binary. The fixture is the demo's `?popup-regression` route.
It uses the real editor/catalog/create-node handler inside clipped, isolated,
transformed adjacent panes, with a custom theme and a disposable scene owner.

The suite covers cross-pane hit testing, mouse/keyboard selection, graph zoom
and creation coordinates, dismissal/refocus/reopen, pane disposal with queued
focus, viewport bounds/scaling, and draft-wire port selection.

## Port layout regression

The same demo build serves `?port-geometry-regression`. Run
`python tests/browser/test_port_geometry.py` against it (`GEOMETRY_TEST_URL`
overrides the URL). This asserts actual registered pin positions against the
rendered dots before/after server-style node movement, body resize and keyed
port reordering. Instrumented DOM getters reject geometry reads during
position-only updates; there are no timing-based performance thresholds.
