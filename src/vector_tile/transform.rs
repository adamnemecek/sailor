use pathfinder_canvas::CanvasRenderingContext2D;
use crate::vector_tile::line_tesselator::tesselate_line2;
use core::ops::Range;
use crate::drawing::mesh::MeshBuilder;

use lyon::path::Path;
use lyon::math::*;
use lyon::tessellation::{
    FillTessellator,
    FillOptions,
};
use varint::ZigZag;

use crate::vector_tile::mod_Tile::GeomType;

use pathfinder_canvas::{Path2D};
use pathfinder_geometry::vector::{Vector2F, Vector2I};

#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub id: u32,
    pub indices_range: Range<u32>,
    pub features: Vec<(u32, Range<u32>)>,
}

fn area(path: &Path) -> f32 {
    let mut points = path.points().to_vec();
    points.push(points.first().expect("Path contains no points!").clone());
    let mut area = 0f32;
    for i in 0..points.len() - 1 {
        area += points[i].x * points[i + 1].y;
    }
    for i in 0..points.len() - 1 {
        area -= points[i + 1].x * points[i].y;
    }
    area + points[points.len() - 1].x * points[1].y - points[points.len() - 1].y * points[1].x
}

fn parse_one_to_path(geometry_type: GeomType, geometry: &Vec<u32>, cursor: &mut usize, gcursor: &mut Point, path: &mut Path2D) -> Path {
    let mut builder = Path::builder();

    while *cursor < geometry.len() {
        let value = geometry[*cursor];
        *cursor += 1;

        let count = value >> 3;
        match value & 0x07 {
            1 => {
                for _ in 0..count {
                    let dx = ZigZag::<i32>::zigzag(&geometry[*cursor]) as f32;
                    *cursor += 1;
                    let dy = ZigZag::<i32>::zigzag(&geometry[*cursor]) as f32;
                    *cursor += 1;
                    *gcursor += vector(dx, dy);
                    builder.move_to(*gcursor);
                    path.move_to(Vector2F::new(gcursor.x, gcursor.y));
                }
                match geometry_type {
                    GeomType::POINT => return builder.build(),
                    _ => {},
                }
            },
            2 => {
                for _ in 0..count {
                    let dx = ZigZag::<i32>::zigzag(&geometry[*cursor]) as f32;
                    *cursor += 1;
                    let dy = ZigZag::<i32>::zigzag(&geometry[*cursor]) as f32;
                    *cursor += 1;
                    *gcursor += vector(dx, dy);
                    builder.line_to(*gcursor);
                    path.line_to(Vector2F::new(gcursor.x, gcursor.y));
                }
                match geometry_type {
                    GeomType::POINT => panic!("This is a bug. Please report it."),
                    GeomType::LINESTRING => return builder.build(),
                    _ => {},
                }
            },
            7 => {
                path.close_path();
                builder.close();
                match geometry_type {
                    GeomType::POINT => panic!("This is a bug. Please report it."),
                    GeomType::LINESTRING => panic!("This is a bug. Please report it."),
                    GeomType::POLYGON => {},
                    _ => panic!("This is a bug. Please report it."),
                }
            },
            _ => {
                panic!("This is a bug. Please report it.");
            },
        }
    }
    match geometry_type {
        GeomType::POINT => panic!("This is a bug. Please report it."),
        GeomType::LINESTRING => panic!("This is a bug. Please report it."),
        GeomType::POLYGON => {
            let path = builder.build();
            if area(&path) < 0f32 {
                return path;
            } else {
                return path;
            }
        },
        _ => panic!("This is a bug. Please report it."),
    }
}

pub fn geometry_commands_to_drawable<'a, 'l>(
    builder: &'a mut MeshBuilder<'l>,
    geometry_type: GeomType,
    geometry: &Vec<u32>,
    extent: f32,
    z: u32,
    canvas: &mut CanvasRenderingContext2D,
) {
    let mut cursor = 0;
    let mut c = point(0f32, 0f32);

    if geometry_type == GeomType::POLYGON {
        while cursor < geometry.len() {
            
            let mut p = Path2D::new();
            let path = parse_one_to_path(geometry_type, geometry, &mut cursor, &mut c, &mut p);

            // Fill
            builder.set_current_extent(extent);
            let mut tessellator = FillTessellator::new();
            let _ = tessellator
                .tessellate_path(
                    &path,
                    &FillOptions::tolerance(0.0001).with_normals(true),
                    builder,
                ).map_err(|_e| { log::error!("Broken path."); });

            canvas.fill_path(p);
        }
    }

    if geometry_type == GeomType::LINESTRING {
        while cursor < geometry.len() {
            let mut p = Path2D::new();
            let path = parse_one_to_path(geometry_type, geometry, &mut cursor, &mut c, &mut p);
            builder.set_current_extent(extent);
            tesselate_line2(&path, builder, z);
        }
    }
}