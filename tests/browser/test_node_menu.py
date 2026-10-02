"""Exercise the real WASM demo inside isolated, clipped, adjacent panes."""
import os
import re
import unittest

from playwright.sync_api import expect, sync_playwright


class NodeMenuTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.playwright = sync_playwright().start()
        executable = os.environ.get("CHROMIUM_PATH")
        cls.browser = cls.playwright.chromium.launch(
            **({"executable_path": executable} if executable else {})
        )

    @classmethod
    def tearDownClass(cls):
        cls.browser.close()
        cls.playwright.stop()

    def setUp(self):
        self.page = self.browser.new_page(viewport={"width": 1100, "height": 750})
        self.errors = []
        self.events = []
        self.page.on("pageerror", lambda error: self.errors.append(str(error)))
        self.page.on("console", lambda message: self.events.append(message.text))
        self.page.goto(os.environ.get("POPUP_TEST_URL", "http://127.0.0.1:8181/?popup-regression"))
        self.editor = self.page.locator(".node-editor")
        expect(self.editor).to_be_visible()
        self.menu = self.page.locator("[data-node-menu]")
        self.search = self.menu.locator("input")

    def tearDown(self):
        self.page.close()
        self.assertEqual(self.errors, [])

    def frame(self):
        self.page.evaluate("() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))")

    def open(self, x=490, y=240):
        self.editor.focus()
        self.page.mouse.move(x, y)
        self.page.keyboard.press("Tab")
        expect(self.search).to_be_focused()

    def created(self):
        return [event for event in self.events if event.startswith("Graph event: CreateNode")]

    def test_cross_pane_mouse_selection_and_theme(self):
        self.open()
        self.assertEqual(self.menu.locator(":scope > div").evaluate("el => getComputedStyle(el).backgroundColor"), "rgb(31, 42, 53)")
        self.assertEqual(self.page.locator("#scene-pane").evaluate("el => getComputedStyle(el).isolation"), "isolate")
        self.search.fill("Math")
        row = self.menu.locator("[data-menu-item-selected]")
        box = row.bounding_box()
        x, y = box["x"] + box["width"] - 15, box["y"] + 10
        self.assertGreater(x, 520)
        self.assertTrue(self.page.evaluate("([x,y]) => !!document.elementFromPoint(x,y)?.closest('[data-node-menu]')", [x, y]))
        self.assertFalse(self.menu.evaluate("el => !!el.closest('#scene-pane')"))
        self.page.mouse.click(x, y)
        expect(self.menu).to_have_count(0)
        self.frame()
        self.assertEqual(len(self.created()), 1)
        self.assertIn('item_id: "math"', self.created()[0])
        expect(self.editor).to_be_focused()

    def test_keyboard_coordinates_after_zoom(self):
        self.page.mouse.move(300, 300)
        self.page.mouse.wheel(0, -250)
        self.frame()
        expected = self.editor.evaluate("el => { const r=el.getBoundingClientRect(); const m=new DOMMatrix(getComputedStyle(el.querySelector('.node-editor__canvas')).transform); return [(490-r.left-m.e)/m.a,(240-r.top-m.f)/m.d]; }")
        self.open()
        self.search.fill("Math")
        self.page.keyboard.press("Enter")
        expect(self.menu).to_have_count(0)
        self.frame()
        event = self.created()[0]
        actual = re.search(r"position: Position \{ x: ([^,]+), y: ([^ }]+)", event)
        self.assertIsNotNone(actual)
        for value, wanted in zip(actual.groups(), expected):
            self.assertAlmostEqual(float(value), wanted, places=3)

    def test_escape_outside_and_reopen(self):
        for _ in range(3):
            self.open()
            self.page.keyboard.press("Escape")
            expect(self.menu).to_have_count(0)
            expect(self.editor).to_be_focused()
        self.open()
        self.page.locator("#neighbor-control").click()
        expect(self.menu).to_have_count(0)
        self.frame()
        expect(self.page.locator("#neighbor-control")).to_be_focused()
        self.open()
        self.page.keyboard.press("Tab")
        expect(self.menu).to_have_count(0)
        self.assertEqual(self.created(), [])

    def test_pane_disposal_and_pending_focus(self):
        self.open()
        # Dispose without outside-pointer dismissal, exercising portal cleanup.
        self.page.locator("#toggle-pane").evaluate("el => el.click()")
        expect(self.menu).to_have_count(0)
        expect(self.editor).to_have_count(0)
        self.frame()
        self.page.locator("#toggle-pane").click()
        expect(self.editor).to_be_visible()
        # Open and dispose in one JS task, before focus/scroll rAF callbacks.
        self.editor.evaluate("el => {el.dispatchEvent(new KeyboardEvent('keydown',{key:'Tab',bubbles:true})); document.querySelector('#toggle-pane').click();}")
        self.frame()
        expect(self.menu).to_have_count(0)
        self.page.locator("#toggle-pane").click()
        self.open()

    def test_viewport_edges_and_scaling(self):
        for width, height, zoom in [(1100, 750, '1.25'), (250, 260, '1')]:
            self.page.set_viewport_size({"width": width, "height": height})
            self.page.locator("#scene-pane").evaluate("(el, zoom) => el.style.zoom=zoom", zoom)
            self.open(min(width - 10, 490), min(height - 10, 540))
            box = self.menu.bounding_box()
            self.assertGreaterEqual(box["x"], 0)
            self.assertGreaterEqual(box["y"], 0)
            self.assertLessEqual(box["x"] + box["width"], width)
            self.assertLessEqual(box["y"] + box["height"], height)
            self.page.keyboard.press("Escape")

    def test_wire_autoconnect_exact_port(self):
        self.open(200, 200)
        self.search.fill("Color Source")
        self.page.keyboard.press("Enter")
        expect(self.menu).to_have_count(0)
        self.frame()
        source = self.page.locator("[data-node]").filter(has_text="Color Source").last
        alpha = source.locator("[data-anchor]").filter(has_text="Alpha").locator("[data-anchor-dot]")
        alpha.click()
        self.page.mouse.move(490, 350)
        self.page.keyboard.press("Tab")
        expect(self.search).to_be_focused()
        self.search.fill("Math")
        self.page.keyboard.press("ArrowDown")
        self.page.keyboard.press("Enter")
        expect(self.menu).to_have_count(0)
        self.frame()
        self.assertEqual(len(self.created()), 2)
        event = self.created()[-1]
        self.assertRegex(event, r'connect_from: Some\("[^\"]+_alpha"\)')
        self.assertIn('connect_to: Some("b")', event)
        self.assertIn('connect_direction: Some(Output)', event)
        # A newly created node remains alive after the portal owner closes.
        expect(self.page.locator("[data-node]").filter(has_text="Math").last).to_be_visible()


if __name__ == "__main__":
    unittest.main(verbosity=2)
