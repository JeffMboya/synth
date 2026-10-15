#!/usr/bin/env python3
"""Run: python3 tools/test_schematic_quality.py

The score reads an exported schematic: wires from the `.kicad_sch`, text boxes
from KiCad's SVG plot of it (each string is a `stroked-text` group of glyph
paths). These tests feed it small hand-written files, so they do not need
kicad-cli.
"""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import schematic_quality as sq  # noqa: E402


def sch(*wires, symbols=()):
    """A `.kicad_sch` with wires given as (x1, y1, x2, y2) and symbol anchors."""
    out = ["(kicad_sch"]
    for x1, y1, x2, y2 in wires:
        out.append(
            f"\t(wire\n\t\t(pts\n\t\t\t(xy {x1} {y1}) (xy {x2} {y2})\n\t\t)\n\t)"
        )
    for x, y in symbols:
        out.append(f'\t(symbol\n\t\t(lib_id "Device:R")\n\t\t(at {x} {y} 0)\n\t)')
    out.append(")")
    return "\n".join(out)


def svg(*texts, width=100.0, height=80.0):
    """A plot with `texts` given as (string, x0, y0, x1, y1) boxes."""
    out = [f'<svg viewBox="0.0000 0.0000 {width} {height}">']
    for s, x0, y0, x1, y1 in texts:
        out.append(
            f'<g class="stroked-text"><desc>{s}</desc>\n'
            f'<path d="M{x0} {y0}\nL{x1} {y0}\n" />\n'
            f'<path d="M{x0} {y1}\nL{x1} {y1}\n" />\n</g>'
        )
    out.append("</svg>")
    return "\n".join(out)


def score(sch_text, svg_text):
    return sq.score(
        sq.parse_wires(sch_text),
        sq.parse_texts(svg_text),
        sq.page_size(svg_text),
        sq.parse_content_points(sch_text),
        sq.parse_group_boxes(sch_text),
    )


class WireThroughText(unittest.TestCase):
    def test_a_wire_across_a_text_is_counted(self):
        got = score(sch((0, 5, 10, 5)), svg(("R1", 3, 4, 6, 6)))
        self.assertEqual(got["texts_with_a_wire_through_them"], 1)

    def test_a_text_clear_of_the_wire_is_not(self):
        got = score(sch((0, 5, 10, 5)), svg(("R1", 3, 7, 6, 9)))
        self.assertEqual(got["texts_with_a_wire_through_them"], 0)

    def test_a_wire_that_only_touches_the_text_edge_is_not(self):
        got = score(sch((0, 6, 10, 6)), svg(("R1", 3, 4, 6, 6)))
        self.assertEqual(got["texts_with_a_wire_through_them"], 0)


class TextOverlap(unittest.TestCase):
    def test_two_overlapping_texts_count_once(self):
        got = score(sch(), svg(("U1", 0, 0, 4, 2), ("MODEM_IDLE", 3, 1, 14, 3)))
        self.assertEqual(got["text_pairs_overlapping"], 1)

    def test_texts_side_by_side_do_not(self):
        got = score(sch(), svg(("U1", 0, 0, 4, 2), ("R1", 5, 0, 8, 2)))
        self.assertEqual(got["text_pairs_overlapping"], 0)

    def test_the_same_string_drawn_twice_in_one_place_is_not_a_collision(self):
        got = score(sch(), svg(("GND", 0, 0, 4, 2), ("GND", 0, 0, 4, 2)))
        self.assertEqual(got["text_pairs_overlapping"], 0)


class Duplicates(unittest.TestCase):
    def test_a_clash_with_a_doubled_string_counts_once(self):
        # KiCad draws "GND" twice here; the clash with U1 is still one clash.
        got = score(
            sch(),
            svg(("GND", 0, 0, 4, 2), ("GND", 0, 0, 4, 2), ("U1", 3, 1, 6, 3)),
        )
        self.assertEqual(got["text_pairs_overlapping"], 1)

    def test_a_doubled_string_with_a_wire_through_it_counts_once(self):
        got = score(
            sch((0, 5, 10, 5)),
            svg(("R1", 3, 4, 6, 6), ("R1", 3, 4, 6, 6)),
        )
        self.assertEqual(got["texts_with_a_wire_through_them"], 1)

    def test_the_same_string_in_two_places_is_two_texts(self):
        got = score(
            sch((0, 5, 10, 5), (0, 25, 10, 25)),
            svg(("R1", 3, 4, 6, 6), ("R1", 3, 24, 6, 26)),
        )
        self.assertEqual(got["texts_with_a_wire_through_them"], 2)


class WireAndTextShapes(unittest.TestCase):
    def test_a_wire_wholly_inside_a_text_box_is_through_it(self):
        got = score(sch((4, 5, 5, 5)), svg(("R1", 3, 4, 6, 6)))
        self.assertEqual(got["texts_with_a_wire_through_them"], 1)

    def test_overlapping_collinear_wires_do_not_cross(self):
        got = score(sch((0, 5, 10, 5), (5, 5, 15, 5)), svg())
        self.assertEqual(got["wire_crossings"], 0)


def sch_with_boxes(*boxes):
    """A `.kicad_sch` with group boxes as (x1, y1, x2, y2), plus one rectangle
    inside a symbol definition, which is not a group box."""
    out = ["(kicad_sch", "\t(lib_symbols", '\t\t(symbol "Device:R"',
           "\t\t\t(rectangle (start -1 -2) (end 1 2))", "\t\t)", "\t)"]
    for x1, y1, x2, y2 in boxes:
        out.append(f"\t(rectangle\n\t\t(start {x1} {y1})\n\t\t(end {x2} {y2})\n\t)")
    out.append(")")
    return "\n".join(out)


class RegionCoverage(unittest.TestCase):
    def cover(self, *boxes):
        return sq.region_coverage(sq.parse_group_boxes(sch_with_boxes(*boxes)))

    def test_boxes_that_tile_a_rectangle_cover_all_of_it(self):
        self.assertAlmostEqual(self.cover((0, 0, 10, 20), (10, 0, 30, 20)), 1.0)

    def test_a_short_box_beside_a_tall_one_leaves_a_hole(self):
        # Bounding rectangle 30 x 20 = 600; the boxes cover 200 + 200.
        self.assertAlmostEqual(self.cover((0, 0, 10, 20), (10, 0, 30, 10)), 400.0 / 600.0, places=4)

    def test_a_rectangle_inside_a_symbol_is_not_a_group_box(self):
        self.assertEqual(len(sq.parse_group_boxes(sch_with_boxes((0, 0, 10, 20)))), 1)

    def test_no_group_boxes_is_zero(self):
        self.assertEqual(self.cover(), 0.0)

    def test_score_carries_it(self):
        got = sq.score([], [], (100.0, 80.0), (), group_boxes=[(0, 0, 10, 20), (10, 0, 30, 10)])
        self.assertAlmostEqual(got["region_coverage"], 400.0 / 600.0, places=4)


class PickPlot(unittest.TestCase):
    def test_the_plot_named_after_the_schematic_wins(self):
        paths = [Path("a/sub.svg"), Path("a/board.svg")]
        self.assertEqual(sq.pick_plot(paths, "board"), Path("a/board.svg"))

    def test_a_single_plot_is_used_whatever_it_is_called(self):
        self.assertEqual(sq.pick_plot([Path("a/x.svg")], "board"), Path("a/x.svg"))

    def test_several_plots_and_none_for_the_root_is_an_error(self):
        with self.assertRaises(ValueError):
            sq.pick_plot([Path("a/one.svg"), Path("a/two.svg")], "board")


class WireCrossings(unittest.TestCase):
    def test_two_wires_that_cross_count_once(self):
        got = score(sch((0, 5, 10, 5), (5, 0, 5, 10)), svg())
        self.assertEqual(got["wire_crossings"], 1)

    def test_a_t_junction_is_not_a_crossing(self):
        got = score(sch((0, 5, 10, 5), (5, 5, 5, 10)), svg())
        self.assertEqual(got["wire_crossings"], 0)

    def test_parallel_wires_do_not_cross(self):
        got = score(sch((0, 5, 10, 5), (0, 7, 10, 7)), svg())
        self.assertEqual(got["wire_crossings"], 0)


class WireLengthAndFill(unittest.TestCase):
    def test_wire_length_is_the_sum_of_the_segments(self):
        got = score(sch((0, 0, 10, 0), (0, 0, 0, 5)), svg())
        self.assertEqual(got["wire_length_mm"], 15.0)

    def test_content_density_is_the_share_of_cells_in_use(self):
        # Two wires 20 mm apart span 3 x 3 cells of 10 mm; the wires run
        # through the 3 cells of the top row and the 3 of the bottom row.
        got = score(sch((0, 0, 20, 0), (0, 20, 20, 20)), svg())
        self.assertAlmostEqual(got["content_density"], 6.0 / 9.0, places=4)

    def test_a_text_fills_the_cells_it_covers(self):
        got = score(sch((0, 20, 0, 20.1), (20, 40, 20, 40.1)), svg(("R1", 0, 30, 14, 32)))
        # Cells 0..2 in x and y: the wires touch 2, the text 2 more.
        self.assertAlmostEqual(got["content_density"], 4.0 / 9.0, places=4)

    def test_the_page_ruler_and_title_block_do_not_stretch_the_content_box(self):
        # Content sits in x 20..40; a ruler letter at x 2 and title block text
        # at x 85 are far outside it and must not count.
        got = score(
            sch((20, 20, 40, 20), (20, 40, 40, 40), symbols=[(30, 30)]),
            svg(("A", 1, 30, 3, 32), ("Sheet", 85, 60, 95, 62)),
        )
        # Cells x 2..4, y 2..4: wires touch the top row (3) and bottom row (3),
        # the symbol the middle one: 7 of 9.
        self.assertAlmostEqual(got["content_density"], 7.0 / 9.0, places=4)

    def test_an_empty_schematic_scores_zero_everywhere(self):
        got = score(sch(), svg())
        self.assertEqual(
            (got["text_pairs_overlapping"], got["wire_crossings"], got["content_density"]),
            (0, 0, 0.0),
        )


if __name__ == "__main__":
    unittest.main()
