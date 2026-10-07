// SPDX-License-Identifier: Apache-2.0

//! Uniform-bucket spatial index over axis-aligned rectangles.
//!
//! The maze's clearance checks ask "is any foreign pad / courtyard within
//! distance d of this step?" for every neighbour of every expanded cell. A
//! linear scan makes each expansion cost O(pads), which dominated routing
//! time on boards with a few hundred pads. The index answers the same
//! question by visiting only the buckets the inflated query touches.
//!
//! Exactness: a rectangle within distance `d` of a query box intersects the
//! query box inflated by `d`, so it is registered in at least one bucket the
//! inflated query overlaps. Callers still run their exact predicate on every
//! candidate, so answers are identical to the linear scan; only the set of
//! rectangles examined shrinks. Rectangles spanning several buckets may be
//! visited more than once, which is harmless for `any`-style predicates.

use synth_geometry::Rect;

/// Bucket edge length. Pads are sub-millimetre to a few millimetres and the
/// clearance envelopes queried are under a millimetre, so 1 mm keeps each
/// query to a handful of buckets with few candidates per bucket.
const BUCKET_NM: i64 = 1_000_000;

pub(crate) struct RectIndex {
    origin_x: i64,
    origin_y: i64,
    cols: usize,
    rows: usize,
    buckets: Vec<Vec<u32>>,
}

impl RectIndex {
    /// Index `rects`; entry `i` of the input is reported as `i` by queries.
    pub(crate) fn new(rects: impl Iterator<Item = Rect> + Clone) -> Self {
        let mut bounds: Option<(i64, i64, i64, i64)> = None;
        for r in rects.clone() {
            bounds = Some(bounds.map_or(
                (r.min.x_nm, r.min.y_nm, r.max.x_nm, r.max.y_nm),
                |(x0, y0, x1, y1)| {
                    (
                        x0.min(r.min.x_nm),
                        y0.min(r.min.y_nm),
                        x1.max(r.max.x_nm),
                        y1.max(r.max.y_nm),
                    )
                },
            ));
        }
        let (x0, y0, x1, y1) = bounds.unwrap_or((0, 0, 0, 0));
        let cols = usize::try_from((x1 - x0) / BUCKET_NM + 1).unwrap_or(1);
        let rows = usize::try_from((y1 - y0) / BUCKET_NM + 1).unwrap_or(1);
        let mut index = Self {
            origin_x: x0,
            origin_y: y0,
            cols,
            rows,
            buckets: vec![Vec::new(); cols * rows],
        };
        for (i, r) in rects.enumerate() {
            let (c0, r0, c1, r1) = index.bucket_span(r);
            for row in r0..=r1 {
                for col in c0..=c1 {
                    index.buckets[row * cols + col].push(i as u32);
                }
            }
        }
        index
    }

    /// Inclusive bucket range covering `r`, clamped to the indexed area.
    fn bucket_span(&self, r: Rect) -> (usize, usize, usize, usize) {
        let clamp = |v: i64, origin: i64, n: usize| -> usize {
            usize::try_from(((v - origin) / BUCKET_NM).max(0))
                .unwrap_or(0)
                .min(n - 1)
        };
        (
            clamp(r.min.x_nm, self.origin_x, self.cols),
            clamp(r.min.y_nm, self.origin_y, self.rows),
            clamp(r.max.x_nm, self.origin_x, self.cols),
            clamp(r.max.y_nm, self.origin_y, self.rows),
        )
    }

    /// True when `pred` holds for any indexed rectangle that could lie within
    /// `reach` of the box spanned by `min`..`max`.
    pub(crate) fn any_near(
        &self,
        min: (i64, i64),
        max: (i64, i64),
        reach: i64,
        mut pred: impl FnMut(usize) -> bool,
    ) -> bool {
        let query = Rect::new(
            synth_geometry::Point::new(min.0 - reach, min.1 - reach),
            synth_geometry::Point::new(max.0 + reach, max.1 + reach),
        );
        let (c0, r0, c1, r1) = self.bucket_span(query);
        for row in r0..=r1 {
            for col in c0..=c1 {
                if self.buckets[row * self.cols + col]
                    .iter()
                    .any(|&i| pred(i as usize))
                {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::RectIndex;
    use synth_geometry::{Point, Rect};

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Rect {
        Rect::new(Point::new(x0, y0), Point::new(x1, y1))
    }

    fn dist_sq(r: Rect, x: i64, y: i64) -> i64 {
        let dx = (r.min.x_nm - x).max(0).max(x - r.max.x_nm);
        let dy = (r.min.y_nm - y).max(0).max(y - r.max.y_nm);
        dx * dx + dy * dy
    }

    #[test]
    fn any_near_matches_linear_scan() {
        // Mixed sizes, including rects spanning several buckets and one far
        // outside the others, queried on and off the indexed area.
        let rects: Vec<Rect> = (0..40_i64)
            .map(|i| {
                let x = (i * 7_919_000) % 30_000_000;
                let y = (i * 3_571_000) % 20_000_000;
                let w = 200_000 + (i % 5) * 900_000;
                rect(x, y, x + w, y + w / 2)
            })
            .chain(std::iter::once(rect(
                90_000_000, 90_000_000, 91_000_000, 91_000_000,
            )))
            .collect();
        let index = RectIndex::new(rects.iter().copied());
        let reach = 450_000;
        for qx in (-2_000_000..95_000_000).step_by(1_337_000) {
            for qy in (-2_000_000..95_000_000).step_by(1_733_000) {
                let near = |r: &Rect| dist_sq(*r, qx, qy) < reach * reach;
                let expected = rects.iter().any(near);
                let got = index.any_near((qx, qy), (qx, qy), reach, |i| near(&rects[i]));
                assert_eq!(got, expected, "query ({qx}, {qy})");
            }
        }
    }
}
