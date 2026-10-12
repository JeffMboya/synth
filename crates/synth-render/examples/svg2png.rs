// SPDX-License-Identifier: Apache-2.0

//! Rasterize an SVG with the same pinned renderer the visual-feedback loop
//! uses, so a board picture does not depend on whichever librsvg the
//! machine happens to have.
//!
//! ```text
//! svg2png <in.svg> <out.png> [width_px]
//! ```

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output, rest @ ..] = args.as_slice() else {
        eprintln!("usage: svg2png <in.svg> <out.png> [width_px]");
        return ExitCode::from(2);
    };
    let width_px = match rest.first().map(|w| w.parse::<u32>()) {
        Some(Ok(width)) => width,
        Some(Err(e)) => {
            eprintln!("svg2png: width_px: {e}");
            return ExitCode::from(2);
        }
        None => 2400,
    };

    let svg = match std::fs::read(input) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("svg2png: {input}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let rendered = match synth_render::svg_to_png(&svg, width_px) {
        Ok(rendered) => rendered,
        Err(e) => {
            eprintln!("svg2png: {input}: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::write(output, &rendered.png) {
        eprintln!("svg2png: {output}: {e}");
        return ExitCode::FAILURE;
    }
    println!("{output} {}x{}", rendered.width_px, rendered.height_px);
    ExitCode::SUCCESS
}
