// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use flatten::
    euler::EulerParams
;
use parley::{FontContext, Layout, LayoutContext};
use vello::{
    Scene,
    kurbo::{
        Affine, BezPath, Circle, Line, PathEl, Point,
        Rect, Shape, Stroke, Vec2, fit_to_bezpath_opt,
    },
    peniko::{Brush, Color, Fill},
};

use crate::{
    log_aesthetic::{self},
    text,
    tile::{Tile, Tiles},
};

enum Progress {
    Original,
    Flat,
    FlatGrid,
    RPath,
    StripGroup,
    Strips,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Phase {
    Initial,
    Sorted,
    Grouped,
    GroupedAlpha,
    StripGroup,
    Strips,
}

pub struct Anims {
    dens_curve: BezPath,
    g_path: BezPath,
    r_path: BezPath,
    r_path_flat: BezPath,
    #[allow(unused)]
    font_context: FontContext,
    #[allow(unused)]
    lcx: LayoutContext<Brush>,
    title_layout: Layout<Brush>,
    strong_layout: Layout<Brush>,
    weak_layout: Layout<Brush>,
    raph_layout: Layout<Brush>,
    arman_layout: Layout<Brush>,
    arclen_layout: Layout<Brush>,
    subdiv_layout: Layout<Brush>,
    es_layout: Layout<Brush>,
    ev_layout: Layout<Brush>,
    spiral: BezPath,
    segments_done: usize,
    tiles: Vec<RenderedTile>,
    this_seg_start: usize,
    phase: Phase,
    perm: Vec<usize>,
    inv_perm: Vec<usize>,
    progress: Progress,
}

struct RenderedTile {
    text: Layout<Brush>,
    line: Line,
    scale: f64,
    position: Point,
    mid_scale: f64,
    mid_pos: Point,
    target_scale: f64,
    target_pos: Point,
    xy: u32,
    alphas: [u8; 16],
}

fn timed(t: &mut f64, duration: f64) -> bool {
    if *t < duration {
        true
    } else {
        *t -= duration;
        false
    }
}

const STROKE_LEN: f64 = 4.5;

const G_PATH_STR: &str = "M470 295h-83c14 32 19 55 19 84c0 50 -15 85 -48 113c-31 27 -71 42 -108 42c-2 0 -21 -2 -57 -5c-27 8 -60 43 -60 63c0 16 24 25 78 27l129 6c74 3 121 45 121 107c0 38 -17 70 -55 101c-52 42 -130 68 -205 68c-96 0 -173 -44 -173 -97c0 -37 26 -70 98 -122
c-42 -20 -53 -31 -53 -53c0 -17 6 -28 27 -50c3 -4 15 -15 36 -35l26 -24c-67 -33 -93 -71 -93 -134c0 -92 74 -163 167 -163c26 0 53 5 80 15l22 8c20 7 34 10 55 10h77v39zM147 685c-40 48 -49 63 -49 86c0 44 57 73 146 73c113 0 189 -39 189 -97c0 -36 -33 -49 -124 -49
c-49 0 -128 -6 -162 -13zM152 345v3c0 96 41 161 103 161c46 0 74 -35 74 -91c0 -40 -11 -85 -30 -120c-16 -30 -42 -47 -73 -47c-46 0 -74 35 -74 94z";

const R_PATH_STR: &str = "M629 978h-239v533h-300v-1456h541q258 0 398 115t140 325q0 149 -64.5 248.5t-195.5 158.5l315 595v14h-322zM390 735h242q113 0 175 -57.5t62 -158.5q0 -103 -58.5 -162t-179.5 -59h-241v437z";

const R_CELL: f64 = 100.0;
const R_WIDTH: usize = 12;
const R_HEIGHT: usize = 15;
const R_ORIGIN: Point = Point::new(40.0, 40.0);

const RIGHT_GRID_X0: f64 = 1350.;
const RIGHT_GRID_N_WIDE: usize = 8;
const RIGHT_GRID_CELL: f64 = 80.0;

const SUBDIV_Y: f64 = 1400.;
const SUBDIV_X: f64 = 1000.;

// https://iamkate.com/data/12-bit-rainbow/
const RAINBOW_PALETTE: [Color; 12] = [
    Color::rgb8(0x88, 0x11, 0x66),
    Color::rgb8(0xaa, 0x33, 0x55),
    Color::rgb8(0xcc, 0x66, 0x66),
    Color::rgb8(0xee, 0x99, 0x44),
    Color::rgb8(0xee, 0xdd, 0x00),
    Color::rgb8(0x99, 0xdd, 0x55),
    Color::rgb8(0x44, 0xdd, 0x88),
    Color::rgb8(0x22, 0xcc, 0xbb),
    Color::rgb8(0x00, 0xbb, 0xcc),
    Color::rgb8(0x00, 0x99, 0xcc),
    Color::rgb8(0x33, 0x66, 0xbb),
    Color::rgb8(0x66, 0x33, 0x99),
];

fn label(
    font_context: &mut FontContext,
    lcx: &mut LayoutContext<Brush>,
    text: &str,
    size: f32,
) -> Layout<Brush> {
    let mut layout_builder = lcx.ranged_builder(font_context, text, 1.0);
    layout_builder.push_default(&parley::style::StyleProperty::Brush(Brush::Solid(
        Color::rgb8(0, 0, 0),
    )));
    layout_builder.push_default(&parley::style::StyleProperty::FontSize(size));
    let mut layout = layout_builder.build();
    layout.break_all_lines(Some(1800.0), parley::layout::Alignment::Start);
    layout
}

impl Anims {
    pub fn new() -> Self {
        let dens_curve = density_curve();
        let g_path = BezPath::from_svg(G_PATH_STR).unwrap();
        let r_path = BezPath::from_svg(R_PATH_STR).unwrap();
        let mut r_path_flat = BezPath::new();
        vello::kurbo::flatten(&r_path, 3.0, |el| r_path_flat.push(el));
        let mut font_context = FontContext::default();
        let mut lcx = LayoutContext::new();
        let mut layout_builder =
            lcx.ranged_builder(&mut font_context, "GPU-Friendly Stroke Expansion", 1.0);
        layout_builder.push_default(&parley::style::StyleProperty::Brush(Brush::Solid(
            Color::rgb8(0, 0, 0),
        )));
        layout_builder.push_default(&parley::style::StyleProperty::FontSize(200.0));
        layout_builder.push_default(&parley::style::StyleProperty::LineHeight(1.2));
        let mut title_layout = layout_builder.build();
        title_layout.break_all_lines(Some(1800.0), parley::layout::Alignment::Start);

        let raph_layout = label(&mut font_context, &mut lcx, "Raph Levien", 100.0);
        let arman_layout = label(&mut font_context, &mut lcx, "Arman Uguray", 100.0);
        let strong_layout = label(&mut font_context, &mut lcx, "strongly correct", 50.0);
        let weak_layout = label(&mut font_context, &mut lcx, "weakly correct", 50.0);
        let arclen_layout = label(&mut font_context, &mut lcx, "arc length", 40.0);
        let subdiv_layout = label(&mut font_context, &mut lcx, "subdivision density", 40.0);
        let es_layout = label(&mut font_context, &mut lcx, "Euler spiral", 40.0);
        let ev_layout = label(&mut font_context, &mut lcx, "Evolute", 40.0);
        let spiral = mk_spiral();
        Anims {
            dens_curve,
            g_path,
            r_path,
            r_path_flat,
            font_context,
            lcx,
            title_layout,
            raph_layout,
            arman_layout,
            strong_layout,
            weak_layout,
            arclen_layout,
            subdiv_layout,
            es_layout,
            ev_layout,
            spiral,
            segments_done: 0,
            tiles: vec![],
            this_seg_start: 0,
            phase: Phase::Initial,
            perm: vec![],
            inv_perm: vec![],
            progress: Progress::Original,
        }
    }

    pub fn render(&mut self, scene: &mut Scene, mut t: f64, advance: bool) {
        const LEAD_IN: f64 = 5.0;
        // if timed(&mut t, 10.0) {
        //    self.end_card(scene);
        //    return
        // }
        match self.progress {
            Progress::Original => {
                self.draw_r_orig(scene);
                if advance {
                    self.progress = Progress::Flat;
                }
            }
            Progress::Flat => {
                self.draw_r_flat(scene, false);
                if advance {
                    self.progress = Progress::FlatGrid;
                }
            }
            Progress::FlatGrid => {
                self.draw_r_flat(scene, true);
                if advance {
                    self.progress = Progress::RPath;
                }
            }
            Progress::RPath => {

                self.draw_r_path(scene, t, advance);
                if advance {
                    match self.phase {
                        Phase::Initial => {
                            self.phase = Phase::Sorted;
                        }
                        Phase::Sorted => {
                            self.phase = Phase::Grouped;
                        }
                        Phase::Grouped => {
                            self.phase = Phase::GroupedAlpha;
                        }
                        Phase::GroupedAlpha => {
                            self.progress = Progress::StripGroup;
                        }
                        _ => (),
                    }
                }
            }
            Progress::StripGroup => {
                self.draw_strip_group(scene);
                if advance {
                    self.progress = Progress::Strips;
                }

            }
            Progress::Strips => {
                self.draw_strips(scene);

            }
        }
    }

    #[allow(unused)]
    fn end_card(&self, scene: &mut Scene) {
        // placeholder for actual end card
        let color = Color::rgb(0.1, 0.1, 0.8);
        let rect = Rect::new(100., 100., 1000., 1000.);
        scene.fill(
            vello::peniko::Fill::NonZero,
            Affine::IDENTITY,
            &color,
            None,
            &rect,
        );
    }

    fn text_card(&mut self, scene: &mut Scene, t: f64) {
        let a = Affine::translate((200., 200.));
        const SPEED: f64 = 40.;
        const WIDTH: f64 = 10.;
        let max_w = t * SPEED;
        let n = 1 + (max_w / WIDTH).floor() as usize;
        for i in 0..n {
            let w = max_w - WIDTH * i as f64;
            let stroke = Stroke::new(w)
                .with_join(vello::kurbo::Join::Round)
                .with_miter_limit(2.0);
            let eo = (i % 2) as f64;
            let s = (0.6 + 0.4 * eo) * (1.0 - 0.5 * ((n - i) as f64 * -0.2).exp());
            let brush = Color::rgb(1. * s, 0.8 * s, 0.3 * s);
            text::render_text_stroked(scene, a, &self.title_layout, &stroke, &brush);
        }
        text::render_text(scene, a, &self.title_layout);
        text::render_text(scene, Affine::translate((200., 1180.)), &self.raph_layout);
        text::render_text(scene, Affine::translate((200., 1300.)), &self.arman_layout);
    }

    fn draw_r_orig(&self, scene: &mut Scene) {
        let stroke = Stroke::new(6.0);
        let stroke_color = Color::rgb(0.2, 0.2, 0.4);
        scene.stroke(&stroke, Affine::IDENTITY, stroke_color, None, &self.r_path);
        self.path_dots(scene, &self.r_path);
    }

    fn draw_r_flat(&self, scene: &mut Scene, show_grid: bool) {
        if show_grid {
            self.grid_for_r(scene);
        }
        let stroke = Stroke::new(6.0);
        let stroke_color = Color::rgb(0.2, 0.2, 0.4);
        scene.stroke(
            &stroke,
            Affine::IDENTITY,
            stroke_color,
            None,
            &self.r_path_flat,
        );
        self.path_dots(scene, &self.r_path_flat);
    }

    fn draw_r_path(&mut self, scene: &mut Scene, t: f64, advance: bool) {
        self.grid_for_r(scene);
        let stroke = Stroke::new(6.0);
        let stroke_color = Color::rgb(0.2, 0.2, 0.4);
        scene.stroke(
            &stroke,
            Affine::IDENTITY,
            stroke_color,
            None,
            &self.r_path_flat,
        );
        let segs = self.r_path_flat.segments().collect::<Vec<_>>();
        const T_PER_SEG: f64 = 0.5;
        let seg_f = t / T_PER_SEG;
        let seg_int = seg_f.floor() as usize;
        if self.phase == Phase::Initial && seg_int < segs.len() {
            let emph_stroke = Stroke::new(10.0); // TODO: ease-in/out
            let emph_color = Color::rgb(0.1, 0.1, 0.3);
            let hi_stroke = Stroke::new(50.0);
            let hi_color = Color::rgba(0.9, 0.9, 0.0, 0.5);
            // make first segment more interesting
            let mut seg_ix = seg_int;
            if seg_ix < 23 {
                seg_ix = (seg_ix + 5) % 23;
                // maybe permute segments on inner contour?
            }
            let path = segs[seg_ix].to_path(1e-3);
            let mut lines = vec![];
            let to_tiles = Affine::scale(4. / R_CELL) * Affine::translate(-R_ORIGIN.to_vec2());
            crate::flatten::from_kurbo(&path, to_tiles, &mut lines);
            //println!("{lines:?}");
            let mut tiles = Tiles::new();
            tiles.make_tiles(&lines, R_WIDTH as u16 * 4, R_HEIGHT as u16 * 4);
            //println!("{tiles:x?}");
            if seg_int == self.segments_done {
                self.segments_done += 1;
                self.this_seg_start = self.tiles.len();
                let mut min_x = tiles.tile_buf[0].x;
                let mut max_x = min_x;
                for tile in &tiles.tile_buf[1..] {
                    min_x = min_x.min(tile.x);
                    max_x = max_x.max(tile.x);
                }
                //let offset = Vec2::new(1270. - R_CELL * min_x as f64, 5.);
                let offset = Vec2::new(105. + R_CELL * (max_x - min_x) as f64, 5.);
                for tile in &tiles.tile_buf {
                    let mut rendered = render_tile(tile, &lines, offset, &self.r_path_flat);
                    let ix = self.tiles.len();
                    let grid_x = RIGHT_GRID_X0 + RIGHT_GRID_CELL * (ix % RIGHT_GRID_N_WIDE) as f64;
                    let grid_y = 20. + RIGHT_GRID_CELL * (ix / RIGHT_GRID_N_WIDE) as f64;
                    rendered.target_pos = Point::new(grid_x, grid_y);
                    self.tiles.push(rendered);
                }
                println!("{}", self.tiles.len());
            }
            scene.stroke(&hi_stroke, Affine::IDENTITY, hi_color, None, &path);
            self.path_dots(scene, &path);
            scene.stroke(&emph_stroke, Affine::IDENTITY, emph_color, None, &path);
        } else {
            self.this_seg_start = self.tiles.len();
        }
        let mut alpha_thresh = 0;
        if advance {
            match self.phase {
                Phase::Initial => {
                    self.make_perm();
                    for (i, ix) in self.perm.iter().enumerate() {
                        let grid_x = RIGHT_GRID_X0 + RIGHT_GRID_CELL * (i % RIGHT_GRID_N_WIDE) as f64;
                        let grid_y = 20. + RIGHT_GRID_CELL * (i / RIGHT_GRID_N_WIDE) as f64;
                        self.tiles[*ix].target_pos = Point::new(grid_x, grid_y);
                    }
                }
                Phase::Sorted => {
                    self.group_by_xy();
                }
                Phase::Grouped => {
                    self.inv_perm = vec![0; self.perm.len()];
                    for (i, p) in self.perm.iter().enumerate() {
                        self.inv_perm[*p] = i;
                    }        
                }
                _ => (),
            }
        }
        if self.phase == Phase::GroupedAlpha {
            alpha_thresh = (50.0 * t) as usize;
        }
        self.draw_tiles_and_tick(scene, alpha_thresh);
    }

    fn group_by_xy(&mut self) {
        let mut last = self.perm[0];
        for i in 1..self.tiles.len() {
            let this = self.perm[i];
            if self.tiles[last].xy == self.tiles[this].xy {
                self.tiles[this].target_pos = self.tiles[last].target_pos;
            } else {
                last = this;
            }
        }
    }

    fn draw_strip_group(&mut self, scene: &mut Scene) {
        if self.phase < Phase::StripGroup {
            self.group_by_strip();
            self.phase = Phase::StripGroup;
        }
        self.draw_tiles_and_tick(scene, 1000);
    }

    fn group_by_strip(&mut self) {
        let mut i = 0;
        let mut x = 0;
        let mut y = 0;
        while i < self.tiles.len() {
            let mut xy = self.tiles[self.perm[i]].xy;
            let mut end = i + 1;
            let mut strip_width = 1;
            while end < self.tiles.len() {
                let diff = self.tiles[self.perm[end]].xy - xy;
                if diff > 1 {
                    break;
                }
                if diff == 1 {
                    strip_width += 1;
                    xy = self.tiles[self.perm[end]].xy;
                }
                end += 1;
            }
            if x > 0 && x + 1 + strip_width > 16 {
                x = 0;
                y += 1;
            }
            let mut xy = self.tiles[self.perm[i]].xy;
            for j in i..end {
                if self.tiles[self.perm[j]].xy != xy {
                    x += 1;
                    xy = self.tiles[self.perm[j]].xy;
                }
                self.tiles[self.perm[j]].target_pos = Point::new(
                    500. + (R_CELL - 5.0) * x as f64,
                    50. + (R_CELL + 40.0) * y as f64,
                );
                self.tiles[self.perm[j]].target_scale = 0.9;
            }
            x += 2;
            i = end;
        }
    }

    fn make_perm(&mut self) {
        let mut perm = (0..self.tiles.len()).collect::<Vec<_>>();
        perm.sort_by(|a, b| self.tiles[*a].xy.cmp(&self.tiles[*b].xy));
        self.perm = perm;
    }

    fn path_dots(&self, scene: &mut Scene, path: &BezPath) {
        let dot_color = Color::rgb(0.0, 0.0, 0.5);
        for el in path.elements() {
            let p = match el {
                PathEl::MoveTo(p) => *p,
                PathEl::LineTo(p) => *p,
                PathEl::QuadTo(_, p) => *p,
                PathEl::CurveTo(_, _, p) => *p,
                _ => continue,
            };
            let c = Circle::new(p, 8.0);
            scene.fill(Fill::NonZero, Affine::IDENTITY, dot_color, None, &c);
        }
    }

    fn draw_tiles_and_tick(&mut self, scene: &mut Scene, alpha_thresh: usize) {
        for (i, tile) in self.tiles.iter().enumerate() {
            if self.phase >= Phase::GroupedAlpha && self.inv_perm[i] < alpha_thresh {
                tile.draw_alphas(scene);
            } else {
                tile.draw_bg(scene);
            }
            tile.draw_layer1(scene);
            if self.phase < Phase::Grouped {
                tile.draw_layer2(scene);
            }
        }

        for (i, tile) in self.tiles.iter_mut().enumerate() {
            if self.phase >= Phase::Grouped {
                tile.draw_layer2(scene);
            }
            if i < self.this_seg_start {
                tile.tick_animation();
            }
        }
    }

    fn grid_for_r(&self, scene: &mut Scene) {
        let grid_stroke = Stroke::new(2.0);
        let grid_color = Color::rgb(0.2, 0.2, 0.2);
        for i in 0..=R_HEIGHT {
            let p0 = R_ORIGIN + Vec2::new(0., R_CELL * i as f64);
            let p1 = p0 + Vec2::new(R_CELL * R_WIDTH as f64, 0.);
            scene.stroke(
                &grid_stroke,
                Affine::IDENTITY,
                grid_color,
                None,
                &Line::new(p0, p1),
            );
        }
        for i in 0..=R_WIDTH {
            let p0 = R_ORIGIN + Vec2::new(R_CELL * i as f64, 0.);
            let p1 = p0 + Vec2::new(0., R_CELL * R_HEIGHT as f64);
            scene.stroke(
                &grid_stroke,
                Affine::IDENTITY,
                grid_color,
                None,
                &Line::new(p0, p1),
            );
        }
    }

    fn draw_strips(&mut self, scene: &mut Scene) {
        if self.phase < Phase::Strips {
            for tile in &mut self.tiles {
                tile.target_pos = R_ORIGIN
                    + Vec2::new(
                        R_CELL * (tile.xy & 0xffff) as f64 + 5.,
                        R_CELL * (tile.xy >> 16) as f64 + 5.,
                    );
                tile.target_scale = 0.9;
            }
            self.phase = Phase::Strips;
        }
        self.draw_tiles_and_tick(scene, 1000);
        // Next: both render and draw strip geometry
    }
}

fn density_curve() -> BezPath {
    const N: usize = 300;
    let mut path = BezPath::new();
    const SCALE: f64 = 300.0;
    let a = Affine::translate((SUBDIV_X, SUBDIV_Y)) * Affine::scale_non_uniform(SCALE, -SCALE);
    for i in 0..=N {
        let x = 3.0 * (i as f64 / N as f64 - 0.38);
        let y = (1.0 - x * x).abs().sqrt();
        let p = a * Point::new(x, y);
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path
}

const K1: f64 = 10.0;

const SPIRAL_Y: f64 = 600.;

fn mk_spiral() -> BezPath {
    let params = log_aesthetic::LogAestheticParams::new(1., -0.5 * K1, K1);
    let p0 = Point::new(500., SPIRAL_Y);
    let p1 = Point::new(1500., SPIRAL_Y);
    let lac = log_aesthetic::LogAestheticCurve::from_points_params(params, p0, p1);
    //let th = lac.sample_pt_deriv(0.0).1.atan2();
    //println!("{:?} {th}", lac.sample_pt_deriv(0.0));
    //println!("{:?}", lac.sample_pt_deriv(1.0));
    fit_to_bezpath_opt(&lac, 0.1)
}

// Originally the idea was to display a more complete Euler spiral over one
// or more of the Euler spiral segments, but I gave up on the idea, partly
// because I wasn't sure it would be clear visually, partly because the
// math to line them up wasn't trivial.
#[allow(unused)]
fn transform_for_es_params(euler_params: EulerParams) -> Affine {
    // TODO: retain the curve
    let params = log_aesthetic::LogAestheticParams::new(1., -0.5 * K1, K1);
    let p0 = Point::new(500., 1000.);
    let p1 = Point::new(1500., 1000.);
    let lac = log_aesthetic::LogAestheticCurve::from_points_params(params, p0, p1);
    let s_center = 0.5 + euler_params.k0 / euler_params.k1;
    todo!()
}

fn easing(t: f64) -> f64 {
    if t >= 1.0 { 1.0 } else { (3. - 2. * t) * t * t }
}

fn render_tile(
    tile: &Tile,
    lines: &[crate::flatten::Line],
    offset: Vec2,
    path: &BezPath,
) -> RenderedTile {
    let mut font_context = FontContext::default();
    let mut lcx = LayoutContext::new();
    let xy = ((tile.y as u32) << 16) | tile.x as u32;
    let text = format!("({}, {})", tile.x, tile.y);
    let layout = label(&mut font_context, &mut lcx, &text, 25.0);
    let src_line = lines[tile.line_idx() as usize];
    let tile_pos = Point::new(
        (tile.x * Tile::WIDTH) as f64,
        (tile.y * Tile::HEIGHT) as f64,
    );
    let mut x0 = src_line.p0.x - tile_pos.x as f32;
    let mut y0 = src_line.p0.y - tile_pos.y as f32;
    let mut x1 = src_line.p1.x - tile_pos.x as f32;
    let mut y1 = src_line.p1.y - tile_pos.y as f32;
    println!("before clip {x0},{y0} {x1},{y1}");
    if y1 < y0 {
        (x0, y0, x1, y1) = (x1, y1, x0, y0);
    }
    if y0 < 0.0 {
        x0 = x0 + (x1 - x0) * (0.0 - y0) / (y1 - y0);
        y0 = 0.0;
    }
    if y1 > Tile::HEIGHT as f32 {
        x1 = x0 + (x1 - x0) * (Tile::HEIGHT as f32 - y0) / (y1 - y0);
        y1 = Tile::HEIGHT as f32;
    }
    if x1 < x0 {
        (x0, y0, x1, y1) = (x1, y1, x0, y0);
    }
    if x0 < 0.0 {
        y0 = y0 + (y1 - y0) * (0.0 - x0) / (x1 - x0);
        x0 = 0.0;
    }
    if x1 > Tile::WIDTH as f32 {
        y1 = y0 + (y1 - y0) * (Tile::WIDTH as f32 - x0) / (x1 - x0);
        x1 = Tile::WIDTH as f32;
    }
    println!("{x0},{y0} {x1},{y1}");
    let f = |x| x as f64 * (R_CELL / Tile::WIDTH as f64);
    let line = Line::new((f(x0), f(y0)), (f(x1), f(y1)));
    let scale = 0.9;
    let dxy = (R_CELL / Tile::WIDTH as f64) * tile_pos.to_vec2();
    let position = R_ORIGIN + offset + dxy;
    let target_scale = 0.7;
    let alphas = core::array::from_fn(|i| {
        let mut area = 255.0f64;
        let dx = ((i % 4) as f64) * (R_CELL / Tile::WIDTH as f64);
        let dy = ((i / 4) as f64) * (R_CELL / Tile::HEIGHT as f64);
        let p = Point::new(dx, dy) + dxy + R_ORIGIN.to_vec2();
        for y in 0..4 {
            for x in 0..4 {
                let p2 = p + Vec2::new(
                    (x as f64 + 0.5) * (0.25 * R_CELL / Tile::WIDTH as f64),
                    (y as f64 + 0.5) * (0.25 * R_CELL / Tile::HEIGHT as f64),
                );
                if path.contains(p2) {
                    area -= 255.0 / 16.0;
                }
            }
        }
        area.round() as u8
    });
    // placeholder; will be set later
    RenderedTile {
        text: layout,
        line,
        scale,
        position,
        mid_scale: scale,
        mid_pos: position,
        target_scale,
        target_pos: position,
        xy,
        alphas,
    }
}

impl RenderedTile {
    fn draw_bg(&self, scene: &mut Scene) {
        let affine = Affine::translate(self.position.to_vec2()) * Affine::scale(self.scale);
        let r = Rect::new(0.0, 0.0, R_CELL, R_CELL);
        scene.fill(Fill::NonZero, affine, &Color::WHITE, None, &r);
    }

    fn draw_layer1(&self, scene: &mut Scene) {
        let affine = Affine::translate(self.position.to_vec2()) * Affine::scale(self.scale);
        let r = Rect::new(0.0, 0.0, R_CELL, R_CELL);
        let stroke_color = Color::rgb(0.2, 0.2, 0.2);
        let stroke = Stroke::new(2.0);
        scene.stroke(&stroke, affine, stroke_color, None, &r);
        text::render_text(scene, Affine::translate((10., 10.)) * affine, &self.text);
    }

    fn draw_layer2(&self, scene: &mut Scene) {
        let affine = Affine::translate(self.position.to_vec2()) * Affine::scale(self.scale);
        let stroke_color = Color::rgb(0.2, 0.2, 0.2);
        let stroke = Stroke::new(2.0);
        scene.stroke(&stroke, affine, stroke_color, None, &self.line);
    }

    fn draw_alphas(&self, scene: &mut Scene) {
        let affine = Affine::translate(self.position.to_vec2()) * Affine::scale(self.scale);
        let r = Rect::new(
            0.0,
            0.0,
            R_CELL / Tile::WIDTH as f64,
            R_CELL / Tile::HEIGHT as f64,
        );
        for y in 0..4 {
            for x in 0..4 {
                let alpha = self.alphas[y * 4 + x];
                let dxy = Affine::translate((
                    self.scale * x as f64 * (R_CELL / Tile::WIDTH as f64),
                    self.scale * y as f64 * (R_CELL / Tile::HEIGHT as f64),
                ));
                let color = Color::rgb8(alpha, alpha / 2 + 127, alpha);
                scene.fill(Fill::NonZero, dxy * affine, &color, None, &r);
            }
        }
    }

    fn tick_animation(&mut self) {
        const ANIM_LERP: f64 = 0.1;
        self.mid_pos += ANIM_LERP * (self.target_pos - self.mid_pos);
        self.mid_scale += ANIM_LERP * (self.target_scale - self.mid_scale);
        self.position += ANIM_LERP * (self.mid_pos - self.position);
        self.scale += ANIM_LERP * (self.mid_scale - self.scale);
    }
}
