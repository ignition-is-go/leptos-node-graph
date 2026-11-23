"""Real-DOM port geometry and no-layout-read regression for server-driven moves."""
import os
import unittest
from playwright.sync_api import expect, sync_playwright


class PortGeometryTests(unittest.TestCase):
    def test_moves_reuse_offsets_and_layout_changes_remeasure(self):
        with sync_playwright() as pw:
            browser = pw.chromium.launch(executable_path=os.environ.get("CHROMIUM_PATH", "/usr/bin/chromium"))
            page = browser.new_page()
            errors = []
            page.on("pageerror", lambda error: errors.append(str(error)))
            page.add_init_script("""{
                window.geometryReads = 0;
                const rect = Element.prototype.getBoundingClientRect;
                Element.prototype.getBoundingClientRect = function() {
                    if (this.hasAttribute('data-anchor-dot')) window.geometryReads++;
                    return rect.call(this);
                };
                const offset = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'offsetTop');
                Object.defineProperty(HTMLElement.prototype, 'offsetTop', {
                    ...offset, get() {
                        if (this.hasAttribute('data-node-ports')) window.geometryReads++;
                        return offset.get.call(this);
                    }
                });
            }""")
            page.goto(os.environ.get("GEOMETRY_TEST_URL", "http://127.0.0.1:8181/?port-geometry-regression"))
            expect(page.locator("[data-anchor-dot]")).to_have_count(2)

            def settled():
                page.evaluate("() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
                page.wait_for_function("""() => {
                    const text = document.querySelector('#geometry').textContent;
                    const positions = Object.fromEntries(text.split(';').map(p => {
                        const [id, xy] = p.split(':'); return [id, xy?.split(',').map(Number)];
                    }));
                    const canvas = document.querySelector('.node-editor__canvas');
                    const m = new DOMMatrix(getComputedStyle(canvas).transform);
                    return [...document.querySelectorAll('[data-anchor]')].every(row => {
                        const id = row.textContent.trim();
                        const rect = row.querySelector('[data-anchor-dot]').getBoundingClientRect();
                        const node = row.closest('[data-node]');
                        const origin = node.getBoundingClientRect();
                        const expected = [(rect.x + rect.width/2-origin.x)/m.a + parseFloat(node.style.left), (rect.y + rect.height/2-origin.y)/m.d + parseFloat(node.style.top)];
                        return positions[id] && expected.every((v,i) => Math.abs(v-positions[id][i]) < 1);
                    });
                }""")

            settled()
            for action in [None, "resize-body", "reverse-ports"]:
                if action:
                    page.locator("#" + action).click()
                    settled()
                page.evaluate("window.geometryReads = 0")
                for _ in range(5):
                    page.locator("#move-node").click()
                    # Probe DOM through the saved native method only after asserting
                    # production has not remeasured; our geometry check reads dots too.
                    page.evaluate("() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
                self.assertEqual(page.evaluate("window.geometryReads"), 0)
                settled()
            self.assertEqual(errors, [])
            browser.close()


if __name__ == "__main__":
    unittest.main(verbosity=2)
