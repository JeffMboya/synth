"""Author a floorplan for the multirail board using TRUE KiCad courtyards.

The first generator sized parts from the registry's declared
`footprint_dimensions`, which is the part body. KiCad's `F.CrtYd` polygon is
what DRC checks, it is larger, and it is not centred on the footprint origin:
the ESP32-WROOM-32's courtyard is 48.0 x 41.2 mm because the footprint draws
the antenna keepout on that layer, and a 1x20 header's courtyard runs
y -1.78 to +50.03 from its origin. Placing by declared body size produced 46
courtyard overlaps.

This version works in courtyard space. Each group is given a rectangular
region; parts are packed inside it largest-first on a 0.5 mm grid, and every
candidate is tested as the true courtyard rectangle against every courtyard
already placed. The anchor written to the sidecar is derived back from the
chosen courtyard position, because the sidecar's x/y is the footprint origin.
"""
import glob
import math
import os
import re
import sys
import tomllib

FP = os.environ["KICAD_FP_DIR"]
ROOT = os.environ.get("BOARD_ROOT", ".")
SRC = os.path.join(ROOT, "qualification/class-c/multirail-compute/multirail_compute.synth")
OUT = os.path.join(ROOT, "qualification/class-c/multirail-compute/multirail_compute.placement.layout.toml")

BOARD_W = float(os.environ.get("BOARD_W", 140.0))
BOARD_H = float(os.environ.get("BOARD_H", 100.0))
EDGE = 3.0
GAP = 1.6          # the placer inflates every courtyard by extra_margin_mm = 1.5 mm
                   # before its legalisation sweep, so a smaller gap here still
                   # reads as an overlap to it and it nudges the pinned part.
GRID = 0.5


def rot_offset(x, y, rot):
    """`synth_geometry::Rotation::rotate_offset`, which is KiCad's file
    convention: a pad at local (2, 0) on a footprint at `(at 20 20 90)` lands
    on board (20, 18)."""
    if rot == 90:
        return (y, -x)
    if rot == 180:
        return (-x, -y)
    if rot == 270:
        return (-y, x)
    return (x, y)


def _fp_text(libid):
    """The `.kicad_mod` body behind a `Lib:Module` id, or None."""
    if not libid or ":" not in libid:
        return None
    lib, mod = libid.split(":", 1)
    path = os.path.join(FP, lib + ".pretty", mod + ".kicad_mod")
    if not os.path.exists(path):
        return None
    return open(path, encoding="utf-8").read()


def _fp_blocks(text, prefix="(fp_"):
    """Each balanced `(fp_... )` block in `text`."""
    pos = 0
    while True:
        k = text.find(prefix, pos)
        if k < 0:
            return
        depth = 0
        for j in range(k, len(text)):
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
                if depth == 0:
                    yield text[k:j + 1]
                    break
        pos = k + len(prefix)


def _arc_points(blk):
    """Sample an `fp_arc`'s true extent.

    An arc is `(start) (mid) (end)` and it bulges past all three: the Keystone
    1058 coin holder draws its courtyard with ten of them, and reading only
    the stated points measured a 23.6 mm tall courtyard as 16.2 mm. That is
    how a 20 mm coin cell came to overlap five neighbours in
    `kicad-cli pcb drc` while this script reported no overlap at all.
    """
    pts = []
    for key in ("start", "mid", "end"):
        m = re.search(r"\(%s ([-0-9.]+) ([-0-9.]+)\)" % key, blk)
        if not m:
            return []
        pts.append((float(m.group(1)), float(m.group(2))))
    (x1, y1), (x2, y2), (x3, y3) = pts
    det = 2 * (x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2))
    if abs(det) < 1e-12:            # collinear: a straight segment
        return pts
    s1, s2, s3 = x1 * x1 + y1 * y1, x2 * x2 + y2 * y2, x3 * x3 + y3 * y3
    ux = (s1 * (y2 - y3) + s2 * (y3 - y1) + s3 * (y1 - y2)) / det
    uy = (s1 * (x3 - x2) + s2 * (x1 - x3) + s3 * (x2 - x1)) / det
    r = math.hypot(x1 - ux, y1 - uy)
    a1 = math.atan2(y1 - uy, x1 - ux)
    sweep = (math.atan2(y3 - uy, x3 - ux) - a1) % (2 * math.pi)
    if (math.atan2(y2 - uy, x2 - ux) - a1) % (2 * math.pi) > sweep:
        sweep -= 2 * math.pi        # the mid point is the other way round
    n = max(8, int(abs(sweep) / 0.05))
    return [(ux + r * math.cos(a1 + sweep * i / n),
             uy + r * math.sin(a1 + sweep * i / n)) for i in range(n + 1)]


def courtyard(libid):
    """(dx_min, dy_min, dx_max, dy_max) of F.CrtYd relative to the origin."""
    text = _fp_text(libid)
    if text is None:
        return None
    pts = []
    pos = 0
    while True:
        k = text.find("(fp_", pos)
        if k < 0:
            break
        depth = 0
        blk = None
        for j in range(k, len(text)):
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
                if depth == 0:
                    blk = text[k:j + 1]
                    break
        # Courtyard AND silkscreen. The courtyard is the contract, but it is
        # not always the larger: the Keystone 1058 coin holder draws a
        # 32.9 x 8.2 mm courtyard around its clips and a 28.6 x 16.2 mm
        # silkscreen around the cell, so a 20 mm cell overhangs its own
        # courtyard by 4 mm top and bottom. Packing to the courtyard alone put
        # the RTC underneath the battery.
        if blk and ("F.CrtYd" in blk or "F.SilkS" in blk):
            # A circle is `(center cx cy) (end ex ey)`, and those two points
            # share a y when the radius runs along x -- so reading the raw
            # points gave fiducials, test points and mounting holes a height
            # of exactly 0.00 mm and the packer stacked them 2 mm apart with
            # no overlap to report. Expand a circle to its bounding box.
            if blk.startswith("(fp_arc"):
                pts += _arc_points(blk)
            elif blk.startswith("(fp_circle"):
                c = re.search(r"\(center ([-0-9.]+) ([-0-9.]+)\)", blk)
                e = re.search(r"\(end ([-0-9.]+) ([-0-9.]+)\)", blk)
                if c and e:
                    cx, cy = float(c.group(1)), float(c.group(2))
                    ex, ey = float(e.group(1)), float(e.group(2))
                    r = math.hypot(ex - cx, ey - cy)
                    pts += [(cx - r, cy - r), (cx + r, cy + r)]
            else:
                pts += [
                    (float(a), float(b))
                    for a, b in re.findall(
                        r"\((?:start|end|center|xy) ([-0-9.]+) ([-0-9.]+)\)", blk
                    )
                ]
        pos = k + 4
    if not pts:
        for m in re.finditer(r"\(pad ", text):
            k = m.start()
            depth = 0
            blk = None
            for j in range(k, len(text)):
                if text[j] == "(":
                    depth += 1
                elif text[j] == ")":
                    depth -= 1
                    if depth == 0:
                        blk = text[k:j + 1]
                        break
            at = re.search(r"\(at ([-0-9.]+) ([-0-9.]+)", blk or "")
            sz = re.search(r"\(size ([0-9.]+) ([0-9.]+)\)", blk or "")
            if at and sz:
                x, y = float(at.group(1)), float(at.group(2))
                w, h = float(sz.group(1)), float(sz.group(2))
                pts += [(x - w / 2 - 0.25, y - h / 2 - 0.25), (x + w / 2 + 0.25, y + h / 2 + 0.25)]
    # Rule areas declared inside the footprint -- a card socket's insertion
    # envelope, a connector's latch space. Nothing may sit in them, so they
    # bound the part as surely as its courtyard does. The microSD socket has
    # five, and fiducials landed in them.
    for m in re.finditer(r"\(zone\b", text):
        k = m.start()
        depth = 0
        blk = None
        for j in range(k, len(text)):
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
                if depth == 0:
                    blk = text[k:j + 1]
                    break
        if blk and "keepout" in blk:
            pts += [
                (float(a), float(b))
                for a, b in re.findall(r"\(xy ([-0-9.]+) ([-0-9.]+)\)", blk)
            ]

    if not pts:
        return None
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    return (min(xs), min(ys), max(xs), max(ys))


def crtyd_center(libid):
    """Centre of the F.CrtYd rect *as the exporter computes it*.

    `synth_layout::pcb_courtyard_geometry_for_part` goes through
    `kicad_footprint_loader::courtyard_rect`, which reads `fp_rect` and
    `fp_line` on `F.CrtYd` and nothing else -- no arc, no circle, no polygon,
    and no silkscreen. The exporter then puts the footprint origin at
    `sidecar_xy - rot(this centre)`. Packing against the wider union below and
    writing *its* centre therefore lands the part a little away from where it
    was packed, by up to 1.20 mm on the radial electrolytic. Returns `(0.0,
    0.0)` when there is no such rect, which is the offset the exporter falls
    back to.
    """
    text = _fp_text(libid)
    if text is None:
        return (0.0, 0.0)
    xs, ys = [], []
    for blk in _fp_blocks(text):
        if not (blk.startswith("(fp_rect") or blk.startswith("(fp_line")):
            continue
        if "F.CrtYd" not in blk:
            continue
        for key in ("(start ", "(end "):
            m = re.search(re.escape(key) + r"([-0-9.]+) ([-0-9.]+)", blk)
            if m:
                xs.append(float(m.group(1)))
                ys.append(float(m.group(2)))
    if not xs:
        return (0.0, 0.0)
    return ((min(xs) + max(xs)) / 2.0, (min(ys) + max(ys)) / 2.0)


src = open(SRC).read()
refs, group_of = {}, {}
current = None
for line in src.splitlines():
    g = re.match(r'\s*group "([^"]+)"', line)
    if g:
        current = g.group(1)
    c = re.match(r"\s*component (\w+): (\w+)\s+\"([^\"]+)\"", line)
    if c:
        refs[c.group(1)] = c.group(3)
        group_of[c.group(1)] = current

parts = {}
for p in glob.glob(os.path.join(ROOT, "registry/parts/*/*.synth.toml")):
    d = tomllib.load(open(p, "rb"))
    parts[d["id"]] = d


def geom(ref, rot=0):
    """Courtyard offsets for a part at a rotation, as (dx0, dy0, dx1, dy1)."""
    d = parts.get(refs[ref]) or {}
    e = courtyard(d.get("kicad_footprint"))
    if e is None:
        fd = d.get("footprint_dimensions") or {}
        w = fd.get("width_mm", 2.0) + 0.5
        h = fd.get("height_mm", 2.0) + 0.5
        e = (-w / 2, -h / 2, w / 2, h / 2)
    x0, y0, x1, y1 = e
    # Rotate the rectangle with `rot_offset`, the same quarter turn the
    # exporter and KiCad apply. The two quarter turns used to be the other way
    # round here, which cost nothing while the anchor below was the packed
    # rectangle's own centre -- a centre does not depend on which way the
    # offsets run -- but puts a rotated part its full length away once the
    # anchor is derived from the origin. J4, a 1x20 header, landed 48.25 mm
    # east of its region and 3 mm over the board edge.
    corners = [rot_offset(x, y, rot) for x, y in ((x0, y0), (x1, y1))]
    xs = [c[0] for c in corners]
    ys = [c[1] for c in corners]
    return (min(xs), min(ys), max(xs), max(ys))


# Region per group, as (x0, y0, x1, y1) in board millimetres. The board reads
# left to right as it works: power down the left edge, the module in the
# middle with its headers above and below, and the interfaces on the right.
REGIONS = {
    "POWER IN":            (3.0, 3.0, 38.0, 12.0),
    "5V BUCK":             (3.0, 12.0, 38.0, 26.0),
    "3V3 RAIL":            (3.0, 26.0, 38.0, 44.0),
    "3V3 ANALOG":          (3.0, 50.0, 38.0, 60.0),
    "SWITCHED RAIL":       (3.0, 44.0, 38.0, 50.0),
    "COMPUTE":             (38.0, 30.0, 99.0, 74.0),
    "EXPANSION HEADERS":   (38.0, 3.0, 99.0, 97.0),
    "USB AND PROGRAMMING": (38.0, 60.0, 99.0, 74.0),
    "STORAGE":             (38.0, 22.0, 99.0, 30.0),
    "ETHERNET":            (99.0, 3.0, 137.0, 22.0),
    "IO EXPANSION":        (95.0, 22.0, 137.0, 52.0),
    "MONITORING":          (38.0, 74.0, 99.0, 88.0),
    "STATUS AND DISPLAY":  (38.0, 88.0, 99.0, 97.0),
    "MECHANICAL":          (3.0, 3.0, 137.0, 97.0),
    "INDUSTRIAL COMMS":    (99.0, 52.0, 137.0, 97.0),
    "RTC AND SUPERVISION": (3.0, 60.0, 38.0, 97.0),
    "REMOVABLE STORAGE":   (99.0, 3.0, 137.0, 52.0),
    "DEBUG AND RECOVERY":  (38.0, 3.0, 99.0, 22.0),
}

# Parts whose rotation or position is dictated by the board edge they serve.
FIXED_ROT = {"J4": 90, "J5": 90}
# Hard anchors: a connector that must face out, or a hole that takes a corner.
# The holes are given a corner rather than a coordinate, because the
# coordinate has to follow the courtyard's size and a mounting hole's
# courtyard is a circle -- which the first floorplan measured as 0.00 mm tall,
# so three of the four hard anchors silently fell off the board once the
# circle was read properly.
HARD_CORNER = {
    "H1": ("left", "top"), "H2": ("right", "top"),
    "H3": ("left", "bottom"), "H4": ("right", "bottom"),
}
HARD = {"J4": (40.0, 3.2), "J5": (40.0, 93.0)}

placed = {}   # ref -> (cx0, cy0, cx1, cy1) courtyard rect actually occupied
chosen = {}   # ref -> (anchor_x, anchor_y, rot)


def blocker(rect, skip=None):
    """The first already-placed courtyard that rules `rect` out, or None.

    Returning the obstacle rather than a bare False lets the sweep below jump
    straight past it. Stepping GRID at a time instead costs
    268 x 188 candidates against 152 placed rectangles for every part that
    does not fit its own region, which is minutes of Python per part -- and
    sizing BT1's courtyard honestly is exactly what pushes parts out of their
    regions.
    """
    x0, y0, x1, y1 = rect
    if x0 < EDGE or y0 < EDGE or x1 > BOARD_W - EDGE or y1 > BOARD_H - EDGE:
        return "edge"
    for r, b in placed.items():
        if r == skip:
            continue
        bx0, by0, bx1, by1 = b
        if x0 < bx1 + GAP and bx0 < x1 + GAP and y0 < by1 + GAP and by0 < y1 + GAP:
            return b
    return None


def fits(rect, skip=None):
    return blocker(rect, skip) is None


def put(ref, cx0, cy0, rot):
    """Place `ref` with its courtyard's top-left at (cx0, cy0)."""
    dx0, dy0, dx1, dy1 = geom(ref, rot)
    rect = (cx0, cy0, cx0 + (dx1 - dx0), cy0 + (dy1 - dy0))
    if not fits(rect):
        return False
    placed[ref] = rect
    # The sidecar's x/y is the COURTYARD CENTRE, not the footprint origin:
    # the exporter puts the origin at `center - rot(courtyard_offset)`.
    # Writing the origin here put every asymmetric part one offset away from
    # where it was designed to be -- 10.11 mm for the ESP32, 24.12 mm for a
    # rotated 1x20 header. Writing the *packed* rectangle's centre is wrong
    # too, by a smaller amount: that rectangle is the union of courtyard,
    # silkscreen and keepouts, and `courtyard_rect` reads none of the last
    # two. So derive the origin from the packed rectangle, then re-apply the
    # exporter's own offset to it.
    d = parts.get(refs[ref]) or {}
    ox = cx0 - dx0
    oy = cy0 - dy0
    cdx, cdy = rot_offset(*crtyd_center(d.get("kicad_footprint")), rot)
    chosen[ref] = (round(ox + cdx, 3), round(oy + cdy, 3), rot)
    return True


def area(ref, rot=0):
    dx0, dy0, dx1, dy1 = geom(ref, rot)
    return (dx1 - dx0) * (dy1 - dy0)


# 1. the hard anchors
for ref, (side_x, side_y) in HARD_CORNER.items():
    if ref not in refs:
        continue
    dx0, dy0, dx1, dy1 = geom(ref, FIXED_ROT.get(ref, 0))
    w, h = dx1 - dx0, dy1 - dy0
    cx0 = EDGE if side_x == "left" else BOARD_W - EDGE - w
    cy0 = EDGE if side_y == "top" else BOARD_H - EDGE - h
    if not put(ref, round(cx0, 3), round(cy0, 3), FIXED_ROT.get(ref, 0)):
        print("HARD ANCHOR FAILED:", ref)
for ref, (cx0, cy0) in HARD.items():
    if ref not in refs:
        continue
    if not put(ref, cx0, cy0, FIXED_ROT.get(ref, 0)):
        print("HARD ANCHOR FAILED:", ref)

# 2. everything else, group by group, largest first inside its region
order = sorted(
    (r for r in refs if r not in chosen),
    key=lambda r: -area(r, FIXED_ROT.get(r, 0)),
)
unplaced = []
for ref in order:
    rot = FIXED_ROT.get(ref, 0)
    grp = group_of.get(ref) or "MECHANICAL"
    rx0, ry0, rx1, ry1 = REGIONS.get(grp, (EDGE, EDGE, BOARD_W - EDGE, BOARD_H - EDGE))
    dx0, dy0, dx1, dy1 = geom(ref, rot)
    w, h = dx1 - dx0, dy1 - dy0
    done = False
    # sweep the region, then the whole board if the region is full
    for (sx0, sy0, sx1, sy1) in ((rx0, ry0, rx1, ry1), (EDGE, EDGE, BOARD_W - EDGE, BOARD_H - EDGE)):
        y = sy0
        while y + h <= sy1 and not done:
            x = sx0
            while x + w <= sx1 and not done:
                x = round(x, 3)
                b = blocker((x, y, x + w, y + h))
                if b is None:
                    put(ref, round(x, 2), round(y, 2), rot)
                    done = True
                    break
                # Every x left of this obstacle's right edge is blocked by it
                # too -- `bx0 < x1 + GAP` only becomes truer as x grows -- so
                # the next candidate worth testing is the first grid step at
                # or past it. A board-edge rejection has no edge to skip to.
                step = x + GRID
                if isinstance(b, tuple):
                    step = max(step, math.ceil((b[2] + GAP) / GRID) * GRID)
                x = step
            y += GRID
        if done:
            break
    if not done:
        unplaced.append(ref)

print("placed %d of %d; unplaced: %s" % (len(chosen), len(refs), unplaced or "none"))

if os.environ.get("FLOORPLAN_DEBUG"):
    print("%-5s %-26s %-20s %s" % ("ref", "packed courtyard rect", "origin", "sidecar x/y, rot"))
    for ref in sorted(os.environ["FLOORPLAN_DEBUG"].split(",")):
        if ref not in placed:
            continue
        x0, y0, x1, y1 = placed[ref]
        cx, cy, rot = chosen[ref]
        d = parts.get(refs[ref]) or {}
        g = geom(ref, rot)
        o = (x0 - g[0], y0 - g[1])
        print("%-5s %-26s %-20s %s  crtyd_center=%s" % (
            ref, "%.2f,%.2f..%.2f,%.2f" % (x0, y0, x1, y1), "%.3f,%.3f" % o,
            "%.3f,%.3f r%d" % (cx, cy, rot),
            "%.3f,%.3f" % crtyd_center(d.get("kicad_footprint"))))

# 3. independent overlap audit on the true courtyards
bad = []
items = list(placed.items())
for i in range(len(items)):
    for j in range(i + 1, len(items)):
        a, (ax0, ay0, ax1, ay1) = items[i]
        b, (bx0, by0, bx1, by1) = items[j]
        if ax0 < bx1 and bx0 < ax1 and ay0 < by1 and by0 < ay1:
            bad.append((a, b))
print("true-courtyard overlaps:", len(bad), bad[:6])

xs = [r[2] for r in placed.values()]
ys = [r[3] for r in placed.values()]
x0s = [r[0] for r in placed.values()]
y0s = [r[1] for r in placed.values()]
print("extent: %.1f..%.1f x %.1f..%.1f mm" % (min(x0s), max(xs), min(y0s), max(ys)))

if unplaced or bad:
    sys.exit(1)

header = '''schema_version = 2

# A complete floorplan, authored because the placer cannot produce one: it
# refuses this board with E-SYNTH-PLACE-002 on whichever passive it reaches
# last, being greedy first-fit with no backtracking.
#
# Positions are computed in COURTYARD space from each footprint's real
# `F.CrtYd` polygon, not from the registry's declared body size. That
# distinction is the whole point of this file: the ESP32-WROOM-32's courtyard
# is 48.0 x 41.2 mm, not the 19.0 x 17.4 mm body, because the footprint draws
# the antenna keepout on the courtyard layer; and a 1x20 header's courtyard
# runs y -1.78 to +50.03 from its origin, so the origin is nowhere near the
# centre. An earlier floorplan that used declared body sizes produced 46
# courtyard overlaps in `kicad-cli pcb drc`.
#
# Each group gets a region and its parts are packed inside it largest-first,
# every candidate tested as a true courtyard rectangle against every courtyard
# already placed, with a %.1f mm gap. The value written here is the COURTYARD
# CENTRE, which is what the sidecar's x/y means: the exporter puts the
# footprint origin at `x/y - rot(courtyard_offset)`. And "courtyard" there is
# specifically the `F.CrtYd` rectangle that `courtyard_rect` reads from
# `fp_rect` and `fp_line` -- not the wider rectangle packed against here,
# which also takes in silkscreen, arcs and the keepouts a card socket carries.
# So the origin is derived from the packed rectangle and the exporter's own
# offset is re-applied to it.
#
# The board reads left to right as it works: 12 V entry and the rails down the
# left edge, the module in the middle with a header above and below, and the
# interfaces on the right.
''' % GAP

lines = [header]
by_group = {}
for ref, (x, y, rot) in chosen.items():
    by_group.setdefault(group_of.get(ref) or "MECHANICAL", []).append((ref, x, y, rot))
for grp in sorted(by_group):
    lines.append("# --- %s %s" % (grp, "-" * max(0, 60 - len(grp))))
    for ref, x, y, rot in sorted(by_group[grp]):
        lines.append("[components.%s]\nx = %.2f\ny = %.2f\nrotation = %d\n" % (ref, x, y, rot))

open(OUT, "w").write("\n".join(lines))
print("wrote", OUT, "with", len(chosen), "entries")
