use std::collections::HashMap;
use eframe::egui::{pos2, Pos2, Vec2};
use crate::shapes::shape::apply_transform;
use crate::types::{Line, ShapePatternData};
use super::dimension;

pub struct FractalResult {
    pub points: Vec<Pos2>,
    pub lines: Vec<Line>,
    pub dimension: f32,
    pub point_scale: Vec<f32>,
    pub box_counting: Option<dimension::BoxCountingResult>,
}

pub struct FractalConfig<'a> {
    pub get_points: &'a dyn Fn(Pos2, f32, f32) -> Vec<Pos2>,
    pub get_lines: &'a dyn Fn(Pos2, f32, f32) -> Vec<[Pos2; 2]>,
    pub pattern: &'a [ShapePatternData],
    pub initial: &'a [ShapePatternData],
    pub iterations: usize,
    pub regroup: bool,
    pub display_parent: bool,
    pub delta_radius: f32,
}

pub fn generate_fractal(config: &FractalConfig<'_>) -> FractalResult {
    let FractalConfig {
        get_points, get_lines, pattern, initial,
        iterations, regroup, display_parent, delta_radius,
    } = config;

    let mut current = initial.to_vec();
    let mut all_shapes: Vec<ShapePatternData> = Vec::new();

    for _ in 0..*iterations {
        let mut children = Vec::new();
        for parent in &current {
            for pat in *pattern {
                let child_scale = parent.scale * (1.0 / pat.scale);
                let child_rotate = parent.rotate + pat.rotate;
                let transformed = apply_transform(
                    pat.translate,
                    Pos2::ZERO,
                    parent.rotate,
                    Vec2::new(parent.scale, parent.scale),
                );
                let child_translate = pos2(
                    parent.translate.x + transformed.x,
                    parent.translate.y + transformed.y,
                );
                children.push(ShapePatternData {
                    translate: child_translate,
                    rotate: child_rotate,
                    scale: child_scale,
                });
            }
        }
        if *display_parent {
            all_shapes.extend(current);
        }
        current = children;
    }
    all_shapes.extend(current);

    let mut final_points: Vec<Pos2> = Vec::new();
    let mut final_lines: Vec<Line> = Vec::new();
    let mut final_point_scale: Vec<f32> = Vec::new();
    let mut point_map: HashMap<u64, usize> = HashMap::new();

    for s in &all_shapes {
        let sp = get_points(s.translate, s.rotate, s.scale);
        let sl = get_lines(s.translate, s.rotate, s.scale);

        for &p in &sp {
            find_or_add_point(&mut final_points, &mut final_point_scale, &mut point_map, p, s.scale, *regroup);
        }
        for seg in &sl {
            let a_idx = find_or_add_point(&mut final_points, &mut final_point_scale, &mut point_map, seg[0], s.scale, *regroup);
            let b_idx = find_or_add_point(&mut final_points, &mut final_point_scale, &mut point_map, seg[1], s.scale, *regroup);
            final_lines.push([a_idx, b_idx]);
        }
    }

    let dimension = if !pattern.is_empty() && pattern[0].scale > 1.0 {
        let n = pattern.len() as f32;
        let s = pattern[0].scale;
        n.log10() / s.log10()
    } else {
        0.0
    };

    if *delta_radius > 0.0 {
        use rand::Rng;
        let mut rng = rand::rng();
        for p in &mut final_points {
            let angle = rng.random::<f32>() * std::f32::consts::TAU;
            let r = *delta_radius * rng.random::<f32>().sqrt();
            p.x += r * angle.cos();
            p.y += r * angle.sin();
        }
    }

    let box_counting = dimension::box_counting(&final_points, *iterations);

    FractalResult {
        points: final_points,
        lines: final_lines,
        point_scale: final_point_scale,
        dimension,
        box_counting,
    }
}

fn point_key(p: Pos2) -> u64 {
    u64::from(p.x.to_bits()) ^ (u64::from(p.y.to_bits()) << 32)
}

fn find_or_add_point(
    points: &mut Vec<Pos2>,
    scales: &mut Vec<f32>,
    map: &mut HashMap<u64, usize>,
    p: Pos2,
    scale: f32,
    regroup: bool,
) -> usize {
    let key = point_key(p);
    if let Some(&idx) = map.get(&key) {
        scales[idx] = scales[idx].max(scale);
        return idx;
    }
    if regroup {
        let tolerance = 0.001;
        let tolerance_sq = tolerance * tolerance;
        for (i, &pt) in points.iter().enumerate() {
            let dx = pt.x - p.x;
            let dy = pt.y - p.y;
            if dx * dx + dy * dy < tolerance_sq {
                scales[i] = scales[i].max(scale);
                map.insert(key, i);
                return i;
            }
        }
    }
    let idx = points.len();
    points.push(p);
    scales.push(scale);
    map.insert(key, idx);
    idx
}
